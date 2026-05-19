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
