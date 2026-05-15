use mac_notification_sys::un::request_auth;

#[cfg(target_os = "windows")]
fn main() {
    println!("this is an xdg only feature")
}

#[cfg(unix)]
#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use notify_rust::Notification;

    #[cfg(target_os = "macos")]
    request_auth().await?;

    Notification::new()
        .summary("async notification")
        .subtitle("subtitle")
        .body("this notification was sent via an async api")
        .icon("dialog-positive")
        .show_async()
        .await?;
    Ok(())
}
