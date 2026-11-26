pub mod model;

mod app_i18n;
mod archive_input;
mod background;
mod browser;
#[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
mod desktop_locales;
mod file_drop;
mod icons;
mod platform;
mod preferences;
#[cfg(test)]
mod regression;
mod runtime;
mod session_log;
#[cfg(not(target_arch = "wasm32"))]
pub use session_log::install_desktop_logger;
mod state;
mod task;
mod ui;
mod widgets;
#[cfg(target_arch = "wasm32")]
mod worker_client;
mod workspace_ui;

use nsh_core::prelude::*;

#[native_module("dzip-archive/ui")]
fn native(registry: &mut Registry) {
    registry.register::<App>();
}

#[component]
pub fn App() -> impl IntoElement {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use nsh_core::desktop::QuitMode;
        use nsh_core::element::desktop::{set_quit_mode, window};

        set_quit_mode(QuitMode::LastWindowClosed);
        window()
            .id("main")
            .title("Dzip Archive")
            .default_size(size(px(1280.0), px(800.0)))
            .content(ui::shell)
    }
    #[cfg(target_arch = "wasm32")]
    {
        ui::shell()
    }
}
