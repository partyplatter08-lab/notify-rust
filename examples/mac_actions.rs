#[cfg(any(target_os = "windows", all(unix, not(target_os = "macos"))))]
fn main() {
    println!("this is a macOS only example — see `actions.rs` for the XDG version");
}

#[cfg(target_os = "macos")]
fn main() {
    use notify_rust::{get_bundle_identifier_or_default, set_application, Notification};

    set_application(&get_bundle_identifier_or_default("safari")).unwrap();

    // Both notifications are dispatched up-front.  Because `wait_for_action_async`
    // returns immediately, neither call blocks the main thread — both banners
    // appear at the same time and can be dismissed in any order.

    // A single action: rendered as a single button on the notification.
    let handle_a = Notification::new()
        .summary("click me")
        .body("This action needs to be clicked")
        .action("clicked_a", "OK") // IDENTIFIER, LABEL
        .show()
        .unwrap()
        .wait_for_action_async(|action| match action {
            "clicked_a" => println!("clicked OK"),
            // FIXME: here "__closed" is a hardcoded keyword, it will be deprecated in 5.0
            "__closed" => println!("the notification was closed"),
            other => println!("unknown action: {other}"),
        });

    // Multiple actions: rendered as a dropdown attached to the main button.
    let handle_b = Notification::new()
        .summary("pick one")
        .body("This menu has several options")
        .action("clicked_a", "button a") // IDENTIFIER, LABEL
        .action("clicked_b", "button b")
        .action("clicked_c", "button c")
        .show()
        .unwrap()
        .wait_for_action_async(|action| match action {
            "clicked_a" => println!("clicked a"),
            "clicked_b" => println!("clicked b"),
            "clicked_c" => println!("clicked c"),
            "__closed" => println!("the notification was closed"),
            other => println!("unknown action: {other}"),
        });

    // This line prints *before* either notification is dismissed, proving
    // the main thread was never blocked.
    println!("both notifications are visible — main thread is free");

    handle_a.join().unwrap();
    handle_b.join().unwrap();
}
