//! Demonstrates concurrent actionable notifications on macOS.
//!
//! Both notifications are submitted together inside a single `block_on_main`
//! call so both banners appear at the same time.  The user can interact with
//! them in any order; `block_on_main` pumps `NSRunLoop` between polls until
//! both responses have been captured.
//!
//! NOTE: requires a valid app bundle — run via
//!   cargo bundle --example mac_actions_async && \
//!     open target/debug/bundle/osx/*.app

#![allow(unused_imports)]
use notify_rust::{ActionResponse, Notification};

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("this is a macOS only example — see `actions.rs` for the XDG version");
}

#[cfg(target_os = "macos")]
fn main() {
    use futures_lite::future::zip;
    use mac_usernotifications::block_on_main;

    notify_rust::request_auth_blocking().unwrap();

    let (result_a, result_b) = block_on_main(zip(
        Notification::new()
            .summary("click me (async)")
            .body("This action is handled asynchronously")
            .action("clicked_a", "OK")
            .show_async(),
        Notification::new()
            .summary("pick one (async)")
            .body("This menu has several options — handled asynchronously")
            .action("clicked_a", "button a")
            .action("clicked_b", "button b")
            .action("clicked_c", "button c")
            .show_async(),
    ));

    println!("Both notifications handled.");

    if let Ok(handle) = result_a {
        handle.wait_for_action(|action| match action {
            ActionResponse::Custom("clicked_a") => println!("clicked OK"),
            ActionResponse::Closed(_) => println!("notification A was closed"),
            ActionResponse::Custom(other) => println!("notification A — unknown action: {other}"),
        });
    }

    if let Ok(handle) = result_b {
        handle.wait_for_action(|action| match action {
            ActionResponse::Custom("clicked_a") => println!("clicked a"),
            ActionResponse::Custom("clicked_b") => println!("clicked b"),
            ActionResponse::Custom("clicked_c") => println!("clicked c"),
            ActionResponse::Closed(_) => println!("notification B was closed"),
            ActionResponse::Custom(other) => println!("notification B — unknown action: {other}"),
        });
    }

    println!("Done.");
}
