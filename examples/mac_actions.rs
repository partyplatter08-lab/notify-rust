//! Demonstrates actionable notifications on macOS using the blocking API.
//!
//! Each notification is sent and waited on sequentially — `show()` blocks
//! until the user responds before the next notification is shown.
//!
//! For a concurrent (async) version see `mac_actions_async.rs`.
//!
//! NOTE: requires a valid app bundle — run via
//!   cargo bundle --example mac_actions && \
//!     open target/debug/bundle/osx/*.app

use notify_rust::{ActionResponse, Notification};

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("this is a macOS only example — see `actions.rs` for the XDG version");
}

#[cfg(target_os = "macos")]
fn main() {
    notify_rust::request_auth_blocking().unwrap();

    let result_a = Notification::new()
        .summary("click me")
        .body("This action needs to be clicked")
        .action("clicked_a", "OK")
        .show();

    if let Ok(handle) = result_a {
        handle.wait_for_action(|action| match action {
            ActionResponse::Custom("clicked_a") => println!("clicked OK"),
            ActionResponse::Closed(_) => println!("the notification was closed"),
            ActionResponse::Custom(other) => println!("unknown action: {other}"),
        });
    }

    let result_b = Notification::new()
        .summary("pick one")
        .body("This menu has several options")
        .action("clicked_a", "button a")
        .action("clicked_b", "button b")
        .action("clicked_c", "button c")
        .show();

    if let Ok(handle) = result_b {
        handle.wait_for_action(|action| match action {
            ActionResponse::Custom("clicked_a") => println!("clicked a"),
            ActionResponse::Custom("clicked_b") => println!("clicked b"),
            ActionResponse::Custom("clicked_c") => println!("clicked c"),
            ActionResponse::Closed(_) => println!("the notification was closed"),
            ActionResponse::Custom(other) => println!("unknown action: {other}"),
        });
    }
}
