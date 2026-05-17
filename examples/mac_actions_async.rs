use notify_rust::{ActionResponse, Notification};

#[cfg(target_os = "macos")]
fn main() {
    use futures_lite::future::zip;
    use mac_usernotifications::block_on_main;

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
}

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("this is a macOS only example — see `actions.rs` for the XDG version");
}
