# Create Pull Request Feature - Documentation Index

This directory contains comprehensive documentation on how the "Create Pull Request" feature works in Zed.

## Quick Start

**What is this feature?**
When you push a branch to a remote repository (GitHub, GitLab, Bitbucket, etc.), Zed automatically detects if the hosting provider offers a URL to create or view a pull/merge request. If found, a clickable button appears in a toast notification that opens the URL in your browser.

## Documentation Files

### 1. [CREATE_PULL_REQUEST_WORKFLOW.md](CREATE_PULL_REQUEST_WORKFLOW.md)
**Best for**: Product managers, designers, and developers wanting a high-level understanding

**Contents**:
- Feature overview and purpose
- Key components and their roles
- Data flow diagram
- Example git output from different providers
- List of key files
- Testing information
- Summary

**Start here if**: You want to understand WHAT the feature does and WHY.

### 2. [CREATE_PULL_REQUEST_TECHNICAL.md](CREATE_PULL_REQUEST_TECHNICAL.md)
**Best for**: Developers who need to modify or extend the feature

**Contents**:
- Architecture overview
- Detailed code analysis with examples
- Component implementation details
- State machine diagram
- Dependencies
- Extension points (how to add new providers)
- Security considerations
- Performance analysis
- Future enhancement ideas

**Start here if**: You need to understand HOW the feature is implemented in detail.

### 3. [CREATE_PULL_REQUEST_DIAGRAM.md](CREATE_PULL_REQUEST_DIAGRAM.md)
**Best for**: Developers debugging issues or tracing code execution

**Contents**:
- File structure
- Complete call stack with line numbers
- Detailed component interaction diagrams
- Data structure definitions
- Pattern matching details
- Key implementation insights

**Start here if**: You need to trace the execution flow or debug an issue.

## Key Takeaways

1. **Simple but Effective**: The feature doesn't create PRs programmatically. It parses git's output and provides a convenient link.

2. **Provider Agnostic**: Works with any git hosting provider that outputs PR URLs (GitHub, GitLab, Bitbucket, etc.).

3. **Three Main Files**:
   - `crates/git_ui/src/git_panel.rs` - UI and push logic
   - `crates/git_ui/src/remote_output.rs` - Output parsing and link extraction
   - `crates/project/src/git_store.rs` - Git command execution

4. **Easy to Extend**: Adding support for a new provider typically requires just adding one line to the `pr_hints` array.

## Common Questions

**Q: Does Zed create the pull request automatically?**
A: No. Zed extracts the PR creation URL from git's output and provides a button to open it in your browser. The actual PR creation happens on the hosting provider's website.

**Q: Which git hosting providers are supported?**
A: GitHub, GitLab, Bitbucket, and any provider that outputs PR/MR URLs in a similar format.

**Q: What if my provider isn't recognized?**
A: The feature gracefully falls back to showing the full git output via a "View Log" button.

**Q: Can users configure this behavior?**
A: Currently, the feature is always enabled when applicable. There's no setting to disable it.

**Q: Does this work with self-hosted instances?**
A: Yes! It works with self-hosted GitHub, GitLab, Bitbucket, etc., as long as they output the standard PR URL format.

## Code Navigation

Quick links to the most important functions:

1. **Entry Point**: `git_panel.rs::push()` - Line 2267
2. **Parser**: `remote_output.rs::format_output()` - Line 35
3. **Display**: `git_panel.rs::show_remote_output()` - Line 2920
4. **Git Execution**: `git_store.rs::push()` - Search for "send_job"

## Testing

Run the existing tests:
```bash
cd crates/git_ui
cargo test remote_output::tests
```

Tests cover:
- GitHub PR link extraction
- GitLab MR creation link
- GitLab MR view link  
- Fallback when no PR link present

## Related Features

This feature is separate from but related to:

1. **Git Hosting Providers** (`crates/git_hosting_providers/`): 
   - Extracts PR numbers from commit messages (#123)
   - Builds permalinks to commits and files
   - Not used for the "Create PR" button

2. **Status Toast** (`crates/notifications/src/status_toast.rs`):
   - Generic notification component
   - Used to display the PR link button

## Contributing

To add support for a new git hosting provider:

1. Identify the pattern in the git push output
2. Add it to `pr_hints` in `remote_output.rs`
3. Add a test case
4. Update this documentation

Example:
```rust
let pr_hints = [
    // ... existing hints ...
    ("start code review", "Start Code Review"), // Your new provider
];
```

## Debugging Tips

1. **Enable git output logging**: Set `RUST_LOG=git=debug`
2. **Check stderr parsing**: Add println! in `format_output()`
3. **Test URL extraction**: Use the linkify crate directly
4. **Verify toast display**: Check StatusToast rendering

## Architecture Principles

This feature follows these principles:

1. **Parse, don't create**: Extract information from git, don't make API calls
2. **Fail gracefully**: Always provide a fallback (View Log button)
3. **Single responsibility**: Each component does one thing well
4. **Extensible**: Easy to add new providers
5. **Tested**: Comprehensive test coverage

## Performance Impact

- Negligible: Only parses text on successful push
- No network requests
- No additional git commands
- O(n) where n = length of git stderr output

## Security

- URLs are extracted from git's output (trusted source)
- linkify crate validates URLs
- System browser handles URL opening
- No credentials sent with URLs

## License

This documentation follows the same license as the Zed project.

## Questions or Issues?

If you have questions about this feature:
1. Check the three documentation files above
2. Look at the test cases in `remote_output.rs`
3. Review the code with the help of these docs
4. Open an issue on the Zed repository

---

**Last Updated**: 2025-11-17
**Covers**: Zed codebase as of commit cd48f95
