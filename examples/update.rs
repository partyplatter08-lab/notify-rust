mod common;

use std::time::Duration;

#[cfg(target_os = "windows")]
fn main() {
    println!("this is not a windows feature")
}

#[cfg(any(
    all(unix, not(target_os = "macos")),
    all(target_os = "macos", feature = "pure_usernotifications")
))]
fn main() {
    if !common::setup() {
        return;
    }

    // use the handle to update a notification
    update_via_handle();

    // or store the id yourself
    update_via_stored_id();
}

#[cfg(any(
    all(unix, not(target_os = "macos")),
    all(target_os = "macos", feature = "pure_usernotifications")
))]
fn update_via_handle() {
    use notify_rust::Notification;

    let mut notification_handle = Notification::new()
        .summary("First Notification")
        .body("This notification will be changed!")
        .icon("dialog-warning")
        .show()
        .unwrap();

    std::thread::sleep(Duration::from_millis(1_500));

    notification_handle
        .appname("foo") // changing appname keeps plasma from merging old and new
        .icon("dialog-ok")
        .body("<b>This</b> has been changed through the handle");

    notification_handle.update().unwra
    p();
}

#[cfg(any(
    all(unix, not(target_os = "macos")),
    all(target_os = "macos", feature = "pure_usernotifications")
))]
fn update_via_stored_id() {
    use notify_rust::Notification;

    let handle = Notification::new()
        .summary("First Notification")
        .body("This notification will be changed!")
        .icon("dialog-warning")
        .show()
        .unwrap();

    let stored_id = handle.id();
    std::thread::sleep(Duration::from_millis(1_500));

    Notification::new()
        .appname("foo") // changing appname keeps plasma from merging old and new
        .icon("dialog-ok")
        .body("<b>This</b> has been changed by sending a new notification with the same id")
        .id(stored_id)
        .show()
        .unwrap();
}

#[cfg(all(target_os = "macos", not(feature = "pure_usernotifications")))]
fn main() {
    println!("this example requires the `pure_usernotifications` feature on macOS");
}
