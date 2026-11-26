//! The Cargo desktop launcher, unlike the NSH release runners, installs the
//! engine logger itself. The UI sees the same filtered records as the host.

use nsh_engine::logging::{self, LogOptions, LogOutput};

pub(super) fn install(app_id: &str) {
    let mut options = LogOptions::default();
    // Include successful archive operations, even in the bundled GUI.
    // An explicit RUST_LOG still takes precedence in the engine.
    options.development = true;
    options.directory = logging::log_dir(app_id);
    options.output = Some(platform_output());
    dzip_gui::install_desktop_logger(options);
}

#[cfg(target_os = "macos")]
fn platform_output() -> LogOutput {
    use nsh_platform_apple::unified_log;
    let mut options = unified_log::Options::default();
    options.filtered = true; // NSH's engine already applied RUST_LOG.
    options.development = cfg!(debug_assertions);
    let unified = unified_log::logger(options);
    Box::new(move |record| unified.log(record))
}

#[cfg(target_os = "windows")]
fn platform_output() -> LogOutput {
    Box::new(|record| {
        let line = format!(
            "{} {}: {}\n",
            record.level(),
            record.target(),
            record.args()
        );
        nsh_platform_windows::debug_output(&line);
        if cfg!(debug_assertions) {
            eprint!("{line}");
        }
    })
}

#[cfg(target_os = "linux")]
fn platform_output() -> LogOutput {
    Box::new(|record| eprintln!("{} {}: {}", record.level(), record.target(), record.args()))
}
