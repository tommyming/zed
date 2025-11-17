# How "Create Pull Request" Works in Zed

This document explains how the "Create Pull Request" functionality works in the Zed codebase.

## Overview

The "Create Pull Request" feature in Zed appears when you push a branch to a remote repository. After a successful push, if the remote hosting provider (GitHub, GitLab, Bitbucket, etc.) returns a pull/merge request URL in the git output, Zed displays a clickable action to open that URL directly in the browser.

## Key Components

### 1. Remote Output Parsing (`crates/git_ui/src/remote_output.rs`)

This module is responsible for parsing the output from git remote operations (fetch, pull, push) and formatting success messages.

**Key Function**: `format_output(action: &RemoteAction, output: RemoteCommandOutput) -> SuccessMessage`

When handling a `Push` action, the code:
1. Checks if the stderr output contains remote hints about creating pull/merge requests
2. Looks for specific indicators from different hosting providers:
   - `"Create a pull request"` → GitHub
   - `"Create pull request"` → Bitbucket  
   - `"create a merge request"` → GitLab (create new)
   - `"View merge request"` → GitLab (view existing)

3. Extracts the URL using the `linkify` crate to find URLs in the output
4. Returns a `PushPrLink` style with the appropriate text and link

**Code excerpt**:
```rust
let pr_hints = [
    ("Create a pull request", "Create Pull Request"), // GitHub
    ("Create pull request", "Create Pull Request"),   // Bitbucket
    ("create a merge request", "Create Merge Request"), // GitLab
    ("View merge request", "View Merge Request"),     // GitLab
];
```

### 2. Git Panel UI (`crates/git_ui/src/git_panel.rs`)

The Git Panel displays the success toast notification after a push operation.

**Key Function**: `show_remote_output()`

After a successful push:
1. Calls `remote_output::format_output()` to parse the git output
2. Creates a `StatusToast` with the message and style
3. If the style is `PushPrLink`, it adds an action button that opens the URL

**Code excerpt**:
```rust
PushPrLink { text, link } => this
    .icon(ToastIcon::new(IconName::GitBranchAlt).color(Color::Muted))
    .action(text, move |_, cx| cx.open_url(&link))
```

### 3. Git Push Flow (`crates/git_ui/src/git_panel.rs`)

**Function**: `push(force_push: bool, select_remote: bool, window, cx)`

The push flow:
1. Validates that the repository can push (not via collab)
2. Gets the current branch
3. Determines push options:
   - If force pushing: `PushOptions::Force`
   - If no upstream or upstream is gone: `PushOptions::SetUpstream`
   - Otherwise: no special options
4. Gets the remote (either prompts user or uses default)
5. Sets up askpass delegate for authentication
6. Executes the push via `repo.push()`
7. On success, calls `show_remote_output()` with the git output

### 4. Git Hosting Providers (`crates/git_hosting_providers/`)

While these providers can extract pull request numbers from commit messages, they are **not** involved in the "Create Pull Request" button functionality. That feature relies on parsing git's stderr output.

**Key trait**: `GitHostingProvider::extract_pull_request()`
- Used to parse PR numbers from commit messages like "Fix bug (#123)"
- Returns a `PullRequest` struct with number and URL
- Different regex patterns for different providers

## Data Flow

```
User clicks "Push"
    ↓
git_panel.push()
    ↓
repo.push() [in git_store]
    ↓
Git command execution (returns RemoteCommandOutput)
    ↓
format_output() [in remote_output.rs]
    ↓
Parse stderr for PR/MR hints
    ↓
Extract URL from output
    ↓
Return SuccessMessage with PushPrLink style
    ↓
show_remote_output() [in git_panel.rs]
    ↓
Create StatusToast with action button
    ↓
User clicks button → Opens URL in browser
```

## Example Git Output

### GitHub
```
remote:
remote: Create a pull request for 'test' on GitHub by visiting:
remote:      https://example.com/test/test/pull/new/test
remote:
```

### GitLab (Create)
```
remote:
remote: To create a merge request for test, visit:
remote:   https://example.com/test/test/-/merge_requests/new?merge_request%5Bsource_branch%5D=test
remote:
```

### GitLab (View Existing)
```
remote:
remote: View merge request for test:
remote:    https://example.com/test/test/-/merge_requests/99999
remote:
```

## Key Files

1. **`crates/git_ui/src/remote_output.rs`** - Parses git output and extracts PR links
2. **`crates/git_ui/src/git_panel.rs`** - UI component that displays the PR link button
3. **`crates/project/src/git_store.rs`** - Executes git commands
4. **`crates/notifications/src/status_toast.rs`** - Toast notification UI component
5. **`crates/git_hosting_providers/src/providers/*.rs`** - Provider-specific parsing (for commit messages, not push output)

## Testing

The functionality has comprehensive tests in `crates/git_ui/src/remote_output.rs`:

- `test_push_new_branch_pull_request()` - Tests GitHub PR link extraction
- `test_push_new_branch_merge_request()` - Tests GitLab MR creation link
- `test_push_branch_existing_merge_request()` - Tests GitLab MR view link
- `test_push_new_branch_no_link()` - Tests fallback when no PR link is present

## Summary

The "Create Pull Request" feature is **not** about programmatically creating pull requests. Instead, it:

1. Parses the output from `git push` commands
2. Detects when the remote hosting provider offers a URL to create/view a PR/MR
3. Presents that URL as a clickable action in a toast notification
4. Opens the URL in the user's browser when clicked

The actual PR/MR creation happens on the hosting provider's website after the user clicks the link.
