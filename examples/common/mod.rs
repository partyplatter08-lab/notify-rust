#![allow(dead_code)]

#[cfg(target_os = "macos")]
pub fn setup() -> bool {
    cfg_select! {
        feature = "pure_usernotifications" => {
            oslog::OsLogger::new("notify-rust")
                .level_filter(log::LevelFilter::Debug)
                .init()
                .unwrap();

            match notify_rust::request_auth_blocking() {
                Ok(true) => {
                    log::info!("Notification permission granted.");
                    true
                }
                Ok(false) => {
                    log::warn!(
                        "Notification permission denied. \
                         Allow it in System Settings -> Notifications."
                    );
                    false
                }
                Err(error) => {
                    log::error!("Authorization error: {error}");
                    false
                }
            }
        }
        not(feature = "pure_usernotifications") => {
            let bundle_id = notify_rust::get_bundle_identifier_or_default("zed");
            notify_rust::set_application(&bundle_id).unwrap();
            true
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn setup() {
    #[cfg(feature = "env_logger")]
    env_logger::init();
}

pub fn wait_for_keypress(msg: &str) {
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    log::info!("{msg}");

    let (sender, receiver) = mpsc::channel();

    let timeout_sender = sender.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(4));
        let _ = timeout_sender.send(());
    });

    // NOTE: the stdin thread will keep blocking after the timeout.
    // There is no portable way to cancel a blocking read, so
    // the thread is intentionally leaked here. This is fine for an example.
    thread::spawn(move || {
        let mut line = String::new();
        if std::io::stdin().read_line(&mut line).is_ok() && !line.is_empty() {
            let _ = sender.send(());
        }
    });

    let _ = receiver.recv();
}
