#![allow(missing_docs)]
use crate::{
    error::*, notification::Notification, ActionResponse, CloseHandler, CloseReason, Timeout,
};

#[cfg(not(feature = "pure_usernotifications"))]
pub use mac_notification_sys::error::{ApplicationError, Error as MacOsError, NotificationError};

use mac_usernotifications::{NotificationResponse, Sound};
#[cfg(not(feature = "pure_usernotifications"))]
use mac_usernotifications::{NotificationResponse, Sound};

#[cfg(feature = "pure_usernotifications")]
pub use mac_usernotifications::{request_auth, request_auth_blocking, Error as MacOsError};

use std::{ops::Deref, time::Duration};

#[derive(Debug)]
pub struct NotificationHandle {
    notification: Notification,
    /// `None` on the legacy (`NSUserNotificationCenter`) path where no
    /// response is available.
    response: Option<NotificationResponse>,
}

impl NotificationHandle {
    fn new(notification: Notification, response: NotificationResponse) -> Self {
        Self {
            notification,
            response: Some(response),
        }
    }

    #[cfg_attr(feature = "pure_usernotifications", allow(dead_code))]
    fn new_legacy(notification: Notification) -> Self {
        Self {
            notification,
            response: None,
        }
    }

    pub fn wait_for_action<F>(self, invocation_closure: F)
    where
        F: FnOnce(&ActionResponse),
    {
        let action = self.response.as_ref().map_or(
            ActionResponse::Closed(CloseReason::Dismissed),
            response_to_action_response,
        );
        invocation_closure(&action);
    }

    pub async fn wait_for_action_async<F>(self, invocation_closure: F)
    where
        F: FnOnce(&ActionResponse),
    {
        let action = self.response.as_ref().map_or(
            ActionResponse::Closed(CloseReason::Dismissed),
            response_to_action_response,
        );
        invocation_closure(&action);
    }

    pub fn on_close<A>(self, handler: impl CloseHandler<A>) {
        let is_dismiss = self
            .response
            .as_ref()
            .map_or(true, |r| r.is_dismiss_action());
        if is_dismiss {
            handler.call(CloseReason::Dismissed);
        }
    }
}

impl Deref for NotificationHandle {
    type Target = Notification;
    fn deref(&self) -> &Notification {
        &self.notification
    }
}

fn response_to_action_response(resp: &NotificationResponse) -> ActionResponse<'_> {
    if resp.is_dismiss_action() {
        ActionResponse::Closed(CloseReason::Dismissed)
    } else if let Some(ref text) = resp.reply_text {
        ActionResponse::Custom(text.as_str())
    } else {
        ActionResponse::Custom(resp.action_identifier.as_str())
    }
}

impl From<&Notification> for mac_usernotifications::Notification {
    fn from(n: &Notification) -> Self {
        let mut un = mac_usernotifications::Notification::new()
            .title(&n.summary)
            .message(&n.body)
            .maybe_subtitle(n.subtitle.as_deref())
            .maybe_sound(n.sound_name.clone().map(Sound::Custom));

        if let Some(ref sound_name) = n.sound_name {
            un = un.sound(sound_name);
        }
        for chunk in n.actions.chunks(2) {
            if let (Some(id), Some(label)) = (chunk.first(), chunk.get(1)) {
                un = un.action(mac_usernotifications::Action::new(id, label));
            }
        }
        if let Timeout::Milliseconds(ms) = n.timeout {
            un = un.timeout(Duration::from_millis(ms as u64));
        }
        un
    }
}

/// Send a fire-and-forget notification via the legacy `NSUserNotificationCenter`
/// API (deprecated macOS ≤ 13, removed macOS 14).  Prefer
/// [`show_notification_async`] on modern systems.
/// **OLD VERSION**
#[cfg(not(feature = "pure_usernotifications"))]
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
    Ok(NotificationHandle::new_legacy(notification.clone()))
}

/// Send a fire-and-forget notification via `UNUserNotificationCenter`
/// (synchronous wrapper, `pure_usernotifications` feature).
#[cfg(feature = "pure_usernotifications")]
pub(crate) fn show_notification(notification: &Notification) -> Result<NotificationHandle> {
    show_notification_blocking(notification)
}

#[cfg(feature = "pure_usernotifications")]
pub(crate) fn show_notification_blocking(
    notification: &Notification,
) -> Result<NotificationHandle> {
    let resp = mac_usernotifications::send_with_actions_blocking(notification.into())?;
    Ok(NotificationHandle::new(notification.clone(), resp))
}

pub(crate) async fn show_notification_async(
    notification: &Notification,
) -> Result<NotificationHandle> {
    let resp = mac_usernotifications::send_with_actions(notification.into()).await?;
    Ok(NotificationHandle::new(notification.clone(), resp))
}

#[cfg(not(feature = "pure_usernotifications"))]
pub(crate) fn schedule_notification(notification: &Notification, delivery_date: f64) -> Result<()> {
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
    Ok(())
}
