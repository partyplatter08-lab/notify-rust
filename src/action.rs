//! Cross-platform action response types.
//!
//! These types describe how a notification was acted upon by the user:
//! either by clicking a configured action, closing the notification, or
//! submitting an inline text reply.
//!
//! They are shared between all backends so that consumer code does not need
//! a `cfg` switch to read responses.

/// Reason a notification was closed without an action being invoked.
///
/// ### Platform notes
///
/// **XDG (Linux/BSD):** maps directly to the `NotificationClosed` D-Bus signal
/// reasons defined in [Table 8 of the spec](https://specifications.freedesktop.org/notification-spec/latest/protocol.html).
///
/// **macOS:** the underlying system does not distinguish between close reasons,
/// so all closes are reported as [`CloseReason::Dismissed`].
///
/// **Windows (`Windows.UI.Notifications`):** maps from [`ToastDismissalReason`]:
/// `UserCanceled` → [`Dismissed`](CloseReason::Dismissed),
/// `TimedOut` → [`Expired`](CloseReason::Expired),
/// `ApplicationHidden` → [`CloseAction`](CloseReason::CloseAction).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum CloseReason {
    /// The notification expired (timed out).
    Expired,
    /// The notification was dismissed by the user.
    Dismissed,
    /// The notification was closed programmatically (e.g. a `CloseNotification`
    /// D-Bus call on XDG, or `ToastNotifier::Hide` on Windows).
    CloseAction,
    /// Undefined or reserved reason.
    ///
    /// The inner value is the raw reason code as reported by the platform.
    /// This variant will never be produced on platforms that do not carry a
    /// numeric reason (macOS, Windows).
    Other(u32),
}

impl From<u32> for CloseReason {
    fn from(raw_reason: u32) -> Self {
        match raw_reason {
            1 => CloseReason::Expired,
            2 => CloseReason::Dismissed,
            3 => CloseReason::CloseAction,
            other => CloseReason::Other(other),
        }
    }
}

/// The response to a notification — every possible outcome of showing one.
///
/// This is what [`NotificationHandle::response`](crate::NotificationHandle::response)
/// resolves to.
///
/// ### Platform notes
///
/// | Variant | XDG | macOS (UN) | Windows |
/// |---------|-----|------------|---------|
/// | `Action` | ✔︎ via `ActionInvoked` signal | ✔︎ | ✔︎ via `Activated` event |
/// | `Reply` | ❌ not in spec | ✔︎ `UNTextInputNotificationAction` | ✔︎ `ToastTextBox` |
/// | `Closed` | ✔︎ full `CloseReason` | ✔︎ only `Dismissed` | ✔︎ `UserCanceled`/`TimedOut`/`ApplicationHidden` |
///
/// On XDG the `"default"` action key is the conventional identifier for a
/// click on the notification body itself, though servers are not required to
/// use it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum UserResponse {
    /// The user clicked the notification body or a labelled action button.
    ///
    /// The key is the action identifier registered with
    /// [`Notification::action`](crate::Notification::action).
    /// The conventional key `"default"` means the user clicked the
    /// notification body itself.
    Action(String),

    /// The user submitted an inline text reply.
    ///
    /// Only produced on macOS (`UNTextInputNotificationAction`) and
    /// Windows (`ToastTextBox` input).
    Reply(String),

    /// The notification was closed or dismissed without an action being taken.
    Closed(CloseReason),
}

impl UserResponse {
    /// Returns `true` if this is an [`Action`](UserResponse::Action) with the
    /// key `"default"`, which conventionally means the notification body was
    /// clicked.
    pub fn is_default_action(&self) -> bool {
        matches!(self, UserResponse::Action(key) if key == "default")
    }
}

/// The response from the user after a notification was shown.
///
/// Match on this to handle every possible outcome:
///
/// ```no_run
/// # use notify_rust::{ActionResponse, CloseReason};
/// # let response = ActionResponse::Closed(CloseReason::Dismissed);
/// match response {
///     ActionResponse::Action(key) if key == "default" => println!("body clicked"),
///     ActionResponse::Action(key) => println!("button '{key}' clicked"),
///     ActionResponse::Reply(text) => println!("user replied: {text}"),
///     ActionResponse::Closed(reason) => println!("closed: {reason:?}"),
/// }
/// ```
///
/// ### Platform notes
///
/// | Variant | XDG | macOS (UN) | Windows |
/// |---------|-----|------------|---------|
/// | `Action` | ✔︎ via `ActionInvoked` signal | ✔︎ | ✔︎ via `Activated` event |
/// | `Reply` | ❌ not in spec | ✔︎ `UNTextInputNotificationAction` | ✔︎ `ToastTextBox` input |
/// | `Closed` | ✔︎ full `CloseReason` | ✔︎ only `Dismissed` | ✔︎ `UserCanceled`/`TimedOut`/`ApplicationHidden` |
///
/// On XDG the `"default"` action key is the conventional identifier for a
/// click on the notification body itself, though servers are not required to
/// use it.
///
/// On macOS (legacy `NSUserNotificationCenter` path) `wait_for_action` is not
/// available.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ActionResponse {
    /// The user clicked the notification body or a labelled action button.
    ///
    /// The key is the action identifier that was registered with
    /// [`Notification::action`](crate::Notification::action).
    /// The conventional key `"default"` means the user clicked the
    /// notification body itself.
    Action(String),

    /// The user submitted an inline text reply.
    ///
    /// Only produced on macOS (via `UNTextInputNotificationAction`) and
    /// Windows (via a `ToastTextBox` input element).
    Reply(String),

    /// The notification was closed without any action being taken.
    Closed(CloseReason),
}

impl ActionResponse {
    /// Returns `true` if this is an [`Action`](ActionResponse::Action) with
    /// the key `"default"`, which conventionally means the notification body
    /// was clicked.
    pub fn is_default_action(&self) -> bool {
        matches!(self, ActionResponse::Action(key) if key == "default")
    }
}

impl From<String> for ActionResponse {
    fn from(key: String) -> Self {
        Self::Action(key)
    }
}

impl From<&str> for ActionResponse {
    fn from(key: &str) -> Self {
        Self::Action(key.to_owned())
    }
}

/// Helper trait implemented by closures used with `wait_for_action`.
///
/// You rarely need to implement this manually — any `FnOnce(&ActionResponse)`
/// closure will do.
pub trait ActionResponseHandler {
    /// Invoke the handler with the given response.
    fn call(self, response: &ActionResponse);
}

impl<F> ActionResponseHandler for F
where
    F: FnOnce(&ActionResponse),
{
    fn call(self, response: &ActionResponse) {
        (self)(response);
    }
}

/// Callback for the close signal of a notification.
///
/// Implemented for both `Fn(CloseReason)` and `Fn()`, so there is rarely a
/// good reason to implement this manually.
pub trait CloseHandler<T> {
    /// Called with the [`CloseReason`].
    fn call(&self, reason: CloseReason);
}

impl<F> CloseHandler<CloseReason> for F
where
    F: Fn(CloseReason),
{
    fn call(&self, reason: CloseReason) {
        self(reason);
    }
}

impl<F> CloseHandler<()> for F
where
    F: Fn(),
{
    fn call(&self, _reason: CloseReason) {
        self();
    }
}
