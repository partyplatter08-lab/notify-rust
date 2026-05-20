# notify-rust Roadmap — 4.x → 5.0

This document plans the next two notify-rust releases.

- **4.18 (final 4.x)**: one more minor release that exposes the new Windows and
  macOS backends behind opt-in feature flags. No public API breaks for users
  on the default feature set.
- **5.0**: unified, cross-platform `NotificationHandle` API. Drops legacy
  shims, removes the `"__closed"` sentinel, makes the new backends the
  default, gates the old ones behind explicit features.

Progress tracking lives in [`notify-rust-progress.md`](./notify-rust-progress.md).
The macOS work is specified separately in [`MACOS_PARITY_SPEC.md`](./MACOS_PARITY_SPEC.md);
this roadmap references it but does not duplicate it.

---

## Guiding principles

1. **4.18 must be a non-breaking minor on the default feature set.**
   Anything that would change a public signature on default cfg must be
   deferred to 5.0 or hidden behind an opt-in feature flag.
2. **Preview backends (`pure_usernotifications`, `win32`) are opt-in only.**
   Enabling them is documented as "preview, semver-exempt within 4.x". The
   API shapes they expose are the ones we intend to ship in 5.0.
3. **5.0 unifies the handle API across all backends.** One `show()` shape,
   one `response()` model, one `id()` type.
4. **Legacy backends do not disappear in 5.0**, they move behind explicit
   feature flags so existing users can still pin to them deliberately.

---

## 4.18 — final 4.x release

### New features (additive only on default cfg)

| # | Feature | Default? | Breaking? | Notes |
|---|---------|----------|-----------|-------|
| F1 | Feature flag `pure_usernotifications` (macOS, `UNUserNotificationCenter`) | no | no (opt-in) | from `feature/macos-usernotifications` |
| F2 | Feature flag `win32` (Windows, `win32_notif`) | no | no (opt-in) | from `feature/win32-notif` |
| F3 | `ActionResponse`, `CloseReason`, `UserResponse` types in `notify_rust::action` | yes | no | new types, no signature changes |
| F4 | `NotificationId` enum | yes | no | new type; not yet returned from `id()` on default cfg |
| F5 | `Notification::hero_image()` (Windows-only, additive, gated `cfg(target_os="windows")`) | yes | no | only takes effect under `win32` feature; no-op otherwise |
| F6 | XDG: `wait_for_action_response(&ActionResponse)` (additive, alongside existing `wait_for_action(&str)`) | yes | no | shipped as the migration target for 5.0 |
| F7 | Deprecation warnings on `wait_for_action(&str)` and the `"__closed"` sentinel | yes | no | `#[deprecated]` only, still functional |
| F8 | Internal Windows polish from `windows_todo.md` that does not touch the public API (e.g. `Scenario::Urgent` runtime fallback, looping audio for `Alarm*`/`Call*`, `with_expiry` honoring ms timeouts) | yes | no | landed in the legacy `winrt-notification` path **and** in the new `win32` path where applicable |
| F9 | Docs: "preview backends" section in `README.md` and crate root, pointing users at the two new flags | yes | no | |

### Explicitly **not** in 4.18 (deferred to 5.0)

| # | Item | Why deferred |
|---|------|--------------|
| D1 | `show()` returning `Result<NotificationHandle>` on default macOS/Windows | breaks the existing `Result<()>` signature |
| D2 | `NotificationHandle::id()` returning `NotificationId` | breaks the existing `u32` return type |
| D3 | Removal of `wait_for_action(&str)` and `"__closed"` | breaks every existing call site |
| D4 | Replacing `tauri-winrt-notification` with `win32_notif` as the Windows default | the dep swap changes default-cfg behaviour and error types |
| D5 | Making `pure_usernotifications` the default macOS path | flips the macOS default cfg |
| D6 | Unified `response().await` / `response_blocking()` on `NotificationHandle` | requires changing the handle's public surface across all backends |
| D7 | Moving `set_application` / `get_bundle_identifier_or_default` behind `macos_legacy` | feature-shuffle that breaks unconditional users |

### Feature-flag layout in 4.18

```toml
[features]
default = ["z"]

# preview backends — semver-exempt within 4.x
pure_usernotifications = []     # macOS UNUserNotificationCenter
win32                  = []     # Windows win32_notif

# existing flags unchanged
z = ["zbus", "serde", "async"]
# ...
```

The preview backends are mutually exclusive with their legacy counterparts on
the same platform (compile-time `cfg` switch).

### Ordering for 4.18

1. **Land the macOS feature flag** (`pure_usernotifications`) by merging
   `feature/macos-usernotifications` to `main`, with the legacy macOS path
   restored as the no-flag default and `show()` reverted to `Result<()>` on
   the legacy path. The macOS spec's Task list applies only behind the flag.
2. **Land the Windows feature flag** (`win32`) by porting
   `feature/win32-notif` to live alongside the existing
   `winrt-notification` path, gated on `feature = "win32"`. Default Windows
   stays on `winrt-notification` and `show() -> Result<()>`.
3. **Land cross-platform `action` module** (`ActionResponse`, `CloseReason`,
   `UserResponse`, `NotificationId`) so 5.0 callers can already write code
   against the target types.
4. **Add XDG `wait_for_action_response`** as an additive method.
5. **Deprecate** `wait_for_action(&str)` and the `"__closed"` sentinel
   with `#[deprecated(since = "4.18.0", note = "…")]`. Do not remove.
6. **Internal Windows polish** from `windows_todo.md` (F8).
7. **Docs and examples** for the two new flags.

### Risks for 4.18

- The macOS branch currently has `default = ["z", "pure_usernotifications"]`
  in `Cargo.toml`. We must change that back so the default `cargo install`
  user keeps the legacy path.
- `feature/macos-usernotifications` changes `show()` to return
  `Result<NotificationHandle>` on the legacy path too. That has to be backed
  out for 4.18 and re-introduced in 5.0.
- The `mac-usernotifications` crate is currently a path dependency. It needs
  to be published before we can release 4.18.
- The macOS branch already touches XDG (`wait_for_action_response`, an
  internal `ActionResponse` rename). We need to confirm those changes are
  additive and do not break existing XDG callers.

---

## 5.0 — the unification release

### What we break (the full list)

| # | API today (4.x) | API in 5.0 | Migration |
|---|-----------------|------------|-----------|
| B1 | `Notification::show() -> Result<()>` on macOS legacy | `Notification::show() -> Result<NotificationHandle>` | drop `?;` semicolon, optionally inspect handle |
| B2 | `Notification::show() -> Result<()>` on Windows | `Notification::show() -> Result<NotificationHandle>` | same |
| B3 | `NotificationHandle::id() -> u32` (XDG) | `NotificationHandle::id() -> NotificationId` | match on `NotificationId::Xdg(u32)` / `Mac(String)` |
| B4 | `wait_for_action<F: FnOnce(&str)>` (XDG + macOS) | removed | use `response_blocking()` and match on `UserResponse` |
| B5 | `"__closed"` sentinel string | removed | match `UserResponse::Closed(CloseReason)` |
| B6 | `wait_for_action_response` (added in 4.18) | removed | superseded by `response()` / `response_blocking()` |
| B7 | `on_close(handler)` (XDG, and macOS UN in 4.18) | removed | match `UserResponse::Closed` |
| B8 | macOS default backend = `mac-notification-sys` (legacy) | default = `UNUserNotificationCenter` (`pure_usernotifications`) | enable `macos_legacy` feature to opt back in |
| B9 | Windows default backend = `tauri-winrt-notification` | default = `win32_notif` | enable `windows_legacy` feature to opt back in |
| B10 | `set_application`, `get_bundle_identifier_or_default` re-exports on macOS | only under `macos_legacy` | feature-gate or migrate |
| B11 | Public re-export of `Urgency` on macOS (`#[deprecated]` today) | removed | use `cfg(not(target_os = "macos"))` |
| B12 | `Notification::show_debug` (already `#[deprecated]`) | removed | use logging |
| B13 | Feature flag name `pure_usernotifications` | may be renamed to `macos_un` or dropped entirely (it's the default) | flag rename |

The list of breaking changes is shorter than it looks because most callers
only use `.show().unwrap()`; the unwrap continues to compile. The two
practical pain points are `wait_for_action` callers and anyone who reads
`handle.id()`.

### The 5.0 `NotificationHandle` (target shape)

```rust
impl NotificationHandle {
    pub fn id(&self) -> NotificationId;

    pub fn close(self);                         // XDG, macOS UN, Windows
    pub fn update(&mut self) -> Result<()>;     // XDG, macOS UN, Windows
    pub async fn update_async(&mut self) -> Result<()>;  // XDG (zbus), macOS UN

    pub async fn response(self) -> UserResponse;        // unified
    pub fn response_blocking(self) -> UserResponse;     // unified
}
```

`UserResponse` carries either an action identifier, a reply text, or a
`Closed(CloseReason)`. No closures, no sentinel strings, no platform-specific
shapes leaking into user code.

### Feature-flag layout in 5.0

```toml
[features]
default = ["z"]

# explicit legacy opt-ins
macos_legacy     = []   # mac-notification-sys / NSUserNotificationCenter
windows_legacy   = []   # tauri-winrt-notification

# existing flags unchanged
z = ["zbus", "serde", "async"]
```

The 4.18 preview flags (`pure_usernotifications`, `win32`) disappear; their
behaviour becomes the default.

### Feature parity target for 5.0

Reading the tables in `MACOS_PARITY_SPEC.md` and `windows_todo.md` against
the current XDG surface, this is where we expect to land. ✅ = supported,
🟡 = supported with caveats, ❌ = not available.

#### `Notification` builder

| method                 | XDG | macOS (UN) | Windows (win32) |
|------------------------|:---:|:----------:|:---------------:|
| `appname`              | ✅  | ❌         | ❌              |
| `summary`              | ✅  | ✅         | ✅              |
| `subtitle`             | ❌  | ✅         | ✅              |
| `body`                 | ✅  | ✅         | ✅              |
| `icon`                 | ✅  | ❌         | 🟡 app-logo override |
| `image_path`           | ✅  | ✅         | ✅              |
| `hero_image`           | ❌  | ❌         | ✅              |
| `auto_icon`            | ✅  | ❌         | ❌              |
| `hint`                 | ✅  | ❌         | ❌              |
| `timeout`              | ✅  | ✅         | 🟡 bucketed     |
| `urgency`              | ✅  | ❌         | 🟡 scenario map |
| `action(id, label)`    | ✅  | ✅         | ✅              |
| `id`                   | ✅  | ✅ (string)| 🟡 (tag-based)  |
| `sound`                | 🟡  | ✅         | ✅              |
| `thread_id`            | ❌  | ✅         | ❌              |
| `schedule_in`          | ❌  | ✅         | ❌              |
| `suppress_popup`       | ❌  | ❌         | ✅              |
| reply actions          | ❌  | ✅         | ❌              |
| progress bar           | ❌  | ❌         | ✅ (stretch)    |

#### `NotificationHandle`

| method                        | XDG | macOS (UN) | Windows (win32) |
|-------------------------------|:---:|:----------:|:---------------:|
| `id`                          | ✅  | ✅         | ✅              |
| `close`                       | ✅  | ✅         | ✅              |
| `update` / `update_async`     | ✅  | ✅         | ✅              |
| `response().await`            | ✅  | ✅         | ✅              |
| `response_blocking()`         | ✅  | ✅         | ✅              |

Net: **the handle API is fully unified across all three backends in 5.0.**
The builder is not: `hint`, `urgency`, `appname`, and `auto_icon` stay
XDG-exclusive because the underlying systems do not model them; `subtitle`,
`thread_id`, `schedule_in`, and reply actions stay macOS-exclusive;
`hero_image`, `suppress_popup`, and progress stay Windows-exclusive. Those
remain `cfg`-gated.

### Ordering for 5.0

1. Cut a 5.0 development branch from 4.18.
2. Flip the macOS default to `UNUserNotificationCenter`, gate the legacy
   path behind `macos_legacy`.
3. Flip the Windows default to `win32_notif`, gate `tauri-winrt-notification`
   behind `windows_legacy`.
4. Unify `NotificationHandle::id()` to `NotificationId`.
5. Remove `wait_for_action(&str)`, `wait_for_action_response`, `on_close`,
   and the `"__closed"` sentinel.
6. Land `response()` / `response_blocking()` on every handle.
7. Remove the `Urgency` re-export on macOS and `show_debug`.
8. Publish a 5.0-rc on crates.io alongside the final 4.18 so users have
   both as installable references.
9. After at least one rc cycle with feedback, publish 5.0.

---

## Open questions

These need a decision before work starts on 4.18:

1. **Preview-flag names.** The macOS branch uses `pure_usernotifications`;
   the Windows branch has no flag yet. Are we happy with
   `pure_usernotifications` and `win32` as the 4.18 names, or do you want a
   uniform scheme like `preview-macos-un` / `preview-windows-win32` (or an
   umbrella `preview-backends`)?
2. **Should 4.18 also publish an `experimental` umbrella feature** that
   enables both preview backends at once, for CI convenience?
3. **Do we deprecate `wait_for_action(&str)` and `"__closed"` in 4.18**
   (my recommendation: yes, with `#[deprecated]`), or leave them silent
   until 5.0?
4. **`mac-usernotifications` publication.** The macOS branch currently
   pulls it as a path dependency. What is the timeline for publishing it to
   crates.io so 4.18 can ship?
5. **Windows dep swap.** Do we tolerate carrying both `tauri-winrt-notification`
   and `win32_notif` in `Cargo.toml` for the duration of 4.18, or do we want
   the `win32` feature to *replace* `tauri-winrt-notification` (which would
   force every Windows user onto the new dep right away)? My recommendation
   is to keep both, since the whole point of 4.18 is non-breaking opt-in.
6. **`NotificationId` in 4.18.** The macOS branch already returns
   `NotificationId` from `handle.id()`, which is a 4.x break. Confirm that
   we are reverting `id()` back to `u32` on default-cfg XDG for 4.18 and
   only exposing `NotificationId` under the preview flags.
7. **XDG `ActionResponse` shape.** The macOS branch reworked the internal
   XDG `ActionResponse` enum. Does the public re-export of
   `ActionResponse` need to stay byte-compatible with 4.17, or can we use
   4.18 to introduce the new shape (since it is a new public type either
   way)?
8. **MSRV.** Are we comfortable bumping MSRV in 5.0 (current is 1.63)? The
   `win32_notif` dep may push that up.
9. **Urgency on macOS.** It is `#[deprecated]` today. Remove in 5.0 (my
   recommendation), or keep as a no-op?
10. **`set_application` / `get_bundle_identifier_or_default`.** Confirm
    that these only make sense under `macos_legacy` in 5.0 and can be
    feature-gated.
11. **Timeline.** Target dates for 4.18 and 5.0-rc1? Are they to ship side
    by side, as the macOS branch notes already suggest?
12. **`Notification::show_debug`.** Already `#[deprecated]`. Remove in 5.0?
13. **Should the unified handle's `response()` consume `self`** (force
    one-shot, my recommendation, matches macOS UN today) or take `&self`
    (would require interior state and complicates Drop semantics)?
