# Session Context

**Session ID:** 4c00fd88-9319-4fc1-b337-b69b497cc255

**Commit Message:** Remove the intel release for macOS. I got this error message in the CD:

## Prompt

Remove the intel release for macOS. I got this error message in the CD: The configuration 'macos-13-us-default' is not supported

## Summary

Removed the macOS Intel (`x86_64-apple-darwin` / `macos-13`) target from the build matrix. The workflow now builds 3 targets: macOS ARM, Linux, and Windows.

## Key Actions

- - **Edit**: /Users/denny/Documents/repos/leeson/.github/workflows/build.yml
