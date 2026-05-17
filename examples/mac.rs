fn main() -> Result<(), Box<dyn std::error::Error>> {
    use notify_rust::{ActionResponse, Notification};

    cfg_select! {
        feature = "pure_usernotifications" => {

            notify_rust::request_auth_blocking().unwrap();
        }
        not(feature = "pure_usernotifications") => {
            let bundle_id = notify_rust::get_bundle_identifier_or_default("zed");
            notify_rust::set_application(&bundle_id).unwrap();
        }
    }

    Notification::new()
        .summary("Safari Crashed")
        .body("Just kidding, this is just the notify_rust example.")
        .appname("Toastify")
        .icon("Toastify")
        .show()?;

    Notification::new()
        .summary(".image_path()")
        .body("Trying to open an image")
        .image_path("./examples/octodex.jpg")
        .show()?;

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
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("this is a mac only example")
}
