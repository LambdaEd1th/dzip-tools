//! A bounded view of this session's GUI records. NSH is the logger: it owns
//! levels, filtering, the native rolling file and browser console output.

use std::{
    collections::VecDeque,
    fmt,
    sync::{Mutex, MutexGuard},
};

use futures_channel::mpsc;
use futures_util::StreamExt;
use log::Level;
use nsh_core::prelude::*;

#[cfg(target_arch = "wasm32")]
mod web;

const LIMIT: usize = 300;
#[derive(Default)]
struct History {
    lines: VecDeque<String>,
    // One pending wake is enough: the UI always reads the latest snapshot.
    wake: Option<mpsc::Sender<()>>,
}

impl History {
    fn push(&mut self, line: String) {
        if self.lines.len() == LIMIT {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
        if let Some(wake) = &mut self.wake {
            let _ = wake.try_send(());
        }
    }

    fn text(&self) -> String {
        self.lines
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn history() -> MutexGuard<'static, History> {
    static HISTORY: Mutex<History> = Mutex::new(History {
        lines: VecDeque::new(),
        wake: None,
    });
    HISTORY
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn capture(level: Level, message: impl fmt::Display) {
    let line = format!("[{level}] {message}");
    history().push(line);
}

#[cfg(not(target_arch = "wasm32"))]
static OUTPUT_ATTACHED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Set up the Cargo desktop runner as NSH's packaged runners do, tapping
/// its output *after* filtering. Called once, before mounting the app.
#[cfg(not(target_arch = "wasm32"))]
pub fn install_desktop_logger(mut options: nsh_engine::logging::LogOptions) {
    let platform_output = options.output.take();
    options.output = Some(Box::new(move |record| {
        if let Some(output) = &platform_output {
            output(record);
        }
        if record.target().split("::").next() == Some("dzip_gui") {
            capture(record.level(), record.args());
        }
    }));
    if nsh_engine::logging::install(options) {
        OUTPUT_ATTACHED.store(true, std::sync::atomic::Ordering::Release);
    }
}

#[cfg(target_arch = "wasm32")]
const WEB_PREFIX: &str = "dzip-gui: ";

#[cfg(target_arch = "wasm32")]
fn capture_web_record(level: &str, message: &str) {
    if let (Ok(level), Some(message)) = (level.parse::<Level>(), message.strip_prefix(WEB_PREFIX)) {
        capture(level, message);
    }
}

/// Preserve the Rust target for RUST_LOG. The Web host's onLog receives
/// severity and text only, so its adapter identifies our records by prefix.
pub(crate) fn emit(target: &'static str, level: Level, message: fmt::Arguments<'_>) {
    #[cfg(target_arch = "wasm32")]
    log::log!(target: target, level, "{WEB_PREFIX}{message}");
    #[cfg(not(target_arch = "wasm32"))]
    if log::log_enabled!(target: target, level) {
        log::log!(target: target, level, "{message}");
        // A CLI development/embedding host already owns its global logger
        // and does not expose an output callback. Keep its GUI view usable.
        if !OUTPUT_ATTACHED.load(std::sync::atomic::Ordering::Acquire) {
            capture(level, message);
        }
    }
}

macro_rules! event {
    ($level:expr, $($args:tt)+) => {
        $crate::session_log::emit(module_path!(), $level, format_args!($($args)+))
    };
}
pub(crate) use event;

#[derive(Clone, Copy)]
pub(crate) struct SessionView(Signal<u64>);

impl SessionView {
    pub(crate) fn attach() -> Self {
        let revision = signal(0_u64);
        let (send, mut changes) = mpsc::channel(1);
        history().wake = Some(send);
        // A native record may come from the preference writer or an archive
        // thread. Only this owner-scoped task touches the reactive signal.
        drop(spawn(async move {
            while changes.next().await.is_some() {
                revision.update(|n| *n = n.wrapping_add(1));
            }
        }));
        #[cfg(target_arch = "wasm32")]
        web::attach();
        Self(revision)
    }

    pub(crate) fn text(self) -> String {
        self.0.get();
        history().text()
    }

    pub(crate) fn clear(self) {
        history().lines.clear();
        self.0.update(|n| *n = n.wrapping_add(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_view_keeps_latest_records_in_order() {
        let mut data = History::default();
        for i in 0..=LIMIT {
            data.push(i.to_string());
        }
        let text = data.text();
        assert_eq!(text.lines().count(), LIMIT);
        assert_eq!(text.lines().next(), Some("1"));
        assert_eq!(text.lines().last(), Some("300"));
    }
}
