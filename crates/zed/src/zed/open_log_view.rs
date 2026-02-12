use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use collections::VecDeque;
use fs::Fs;
use futures::StreamExt;
use gpui::{
    App, Empty, Entity, EventEmitter, FocusHandle, Focusable, ListAlignment, ListState, Task,
    Window, list, prelude::*, px,
};
use project::Project;
use ui::{
    Icon, IconButton, IconName, IconSize, Label, LabelSize, TextSize, Tooltip, WithScrollbar,
    prelude::*,
};
use workspace::{
    Item, ItemHandle, Toast, ToolbarItemEvent, ToolbarItemLocation, ToolbarItemView,
    notifications::NotificationId,
};

const MAX_LINES: usize = 1000;

pub struct OpenLogView {
    focus_handle: FocusHandle,
    lines: VecDeque<SharedString>,
    list_state: ListState,
    search_query: String,
    filtered_indices: Vec<usize>,
    _subscription: Task<()>,
}

pub enum OpenLogEvent {
    ShowToast(Toast),
}

impl EventEmitter<OpenLogEvent> for OpenLogView {}

impl OpenLogView {
    pub fn new(_project: Entity<Project>, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let fs = <dyn Fs>::global(cx);
        let list_state = ListState::new(0, ListAlignment::Bottom, px(2048.));
        let last_line_count = Arc::new(AtomicUsize::new(0));

        let subscription = cx.spawn({
            let last_line_count = last_line_count.clone();
            async move |this, cx| {
                let (old_log_result, new_log_result) =
                    futures::join!(fs.load(&paths::old_log_file()), fs.load(&paths::log_file()),);

                let update_result = this.update(cx, |this, cx| {
                    let new_log = match &new_log_result {
                        Ok(content) => Some(content.as_str()),
                        Err(err) => {
                            if old_log_result.is_err() {
                                this.show_read_error_toast(err, cx);
                                return;
                            }
                            None
                        }
                    };

                    let mut combined_lines = Vec::new();
                    if let Ok(content) = &old_log_result {
                        combined_lines.extend(content.lines().map(|line| line.to_string()));
                    }

                    let new_lines = new_log
                        .map(|content| {
                            content
                                .lines()
                                .map(|line| line.to_string())
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();

                    last_line_count.store(new_lines.len(), Ordering::SeqCst);

                    combined_lines.extend(new_lines);
                    this.set_lines(combined_lines.into_iter(), cx);
                });

                if update_result.is_err() {
                    return;
                }

                let log_file_path = paths::log_file();
                let (events, _watcher) = fs
                    .watch(&log_file_path, std::time::Duration::from_millis(100))
                    .await;
                futures::pin_mut!(events);

                while let Some(_) = events.next().await {
                    let new_content = match fs.load(&log_file_path).await {
                        Ok(content) => content,
                        Err(err) => {
                            let update_result = this.update(cx, |this, cx| {
                                this.show_read_error_toast(&err, cx);
                            });
                            if update_result.is_err() {
                                break;
                            }
                            continue;
                        }
                    };

                    let new_lines: Vec<String> =
                        new_content.lines().map(|line| line.to_string()).collect();
                    let new_line_count = new_lines.len();
                    let last_count = last_line_count.load(Ordering::SeqCst);

                    let update_result = match new_line_count.cmp(&last_count) {
                        std::cmp::Ordering::Less => this.update(cx, |this, cx| {
                            this.set_lines(new_lines.into_iter(), cx);
                        }),
                        std::cmp::Ordering::Greater => this.update(cx, |this, cx| {
                            this.append_lines(new_lines.into_iter().skip(last_count), cx);
                        }),
                        std::cmp::Ordering::Equal => Ok(()),
                    };

                    if update_result.is_err() {
                        break;
                    }

                    last_line_count.store(new_line_count, Ordering::SeqCst);
                }
            }
        });

        Self {
            focus_handle: cx.focus_handle(),
            lines: VecDeque::with_capacity(MAX_LINES),
            list_state,
            search_query: String::new(),
            filtered_indices: Vec::new(),
            _subscription: subscription,
        }
    }

    fn show_read_error_toast(&self, error: &anyhow::Error, cx: &mut Context<Self>) {
        struct OpenLogReadError;
        cx.emit(OpenLogEvent::ShowToast(Toast::new(
            NotificationId::unique::<OpenLogReadError>(),
            format!("Failed to read log: {}", error),
        )));
    }

    fn set_lines(&mut self, lines: impl Iterator<Item = String>, cx: &mut Context<Self>) {
        self.lines.clear();
        for line in lines {
            if self.lines.len() == MAX_LINES {
                self.lines.pop_front();
            }
            self.lines.push_back(line.into());
        }
        self.recompute_filtered_indices();
        cx.notify();
    }

    fn append_lines(&mut self, lines: impl Iterator<Item = String>, cx: &mut Context<Self>) {
        for line in lines {
            if self.lines.len() == MAX_LINES {
                self.lines.pop_front();
            }
            self.lines.push_back(line.into());
        }
        self.recompute_filtered_indices();
        cx.notify();
    }

    fn entry_matches_filter(&self, line: &SharedString) -> bool {
        if self.search_query.is_empty() {
            return true;
        }

        let query_lower = self.search_query.to_lowercase();
        let line_lower = line.as_ref().to_lowercase();
        line_lower.contains(&query_lower)
    }

    fn recompute_filtered_indices(&mut self) {
        let previous_count = self.filtered_indices.len();
        self.filtered_indices.clear();
        for (idx, line) in self.lines.iter().enumerate() {
            if self.entry_matches_filter(line) {
                self.filtered_indices.push(idx);
            }
        }
        let new_count = self.filtered_indices.len();
        if new_count != previous_count {
            self.list_state.reset(new_count);
        } else {
            self.list_state.remeasure();
        }
    }

    pub fn set_search_query(&mut self, query: String, cx: &mut Context<Self>) {
        self.search_query = query;
        self.recompute_filtered_indices();
        cx.notify();
    }

    fn clear_lines(&mut self, cx: &mut Context<Self>) {
        self.lines.clear();
        self.filtered_indices.clear();
        self.list_state.reset(0);
        cx.notify();
    }

    fn render_entry(
        &mut self,
        filtered_index: usize,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(&line_index) = self.filtered_indices.get(filtered_index) else {
            return Empty.into_any();
        };

        let Some(line) = self.lines.get(line_index) else {
            return Empty.into_any();
        };

        let base_size = TextSize::Editor.rems(cx);
        let colors = cx.theme().colors();

        v_flex()
            .id(filtered_index)
            .group("open-log-entry")
            .font_buffer(cx)
            .w_full()
            .py_2()
            .pl_4()
            .pr_5()
            .gap_1()
            .items_start()
            .text_size(base_size)
            .border_color(colors.border)
            .border_b_1()
            .hover(|this| this.bg(colors.element_background.opacity(0.5)))
            .child(
                Label::new(line.clone())
                    .buffer_font(cx)
                    .size(LabelSize::Small)
                    .color(Color::Default),
            )
            .into_any()
    }
}

impl Item for OpenLogView {
    type Event = OpenLogEvent;

    fn tab_content_text(&self, _detail: usize, _cx: &App) -> SharedString {
        "Log".into()
    }

    fn tab_icon(&self, _window: &Window, _cx: &App) -> Option<Icon> {
        Some(Icon::new(IconName::File))
    }
}

impl Focusable for OpenLogView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for OpenLogView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(cx.theme().colors().editor_background)
            .child(if self.filtered_indices.is_empty() {
                h_flex()
                    .size_full()
                    .justify_center()
                    .items_center()
                    .child(if self.lines.is_empty() {
                        "No log entries recorded yet"
                    } else {
                        "No log entries match the current filter"
                    })
                    .into_any()
            } else {
                div()
                    .size_full()
                    .flex_grow()
                    .child(
                        list(self.list_state.clone(), cx.processor(Self::render_entry))
                            .with_sizing_behavior(gpui::ListSizingBehavior::Auto)
                            .size_full(),
                    )
                    .vertical_scrollbar_for(&self.list_state, window, cx)
                    .into_any()
            })
    }
}

pub struct OpenLogToolbarItemView {
    log_view: Option<Entity<OpenLogView>>,
    search_editor: Entity<editor::Editor>,
}

impl OpenLogToolbarItemView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_editor = cx.new(|cx| {
            let mut editor = editor::Editor::single_line(window, cx);
            editor.set_placeholder_text("Filter log...", window, cx);
            editor
        });

        cx.subscribe(
            &search_editor,
            |this, editor, event: &editor::EditorEvent, cx| {
                if let editor::EditorEvent::BufferEdited { .. } = event {
                    let query = editor.read(cx).text(cx);
                    if let Some(log_view) = &this.log_view {
                        log_view.update(cx, |log_view, cx| {
                            log_view.set_search_query(query, cx);
                        });
                    }
                }
            },
        )
        .detach();

        Self {
            log_view: None,
            search_editor,
        }
    }
}

impl Render for OpenLogToolbarItemView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(log_view) = self.log_view.as_ref() else {
            return Empty.into_any_element();
        };

        let log_view_clone = log_view.clone();
        let has_lines = !log_view.read(cx).lines.is_empty();

        h_flex()
            .gap_2()
            .child(div().w(px(200.)).child(self.search_editor.clone()))
            .child(
                IconButton::new("clear_log", IconName::Trash)
                    .icon_size(IconSize::Small)
                    .tooltip(Tooltip::text("Clear Log"))
                    .disabled(!has_lines)
                    .on_click(cx.listener(move |_this, _, _window, cx| {
                        log_view_clone.update(cx, |log_view, cx| {
                            log_view.clear_lines(cx);
                        });
                    })),
            )
            .child(
                IconButton::new("open_raw_log_file", IconName::File)
                    .icon_size(IconSize::Small)
                    .tooltip(Tooltip::text("Open Raw Log File"))
                    .on_click(|_, _window, cx| {
                        let path = paths::log_file();
                        cx.open_url(&format!("file://{}", path.display()));
                    }),
            )
            .into_any()
    }
}

impl EventEmitter<ToolbarItemEvent> for OpenLogToolbarItemView {}

impl ToolbarItemView for OpenLogToolbarItemView {
    fn set_active_pane_item(
        &mut self,
        active_pane_item: Option<&dyn ItemHandle>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> ToolbarItemLocation {
        if let Some(item) = active_pane_item
            && let Some(log_view) = item.downcast::<OpenLogView>()
        {
            self.log_view = Some(log_view);
            cx.notify();
            return ToolbarItemLocation::PrimaryRight;
        }
        if self.log_view.take().is_some() {
            cx.notify();
        }
        ToolbarItemLocation::Hidden
    }
}
