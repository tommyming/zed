# Technical Deep Dive: Create Pull Request Implementation

This document provides a detailed technical analysis of how the "Create Pull Request" feature is implemented in Zed.

## Architecture Overview

The feature uses a pipeline architecture:
1. **Execution Layer** - Git command execution
2. **Parsing Layer** - Output parsing and link extraction  
3. **Presentation Layer** - UI toast with action button

## Component Details

### 1. RemoteAction Enum

Defines the types of remote operations:

```rust
#[derive(Clone)]
pub enum RemoteAction {
    Fetch(Option<Remote>),
    Pull(Remote),
    Push(SharedString, Remote),
}
```

### 2. SuccessStyle Enum

Defines how to display the success message:

```rust
pub enum SuccessStyle {
    Toast,                                         // Simple toast
    ToastWithLog { output: RemoteCommandOutput },  // Toast with "View Log" button
    PushPrLink { text: String, link: String },     // Toast with PR link button
}
```

### 3. Remote Output Parsing Logic

**Location**: `crates/git_ui/src/remote_output.rs:120-158`

```rust
RemoteAction::Push(branch_name, remote_ref) => {
    let message = if output.stderr.ends_with("Everything up-to-date\n") {
        "Push: Everything is up-to-date".to_string()
    } else {
        format!("Pushed {} to {}", branch_name, remote_ref.name)
    };

    let style = if output.stderr.ends_with("Everything up-to-date\n") {
        Some(SuccessStyle::Toast)
    } else if output.stderr.contains("\nremote: ") {
        // Look for PR/MR hints in the remote output
        let pr_hints = [
            ("Create a pull request", "Create Pull Request"), // GitHub
            ("Create pull request", "Create Pull Request"),   // Bitbucket
            ("create a merge request", "Create Merge Request"), // GitLab
            ("View merge request", "View Merge Request"),     // GitLab
        ];
        
        pr_hints
            .iter()
            .find(|(indicator, _)| output.stderr.contains(indicator))
            .and_then(|(_, mapped)| {
                let finder = LinkFinder::new();
                finder
                    .links(&output.stderr)
                    .filter(|link| *link.kind() == LinkKind::Url)
                    .map(|link| link.start()..link.end())
                    .next()
                    .map(|link| SuccessStyle::PushPrLink {
                        text: mapped.to_string(),
                        link: output.stderr[link].to_string(),
                    })
            })
    } else {
        None
    };
    
    SuccessMessage {
        message,
        style: style.unwrap_or(SuccessStyle::ToastWithLog { output }),
    }
}
```

**Key points**:
- Uses the `linkify` crate to robustly extract URLs from text
- Matches against known patterns from different hosting providers
- Falls back to `ToastWithLog` if no PR link is found
- The link is always the first URL found in the stderr output

### 4. UI Display Logic

**Location**: `crates/git_ui/src/git_panel.rs:2933-2961`

```rust
fn show_remote_output(
    &self,
    action: RemoteAction,
    info: RemoteCommandOutput,
    cx: &mut App,
) {
    let Some(workspace) = self.workspace.upgrade() else {
        return;
    };

    workspace.update(cx, |workspace, cx| {
        let SuccessMessage { message, style } = remote_output::format_output(&action, info);
        let workspace_weak = cx.weak_entity();
        let operation = action.name();

        let status_toast = StatusToast::new(message, cx, move |this, _cx| {
            use remote_output::SuccessStyle::*;
            match style {
                Toast => this.icon(ToastIcon::new(IconName::GitBranchAlt).color(Color::Muted)),
                ToastWithLog { output } => this
                    .icon(ToastIcon::new(IconName::GitBranchAlt).color(Color::Muted))
                    .action("View Log", move |window, cx| {
                        // Opens modal with full git output
                    }),
                PushPrLink { text, link } => this
                    .icon(ToastIcon::new(IconName::GitBranchAlt).color(Color::Muted))
                    .action(text, move |_, cx| cx.open_url(&link)),
            }
        });
        workspace.toggle_status_toast(status_toast, cx)
    });
}
```

**Key points**:
- Uses pattern matching on `SuccessStyle` to determine button behavior
- For `PushPrLink`, creates an action that calls `cx.open_url(&link)`
- The button text is dynamically set based on the hosting provider

### 5. Git Push Implementation

**Location**: `crates/git_ui/src/git_panel.rs:2267-2342`

```rust
pub(crate) fn push(
    &mut self,
    force_push: bool,
    select_remote: bool,
    window: &mut Window,
    cx: &mut Context<Self>,
) {
    // Validation
    if !self.can_push_and_pull(cx) {
        return;
    }
    let Some(repo) = self.active_repository.clone() else {
        return;
    };
    let Some(branch) = repo.read(cx).branch.as_ref() else {
        return;
    };
    
    telemetry::event!("Git Pushed");
    let branch = branch.clone();

    // Determine push options
    let options = if force_push {
        Some(PushOptions::Force)
    } else {
        match branch.upstream {
            Some(Upstream {
                tracking: UpstreamTracking::Gone,
                ..
            })
            | None => Some(PushOptions::SetUpstream),
            _ => None,
        }
    };
    
    let remote = self.get_remote(select_remote, window, cx);

    cx.spawn_in(window, async move |this, cx| {
        let remote = match remote.await {
            Ok(Some(remote)) => remote,
            Ok(None) => return Ok(()),
            Err(e) => {
                log::error!("Failed to get current remote: {}", e);
                this.update(cx, |this, cx| this.show_error_toast("push", e, cx))
                    .ok();
                return Ok(());
            }
        };

        // Setup authentication delegate
        let askpass_delegate = this.update_in(cx, |this, window, cx| {
            this.askpass_delegate(format!("git push {}", remote.name), window, cx)
        })?;

        // Execute push
        let push = repo.update(cx, |repo, cx| {
            repo.push(
                branch.name().to_owned().into(),
                remote.name.clone(),
                options,
                askpass_delegate,
                cx,
            )
        })?;

        let remote_output = push.await?;

        // Handle result
        let action = RemoteAction::Push(branch.name().to_owned().into(), remote);
        this.update(cx, |this, cx| match remote_output {
            Ok(remote_message) => this.show_remote_output(action, remote_message, cx),
            Err(e) => {
                log::error!("Error while pushing {:?}", e);
                this.show_error_toast(action.name(), e, cx)
            }
        })?;

        anyhow::Ok(())
    })
    .detach_and_log_err(cx);
}
```

**Key points**:
- Async operation using `cx.spawn_in()`
- Handles authentication via `askpass_delegate`
- Push options are determined based on upstream tracking status
- Errors are displayed via `show_error_toast()`
- Success triggers `show_remote_output()` which may show the PR link

### 6. Git Hosting Provider Trait (Not Used for Push)

**Location**: `crates/git_hosting_providers/src/providers/github.rs:227-237`

This is a separate feature for extracting PR references from commit messages:

```rust
fn extract_pull_request(&self, remote: &ParsedGitRemote, message: &str) -> Option<PullRequest> {
    let line = message.lines().next()?;
    let capture = pull_request_number_regex().captures(line)?;
    let number = capture.get(1)?.as_str().parse::<u32>().ok()?;

    let mut url = self.base_url();
    let path = format!("/{}/{}/pull/{}", remote.owner, remote.repo, number);
    url.set_path(&path);

    Some(PullRequest { number, url })
}
```

**Note**: This is **not** used for the "Create Pull Request" button. It's used to parse PR numbers like "#123" from commit messages in the git history view.

## State Machine

```
┌─────────────────┐
│  User Action:   │
│  Click Push     │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Validate State  │
│ - Has repo?     │
│ - Has branch?   │
│ - Can push?     │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Get Remote      │
│ - Select or use │
│   default       │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Execute Push    │
│ - Setup auth    │
│ - Call git push │
└────────┬────────┘
         │
         ├─────────────┐
         │             │
         ▼             ▼
    ┌────────┐   ┌─────────┐
    │Success │   │ Error   │
    └────┬───┘   └────┬────┘
         │            │
         ▼            ▼
┌────────────────┐  ┌──────────────┐
│ Parse Output   │  │ Show Error   │
│ - Check stderr │  │ Toast        │
│ - Find PR hint │  └──────────────┘
└────────┬───────┘
         │
         ├────────────┬────────────┐
         │            │            │
         ▼            ▼            ▼
    ┌────────┐  ┌──────────┐ ┌──────────┐
    │ Toast  │  │ Toast +  │ │ Toast +  │
    │        │  │ View Log │ │ PR Link  │
    └────────┘  └──────────┘ └─────┬────┘
                                    │
                            ┌───────▼────────┐
                            │ User clicks    │
                            │ Opens browser  │
                            └────────────────┘
```

## Dependencies

- **linkify**: URL extraction from text
- **regex**: Pattern matching for hosting provider detection
- **anyhow**: Error handling
- **gpui**: UI framework
- **url**: URL parsing and manipulation

## Extension Points

To add support for a new hosting provider:

1. Add the provider's PR hint pattern to the `pr_hints` array in `remote_output.rs`
2. Add a test case in the `tests` module
3. Optionally add a provider implementation in `git_hosting_providers/src/providers/`

Example for a hypothetical "CodeForge" provider:

```rust
let pr_hints = [
    ("Create a pull request", "Create Pull Request"),     // GitHub
    ("Create pull request", "Create Pull Request"),       // Bitbucket
    ("create a merge request", "Create Merge Request"),   // GitLab
    ("View merge request", "View Merge Request"),         // GitLab
    ("Start a code review", "Start Code Review"),         // CodeForge - NEW
];
```

## Security Considerations

- URLs are extracted from git's stderr, which could theoretically be manipulated by a malicious git server
- The `linkify` crate provides robust URL validation
- URLs are opened using `cx.open_url()` which should use the system's default browser
- No credentials or sensitive data are sent with the URL

## Performance

- URL extraction is O(n) where n is the length of stderr
- Pattern matching happens only on push operations
- No network requests are made by this code
- The feature has minimal performance impact

## Future Enhancements

Potential improvements:

1. **Smart PR Creation**: Pre-fill PR title/description based on recent commits
2. **In-app PR Creation**: Create PRs without leaving Zed
3. **PR Status Display**: Show PR status in the git panel
4. **Multiple PRs**: Handle multiple PRs for the same branch
5. **Custom Providers**: Allow users to define custom URL patterns

## Conclusion

The "Create Pull Request" feature is a thoughtful UX enhancement that:
- Reduces friction in the git push → PR creation workflow
- Supports multiple hosting providers
- Has a clean, testable architecture
- Degrades gracefully when PR links aren't available
