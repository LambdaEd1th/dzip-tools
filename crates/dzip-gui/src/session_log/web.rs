use nsh_core::prelude::on_host_event;

pub(super) fn attach() {
    on_host_event("dzip.log", |record| {
        let level = match record["level"].as_str() {
            Some("log") => "INFO",
            Some("warn") => "WARN",
            Some("error") => "ERROR",
            _ => return,
        };
        if let Some(message) = record["message"].as_str() {
            super::capture_web_record(level, message);
        }
    });
}
