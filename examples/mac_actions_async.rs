//! Demonstrates non-blocking notification actions on macOS.
//!
//! Unlike `mac_actions.rs`, which blocks the calling thread while waiting for
//! the user to interact, this example uses `wait_for_action_async` so the main
//! thread is free to do other work while the notification is pending.  Both
//! notifications are shown immediately and the results are collected at the end
//! by joining the returned handles.

#![allow(unused_imports)]
use notify_rust::Notification;

#[cfg(any(target_os = "windows", all(unix, not(target_os = "macos"))))]
fn main() {
    println!("this is a macOS only example — see `actions.rs` for the XDG version");
}

#[cfg(target_os = "macos")]
fn main() {
    use notify_rust::{get_bundle_identifier_or_default, set_application};

    set_application(&get_bundle_identifier_or_default("safari")).unwrap();

    // Show two notifications concurrently; neither call blocks.
    let handle_a = Notification::new()
        .summary("click me (async)")
        .body("This action is handled asynchronously")
        .action("clicked_a", "OK")
        .show()
        .unwrap()
        .wait_for_action_async(|action| match action {
            "clicked_a" => println!("clicked OK"),
            "__closed" => println!("notification A was closed"),
            other => println!("notification A — unknown action: {other}"),
        });

    let handle_b = Notification::new()
        .summary("pick one (async)")
        .body("This menu has several options — handled asynchronously")
        .action("clicked_a", "button a")
        .action("clicked_b", "button b")
        .action("clicked_c", "button c")
        .show()
        .unwrap()
        .wait_for_action_async(|action| match action {
            "clicked_a" => println!("clicked a"),
            "clicked_b" => println!("clicked b"),
            "clicked_c" => println!("clicked c"),
            "__closed" => println!("notification B was closed"),
            other => println!("notification B — unknown action: {other}"),
        });

    println!("Both notifications are now visible — waiting for user interaction…");

    // Wait for both interactions to complete before exiting.
    handle_a.join().unwrap();
    handle_b.join().unwrap();

    println!("Done.");
}
