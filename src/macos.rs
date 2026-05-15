#![allow(missing_docs)]
use crate::{
    error::*, notification::Notification, ActionResponse, CloseHandler, CloseReason, Timeout,
};

pub use mac_notification_sys::error::{ApplicationError, Error as MacOsError, NotificationError};
use mac_notification_sys::un::{self, NotificationResponse as UnNotificationResponse};

use std::{
    ops::{Deref, DerefMut},
    time::Duration,
};

///  A handle to a shown notification.
#[derive(Debug)]
pub struct NotificationHandle {
    notification: Notification,
    /// Response captured by [`show_notification`] / [`show_notification_async`]
    /// via `UNUserNotificationCenter`. `Some` once the user has interacted;
    /// `None` only for handles created by the legacy `macos_legacy` path before
    /// `wait_for_action` / `on_close` has been called.
    un_response: Option<UnNotificationResponse>,
}

impl From<(Notification, Option<UnNotificationResponse>)> for NotificationHandle {
    fn from(
        (notification, un_response): (Notification, Option<UnNotificationResponse>),
    ) -> NotificationHandle {
        NotificationHandle {
            notification,
            un_response,
        }
    }
}
impl NotificationHandle {
    #[allow(missing_docs)]
    pub fn new(notification: Notification) -> NotificationHandle {
        NotificationHandle {
            notification,
            un_response: None,
        }
    }

    pub fn wait_for_action<F>(self, invocation_closure: F)
    where
        F: FnOnce(&str),
    {
        let identifier = if let Some(ref resp) = self.un_response {
            // Fast path: response already captured by show_notification.
            un_response_to_identifier(resp, self.first_identifier())
        } else {
            // `un_response` is None (legacy path): send via the UN API now,
            // always using send_with_actions so that close/dismiss events are
            // observable even for notifications without explicit action buttons.
            let first_id = self.first_identifier().map(str::to_owned);
            let un_notification: un::Notification = (&self.notification).into();
            match un::send_with_actions_blocking(un_notification) {
                Ok(resp) => un_response_to_identifier(&resp, first_id.as_deref()),
                Err(_) => "__closed".to_owned(),
            }
        };
        invocation_closure(&identifier);
    }

    /// Async variant of [`wait_for_action`][Self::wait_for_action].
    ///
    /// Delivers the notification (if not yet delivered) using the
    /// `UNUserNotificationCenter` async API and `await`s the user's response.
    /// The closure is called with the action identifier once the user acts,
    /// or with `"__closed"` if the notification was dismissed without
    /// interaction.
    ///
    /// Unlike the old thread-based variant, no background thread is spawned;
    /// the work is driven by the caller's async executor.
    pub async fn wait_for_action_async<F>(self, invocation_closure: F)
    where
        F: FnOnce(&str),
    {
        let identifier = if let Some(ref resp) = self.un_response {
            // Fast path: response already captured by show_notification_async.
            un_response_to_identifier(resp, self.first_identifier())
        } else {
            // `un_response` is None (legacy path): send via the UN API now,
            // always using send_with_actions so that close/dismiss events are
            // observable even for notifications without explicit action buttons.
            let first_id = self.first_identifier().map(str::to_owned);
            let un_notification: un::Notification = (&self.notification).into();
            match un::send_with_actions(un_notification).await {
                Ok(resp) => un_response_to_identifier(&resp, first_id.as_deref()),
                Err(_) => "__closed".to_owned(),
            }
        };
        invocation_closure(&identifier);
    }

    pub fn on_close<A>(self, handler: impl CloseHandler<A>) {
        // If the UN path already delivered the notification and captured
        // the response, there's nothing more to do — just call the handler.
        // Otherwise (legacy path) send now, capturing the close event via
        // send_with_actions regardless of whether action buttons are present.
        if self.un_response.is_none() {
            let un_notification: un::Notification = (&self.notification).into();
            let _ = un::send_with_actions_blocking(un_notification);
        }
        handler.call(CloseReason::Dismissed);
    }

    // fn identifier_for_label(&self, label: &str) -> Option<&str> {
    //     self.notification
    //         .actions
    //         .chunks(2)
    //         .find_map(|chunk| match (chunk.first(), chunk.get(1)) {
    //             (Some(id), Some(lbl)) if lbl == label => Some(id.as_str()),
    //             _ => None,
    //         })
    // }

    fn first_identifier(&self) -> Option<&str> {
        self.notification.actions.first().map(String::as_str)
    }
}

impl Deref for NotificationHandle {
    type Target = Notification;

    fn deref(&self) -> &Notification {
        &self.notification
    }
}

/// Allow to easily modify notification properties
impl DerefMut for NotificationHandle {
    fn deref_mut(&mut self) -> &mut Notification {
        &mut self.notification
    }
}

/// Map a [`UnNotificationResponse`] to the `&str` identifier that
/// `wait_for_action` passes to its closure.
///
/// Mirrors the mapping used by the blocking path:
/// * default action (body click)  → first configured identifier, or `"__closed"`
/// * dismiss                       → `"__closed"`
/// * reply action                  → the text typed by the user
/// * any other action              → the action identifier set by the caller
fn un_response_to_identifier(resp: &UnNotificationResponse, first_id: Option<&str>) -> String {
    if resp.is_default_action() {
        first_id.map_or_else(|| "__closed".to_owned(), str::to_owned)
    } else if resp.is_dismiss_action() {
        "__closed".to_owned()
    } else if let Some(ref text) = resp.reply_text {
        text.clone()
    } else {
        resp.action_identifier.clone()
    }
}

impl From<&Notification> for un::Notification {
    fn from(notification: &Notification) -> Self {
        use mac_notification_sys::un;

        let mut un_notification = un::Notification::new()
            .title(&notification.summary)
            .message(&notification.body);

        if let Some(ref subtitle) = notification.subtitle {
            un_notification = un_notification.subtitle(subtitle);
        }

        if let Some(ref sound_name) = notification.sound_name {
            un_notification = un_notification.sound(sound_name);
        }

        for chunk in notification.actions.chunks(2) {
            if let (Some(id), Some(label)) = (chunk.first(), chunk.get(1)) {
                un_notification = un_notification.action(un::Action::new(id, label));
            }
        }

        // Forward a millisecond timeout from notify-rust to the UN layer so that
        // the future doesn't block indefinitely when the notification is cleared
        // without interaction (e.g. "Clear All" in Notification Center).
        if let Timeout::Milliseconds(ms) = notification.timeout {
            un_notification = un_notification.timeout(Duration::from_millis(ms as u64));
        }
        un_notification
    }
}

#[cfg(not(feature = "macos_pure_unusernotification_center"))]
pub(crate) fn show_notification(notification: &Notification) -> Result<NotificationHandle> {
    let mut n = mac_notification_sys::Notification::default();
    n.title(notification.summary.as_str())
        .message(&notification.body)
        .maybe_subtitle(notification.subtitle.as_deref())
        .maybe_sound(notification.sound_name.as_deref());

    if let Some(ref image_path) = notification.path_to_image {
        n.content_image(image_path);
    }

    n.send()?;

    Ok(NotificationHandle::new(notification.clone()))
}

#[cfg(feature = "macos_pure_unusernotification_center")]
pub(crate) fn show_notification(notification: &Notification) -> Result<NotificationHandle> {
    use mac_notification_sys::un;
    let un_notification: un::Notification = notification.into();
    // Always use send_with_actions_blocking so that a response handler is
    // registered for every notification — close/dismiss events are then
    // observable even when no explicit action buttons are configured.
    let resp = un::send_with_actions_blocking(un_notification)?;
    Ok(NotificationHandle::from((notification.clone(), Some(resp))))
}

pub(crate) async fn show_notification_async(
    notification: &Notification,
) -> Result<NotificationHandle> {
    use mac_notification_sys::un;
    let un_notification: un::Notification = notification.into();
    // Always use send_with_actions so that a response handler is registered
    // for every notification — close/dismiss events are then observable even
    // when no explicit action buttons are configured.  The caller may drop
    // the returned handle (and its captured response) if they do not care.
    let resp = un::send_with_actions(un_notification).await?;
    Ok(NotificationHandle::from((notification.clone(), Some(resp))))
}

pub(crate) fn schedule_notification(
    notification: &Notification,
    delivery_date: f64,
) -> Result<NotificationHandle> {
    let mut n = mac_notification_sys::Notification::default();
    n.title(notification.summary.as_str())
        .message(&notification.body)
        .maybe_subtitle(notification.subtitle.as_deref())
        .maybe_sound(notification.sound_name.as_deref())
        .delivery_date(delivery_date);

    if let Some(ref image_path) = notification.path_to_image {
        n.content_image(image_path);
    }

    n.send()?;

    Ok(NotificationHandle::new(notification.clone()))
}
