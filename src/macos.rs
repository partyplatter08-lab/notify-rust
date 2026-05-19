#![allow(missing_docs)]
//! macOS notification back-ends.
//!
//! Two implementations are available, selected at compile time via the
//! `pure_usernotifications` feature flag.
//!
//! | Feature | `legacy` (`NSUserNotificationCenter`) | `pure_usernotifications` (`UNUserNotificationCenter`) |
//! |---|---|---|
//! | Crate | `mac-notification-sys` | `mac-usernotifications` |
//! | macOS requirement | Any supported macOS | macOS 10.14+ |
//! | Bundle ID required | No | Yes (ad-hoc signature is enough) |
//! | `show()` resolves | After user responds (synchronous) | Once delivered (before user responds) |
//! | `response().await` | No — response already available on handle | Yes — true future, suspends until interaction |
//! | `response_blocking()` | Yes | Yes |
//! | `wait_for_action()` | Yes | Yes (blocks) |
//! | `on_close()` | Yes | Yes (blocks) |
//! | `update()` | Yes (re-sends by title, no stable ID) | Yes (re-sends by UUID) |
//! | `update_async()` | No | Yes |
//! | `notification_id()` | No | Yes |
//! | `close_delivered()` | No | Yes (via `notification_id()`) |
//! | Reply actions | No | Yes |
//! | Action buttons | Yes (single main button) | Yes (multiple) |
//! | Timeout support | No | Yes |
//! | Authorization request | No | Yes (`request_auth`) |


/// Items that belong exclusively to the legacy `NSUserNotificationCenter` path.
#[cfg(not(feature = "pure_usernotifications"))]
pub mod legacy {
    use crate::{
        action::UserResponse, error::*, notification::Notification, ActionResponse,
        ActionResponseHandler, CloseHandler, CloseReason,
    };
    use std::ops::{Deref, DerefMut};

    pub use mac_notification_sys::error::{
        ApplicationError, Error as MacOsError, NotificationError,
    };

    use mac_notification_sys::NotificationResponse as LegacyResponse;

    /// A handle to a sent notification (`NSUserNotificationCenter` path).
    ///
    /// `NSUserNotificationCenter` delivers the user's response synchronously
    /// inside `send()`, so by the time you hold this handle the response is
    /// already available.  There is no async path and no close-by-ID API on
    /// this legacy stack.
    #[derive(Debug)]
    pub struct NotificationHandle {
        notification: Notification,
        response: LegacyResponse,
    }

    impl NotificationHandle {
        pub(crate) fn new(notification: Notification, response: LegacyResponse) -> Self {
            Self {
                notification,
                response,
            }
        }

        /// Returns the user's response.
        ///
        /// Because `NSUserNotificationCenter` is synchronous, the response is
        /// already resolved — this is not a future.
        pub fn response_blocking(&self) -> UserResponse {
            legacy_response_to_user_response(&self.response)
        }

        /// Call `invocation_closure` with the action the user took.
        pub fn wait_for_action(self, invocation_closure: impl ActionResponseHandler) {
            invocation_closure.call(&legacy_response_to_action_response(&self.response));
        }

        /// Call `handler` if the notification was dismissed without interaction.
        pub fn on_close<A>(self, handler: impl CloseHandler<A>) {
            if matches!(
                legacy_response_to_action_response(&self.response),
                ActionResponse::Closed(_)
            ) {
                handler.call(CloseReason::Dismissed);
            }
        }

        /// Re-send the notification with any mutations applied via `DerefMut`.
        pub fn update(&mut self) -> Result<()> {
            show_notification(&self.notification)?;
            Ok(())
        }
    }

    impl Deref for NotificationHandle {
        type Target = Notification;

        fn deref(&self) -> &Notification {
            &self.notification
        }
    }

    impl DerefMut for NotificationHandle {
        fn deref_mut(&mut self) -> &mut Notification {
            &mut self.notification
        }
    }

    fn legacy_response_to_action_response(resp: &LegacyResponse) -> ActionResponse {
        match resp {
            LegacyResponse::ActionButton(label) => ActionResponse::Action(label.clone()),
            LegacyResponse::Click => ActionResponse::Action("default".to_owned()),
            LegacyResponse::None => ActionResponse::Closed(CloseReason::Dismissed),
        }
    }

    fn legacy_response_to_user_response(resp: &LegacyResponse) -> UserResponse {
        match resp {
            LegacyResponse::ActionButton(label) => UserResponse::Action(label.clone()),
            LegacyResponse::Click => UserResponse::Action("default".to_owned()),
            LegacyResponse::None => UserResponse::Closed(CloseReason::Dismissed),
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

        let response = n.send()?;

        Ok(NotificationHandle::new(notification.clone(), response))
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

        let response = n.send()?;

        Ok(NotificationHandle::new(notification.clone(), response))
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
    use mac_usernotifications::Sound;
    use std::{ops::Deref, time::Duration};

    /// A handle to a sent notification (`UNUserNotificationCenter` path).
    ///
    /// `show()` returns this handle as soon as macOS accepts the notification
    /// request — before the user has interacted.  Call
    /// [`response().await`](NotificationHandle::response) to wait for the
    /// user's response, or drop the handle to stop observing it (the
    /// notification stays visible; the response channel is cleaned up).
    #[derive(Debug)]
    pub struct NotificationHandle {
        notification: Notification,
        inner: mac_usernotifications::NotificationHandle,
    }

    impl NotificationHandle {
        pub(crate) fn new(
            notification: Notification,
            inner: mac_usernotifications::NotificationHandle,
        ) -> Self {
            Self {
                notification,
                inner,
            }
        }

        /// The notification's request identifier.
        ///
        /// Can be passed to [`mac_usernotifications::close_delivered`] or
        /// [`mac_usernotifications::cancel_pending`].
        pub fn notification_id(&self) -> &str {
            self.inner.notification_id()
        }

        /// Wait for the user's response.
        ///
        /// Returns as soon as the user interacts with the notification or the
        /// timeout elapses.  For fire-and-forget notifications (no actions)
        /// this resolves immediately with a dismissed response.
        pub async fn response(self) -> UserResponse {
            match self.inner.response().await {
                Ok(resp) => mac_response_to_user_response(&resp),
                Err(_) => UserResponse::Closed(CloseReason::Expired),
            }
        }

        /// Blocking version of [`response`](Self::response).
        pub fn response_blocking(self) -> UserResponse {
            match self.inner.response_blocking() {
                Ok(resp) => mac_response_to_user_response(&resp),
                Err(_) => UserResponse::Closed(CloseReason::Expired),
            }
        }

        /// Call `invocation_closure` with the action the user took.
        ///
        /// Blocks until the user responds or the timeout elapses.
        pub fn wait_for_action(self, invocation_closure: impl ActionResponseHandler) {
            let action = match self.inner.response_blocking() {
                Ok(ref resp) => mac_response_to_action_response(resp),
                Err(_) => ActionResponse::Closed(CloseReason::Expired),
            };
            invocation_closure.call(&action);
        }

        /// Call `handler` if the notification was dismissed without interaction.
        ///
        /// Blocks until the user responds or the timeout elapses.
        pub fn on_close<A>(self, handler: impl CloseHandler<A>) {
            if let ActionResponse::Closed(reason) = match self.inner.response_blocking() {
                Ok(ref resp) => mac_response_to_action_response(resp),
                Err(_) => ActionResponse::Closed(CloseReason::Expired),
            } {
                handler.call(reason);
            }
        }

        /// Re-send the notification in-place, preserving its id.
        ///
        /// Mutate the handle via `DerefMut` first to change title, body, etc.,
        /// then call `update()` to push the changes to Notification Center.
        pub fn update(&mut self) -> Result<()> {
            let nid = self.inner.notification_id().to_owned();
            self.notification.id = Some(crate::NotificationId::Mac(nid));
            show_notification_blocking(&self.notification)?;
            Ok(())
        }

        /// Async version of [`update`](Self::update).
        pub async fn update_async(&mut self) -> Result<()> {
            let nid = self.inner.notification_id().to_owned();
            self.notification.id = Some(crate::NotificationId::Mac(nid));
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

    fn mac_response_to_action_response(
        resp: &mac_usernotifications::NotificationResponse,
    ) -> ActionResponse {
        if resp.is_dismiss_action() {
            ActionResponse::Closed(CloseReason::Dismissed)
        } else if let Some(ref text) = resp.reply_text {
            ActionResponse::Reply(text.clone())
        } else {
            ActionResponse::Action(resp.action_identifier.clone())
        }
    }

    fn mac_response_to_user_response(
        resp: &mac_usernotifications::NotificationResponse,
    ) -> UserResponse {
        if resp.is_dismiss_action() {
            UserResponse::Closed(CloseReason::Dismissed)
        } else if let Some(ref text) = resp.reply_text {
            UserResponse::Reply(text.clone())
        } else {
            UserResponse::Action(resp.action_identifier.clone())
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
        let un = mac_usernotifications::Notification::from(notification)
            .schedule_in(Duration::from_secs_f64(delay));
        let inner = mac_usernotifications::send_and_wait_for_delivery_blocking(un)?;
        Ok(NotificationHandle::new(notification.clone(), inner))
    }

    pub(crate) fn show_notification(notification: &Notification) -> Result<NotificationHandle> {
        show_notification_blocking(notification)
    }

    pub(crate) fn show_notification_blocking(
        notification: &Notification,
    ) -> Result<NotificationHandle> {
        let un = mac_usernotifications::Notification::from(notification);
        let inner = mac_usernotifications::send_and_wait_for_delivery_blocking(un)?;
        Ok(NotificationHandle::new(notification.clone(), inner))
    }

    pub(crate) async fn show_notification_async(
        notification: &Notification,
    ) -> Result<NotificationHandle> {
        let un = mac_usernotifications::Notification::from(notification);
        let inner = mac_usernotifications::send_and_wait_for_delivery(un).await?;
        Ok(NotificationHandle::new(notification.clone(), inner))
    }
}

#[cfg(not(feature = "pure_usernotifications"))]
pub(crate) use legacy::{schedule_notification, show_notification};

#[cfg(feature = "pure_usernotifications")]
pub(crate) use pure_usernotifications::{
    schedule_notification, show_notification, show_notification_async,
};
