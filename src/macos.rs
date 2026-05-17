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
        error::*, notification::Notification, ActionResponse, CloseHandler, CloseReason, Timeout,
    };
    use mac_usernotifications::{NotificationResponse, Sound};
    use std::{ops::Deref, time::Duration};

    pub use mac_usernotifications::{request_auth, request_auth_blocking, Error as MacOsError};

    /// A handle to a shown notification (UNUserNotificationCenter path).
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

    /// Send a fire-and-forget notification via `UNUserNotificationCenter`
    /// (synchronous wrapper).
    pub(crate) fn show_notification(notification: &Notification) -> Result<NotificationHandle> {
        show_notification_blocking(notification)
    }

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
}

#[cfg(not(feature = "pure_usernotifications"))]
pub(crate) use legacy::schedule_notification;
#[cfg(not(feature = "pure_usernotifications"))]
pub(crate) use legacy::show_notification;

#[cfg(feature = "pure_usernotifications")]
pub(crate) use pure_usernotifications::show_notification;
#[cfg(feature = "pure_usernotifications")]
pub(crate) use pure_usernotifications::show_notification_async;
