# Contributing to OnPen

Thank you for helping make screen annotation useful during real conversations.

## Report a problem

Include the app version, OS version, monitor arrangement, scaling/DPI, input device, steps to reproduce, expected behavior, and actual behavior. For screen-sharing problems, include the meeting app version, share mode (monitor/window/tab), and what a separate participant sees.

Use synthetic slides or redact private material. Do not upload meeting recordings, tokens, or confidential company documents. Reproduction steps are more useful than a large unfiltered log.

## Make a change

1. Follow the build instructions in README.md.
2. Keep a change focused and explain the user-visible behavior.
3. Run `npm test` and `cargo test --locked --manifest-path src-tauri/Cargo.toml --features custom-protocol`.
4. For coordinate/input changes, check zoom, mixed DPI, erasing, selection, and export. For native behavior, record the platform actually tested.
5. Document untested cases instead of claiming cross-platform support from a successful compile.

Project-authored contributions use the MIT license. Keep upstream notices with third-party code. Open an issue or pull request in the hosting repository once one is published; no public repository URL is configured yet.
