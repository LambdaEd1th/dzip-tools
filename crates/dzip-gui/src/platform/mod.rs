#[cfg(not(target_arch = "wasm32"))]
mod desktop;
#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(not(target_arch = "wasm32"))]
pub use desktop::{export_files, save_archive_volumes, save_bytes};
#[cfg(target_arch = "wasm32")]
pub use web::{export_files, save_archive_volumes, save_bytes};
