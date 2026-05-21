//! Demonstrates waiting for a notification to be dismissed on macOS using
//! `UNUserNotificationCenter`.
//!
//! `response().await` resolves to `UserResponse::Closed` when the user swipes
//! the banner away or clicks a "Dismiss" button.  At least one action button
//! must be present for macOS to deliver a dismiss event; without actions the
//! notification centre swallows the dismiss silently.
//!
//! Requires a valid app bundle:
//!   cargo bundle --example on_close && open target/debug/bundle/osx/*.app

#![allow(unused_imports, dead_code)]
use notify_rust::Notification;
#[cfg(all(target_os = "macos", not(feature = "macos_legacy")))]
use notify_rust::UserResponse;

mod common;

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

#[cfg(all(target_os = "macos", feature = "macos_legacy"))]
fn main() {
    println!("this example requires the default macOS backend (UNUserNotificationCenter)");
}

#[cfg(all(target_os = "macos", not(feature = "macos_legacy")))]
async_main!(async {
    // show_async sends the notification and waits for delivery.
    // response().await then suspends until the user interacts.
    // A "Dismiss" button gives the user a visible affordance and ensures
    // macOS delivers the dismiss event.
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

    match handle.response().await {
        UserResponse::Closed(_) => log::info!("notification was closed"),
        UserResponse::Action(key) => log::info!("action invoked: {key}"),
        UserResponse::Reply(text) => log::info!("reply: {text}"),
    }

    if let Err(e) = Notification::new()
        .summary("Done")
        .body("Notification was closed.")
        .show_async()
        .await
    {
        log::error!("follow-up notification failed: {e}");
    }
});
