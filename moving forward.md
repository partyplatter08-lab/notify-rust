# towards notify-rust 5.0

There are effectively **three** macOS stacks in play:

1. **`mac-notification-sys` / legacy** (`NSUserNotificationCenter`, no feature flag, default on macOS) — deprecated by Apple, no dismiss detection, no async, blocking `send()` that can't even observe close. The whole `NotificationHandle` we added here is new and partly fake.
2. **`pure_usernotifications`** (`UNUserNotificationCenter`, feature flag) — the new path, futures-based, proper dismiss detection, the right long-term home.
3. **XDG** (Linux/BSD) — callback/closure based today, but already has `response().await` and `response_blocking()` as the modern path alongside the deprecated `wait_for_action(&str)` shim.

### Phase 1 — now (this branch)

**Stop adding new surface area to the legacy path.** The `NotificationHandle` for `mac-notification-sys` should be minimal and clearly tombstoned:

- Keep `show()` working (fire and forget, no blocking) — the banner still appears.
- Keep `wait_for_action<F: FnOnce(&str)>` as the deprecated shim, since that's what we promised not to break.
- **Do not** add `wait_for_action_response`, `response_blocking`, `on_close` to the legacy handle. Those are new API surface on a dead path. If someone calls `show()` on legacy, they get a handle that only has the old deprecated method.
- The `send_and_wait` approach we just added is honest — it at least blocks for a click — but it still can't detect dismiss. That's fine, just document it clearly.

**Align `pure_usernotifications::NotificationHandle` with the XDG shape** as closely as possible:

- `response().await` → `UserResponse` (already there)
- `response_blocking()` → `UserResponse` (already there)
- Deprecated `wait_for_action<F: FnOnce(&str)>` shim (already there)
- No `wait_for_action_response` — skip the middle generation entirely on the new path.

### Phase 2 — 4.x minor

**Deprecate `wait_for_action_response` on XDG** (already done with `since = "4.1.18"`). Make `response().await` / `response_blocking()` the single blessed API on all platforms that support it.

**Add a `#[deprecated]` to the entire `mac-notification-sys` feature path** — at the `use mac_notification_sys` / `show_notification` level — with a message pointing to `pure_usernotifications`.

**The canonical `NotificationHandle` API going forward is just two methods:**

```/dev/null/strategy.rs#L1-4
// modern, works on XDG + pure_usernotifications
async fn response(&self) -> UserResponse;
fn response_blocking(&self) -> UserResponse;  // blocks thread, use sparingly
```

Everything else (`wait_for_action`, `wait_for_action_response`, `on_close`) is deprecated scaffolding kept for back-compat.

### Phase 3 — 5.0

- Remove `wait_for_action(&str)` and the `"__closed"` sentinel everywhere.
- Remove `wait_for_action_response`.
- Remove `on_close` (callers use `response().await` and match on `Closed`).
- Drop `mac-notification-sys` as a dependency entirely. The legacy path becomes a compile-time error with a helpful message.
- `pure_usernotifications` becomes the default on macOS (or gets renamed to something without "pure").

---

## Immediate concrete actions for this branch

1. **Remove `wait_for_action_response` and `on_close` from `legacy::NotificationHandle`** — we just added them and they're new API on a path we're killing.
2. **Remove `wait_for_action_response` from `pure_usernotifications::NotificationHandle`** — skip the middle generation, the new path should only have `response()` and the deprecated `&str` shim.
3. **Add a `#[deprecated]` to `legacy::show_notification`** and the `set_application` / `get_bundle_identifier_or_default` re-exports, pointing users to enable `pure_usernotifications`.
4. Update the platform table in the docs to reflect the new reality.

Want me to go ahead and make those changes?
