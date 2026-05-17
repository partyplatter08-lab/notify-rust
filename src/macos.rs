#![allow(missing_docs)]
use crate::{error::*, notification::Notification, CloseHandler, CloseReason, Timeout};

pub use mac_notification_sys::error::{ApplicationError, Error as MacOsError, NotificationError};
use mac_notification_sys::un::{self, NotificationResponse as UnNotificationResponse};
use mac_usernotifications::{NotificationResponse, Sound};

use std::{ops::Deref, time::Duration};

#[derive(Debug)]
pub struct NotificationHandle {
    notification: Notification,
    response: NotificationResponse,
}

impl NotificationHandle {
    fn new(notification: Notification, response: NotificationResponse) -> Self {
        Self {
            notification,
            response,
        }
    }

    pub fn wait_for_action<F>(self, invocation_closure: F)
    where
        F: FnOnce(&str),
    {
        let identifier = self.response.action_identifier.as_str();
        invocation_closure(identifier);
    }

    pub fn on_close<A>(self, handler: impl CloseHandler<A>) {
        handler.call(CloseReason::Dismissed);
    }
}

impl Deref for NotificationHandle {
    type Target = Notification;
    fn deref(&self) -> &Notification {
        &self.notification
    }
}

/// TODO: we don't need this if we just remove `__closed`
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
    fn from(n: &Notification) -> Self {
        let mut un = un::Notification::new().title(&n.summary).message(&n.body);

        if let Some(ref subtitle) = n.subtitle {
            un = un.subtitle(subtitle);
        }
        if let Some(ref sound_name) = n.sound_name {
            un = un.sound(sound_name);
        }
        for chunk in n.actions.chunks(2) {
            if let (Some(id), Some(label)) = (chunk.first(), chunk.get(1)) {
                un = un.action(un::Action::new(id, label));
            }
        }
        if let Timeout::Milliseconds(ms) = n.timeout {
            un = un.timeout(Duration::from_millis(ms as u64));
        }
        un
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
#[cfg(not(feature = "macos_pure_unusernotification_center"))]
pub(crate) fn show_notification(notification: &Notification) -> Result<()> {
    let mut n = mac_notification_sys::Notification::default();
    n.title(notification.summary.as_str())
        .message(&notification.body)
        .maybe_subtitle(notification.subtitle.as_deref())
        .maybe_sound(notification.sound_name.as_deref());

    if let Some(ref image_path) = notification.path_to_image {
        n.content_image(image_path);
    }
    n.send()?;
    Ok(())
}

/// Send a fire-and-forget notification via `UNUserNotificationCenter`
/// (synchronous wrapper, `macos_pure_unusernotification_center` feature).
#[cfg(feature = "macos_pure_unusernotification_center")]
pub(crate) fn show_notification(notification: &Notification) -> Result<()> {
    // un::send_blocking(notification.into())?;
    mac_usernotifications::send_blocking(notification.into())?;
    Ok(())
}

/// Send a notification via `UNUserNotificationCenter` and wait for the user
/// to respond.
///
/// For notifications **with** action buttons the user must click one (or
/// dismiss with a swipe).  For notifications **without** action buttons a
/// synthetic category is registered internally so that swipe-to-dismiss also
/// delivers a response — the notification looks identical to a fire-and-forget
/// banner but its lifecycle is still fully observable.
///
/// Returns a [`NotificationHandle`] whose [`wait_for_action`] / [`on_close`]
/// methods never block — the response is already captured by the time this
/// future resolves.
///
/// The future resolves to `Err(`[`un::Error::ResponseTimeout`]`)` if a timeout
/// was configured on the notification and the deadline passes.
///
/// [`wait_for_action`]: NotificationHandle::wait_for_action
/// [`on_close`]: NotificationHandle::on_close
pub(crate) async fn show_notification_async(
    notification: &Notification,
) -> Result<NotificationHandle> {
    let resp = mac_usernotifications::send_with_actions(notification.into()).await?;
    Ok(NotificationHandle::new(notification.clone(), resp))
}

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
