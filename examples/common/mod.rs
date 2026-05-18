#[cfg(target_os = "macos")]
pub fn setup() {
    oslog::OsLogger::new("mac-usernotifications")
        .level_filter(log::LevelFilter::Debug)
        .init()
        .unwrap();
}

#[cfg(not(target_os = "macos"))]
pub fn setup() {
    #[cfg(feature = "env_logger")]
    env_logger::init();
}
