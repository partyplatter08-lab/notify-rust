#[cfg(all(feature = "pure_usernotifications", target_os = "macos"))]
fn main() {
    use futures_lite::future::zip;
    use mac_usernotifications::block_on_main;
    use notify_rust::{Notification, UserResponse};

    cfg_select! {
        feature = "pure_usernotifications" => {
            notify_rust::request_auth_blocking().unwrap();
        }
        not(feature = "pure_usernotifications") => {
            let bundle_id = notify_rust::get_bundle_identifier_or_default("zed");
            notify_rust::set_application(&bundle_id).unwrap();
        }
    }

    // a bundled app can not log to stdout
    oslog::OsLogger::new("notify-rust")
        .level_filter(log::LevelFilter::Debug)
        .init()
        .unwrap();

    // Send all notifications concurrently and collect their handles.
    let ((result_plain, result_image), (result_a, result_b)) = block_on_main(zip(
        zip(
            Notification::new()
                .summary("Safari Crashed")
                .body("Just kidding, this is just the notify_rust example.")
                .appname("Toastify")
                .icon("Toastify")
                .show_async(),
            Notification::new()
                .summary(".image_path()")
                .body("Trying to open an image")
                .image_path("./examples/octodex.jpg")
                .show_async(),
        ),
        zip(
            Notification::new()
                .summary("click me (async)")
                .body("This action needs to be clicked")
                .action("clicked_a", "OK")
                .show_async(),
            Notification::new()
                .summary("pick one (async)")
                .body("This menu has several options")
                .action("clicked_a", "button a")
                .action("clicked_b", "button b")
                .action("clicked_c", "button c")
                .show_async(),
        ),
    ));

    if let Err(e) = result_plain {
        eprintln!("plain notification failed: {e}");
    }
    if let Err(e) = result_image {
        eprintln!("image notification failed: {e}");
    }

    if let Ok(handle) = result_a {
        match handle.response_blocking() {
            UserResponse::Action(key) if key == "clicked_a" => println!("clicked OK"),
            UserResponse::Action(other) => println!("notification A: unknown action: {other}"),
            UserResponse::Reply(text) => println!("notification A: reply: {text}"),
            UserResponse::Closed(_) => println!("notification A was closed"),
        }
    }

    if let Ok(handle) = result_b {
        match handle.response_blocking() {
            UserResponse::Action(key) if key == "clicked_a" => println!("clicked a"),
            UserResponse::Action(key) if key == "clicked_b" => println!("clicked b"),
            UserResponse::Action(key) if key == "clicked_c" => println!("clicked c"),
            UserResponse::Action(other) => println!("notification B - unknown action: {other}"),
            UserResponse::Reply(text) => println!("notification B - reply: {text}"),
            UserResponse::Closed(_) => println!("notification B was closed"),
        }
    }
}

#[cfg(all(not(feature = "pure_usernotifications"), target_os = "macos"))]
fn main() {
    println!("this example requires the `pure_usernotifications` feature")
}

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("this is a macOS only example - see `actions.rs` for the XDG version");
}
