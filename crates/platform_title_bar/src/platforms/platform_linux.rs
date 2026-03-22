use gpui::{Action, Hsla, MouseButton, WindowControls, prelude::*, svg};
use ui::prelude::*;

// Linux-only parsed representation of window controls from the system
// `button-layout` setting. This keeps parsing and rendering
// simple by using known control types instead of raw strings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinuxWindowControl {
    Close,
    Minimize,
    Maximize,
}

impl LinuxWindowControl {
    // Parses recognized control tokens from the Linux `button-layout` value.
    // Unknown tokens like `icon` or `appmenu`
    // are ignored on purpose.
    #[cfg(any(target_os = "linux", test))]
    fn from_layout_token(token: &str) -> Option<Self> {
        match token.trim() {
            "close" => Some(Self::Close),
            "minimize" => Some(Self::Minimize),
            "maximize" => Some(Self::Maximize),
            _ => None,
        }
    }

    // Generates stable per-control element ids so left/right rendered controls
    // can coexist without reusing the same id.
    fn element_id(self, index: usize) -> SharedString {
        match self {
            Self::Close => format!("close-{index}").into(),
            Self::Minimize => format!("minimize-{index}").into(),
            Self::Maximize => format!("maximize-or-restore-{index}").into(),
        }
    }

    // Maps parsed Linux controls to the existing window control rendering
    // types. `Maximize` becomes `Restore` when the window is already
    // maximized so current behavior is preserved.
    fn window_control_type(self, window: &Window) -> WindowControlType {
        match self {
            Self::Close => WindowControlType::Close,
            Self::Minimize => WindowControlType::Minimize,
            Self::Maximize => {
                if window.is_maximized() {
                    WindowControlType::Restore
                } else {
                    WindowControlType::Maximize
                }
            }
        }
    }

    pub fn is_supported(self, supported_controls: WindowControls) -> bool {
        match self {
            Self::Close => true,
            Self::Minimize => supported_controls.minimize,
            Self::Maximize => supported_controls.maximize,
        }
    }
}

// Minimal parsed layout for Linux window controls. The system setting is
// represented as left and right control groups, so rendering can
// place controls on either side without extra layout heuristics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinuxWindowControlsLayout {
    pub left: Vec<LinuxWindowControl>,
    pub right: Vec<LinuxWindowControl>,
}

impl LinuxWindowControlsLayout {
    // Parses the full Linux `button-layout` string by splitting it into
    // left/right sides around `:` and then parsing recognized tokens
    // from each side independently.
    #[cfg(any(target_os = "linux", test))]
    pub fn parse(value: &str) -> Option<Self> {
        let (left, right) = value.split_once(':')?;

        Some(Self {
            left: Self::parse_side(left),
            right: Self::parse_side(right),
        })
    }

    // Parses one side of the layout and preserves the order of recognized
    // controls while dropping unsupported tokens.
    #[cfg(any(target_os = "linux", test))]
    fn parse_side(side: &str) -> Vec<LinuxWindowControl> {
        side.split(',')
            .filter_map(LinuxWindowControl::from_layout_token)
            .collect()
    }

    #[cfg(any(target_os = "linux", test))]
    pub fn parse_or_default(value: &str) -> Self {
        Self::parse(value).unwrap_or_default()
    }

    pub fn filter_supported(self, supported_controls: WindowControls) -> Self {
        Self {
            left: self
                .left
                .into_iter()
                .filter(|control| control.is_supported(supported_controls))
                .collect(),
            right: self
                .right
                .into_iter()
                .filter(|control| control.is_supported(supported_controls))
                .collect(),
        }
    }
}

impl Default for LinuxWindowControlsLayout {
    // Default matches the current Linux behavior in Zed: controls on the
    // right in minimize/maximize/close order.
    fn default() -> Self {
        Self {
            left: Vec::new(),
            right: vec![
                LinuxWindowControl::Minimize,
                LinuxWindowControl::Maximize,
                LinuxWindowControl::Close,
            ],
        }
    }
}

// Renders a supplied list of Linux controls instead of using a single
// hardcoded button order. This allows title bar code to render controls
// on the left, right, or both sides based on the parsed system layout.
#[derive(IntoElement)]
pub struct LinuxWindowControls {
    id: ElementId,
    controls: Vec<LinuxWindowControl>,
    close_window_action: Box<dyn Action>,
}

impl LinuxWindowControls {
    // `controls` comes from the parsed Linux layout for one side of the title
    // bar.
    pub fn new(
        id: impl Into<ElementId>,
        controls: Vec<LinuxWindowControl>,
        close_window_action: Box<dyn Action>,
    ) -> Self {
        Self {
            id: id.into(),
            controls,
            close_window_action,
        }
    }
}

impl RenderOnce for LinuxWindowControls {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Build controls from the parsed list in order so the rendered
        // sequence matches the desktop setting.
        self.controls.into_iter().enumerate().fold(
            h_flex()
                .id(self.id)
                .px_3()
                .gap_3()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation()),
            |controls, (index, control)| {
                let window_control_type = control.window_control_type(window);
                let element_id = control.element_id(index);

                match window_control_type {
                    WindowControlType::Close => controls.child(WindowControl::new_close(
                        element_id,
                        window_control_type,
                        self.close_window_action.boxed_clone(),
                        cx,
                    )),
                    _ => controls.child(WindowControl::new(element_id, window_control_type, cx)),
                }
            },
        )
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub enum WindowControlType {
    Minimize,
    Restore,
    Maximize,
    Close,
}

impl WindowControlType {
    pub fn icon(&self) -> IconName {
        match self {
            WindowControlType::Minimize => IconName::GenericMinimize,
            WindowControlType::Restore => IconName::GenericRestore,
            WindowControlType::Maximize => IconName::GenericMaximize,
            WindowControlType::Close => IconName::GenericClose,
        }
    }
}

#[allow(unused)]
pub struct WindowControlStyle {
    background: Hsla,
    background_hover: Hsla,
    icon: Hsla,
    icon_hover: Hsla,
}

impl WindowControlStyle {
    pub fn default(cx: &mut App) -> Self {
        let colors = cx.theme().colors();

        Self {
            background: colors.ghost_element_background,
            background_hover: colors.ghost_element_hover,
            icon: colors.icon,
            icon_hover: colors.icon_muted,
        }
    }

    #[allow(unused)]
    pub fn background(mut self, color: impl Into<Hsla>) -> Self {
        self.background = color.into();
        self
    }

    #[allow(unused)]
    pub fn background_hover(mut self, color: impl Into<Hsla>) -> Self {
        self.background_hover = color.into();
        self
    }

    #[allow(unused)]
    pub fn icon(mut self, color: impl Into<Hsla>) -> Self {
        self.icon = color.into();
        self
    }

    #[allow(unused)]
    pub fn icon_hover(mut self, color: impl Into<Hsla>) -> Self {
        self.icon_hover = color.into();
        self
    }
}

#[derive(IntoElement)]
pub struct WindowControl {
    id: ElementId,
    icon: WindowControlType,
    style: WindowControlStyle,
    close_action: Option<Box<dyn Action>>,
}

impl WindowControl {
    pub fn new(id: impl Into<ElementId>, icon: WindowControlType, cx: &mut App) -> Self {
        let style = WindowControlStyle::default(cx);

        Self {
            id: id.into(),
            icon,
            style,
            close_action: None,
        }
    }

    pub fn new_close(
        id: impl Into<ElementId>,
        icon: WindowControlType,
        close_action: Box<dyn Action>,
        cx: &mut App,
    ) -> Self {
        let style = WindowControlStyle::default(cx);

        Self {
            id: id.into(),
            icon,
            style,
            close_action: Some(close_action.boxed_clone()),
        }
    }

    #[allow(unused)]
    pub fn custom_style(
        id: impl Into<ElementId>,
        icon: WindowControlType,
        style: WindowControlStyle,
    ) -> Self {
        Self {
            id: id.into(),
            icon,
            style,
            close_action: None,
        }
    }
}

impl RenderOnce for WindowControl {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let icon = svg()
            .size_4()
            .flex_none()
            .path(self.icon.icon().path())
            .text_color(self.style.icon)
            .group_hover("", |this| this.text_color(self.style.icon_hover));

        h_flex()
            .id(self.id)
            .group("")
            .cursor_pointer()
            .justify_center()
            .content_center()
            .rounded_2xl()
            .w_5()
            .h_5()
            .hover(|this| this.bg(self.style.background_hover))
            .active(|this| this.bg(self.style.background_hover))
            .child(icon)
            .on_mouse_move(|_, _, cx| cx.stop_propagation())
            .on_click(move |_, window, cx| {
                cx.stop_propagation();
                match self.icon {
                    WindowControlType::Minimize => window.minimize_window(),
                    WindowControlType::Restore => window.zoom_window(),
                    WindowControlType::Maximize => window.zoom_window(),
                    WindowControlType::Close => window.dispatch_action(
                        self.close_action
                            .as_ref()
                            .expect("Use WindowControl::new_close() for close control.")
                            .boxed_clone(),
                        cx,
                    ),
                }
            })
    }
}

// Parser-focused tests. These validate the Linux layout parsing behavior
// independently from portal integration or title bar rendering.
#[cfg(test)]
mod tests {
    use super::{LinuxWindowControl, LinuxWindowControlsLayout};
    use gpui::WindowControls;

    #[test]
    fn parses_buttons_on_left_and_right() {
        let layout = LinuxWindowControlsLayout::parse("close,minimize:maximize").unwrap();

        assert_eq!(
            layout,
            LinuxWindowControlsLayout {
                left: vec![LinuxWindowControl::Close, LinuxWindowControl::Minimize],
                right: vec![LinuxWindowControl::Maximize],
            }
        );
    }

    #[test]
    fn ignores_unknown_tokens() {
        let layout =
            LinuxWindowControlsLayout::parse("icon,close:appmenu,minimize,unknown").unwrap();

        assert_eq!(
            layout,
            LinuxWindowControlsLayout {
                left: vec![LinuxWindowControl::Close],
                right: vec![LinuxWindowControl::Minimize],
            }
        );
    }

    #[test]
    fn preserves_empty_layout() {
        let layout = LinuxWindowControlsLayout::parse(":").unwrap();

        assert_eq!(
            layout,
            LinuxWindowControlsLayout {
                left: Vec::new(),
                right: Vec::new(),
            }
        );
    }

    #[test]
    fn falls_back_for_missing_separator() {
        let layout = LinuxWindowControlsLayout::parse("close,minimize,maximize");

        assert_eq!(layout, None);
    }

    #[test]
    fn preserves_empty_recognized_layout() {
        let layout = LinuxWindowControlsLayout::parse("icon:appmenu").unwrap();

        assert_eq!(
            layout,
            LinuxWindowControlsLayout {
                left: Vec::new(),
                right: Vec::new(),
            }
        );
    }

    #[test]
    fn falls_back_to_default_for_invalid_layout() {
        let layout = LinuxWindowControlsLayout::parse_or_default("close,minimize,maximize");

        assert_eq!(layout, LinuxWindowControlsLayout::default());
    }

    #[test]
    fn filters_layout_by_platform_support() {
        let supported_controls = WindowControls {
            fullscreen: true,
            maximize: false,
            minimize: true,
            window_menu: true,
        };
        let layout = LinuxWindowControlsLayout {
            left: vec![LinuxWindowControl::Close, LinuxWindowControl::Maximize],
            right: vec![LinuxWindowControl::Minimize],
        }
        .filter_supported(supported_controls);

        assert_eq!(
            layout,
            LinuxWindowControlsLayout {
                left: vec![LinuxWindowControl::Close],
                right: vec![LinuxWindowControl::Minimize],
            }
        );
    }
}
