//! Demonstrates `on_close` on macOS using `UNUserNotificationCenter`.
//!
//! Important: the UN API only fires a dismiss callback when the notification
//! has at least one action button.  A "Dismiss" button is added here so that
//! swiping the banner away also triggers the callback.
//!
//! Requires a valid app bundle:
//!   cargo bundle --example on_close && open target/debug/bundle/osx/*.app

#![allow(unused_imports, dead_code)]
use notify_rust::Notification;

#[cfg(target_os = "windows")]
fn main() {
    println!("this is a xdg/macos only feature")
}

#[cfg(all(unix, not(target_os = "macos")))]
fn main() {
    use std::{io, thread};

    fn wait_for_keypress() {
        println!("halted until you hit the \"ANY\" key");
        io::stdin().read_line(&mut String::new()).unwrap();
    }

    fn print() {
        println!("notification was closed, don't know why");
    }

    thread::spawn(|| {
        Notification::new()
            .summary("Time is running out")
            .body("This will go away.")
            .icon("clock")
            .show()
            .map(|handler| handler.on_close(print))
    });
    wait_for_keypress();
}

#[cfg(all(feature = "pure_usernotifications", target_os = "macos"))]
fn main() {
    use mac_usernotifications::block_on_main;

    oslog::OsLogger::new("de.hoodie.notify-rust.example")
        .level_filter(log::LevelFilter::Debug)
        .init()
        .unwrap();

    block_on_main(async {
        // Request notification permission.  request_auth returns Ok(bool) —
        // Ok(false) means the user denied permission; that is NOT an Err and
        // must be handled explicitly or show_async will fail silently.
        match mac_usernotifications::request_auth().await {
            Ok(true) => log::info!("notification permission granted"),
            Ok(false) => {
                log::error!("notification permission denied — allow in System Settings → Notifications");
                return;
            }
            Err(e) => {
                log::error!("auth error: {e}");
                return;
            }
        }

        // show_async sends the notification and waits for the user to
        // interact.  A synthetic dismiss-tracking category is registered
        // internally, so swiping the banner away delivers the response even
        // without explicit action buttons.  The extra "Dismiss" button below
        // is optional but gives the user a visible affordance.
        let handle = match Notification::new()
            .summary("Time is running out")
            .body("This will go away.")
            .action("dismiss", "Dismiss")
            .show_async()
            .await
        {
            Ok(h) => h,
            Err(e) => {
                log::error!("failed to show notification: {e}");
                return;
            }
        };

        // on_close never blocks — the response is already captured.
        // CloseHandler is a sync trait; async follow-up work goes after.
        handle.on_close(|| log::info!("notification was closed"));

        if let Err(e) = Notification::new()
            .summary("Done")
            .body("Notification was closed.")
            .show_async()
            .await
        {
            log::error!("follow-up notification failed: {e}");
        }
    });
}

#[cfg(all(not(feature = "pure_usernotifications"), target_os = "macos"))]
fn main() {
    println!("this example requires the `pure_usernotifications` feature on macOS")
}
