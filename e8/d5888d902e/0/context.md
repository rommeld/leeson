# Session Context

**Session ID:** 35d71787-cb22-4fc8-a2f7-b8ce61409f43

**Commit Message:** During CI this error occured: Run COMMIT_MSG="Disable the logfire live w

## Prompt

during CI this error occured: Run COMMIT_MSG="Disable the logfire live when I close the TUI with `q` and start a clean
REDACTED.sh: line 3: q: command not found
Error: Process completed with exit code 127.

## Summary

The fix is clean. Here's what was wrong and what changed:

**Root cause:** The workflow used `${{ steps.get-commit.outputs.message }}` directly inside `run:` scripts, which means GitHub Actions expands the expression *before* the shell executes. When the commit message contained backticks (`` `q` ``), the shell treated them as command substitution and tried to run `q` as a command.

**Fix:** Moved the commit message (and other step outputs) from inline `${{ }}` interpolation to `env:` blocks. When values are passed via `env:`, GitHub Actions sets them as environment variables without shell interpretation, so backticks, quotes, and other special characters are handled safely.

This is also a security best practice — inline `${{ }}` in `run:` blocks is a known script injection vector (GitHub's [security hardening guide](https://docs.github.com/en/actions/security-guides/security-hardening-for-github-actions#understanding-the-risk-of-script-injections) recommends using environment variables instead).

## Key Actions

- - **Glob**: .github/workflows/*.yml
- - **Glob**: .github/workflows/*.yaml
- - **Grep**: COMMIT_MSG
- - **Read**: /Users/denny/Documents/repos/leeson/.github/workflows/changelog.yml
- - **Edit**: /Users/denny/Documents/repos/leeson/.github/workflows/changelog.yml
- - **Edit**: /Users/denny/Documents/repos/leeson/.github/workflows/changelog.yml
- - **Edit**: /Users/denny/Documents/repos/leeson/.github/workflows/changelog.yml
- - **Read**: /Users/denny/Documents/repos/leeson/.github/workflows/changelog.yml
- - **Edit**: /Users/denny/Documents/repos/leeson/.github/workflows/changelog.yml
- - **Read**: /Users/denny/Documents/repos/leeson/.github/workflows/changelog.yml
