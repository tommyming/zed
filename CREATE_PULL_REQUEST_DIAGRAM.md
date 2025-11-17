# Code Flow Diagram: Create Pull Request Feature

## File Structure

```
zed/
├── crates/
│   ├── git_ui/
│   │   └── src/
│   │       ├── git_panel.rs          [Main UI - handles push action]
│   │       └── remote_output.rs      [Parser - extracts PR links]
│   │
│   ├── git_hosting_providers/
│   │   └── src/
│   │       ├── git_hosting_providers.rs
│   │       └── providers/
│   │           ├── github.rs         [GitHub-specific logic]
│   │           ├── gitlab.rs         [GitLab-specific logic]
│   │           ├── bitbucket.rs      [Bitbucket-specific logic]
│   │           └── ...
│   │
│   ├── project/
│   │   └── src/
│   │       └── git_store.rs          [Git command execution]
│   │
│   └── notifications/
│       └── src/
│           └── status_toast.rs       [Toast UI component]
```

## Call Stack for Push Operation

```
1. User clicks "Push" button in Git Panel
   │
   │  File: git_panel.rs
   │  Function: push()
   │
   ▼

2. Determine push options
   │  - Force push?
   │  - Need to set upstream?
   │  - Regular push?
   │
   ▼

3. Get remote
   │  - Prompt user to select remote (if multiple)
   │  - Or use default remote
   │
   ▼

4. Setup authentication
   │  File: git_panel.rs
   │  Function: askpass_delegate()
   │  - Creates modal for password/token if needed
   │
   ▼

5. Execute git push
   │  File: git_store.rs
   │  Function: push()
   │  - Spawns git process
   │  - Command: "git push [options] <remote> <branch>"
   │
   ▼

6. Capture output
   │  Returns: RemoteCommandOutput
   │    - stdout: String
   │    - stderr: String (contains remote messages)
   │
   ▼

7. Parse output for PR link
   │  File: remote_output.rs
   │  Function: format_output()
   │
   │  Input: RemoteCommandOutput
   │  Output: SuccessMessage { message, style }
   │
   │  Processing:
   │  ┌─────────────────────────────────────┐
   │  │ Check stderr for "remote:" lines    │
   │  │                                     │
   │  │ Pattern match:                      │
   │  │ • "Create a pull request" → GitHub  │
   │  │ • "create a merge request" → GitLab │
   │  │ • "Create pull request" → Bitbucket │
   │  │ • "View merge request" → GitLab     │
   │  │                                     │
   │  │ Extract URL using linkify:          │
   │  │ • Find first URL in matched section │
   │  │                                     │
   │  │ Return:                             │
   │  │ • PushPrLink { text, link }         │
   │  │   OR                                │
   │  │ • ToastWithLog { output }           │
   │  └─────────────────────────────────────┘
   │
   ▼

8. Display toast notification
   │  File: git_panel.rs
   │  Function: show_remote_output()
   │
   │  Creates StatusToast with:
   │  ┌─────────────────────────────────────┐
   │  │ Icon: GitBranchAlt (muted)          │
   │  │ Message: "Pushed <branch> to ..."   │
   │  │                                     │
   │  │ Action button (conditional):        │
   │  │ • "Create Pull Request" → Opens URL │
   │  │   OR                                │
   │  │ • "View Log" → Shows full output    │
   │  │   OR                                │
   │  │ • No button (simple toast)          │
   │  └─────────────────────────────────────┘
   │
   ▼

9. User interaction
   │  User clicks "Create Pull Request" button
   │
   ▼

10. Open browser
    Function: cx.open_url(&link)
    - Opens system default browser
    - Navigates to PR creation page
```

## Detailed Component Interaction

```
┌─────────────────────────────────────────────────────────────────┐
│                          GitPanel                               │
│  (crates/git_ui/src/git_panel.rs)                              │
│                                                                 │
│  ┌──────────────┐                                              │
│  │ push()       │                                              │
│  │              │                                              │
│  │ 1. Validate  │                                              │
│  │ 2. Get opts  │──────┐                                       │
│  │ 3. Get remote│      │                                       │
│  └──────────────┘      │                                       │
│         │              │                                       │
│         │              ▼                                       │
│         │      ┌──────────────────┐                            │
│         │      │ get_remote()     │                            │
│         │      │                  │                            │
│         │      │ - Prompt if      │                            │
│         │      │   needed         │                            │
│         │      └──────────────────┘                            │
│         │                                                      │
│         ▼                                                      │
│  ┌──────────────┐                                              │
│  │ askpass_     │                                              │
│  │ delegate()   │                                              │
│  │              │                                              │
│  │ - Auth modal │                                              │
│  └──────────────┘                                              │
│         │                                                      │
└─────────┼──────────────────────────────────────────────────────┘
          │
          │ repo.push(branch, remote, options, auth, cx)
          │
          ▼
┌─────────────────────────────────────────────────────────────────┐
│                         GitStore                                │
│  (crates/project/src/git_store.rs)                             │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ push()                                                    │  │
│  │                                                           │  │
│  │ 1. Build git command:                                    │  │
│  │    "git push [--force] [--set-upstream] <remote> <ref>"  │  │
│  │                                                           │  │
│  │ 2. Execute via send_job()                                │  │
│  │                                                           │  │
│  │ 3. Capture stdout and stderr                             │  │
│  │                                                           │  │
│  │ 4. Return RemoteCommandOutput                            │  │
│  └──────────────────────────────────────────────────────────┘  │
│         │                                                       │
└─────────┼───────────────────────────────────────────────────────┘
          │
          │ Result<RemoteCommandOutput>
          │
          ▼
┌─────────────────────────────────────────────────────────────────┐
│                      RemoteOutput                               │
│  (crates/git_ui/src/remote_output.rs)                          │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ format_output(action, output)                            │  │
│  │                                                           │  │
│  │ Match action:                                            │  │
│  │   Push(branch, remote) =>                                │  │
│  │                                                           │  │
│  │     if stderr.contains("\nremote: "):                    │  │
│  │       for (hint, text) in pr_hints:                      │  │
│  │         if stderr.contains(hint):                        │  │
│  │           url = extract_url(stderr)                      │  │
│  │           return PushPrLink { text, url }                │  │
│  │                                                           │  │
│  │     else:                                                 │  │
│  │       return ToastWithLog { output }                     │  │
│  └──────────────────────────────────────────────────────────┘  │
│         │                                                       │
└─────────┼───────────────────────────────────────────────────────┘
          │
          │ SuccessMessage { message, style }
          │
          ▼
┌─────────────────────────────────────────────────────────────────┐
│                          GitPanel                               │
│  (crates/git_ui/src/git_panel.rs)                              │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │ show_remote_output(action, output, cx)                   │  │
│  │                                                           │  │
│  │ let { message, style } = format_output(action, output)   │  │
│  │                                                           │  │
│  │ StatusToast::new(message, cx, |toast, cx| {              │  │
│  │   match style {                                          │  │
│  │     PushPrLink { text, link } =>                         │  │
│  │       toast.action(text, |_, cx| cx.open_url(&link))     │  │
│  │     ...                                                   │  │
│  │   }                                                       │  │
│  │ })                                                        │  │
│  └──────────────────────────────────────────────────────────┘  │
│         │                                                       │
└─────────┼───────────────────────────────────────────────────────┘
          │
          │ Display toast
          │
          ▼
┌─────────────────────────────────────────────────────────────────┐
│                       StatusToast                               │
│  (crates/notifications/src/status_toast.rs)                    │
│                                                                 │
│  ┌───────────────────────────────────────────────────────┐     │
│  │  ┌─────────────────────────────────────────────────┐  │     │
│  │  │  [Icon] Pushed my-branch to origin             │  │     │
│  │  │                                                 │  │     │
│  │  │               [Create Pull Request] ←───────────┼──┼─────┤
│  │  └─────────────────────────────────────────────────┘  │     │
│  └───────────────────────────────────────────────────────┘     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                            │
                            │ User clicks button
                            │
                            ▼
                    cx.open_url(&link)
                            │
                            ▼
                   Opens in browser →  https://github.com/...
```

## Data Structures

```rust
// Input to format_output()
RemoteCommandOutput {
    stdout: String,  // Usually empty for push
    stderr: String,  // Contains remote messages
}

// Example stderr for GitHub:
// "remote: \n\
//  remote: Create a pull request for 'feature' on GitHub by visiting:\n\
//  remote:      https://github.com/user/repo/pull/new/feature\n\
//  remote: \n"

// ↓ format_output() ↓

// Output from format_output()
SuccessMessage {
    message: String,        // "Pushed feature to origin"
    style: SuccessStyle,
}

// Where SuccessStyle is one of:
enum SuccessStyle {
    Toast,                                        // Simple message
    ToastWithLog { output: RemoteCommandOutput }, // With log viewer
    PushPrLink { text: String, link: String },   // With PR link button
}

// Example PushPrLink:
PushPrLink {
    text: "Create Pull Request",
    link: "https://github.com/user/repo/pull/new/feature"
}
```

## Pattern Matching Details

```
Git stderr output analysis:
──────────────────────────────────────────────────────

GitHub:
  Pattern: "Create a pull request"
  Example: "remote: Create a pull request for 'feat' on GitHub by visiting:"
  Button:  "Create Pull Request"
  
GitLab (New):
  Pattern: "create a merge request"  (lowercase!)
  Example: "remote: To create a merge request for feat, visit:"
  Button:  "Create Merge Request"

GitLab (Existing):
  Pattern: "View merge request"
  Example: "remote: View merge request for feat:"
  Button:  "View Merge Request"

Bitbucket:
  Pattern: "Create pull request"
  Example: "remote: Create pull request for feat:"
  Button:  "Create Pull Request"

URL Extraction:
  1. Find pattern in stderr
  2. Use linkify::LinkFinder to extract URLs
  3. Take first URL with LinkKind::Url
  4. Return as part of PushPrLink
```

## Key Insights

1. **No API calls**: The feature doesn't call any APIs. It simply parses git's output.

2. **Provider agnostic**: Works with any git hosting provider that outputs PR URLs during push.

3. **Graceful degradation**: If no PR hint is found, falls back to showing the full log.

4. **Single responsibility**: Each component has a clear role:
   - `git_store`: Execute git commands
   - `remote_output`: Parse output
   - `git_panel`: Display UI
   - `status_toast`: Render toast

5. **Extensible**: Adding new providers requires only updating the `pr_hints` array.
