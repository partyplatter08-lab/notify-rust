//! Demonstrates concurrent actionable notifications on macOS using the
//! `UNUserNotificationCenter` API.
//!
//! Both notifications are submitted together inside a single `block_on_main`
//! call that drives the future and pumps `NSRunLoop`.  They appear at the same
//! time; the user can dismiss them in any order.  The main thread is never
//! blocked on just one notification while the other is still visible.
//!
//! NOTE: requires a valid app bundle — run via
//!   cargo bundle --example mac_actions && \
//!     open target/debug/bundle/osx/*.app

#![allow(unused_imports)]
use notify_rust::Notification;

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("this is a macOS only example — see `actions.rs` for the XDG version");
}

#[cfg(target_os = "macos")]
fn main() {
    use futures_lite::future::zip;
    use mac_notification_sys::un::block_on_main;

    // Both show_and_wait_async futures run concurrently inside zip; block_on_main
    // polls both while pumping NSRunLoop, so both banners appear at once.
    let (result_a, result_b) = block_on_main(zip(
        Notification::new()
            .summary("click me")
            .body("This action needs to be clicked")
            .action("clicked_a", "OK")
            .show_async(),
        Notification::new()
            .summary("pick one")
            .body("This menu has several options")
            .action("clicked_a", "button a")
            .action("clicked_b", "button b")
            .action("clicked_c", "button c")
            .show_async(),
    ));

    // By the time we reach here the user has interacted with both notifications.
    // wait_for_action just reads the already-captured response — it never blocks.
    if let Ok(handle) = result_a {
        handle.wait_for_action(|action| match action {
            "clicked_a" => println!("clicked OK"),
            "__closed" => println!("the notification was closed"),
            other => println!("unknown action: {other}"),
        });
    }

    if let Ok(handle) = result_b {
        handle.wait_for_action(|action| match action {
            "clicked_a" => println!("clicked a"),
            "clicked_b" => println!("clicked b"),
            "clicked_c" => println!("clicked c"),
            "__closed" => println!("the notification was closed"),
            other => println!("unknown action: {other}"),
        });
    }
}
