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
//! | `show()` resolves | After user responds (blocking) | Once delivered (before user responds) |
//! | `response().await` | No | Yes — true async, suspends until interaction |
//! | `response_blocking()` | No | Yes |
//! | `wait_for_action()` | Yes (deprecated) | Yes (deprecated) |
//! | `wait_for_action_response()` | No | Yes |
//! | `on_close()` | No | Yes |
//! | `update()` | No | Yes (re-sends by UUID) |
//! | `update_async()` | No | Yes |
//! | `notification_id()` | No | Yes |
//! | `close()` | No | Yes |
//! | Reply actions | No | Yes |
//! | Action buttons | No | Yes (multiple) |
//! | Timeout support | No | Yes |
//! | Authorization request | No | Yes (`request_auth`) |

/// Items that belong exclusively to the legacy `NSUserNotificationCenter` path.
///
/// Enable the `macos_legacy` feature to activate this module.
#[cfg(feature = "macos_legacy")]
mod nsusernotification;

#[cfg(feature = "macos_legacy")]
pub use nsusernotification::{schedule_notification, show_notification};

#[cfg(not(feature = "macos_legacy"))]
mod usernotifications;
pub use usernotifications::{MacOsError, NotificationHandle};

/// The default macOS backend: `UNUserNotificationCenter` (`mac-usernotifications`).
///
/// This is the only available backend unless the `macos_legacy` feature is enabled.
#[cfg(not(feature = "macos_legacy"))]
pub use mac_usernotifications::*;

#[cfg(feature = "macos_legacy")]
pub use mac_notification_sys::{get_bundle_identifier_or_default, set_application};
