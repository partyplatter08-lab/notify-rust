# macOS Feature Parity Spec

> Status: **complete** — all tasks implemented

## Goal

Bring `notify-rust` on macOS to feature parity with the XDG (Linux/BSD) backend by adding:

1. Fire-and-forget **async send** (`show_async` → `Result<()>`) — _deferred, see §3_
2. **Blocking wait-for-response**: `show()` → `Result<NotificationHandle>`
3. **Async wait-for-response**: `show_async()` → `Result<NotificationHandle>` (already exists)
4. `wait_for_action_async` on `NotificationHandle`
5. Proper `on_close` / `wait_for_action` semantics using `mac_usernotifications` response

All new behaviour lives exclusively behind the feature flag **`pure_usernotifications`**
(renamed from `macos_pure_unusernotification_center`). The legacy `mac_notification_sys`
path is untouched.

---

## Decisions recorded

| #   | Question             | Decision                                                     |
| --- | -------------------- | ------------------------------------------------------------ |
| 1   | Feature flag name    | Rename to `pure_usernotifications`                           |
| 2   | `show()` return type | Mirror Linux: `Result<NotificationHandle>`                   |
| 3   | `show_async()` shape | Deferred — keep current `Result<NotificationHandle>` for now |
| 4   | `on_close` semantics | Delegate to `mac_usernotifications`'s `is_dismiss_action()`  |
| 5   | `__closed` sentinel  | Drop it — use `ActionResponse` enum directly                 |

---

## API contract after changes

### `Notification` send methods on macOS

| Method         | Flag                     | Returns                      | Behaviour                                                        |
| -------------- | ------------------------ | ---------------------------- | ---------------------------------------------------------------- |
| `show()`       | none (legacy)            | `Result<NotificationHandle>` | blocking, via `mac_notification_sys` — **return type changes**   |
| `show()`       | `pure_usernotifications` | `Result<NotificationHandle>` | blocking wait-for-response via `send_with_actions_blocking`      |
| `show_async()` | `pure_usernotifications` | `Result<NotificationHandle>` | async wait-for-response via `send_with_actions` (already exists) |

> **Note on legacy `show()`**: The legacy path (no flag) currently returns `Result<()>`.
> Changing it to `Result<NotificationHandle>` is a semver-breaking change. It should be
> done in the same release as the `pure_usernotifications` work so it lands as one breaking
> bump rather than two. The legacy `NotificationHandle` will not carry a meaningful
> response — it would just wrap the `Notification` itself, consistent with the current
> no-op `on_close` behaviour.

### `NotificationHandle` on macOS (after changes)

| Method                           | Status                    | Notes                                                           |
| -------------------------------- | ------------------------- | --------------------------------------------------------------- |
| `wait_for_action(closure)`       | ✅ keep, update signature | closure receives `&ActionResponse` not `&str` (drop `__closed`) |
| `wait_for_action_async(closure)` | ➕ new                    | async mirror; response already captured                         |
| `on_close(handler)`              | ✅ keep, improve          | delegate to `is_dismiss_action()` from `mac_usernotifications`  |

---

## `wait_for_action` closure signature — dropping `__closed`

On XDG, `wait_for_action` today takes `FnOnce(&str)` and passes the synthetic
string `"__closed"` for dismissals. The FIXME comments in the source mark this for
removal in v5. On macOS the existing implementation already does the same mapping
via `un_response_to_identifier`.

Since we are making a breaking change anyway (return type of `show()`), we should
**not** carry `__closed` forward on macOS. Instead, `wait_for_action` and
`wait_for_action_async` on macOS should take `FnOnce(&ActionResponse)`, which is
already the internal XDG type (see `action.rs`):

```notify-rust4/src/action.rs#L40-46
pub enum ActionResponse<'a> {
    /// Custom action configured by the notification.
    Custom(&'a str),
    /// The notification was closed.
    Closed(CloseReason),
}
```

The mapping from `mac_usernotifications::NotificationResponse` to `ActionResponse`
follows the same transparent pass-through that Linux/XDG uses — action identifiers
are forwarded as-is, only a true dismiss becomes `Closed`:

| `NotificationResponse` | `ActionResponse`                                                        |
| ---------------------- | ----------------------------------------------------------------------- |
| `is_dismiss_action()`  | `Closed(CloseReason::Dismissed)`                                        |
| `is_default_action()`  | `Custom(action_identifier)` — raw Apple string, passed through verbatim |
| reply text present     | `Custom(reply_text)`                                                    |
| any other identifier   | `Custom(action_identifier)`                                             |

On Linux, a body-tap comes through as `ActionResponse::Custom("default")` (whatever
the notification server sends). macOS mirrors that: the raw identifier is forwarded
and the user can match on it or ignore it. No special casing, no named constants.

---

## `on_close` semantics

`on_close` calls its handler only when `NotificationResponse::is_dismiss_action()` is true.

Custom-action clicks do **not** trigger `on_close` (matches XDG, which fires `on_close`
only on the `NotificationClosed` D-Bus signal, not on action clicks).

---

## Concrete task list

### Task 1 — Feature flag rename

**File**: `notify-rust4/Cargo.toml`

- Rename `macos_pure_unusernotification_center` → `pure_usernotifications` in `[features]`
- Update `default` array
- Find and replace all `#[cfg(feature = "macos_pure_unusernotification_center")]` guards

**Files**: `src/macos.rs`, `src/notification.rs`

---

### Task 2 — `macos.rs`: new `show_notification_blocking` + updated response mapping

**File**: `notify-rust4/src/macos.rs`

Add under `#[cfg(feature = "pure_usernotifications")]`:

```rs
/// Blocking send-and-wait-for-response via `UNUserNotificationCenter`.
pub(crate) fn show_notification_blocking(notification: &Notification) -> Result<NotificationHandle> {
    let resp = mac_usernotifications::send_with_actions_blocking(notification.into())?;
    Ok(NotificationHandle::new(notification.clone(), resp))
}
```

Update `un_response_to_identifier` — or remove it entirely — replacing its use in
`wait_for_action` with a proper `NotificationResponse → ActionResponse` mapping function:

```rs
fn response_to_action_response(resp: &NotificationResponse) -> ActionResponse<'_> {
    if resp.is_dismiss_action() {
        ActionResponse::Closed(CloseReason::Dismissed)
    } else if let Some(ref text) = resp.reply_text {
        ActionResponse::Custom(text.as_str())
    } else {
        ActionResponse::Custom(resp.action_identifier.as_str())
    }
}
```

---

### Task 3 — `NotificationHandle`: update `wait_for_action`, add `wait_for_action_async`, fix `on_close`

**File**: `notify-rust4/src/macos.rs`

**`wait_for_action`** — change closure type from `FnOnce(&str)` to `FnOnce(&ActionResponse)`:

```rs
pub fn wait_for_action<F>(self, invocation_closure: F)
where
    F: FnOnce(&ActionResponse),
{
    invocation_closure(&response_to_action_response(&self.response));
}
```

**`wait_for_action_async`** — new method, response already in hand so no blocking:

```rs
pub async fn wait_for_action_async<F>(self, invocation_closure: F)
where
    F: FnOnce(&ActionResponse),
{
    invocation_closure(&response_to_action_response(&self.response));
}
```

**`on_close`** — fire handler only on dismiss:

```rs
pub fn on_close<A>(self, handler: impl CloseHandler<A>) {
    if self.response.is_dismiss_action() {
        handler.call(CloseReason::Dismissed);
    }
}
```

---

### Task 4 — `notification.rs`: `show()` return type + new `show_async` variant

**File**: `notify-rust4/src/notification.rs`

Under `#[cfg(feature = "pure_usernotifications")]`:

- Change `show()` → `Result<macos::NotificationHandle>` backed by `show_notification_blocking`
- Keep `show_async()` → `Result<macos::NotificationHandle>` as-is

Under legacy (no flag):

- Change `show()` → `Result<macos::NotificationHandle>` backed by legacy path
  — the legacy `NotificationHandle` carries no meaningful response; `wait_for_action`
  will yield `Closed(Dismissed)` unconditionally (same as current `__closed` fallback)

---

### Task 5 — Update examples

**Files**: `examples/mac_actions.rs`, `examples/mac_actions_async.rs`

Update `wait_for_action` call sites to use `&ActionResponse` match arms instead of
`&str` / `"__closed"`.

---

## Files to touch (complete list)

| File                                         | Tasks      |
| -------------------------------------------- | ---------- |
| `notify-rust4/Cargo.toml`                    | Task 1     |
| `notify-rust4/src/macos.rs`                  | Tasks 2, 3 |
| `notify-rust4/src/notification.rs`           | Task 4     |
| `notify-rust4/examples/mac_actions.rs`       | Task 5     |
| `notify-rust4/examples/mac_actions_async.rs` | Task 5     |

---

## Status

All tasks implemented.

---

## Legacy vs `pure_usernotifications` comparison

This table shows what `NSUserNotificationCenter` (via `mac-notification-sys`) supported
versus what `UNUserNotificationCenter` (via `mac-usernotifications`) provides.

### `Notification` builder

| method                | legacy (`NSUserNotif.`) | `pure_usernotifications` |
| --------------------- | ----------------------- | ------------------------ |
| `fn summary(...)`     | ✅                      | ✅                       |
| `fn subtitle(...)`    | ✅                      | ✅                       |
| `fn body(...)`        | ✅                      | ✅                       |
| `fn image_path(...)`  | ✅ (content image)      | ✅ (attachment)          |
| `fn action(...)`      | ✅ (main button only)   | ✅ (multiple buttons)    |
| `fn sound(...)`       | ✅                      | ✅                       |
| `fn timeout(...)`     | ❌                      | ✅                       |
| `fn id(...)`          | ❌                      | ✅ (string id)           |
| `fn thread_id(...)`   | ❌                      | ✅                       |
| `fn schedule_in(...)` | ✅ (delivery date)      | ✅ (time interval)       |
| `fn show_async(...)`  | ❌                      | ✅                       |
| reply actions         | ❌                      | ✅                       |
| image attachments     | ✅                      | ✅                       |

### `NotificationHandle`

| method                          | legacy | `pure_usernotifications` |
| ------------------------------- | ------ | ------------------------ |
| `fn wait_for_action(...)`       | ❌     | ✅                       |
| `fn wait_for_action_async(...)` | ❌     | ✅                       |
| `fn on_close(...)`              | ❌     | ✅                       |
| `fn update(...)`                | ❌     | ✅                       |
| `fn update_async(...)`          | ❌     | ✅                       |
| `fn close(...)`                 | ❌     | ❌ (not yet)             |
| `fn id(...)`                    | ❌     | ❌ (not yet)             |

### Notable legacy-only features

- `fn close_button(...)` — a dedicated dismiss button label
- `fn app_icon(...)` — override the icon shown next to the notification title
- `fn wait_for_click(...)` / `fn asynchronous(...)` — lower-level delivery control

These have no equivalent in `UNUserNotificationCenter` and are not forwarded.
