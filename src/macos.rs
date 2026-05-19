#![allow(missing_docs)]

/// Items that belong exclusively to the legacy `NSUserNotificationCenter` path.
#[cfg(not(feature = "pure_usernotifications"))]
pub mod legacy {
    use crate::{error::*, notification::Notification};
    use std::ops::{Deref, DerefMut};

    pub use mac_notification_sys::error::{
        ApplicationError, Error as MacOsError, NotificationError,
    };

    /// A handle to a shown notification.
    ///
    /// This keeps a connection alive to ensure actions work on certain desktops.
    #[derive(Debug)]
    pub struct NotificationHandle {
        notification: Notification,
    }

    impl NotificationHandle {
        #[allow(missing_docs)]
        pub fn new(notification: Notification) -> NotificationHandle {
            NotificationHandle { notification }
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
}

/// Items that belong exclusively to the `pure_usernotifications` path
/// (`UNUserNotificationCenter`).
#[cfg(feature = "pure_usernotifications")]
pub mod pure_usernotifications {
    use crate::{
        action::UserResponse,
        error::*, notification::Notification, ActionResponse, ActionResponseHandler, CloseHandler,
        CloseReason, Timeout,
    };
    pub use mac_usernotifications::{request_auth, request_auth_blocking, Error as MacOsError};
    use mac_usernotifications::{NotificationResponse, Sound};
    use std::{ops::Deref, time::Duration};

    /// A handle to a shown notification (`UNUserNotificationCenter` path).
    #[derive(Debug)]
    pub struct NotificationHandle {
        notification: Notification,
        /// `None` on the legacy (`NSUserNotificationCenter`) path where no
        /// response is available.
        response: Option<NotificationResponse>,
    }

    impl NotificationHandle {
        pub(crate) fn new(notification: Notification, response: NotificationResponse) -> Self {
            Self {
                notification,
                response: Some(response),
            }
        }

        pub fn wait_for_action(self, invocation_closure: impl ActionResponseHandler) {
            let action = self.response.as_ref().map_or(
                ActionResponse::Closed(CloseReason::Dismissed),
                response_to_action_response,
            );
            invocation_closure.call(&action);
        }

        /// Returns the user's response, or `None` if the notification was
        /// dismissed without interaction.
        ///
        /// On macOS this is not a true future — the response is already
        /// available synchronously after `show()` returns. The `async fn`
        /// signature is provided so call sites look identical across platforms.
        pub async fn response(&self) -> UserResponse {
            self.response_blocking()
        }

        /// Blocking version of [`response`](Self::response).
        pub fn response_blocking(&self) -> UserResponse {
            self.response.as_ref().map_or(
                UserResponse::Closed(CloseReason::Dismissed),
                |resp| {
                    if resp.is_dismiss_action() {
                        UserResponse::Closed(CloseReason::Dismissed)
                    } else if let Some(ref text) = resp.reply_text {
                        UserResponse::Reply(text.clone())
                    } else {
                        UserResponse::Action(resp.action_identifier.clone())
                    }
                },
            )
        }

        pub fn on_close<A>(self, handler: impl CloseHandler<A>) {
            let is_dismiss = self
                .response
                .as_ref()
                .map_or(true, |resp| resp.is_dismiss_action());
            if is_dismiss {
                handler.call(CloseReason::Dismissed);
            }
        }

        /// Re-send the notification in-place, preserving its id.
        ///
        /// Mutate the handle via `DerefMut` first to change title, body, etc.,
        /// then call `update()` to push the changes to Notification Center.
        pub fn update(&mut self) -> Result<()> {
            let notification_id = self
                .response
                .as_ref()
                .map(|resp| resp.notification_id.clone())
                .unwrap_or_default();
            self.notification.id = Some(crate::NotificationId::Mac(notification_id));
            show_notification_blocking(&self.notification)?;
            Ok(())
        }

        /// Async version of [`update`](Self::update).
        pub async fn update_async(&mut self) -> Result<()> {
            let notification_id = self
                .response
                .as_ref()
                .map(|resp| resp.notification_id.clone())
                .unwrap_or_default();
            self.notification.id = Some(crate::NotificationId::Mac(notification_id));
            show_notification_async(&self.notification).await?;
            Ok(())
        }
    }

    impl Deref for NotificationHandle {
        type Target = Notification;
        fn deref(&self) -> &Notification {
            &self.notification
        }
    }

    impl std::ops::DerefMut for NotificationHandle {
        fn deref_mut(&mut self) -> &mut Notification {
            &mut self.notification
        }
    }

    fn response_to_action_response(resp: &NotificationResponse) -> ActionResponse {
        if resp.is_dismiss_action() {
            ActionResponse::Closed(CloseReason::Dismissed)
        } else if let Some(ref text) = resp.reply_text {
            ActionResponse::Reply(text.clone())
        } else {
            ActionResponse::Action(resp.action_identifier.clone())
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
                un = un.sound(sound_name.as_str());
            }
            for chunk in n.actions.chunks(2) {
                if let (Some(id), Some(label)) = (chunk.first(), chunk.get(1)) {
                    un = un.action(mac_usernotifications::Action::new(id, label));
                }
            }
            if let Timeout::Milliseconds(ms) = n.timeout {
                un = un.timeout(Duration::from_millis(ms as u64));
            }
            if let Some(ref path) = n.path_to_image {
                un = un.image_path(path);
            }
            if let Some(crate::NotificationId::Mac(ref nid)) = n.id {
                un = un.id(nid);
            }
            un
        }
    }

    pub(crate) fn schedule_notification(
        notification: &Notification,
        delivery_date: f64,
    ) -> Result<NotificationHandle> {
        use std::time::{Duration, SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
        let delay = (delivery_date - now).max(0.1);
        let un: mac_usernotifications::Notification =
            mac_usernotifications::Notification::from(notification)
                .schedule_in(Duration::from_secs_f64(delay));
        let resp = mac_usernotifications::send_with_actions_blocking(un)?;
        Ok(NotificationHandle::new(notification.clone(), resp))
    }

    /// Send a fire-and-forget notification via `UNUserNotificationCenter`
    /// (synchronous wrapper).
    pub(crate) fn show_notification(notification: &Notification) -> Result<NotificationHandle> {
        show_notification_blocking(notification)
    }

    pub(crate) fn show_notification_blocking(
        notification: &Notification,
    ) -> Result<NotificationHandle> {
        let un = mac_usernotifications::Notification::from(notification);
        if notification.actions.is_empty() {
            let notification_id = mac_usernotifications::send_blocking(un)?;
            Ok(NotificationHandle::new(
                notification.clone(),
                NotificationResponse::dismissed(notification_id),
            ))
        } else {
            let resp = mac_usernotifications::send_with_actions_blocking(un)?;
            Ok(NotificationHandle::new(notification.clone(), resp))
        }
    }

    pub(crate) async fn show_notification_async(
        notification: &Notification,
    ) -> Result<NotificationHandle> {
        let un = mac_usernotifications::Notification::from(notification);
        if notification.actions.is_empty() {
            let notification_id = mac_usernotifications::send(un).await?;
            Ok(NotificationHandle::new(
                notification.clone(),
                NotificationResponse::dismissed(notification_id),
            ))
        } else {
            let resp = mac_usernotifications::send_with_actions(un).await?;
            Ok(NotificationHandle::new(notification.clone(), resp))
        }
    }
}

#[cfg(not(feature = "pure_usernotifications"))]
pub(crate) use legacy::{schedule_notification, show_notification};

#[cfg(feature = "pure_usernotifications")]
pub(crate) use pure_usernotifications::{
    schedule_notification, show_notification, show_notification_async,
};
