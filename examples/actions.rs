#![allow(unused_imports)]
use notify_rust::{Hint, Notification, Timeout, UserResponse};
mod common;

#[cfg(target_os = "windows")]
fn main() {
    log::info!("this is a xdg only feature");
}

// linux and mac, but on mac only with `"pure_usernotifications"` feature
#[cfg(any(
    target_os = "linux",
    all(target_os = "macos", feature = "pure_usernotifications")
))]
fn main() {
    common::setup();
    Notification::new()
        .summary("click me")
        .body("This will disappear by itself")
        .action("clicked_a", "button a") // IDENTIFIER, LABEL
        .hint(Hint::Transient(true)) // needed to work on kde
        .show()
        .unwrap()
        .wait_for_action(|action| match action {
            "clicked_a" => log::info!("clicked a"),
            // FIXME: here "__closed" is a hardcoded keyword, it will be deprecated!!
            "__closed" => log::info!("the notification was closed"),
            _ => (),
        });

    Notification::new()
        .summary("click me")
        .body("This action needs to be clicked")
        .action("default", "default") // IDENTIFIER, LABEL
        .action("clicked_a", "button a") // IDENTIFIER, LABEL
        .action("clicked_b", "button b") // IDENTIFIER, LABEL
        .hint(Hint::Resident(true)) // does not work on kde
        .timeout(Timeout::Never) // works on kde and gnome
        .show()
        .unwrap()
        .wait_for_action(|action| match action {
            "default" => log::info!("default"),
            "clicked_a" => log::info!("clicked a"),
            "clicked_b" => log::info!("clicked b"),
            // FIXME: here "__closed" is a hardcoded keyword, it will be deprecated!!
            "__closed" => log::info!("the notification was closed"),
            _ => (),
        });

    // new API: response_blocking() returns a UserResponse directly
    match Notification::new()
        .summary("click me")
        .body("Using the new response API")
        .action("default", "default")
        .action("clicked_a", "button a")
        .action("clicked_b", "button b")
        .hint(Hint::Resident(true))
        .timeout(Timeout::Never)
        .show()
        .unwrap()
        .response_blocking()
    {
        UserResponse::Action(key) if key == "default" => log::info!("default"),
        UserResponse::Action(key) if key == "clicked_a" => log::info!("clicked a"),
        UserResponse::Action(key) if key == "clicked_b" => log::info!("clicked b"),
        UserResponse::Action(other) => log::info!("unknown action: {other}"),
        UserResponse::Reply(text) => log::info!("replied: {text}"),
        UserResponse::Closed(reason) => log::info!("closed: {reason:?}"),
    }
}
