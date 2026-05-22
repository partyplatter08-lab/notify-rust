# notify-rust 4.18 / 5.0 — Progress Tracker

Companion to [`notify-rust-roadmap.md`](./notify-rust-roadmap.md). The
roadmap describes intent. This file tracks state.

Status legend: ☐ todo · 🛠 in progress · ✅ done · ⛔ blocked · ❎ dropped

---

## 4.18 — final 4.x release

### Pre-flight (decisions to lock in)

| ID  | Item                                                               | Status | Owner | Notes |
|-----|--------------------------------------------------------------------|:------:|-------|-------|
| Q1  | Lock preview-flag names (`pure_usernotifications`, `win32`)        |   ☐    |       | see roadmap Q1 |
| Q2  | Decide on `experimental` umbrella feature                          |   ☐    |       | see roadmap Q2 |
| Q3  | Decide whether `wait_for_action(&str)` gets `#[deprecated]` in 4.18 |   ☐    |       | see roadmap Q3 |
| Q4  | Publish `mac-usernotifications` to crates.io                       |   ☐    |       | see roadmap Q4, blocks F1 |
| Q5  | Decide whether `win32_notif` and `tauri-winrt-notification` coexist |   ☐    |       | see roadmap Q5 |
| Q6  | Confirm `id()` stays `u32` on default-cfg XDG in 4.18              |   ☐    |       | see roadmap Q6 |
| Q7  | Confirm public shape of `ActionResponse` for 4.18                  |   ☐    |       | see roadmap Q7 |

### Features

| ID  | Feature                                                                                  | Status | Source branch                          | Notes |
|-----|------------------------------------------------------------------------------------------|:------:|----------------------------------------|-------|
| F1  | macOS preview backend behind `pure_usernotifications`                                    |   ☐    | `feature/macos-usernotifications`     | depends on Q4 |
| F2  | Windows preview backend behind `win32`                                                   |   ☐    | `feature/win32-notif`                  | rebase as opt-in feature, do not flip default |
| F3  | Cross-platform `action` module (`ActionResponse`, `CloseReason`, `UserResponse`)         |   ☐    | `feature/macos-usernotifications`     | additive on default-cfg |
| F4  | `NotificationId` enum exists, **not** yet returned from default-cfg `id()`               |   ☐    | `feature/macos-usernotifications`     | depends on Q6 |
| F5  | `Notification::hero_image` (Windows-only, additive)                                      |   ☐    | `feature/win32-notif`                  | no-op without `win32` feature |
| F6  | XDG `wait_for_action_response(&ActionResponse)` additive method                          |   ☐    | `feature/macos-usernotifications`     | |
| F7  | `#[deprecated]` on `wait_for_action(&str)` and `"__closed"`                              |   ☐    |                                        | depends on Q3 |
| F8a | Windows polish: `Scenario::Urgent` → `Reminder` fallback on pre-22H2                     |   ☐    | `feature/win32-notif` (`windows_todo.md`) | applies to `win32` path |
| F8b | Windows polish: looping audio for `Alarm*`/`Call*`                                       |   ☐    | `feature/win32-notif` (`windows_todo.md`) | |
| F8c | Windows polish: `with_expiry` honoring ms timeouts                                       |   ☐    | `feature/win32-notif` (`windows_todo.md`) | |
| F9  | Docs: preview-backends section in README and crate root                                  |   ☐    |                                        | |
| F10 | macOS UN `interruption_level()` builder method                                          |   ✅   | main | re-exports `InterruptionLevel` from `mac-usernotifications` |

### Revert work needed on the source branches

| ID  | What                                                                  | Status | Notes |
|-----|-----------------------------------------------------------------------|:------:|-------|
| R1  | Restore macOS legacy default in `Cargo.toml` (`default = ["z"]`)      |   ☐    | macOS branch currently defaults to `pure_usernotifications` |
| R2  | Restore legacy macOS `show() -> Result<()>`                           |   ☐    | macOS branch changed it on the legacy path too |
| R3  | Restore default Windows `show() -> Result<()>`                        |   ☐    | Windows branch changed it unconditionally |
| R4  | Restore default XDG `handle.id() -> u32`                              |   ☐    | macOS branch changed it crate-wide |
| R5  | Audit XDG public surface for accidental 4.x breaks on default cfg      |   ☐    | macOS branch touched `src/xdg/*` |

### Validation gates for 4.18

| ID  | Gate                                                                           | Status |
|-----|--------------------------------------------------------------------------------|:------:|
| V1  | `cargo check` with default features (Linux, macOS, Windows)                    |   ☐    |
| V2  | `cargo check --no-default-features`                                            |   ☐    |
| V3  | `cargo check --features pure_usernotifications` on macOS                       |   ☐    |
| V4  | `cargo check --features win32` on Windows                                      |   ☐    |
| V5  | `cargo hack` feature powerset (depth 2) on all three platforms                 |   ☐    |
| V6  | All existing examples compile unchanged on default features                    |   ☐    |
| V7  | New examples for `pure_usernotifications` and `win32`                          |   ☐    |
| V7b | Example for `interruption_level` feature                                        |   ✅   | `examples/interruption_level.rs` |
| V8  | CHANGELOG entry for 4.18                                                       |   ☐    |
| V9  | Public API diff vs 4.17 reviewed (`cargo public-api` or manual)                |   ☐    |

---

## 5.0 — unification release

### Breaking-change checklist

| ID  | Change                                                                       | Status | Notes |
|-----|------------------------------------------------------------------------------|:------:|-------|
| B1  | `show() -> Result<NotificationHandle>` on macOS legacy                       |   ☐    | |
| B2  | `show() -> Result<NotificationHandle>` on Windows                            |   ☐    | |
| B3  | `NotificationHandle::id() -> NotificationId` everywhere                      |   ☐    | |
| B4  | Remove `wait_for_action(&str)` from macOS UN handle                          |   ✅   | removed from `pure_usernotifications::NotificationHandle` |
| B5  | Remove `"__closed"` sentinel                                                 |   ☐    | still present on XDG |
| B6  | Remove `wait_for_action_response` from macOS UN handle                       |   ✅   | removed from `pure_usernotifications::NotificationHandle` |
| B7  | Remove `on_close` from macOS UN handle                                       |   ✅   | removed from `pure_usernotifications::NotificationHandle`; still present on XDG |
| B8  | Flip macOS default to UN, gate legacy behind `macos_legacy`                  |   ✅   | `macos_legacy` feature added; `mac-notification-sys` now optional |
| B9  | Flip Windows default to `win32_notif`, gate legacy behind `windows_legacy`   |   ☐    | |
| B10 | Move `set_application` / `get_bundle_identifier_or_default` under `macos_legacy` |   ✅   | gated on `feature = "macos_legacy"` in `lib.rs` |
| B11 | Remove macOS `Urgency` re-export                                             |   ☐    | depends on Q9 |
| B12 | Remove `show_debug`                                                          |   ☐    | depends on Q12 |
| B13 | Rename / drop `pure_usernotifications` flag                                  |   🛠   | kept as empty no-op alias; module still named `pure_usernotifications` |

### New unified API

| ID  | Item                                                                  | Status | Notes |
|-----|-----------------------------------------------------------------------|:------:|-------|
| U1  | `NotificationHandle::response() -> UserResponse` on XDG               |   ☐    | |
| U2  | `NotificationHandle::response_blocking() -> UserResponse` on XDG      |   ☐    | |
| U3  | Same on macOS UN                                                       |   ✅   | `response().await` and `response_blocking()` on `pure_usernotifications::NotificationHandle` |
| U4  | Same on Windows                                                        |   ☐    | requires plumbing on `win32_notif` path |
| U5  | `close()` on Windows                                                   |   ☐    | from `windows_todo.md` future-work |
| U6  | `update()` / `update_async()` on Windows                               |   ☐    | partially present on branch |

### Validation gates for 5.0

| ID  | Gate                                                                           | Status |
|-----|--------------------------------------------------------------------------------|:------:|
| W1  | `cargo check` with default features on all three platforms                     |   🛠   | macOS ✅; Linux/Windows pending |
| W2  | `cargo check --features macos_legacy` builds on macOS                          |   ✅   |
| W3  | `cargo check --features windows_legacy` builds on Windows                      |   ☐    |
| W4  | `cargo hack` feature powerset (depth 2) on all three platforms                 |   ☐    |
| W5  | Migration guide published (covers B1..B13)                                     |   ☐    |
| W6  | 5.0-rc1 published alongside 4.18                                               |   ☐    |
| W7  | rc feedback cycle (at least 2 weeks)                                           |   ☐    |
| W8  | CHANGELOG entry for 5.0                                                        |   ☐    |
| W9  | docs.rs build green on default features                                        |   ☐    |

---

## Open follow-ups (post 5.0, optional)

| ID  | Idea                                                              | Notes |
|-----|-------------------------------------------------------------------|-------|
| P1  | Windows progress-bar API                                          | `windows_todo.md` future work |
| P2  | Windows `Scenario::Alarm` / `IncomingCall`                        | requires cross-platform scenario design |
| P3  | macOS UN `close()` and `notification_id()` on the handle           | spec table shows "not yet" |
| P4  | Remove `macos_legacy` / `windows_legacy` in a later 5.x minor      | once usage is gone |
