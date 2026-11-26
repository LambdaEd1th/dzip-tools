#![cfg_attr(
    all(target_os = "windows", feature = "bundle"),
    windows_subsystem = "windows"
)]

#[cfg(target_os = "linux")]
use nsh_platform_linux as platform;
#[cfg(target_os = "macos")]
use nsh_platform_macos as platform;
#[cfg(target_os = "windows")]
use nsh_platform_windows as platform;

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
mod desktop_logging;

#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
fn main() -> impl std::process::Termination {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../nsh.json")).expect("valid NSH manifest");
    let app_id = manifest["id"].as_str().expect("NSH application id");
    desktop_logging::install(app_id);
    let root = platform::Root::of::<dzip_gui::App>()
        .translations_json(include_str!(concat!(env!("OUT_DIR"), "/translations.json")))
        .app_id(app_id)
        .capabilities_json(&manifest["capabilities"].to_string());
    platform::run_app(
        platform::AppSource::Rust(root),
        platform::AppOptions {
            app_id: app_id.into(),
            title: "Dzip Archive".into(),
            size: (1280, 800),
            ..Default::default()
        },
    )
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn main() {}
