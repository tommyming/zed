# Quick Reference: Create Pull Request Feature

## One-Minute Summary

When you push to a remote, Zed parses git's stderr output for PR/MR creation links and displays them as clickable buttons in a toast notification.

## The Three Critical Code Blocks

### 1. Pattern Matching (remote_output.rs:130-138)

```rust
let pr_hints = [
    ("Create a pull request", "Create Pull Request"), // GitHub
    ("Create pull request", "Create Pull Request"),   // Bitbucket
    ("create a merge request", "Create Merge Request"), // GitLab
    ("View merge request", "View Merge Request"),     // GitLab
];

pr_hints
    .iter()
    .find(|(indicator, _)| output.stderr.contains(indicator))
```

**What it does**: Searches git's stderr for known PR/MR hint patterns.

### 2. URL Extraction (remote_output.rs:139-149)

```rust
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
```

**What it does**: Uses linkify to extract the first URL from the matched section.

### 3. UI Display (git_panel.rs:2954-2956)

```rust
PushPrLink { text, link } => this
    .icon(ToastIcon::new(IconName::GitBranchAlt).color(Color::Muted))
    .action(text, move |_, cx| cx.open_url(&link))
```

**What it does**: Creates a toast with an action button that opens the URL.

## Example Git Output → Result

### GitHub
```
Input (git stderr):
  remote: Create a pull request for 'feat' on GitHub by visiting:
  remote:      https://github.com/user/repo/pull/new/feat

Output (UI):
  Toast: "Pushed feat to origin"
  Button: "Create Pull Request" → Opens URL
```

### GitLab
```
Input (git stderr):
  remote: To create a merge request for feat, visit:
  remote:   https://gitlab.com/user/repo/-/merge_requests/new?...

Output (UI):
  Toast: "Pushed feat to origin"
  Button: "Create Merge Request" → Opens URL
```

## File → Function Map

| File | Key Functions | Purpose |
|------|---------------|---------|
| `crates/git_ui/src/git_panel.rs` | `push()` | Initiates push operation |
| | `show_remote_output()` | Displays toast with PR link |
| `crates/git_ui/src/remote_output.rs` | `format_output()` | Parses git output for PR links |
| `crates/project/src/git_store.rs` | `push()` | Executes git push command |
| `crates/notifications/src/status_toast.rs` | `StatusToast::new()` | Toast UI component |

## Data Flow in 5 Steps

```
1. git_panel::push()
   ↓
2. git_store::push() → executes "git push ..."
   ↓
3. Returns RemoteCommandOutput { stdout, stderr }
   ↓
4. remote_output::format_output() → parses stderr
   ↓
5. git_panel::show_remote_output() → displays toast
```

## Enum Types

```rust
// What kind of remote operation?
enum RemoteAction {
    Push(branch_name, remote),
    Pull(remote),
    Fetch(Option<remote>),
}

// How to display the result?
enum SuccessStyle {
    Toast,                           // Simple message
    ToastWithLog { output },         // With "View Log" button
    PushPrLink { text, link },      // With PR link button ← This one!
}
```

## Adding a New Provider (Example: Codeberg)

**Step 1**: Find the pattern in git push output
```bash
$ git push origin new-branch
...
remote: Create a merge proposal at:
remote:   https://codeberg.org/user/repo/compare/main...new-branch
```

**Step 2**: Add to pr_hints array
```rust
let pr_hints = [
    ("Create a pull request", "Create Pull Request"),
    ("Create pull request", "Create Pull Request"),
    ("create a merge request", "Create Merge Request"),
    ("View merge request", "View Merge Request"),
    ("Create a merge proposal", "Create Merge Proposal"), // ← Add this
];
```

**Step 3**: Add test
```rust
#[test]
fn test_push_new_branch_merge_proposal() {
    let output = RemoteCommandOutput {
        stderr: "remote: Create a merge proposal at:\nremote:   https://...".into(),
        ..
    };
    let msg = format_output(&RemoteAction::Push(...), output);
    assert!(matches!(msg.style, SuccessStyle::PushPrLink { .. }));
}
```

Done! That's all you need.

## Common Gotchas

1. **Case sensitivity matters**: "create" vs "Create"
2. **Pattern must be unique**: Don't overlap with other patterns
3. **URL extraction**: Uses linkify, so URL must be valid
4. **Stderr, not stdout**: Git puts remote messages in stderr
5. **First URL wins**: If multiple URLs, only first is used

## Testing Locally

```bash
# 1. Make changes to remote_output.rs
# 2. Run tests
cd crates/git_ui
cargo test remote_output::tests

# 3. Build Zed
cd ../..
cargo build

# 4. Test with real git push
# (Zed will parse the output and show the button)
```

## Debugging

**Problem**: Button doesn't appear after push

**Check**:
1. Is there "remote:" in stderr? → Print stderr
2. Does pattern match? → Check pr_hints
3. Is there a URL? → Use linkify directly
4. Is toast rendered? → Check show_remote_output

**Add logging**:
```rust
// In format_output()
eprintln!("stderr: {}", output.stderr);
eprintln!("found pattern: {:?}", found_pattern);
eprintln!("extracted url: {:?}", url);
```

## Related Code Patterns

**This feature (push output parsing)**:
- Parses git stderr after push
- Extracts URL from remote messages
- Shows clickable link in toast

**Different: Git hosting providers**:
- Parses commit messages for "#123"
- Builds permalinks to commits/files
- Used in git history view

Don't confuse them!

## Real-World Example

```rust
// User pushes a new branch
git_panel.push(
    force: false,
    select_remote: false,
    window, cx
)

// Git executes: git push --set-upstream origin feature-branch

// Git responds with:
// stderr = "
//   remote: Create a pull request for 'feature-branch' on GitHub by visiting:
//   remote:      https://github.com/zed-industries/zed/pull/new/feature-branch
// "

// format_output() finds pattern "Create a pull request"
// Extracts URL: "https://github.com/zed-industries/zed/pull/new/feature-branch"
// Returns: PushPrLink { text: "Create Pull Request", link: "https://..." }

// show_remote_output() creates toast:
//   ┌────────────────────────────────────────────────┐
//   │ [Icon] Pushed feature-branch to origin         │
//   │                     [Create Pull Request]      │
//   └────────────────────────────────────────────────┘

// User clicks button → Opens browser to PR creation page
```

## Performance

- **Time complexity**: O(n) where n = stderr length
- **Space complexity**: O(1) (no allocations except result)
- **Network**: None (0 API calls)
- **Impact**: Negligible (<1ms typically)

## Security

✅ Safe:
- Parses trusted git output
- linkify validates URLs
- System browser handles navigation

❌ Risks:
- Malicious git server could inject URLs
- But linkify prevents XSS
- Browser sandbox provides isolation

## Extension Ideas

1. **Auto-fill PR description** from recent commits
2. **In-app PR creation** without browser
3. **Show PR status** after creation
4. **Multiple PRs** support
5. **Custom URL templates** for internal tools

## Key Insight

This is a **passive** feature:
- Doesn't make API calls
- Doesn't modify git state  
- Doesn't create PRs itself
- Just makes the workflow smoother

It's **smart parsing** + **good UX**, not automation.

## Full Call Chain

```
User → Git Panel → Git Store → Git Binary
         ↓           ↓           ↓
       UI code    Send job    Execute cmd
                              Return output
                              ↓
                         Parse output ← Remote Output module
                              ↓
                         Create toast ← Git Panel
                              ↓
                         Display UI ← Status Toast
                              ↓
                         User clicks
                              ↓
                         Open browser
```

## Summary

**What**: Extracts PR/MR links from git push output  
**Where**: `crates/git_ui/src/remote_output.rs`  
**When**: After successful push operation  
**Why**: Reduce friction in git → PR workflow  
**How**: Pattern matching + URL extraction + clickable toast

Total code: ~50 lines (excluding tests)  
Total impact: Huge UX improvement

---

**See also**:
- CREATE_PULL_REQUEST_README.md - Start here
- CREATE_PULL_REQUEST_WORKFLOW.md - High-level overview
- CREATE_PULL_REQUEST_TECHNICAL.md - Deep dive
- CREATE_PULL_REQUEST_DIAGRAM.md - Visual diagrams
