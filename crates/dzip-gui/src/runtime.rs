//! Archive workspace and Native Shell event handlers.

use std::{collections::HashSet, future::Future};

use crate::session_log::{SessionView, event};
use crate::{
    archive_input::{DroppedArchiveError, prepare_dropped_archive, rebase_extracted_files},
    background, browser,
    file_drop::DroppedFile,
    model::*,
    platform, preferences,
    state::ArchiveEditorMode,
};
use log::Level;
use nsh_core::prelude::*;

mod input;
#[cfg(target_arch = "wasm32")]
mod web_files;

#[derive(Clone, Copy)]
pub(crate) struct Workspace {
    owner: Owner,
    next_id: Signal<u64>,
    pub page: Signal<WorkspacePage>,
    pub archive: Signal<Option<LoadedArchive>>,
    pub selected: Signal<HashSet<usize>>,
    pub focused_entry: Signal<Option<usize>>,
    pub search: Signal<String>,
    pub browse_path: Signal<String>,
    pub draft_path: Signal<String>,
    pub draft_files: Signal<Vec<DraftFile>>,
    pub compression: Signal<CompressionChoice>,
    pub archive_name: Signal<String>,
    pub alignment: Signal<u32>,
    pub random_access: Signal<bool>,
    pub dz_options: Signal<DzCompressionOptions>,
    pub editor_mode: Signal<ArchiveEditorMode>,
    pub busy: Signal<Option<String>>,
    pub toast: Signal<Option<(bool, String)>>,
    pub logs: SessionView,
    pub dragging: Signal<bool>,
}

impl Workspace {
    pub fn new() -> Self {
        let prefs = preferences::read_archive_preferences();
        let ws = Self {
            owner: Owner::current(),
            next_id: signal(1),
            page: signal(WorkspacePage::Browse),
            archive: signal(None),
            selected: signal(HashSet::new()),
            focused_entry: signal(None),
            search: signal(String::new()),
            browse_path: signal(String::new()),
            draft_path: signal(String::new()),
            draft_files: signal(Vec::new()),
            compression: signal(prefs.compression),
            archive_name: signal("game-assets.dz".to_owned()),
            alignment: signal(prefs.alignment),
            random_access: signal(prefs.random_access),
            dz_options: signal(prefs.dz_options),
            editor_mode: signal(ArchiveEditorMode::New),
            busy: signal(None),
            toast: signal(None),
            logs: SessionView::attach(),
            dragging: signal(false),
        };
        event!(
            Level::Info,
            "Dzip Archive v{} started",
            env!("CARGO_PKG_VERSION")
        );
        effect(move || {
            if ws.page.get() == WorkspacePage::Editor && ws.editor_mode.get().is_new() {
                let prefs = preferences::ArchivePreferences {
                    compression: ws.compression.get(),
                    alignment: ws.alignment.get(),
                    random_access: ws.random_access.get(),
                    dz_options: ws.dz_options.get(),
                };
                if let Err(error) = preferences::queue_archive_preferences(prefs) {
                    event!(Level::Error, "{error}");
                }
            }
        });
        ws
    }

    fn notice(self, success: bool, message: String) {
        event!(
            if success { Level::Info } else { Level::Error },
            "{message}"
        );
        self.toast.set(Some((success, message)));
    }

    // Keep the task on the app owner: its initiating button may be replaced
    // as soon as a dialog, a busy overlay or another page appears.
    fn run(self, status: String, task: impl Future<Output = Result<(), String>> + 'static) {
        if self.busy.get_untracked().is_some() {
            return;
        }
        self.busy.set(Some(status));
        self.owner.run(|| {
            drop(spawn(async move {
                if let Err(error) = task.await
                    && !matches!(error.as_str(), "已取消解压" | "已取消保存")
                {
                    self.notice(false, error);
                }
                self.busy.set(None);
            }))
        });
    }

    fn new_id(self) -> u64 {
        let id = self.next_id.get_untracked();
        self.next_id.set(id + 1);
        id
    }

    pub fn clear_draft(self) {
        self.draft_files.set(Vec::new());
        self.draft_path.set(String::new());
    }

    fn reset_editor(self) {
        let prefs = preferences::read_archive_preferences();
        self.clear_draft();
        self.compression.set(prefs.compression);
        self.archive_name.set("game-assets.dz".to_owned());
        self.alignment.set(prefs.alignment);
        self.random_access.set(prefs.random_access);
        self.dz_options.set(prefs.dz_options);
        self.editor_mode.set(ArchiveEditorMode::New);
    }

    pub fn new_archive(self) {
        batch(|| {
            self.reset_editor();
            self.page.set(WorkspacePage::Editor);
        });
    }

    fn install_archive(self, archive: Option<LoadedArchive>) {
        batch(|| {
            self.archive.set(archive);
            self.focused_entry.set(None);
            self.selected.set(HashSet::new());
            self.search.set(String::new());
            self.browse_path.set(String::new());
            self.page.set(WorkspacePage::Browse);
        });
    }

    async fn replace(self, value: LoadedArchive) {
        let previous = self.archive.get_untracked().map(|a| a.session_id);
        let session = value.session_id;
        self.install_archive(Some(value));
        if let Some(old) = previous.filter(|s| *s != session) {
            let _ = background::close_archive(old).await;
        }
    }

    pub fn close(self) {
        if let Some(value) = self.archive.get_untracked() {
            self.run(t!("close-archive"), async move {
                background::close_archive(value.session_id).await?;
                self.install_archive(None);
                self.notice(true, t!("closed-archive", name = value.name));
                Ok(())
            });
        }
    }

    async fn import_files(
        self,
        files: Vec<DroppedFile>,
        page: WorkspacePage,
    ) -> Result<(), String> {
        if files.is_empty() {
            return Ok(());
        }
        if page == WorkspacePage::Editor {
            let n = files.len();
            let method = self.compression.get_untracked();
            let directory = self.draft_path.get_untracked();
            let drafts: Vec<_> = files
                .into_iter()
                .map(|file| {
                    DraftFile::simple(
                        self.new_id(),
                        browser::join_archive_path(&directory, &file.path),
                        file.bytes,
                        method,
                        0,
                    )
                })
                .collect();
            self.draft_files.update(|all| all.extend(drafts));
            self.notice(true, t!("dropped-files", count = n));
        } else {
            let (name, bytes, volumes) = prepare_dropped_archive(files).map_err(|e| match e {
                DroppedArchiveError::NoMainArchive => t!("drop-archive-no-main"),
                DroppedArchiveError::MultipleMainArchives => t!("drop-archive-multiple-main"),
            })?;
            let archive = background::open_archive(name.clone(), bytes, volumes).await?;
            self.replace(archive).await;
            self.notice(true, t!("opened-archive", name = name));
        }
        Ok(())
    }

    pub fn edit(self) {
        let Some(archive) = self.archive.get_untracked() else {
            return;
        };
        self.run(t!("preparing-edit", name = archive.name), async move {
            let ids = archive.entries.iter().map(|e| e.id).collect::<Vec<_>>();
            let files = background::editable_entries(&archive, &ids).await?;
            let method = archive
                .entries
                .first()
                .map_or(CompressionChoice::Dz, |e| e.compression);
            let drafts = files
                .into_iter()
                .map(|file| {
                    let segment = file.segments.first();
                    DraftFile {
                        id: self.new_id(),
                        path: file.path,
                        bytes: file.bytes.into(),
                        compression: segment.map_or(method, |s| s.encoding.compression),
                        volume: segment.map_or(0, |s| s.volume),
                        preserved_segments: Some(file.segments),
                    }
                })
                .collect();
            batch(|| {
                self.draft_files.set(drafts);
                self.draft_path.set(String::new());
                self.archive_name.set(archive.name.clone());
                self.editor_mode.set(ArchiveEditorMode::Existing {
                    source_name: archive.name.clone(),
                });
                self.compression.set(method);
                self.alignment.set(0);
                self.random_access.set(false);
                self.dz_options.set(archive.dz_options);
                self.page.set(WorkspacePage::Editor);
            });
            self.notice(true, t!("loaded-edit", name = archive.name));
            Ok(())
        });
    }

    pub fn extract(self, all: bool) {
        let Some(value) = self.archive.get_untracked() else {
            return;
        };
        let ids: Vec<_> = if all {
            value.entries.iter().map(|e| e.id).collect()
        } else {
            self.selected
                .with_untracked(|s| s.iter().copied().collect())
        };
        if ids.is_empty() {
            return;
        }
        let base = (!all).then(|| self.browse_path.get_untracked());
        let count = ids.len();
        self.run(t!("extracting-files", count = count), async move {
            let files = background::read_entries(&value, &ids).await?;
            let files = rebase_extracted_files(files, base.as_deref());
            platform::export_files(&value.name, files).await?;
            self.notice(true, t!("extract-success", count = count));
            Ok(())
        });
    }

    pub fn save(self) {
        if self.draft_files.with_untracked(Vec::is_empty) {
            return;
        }
        let name = dzip_workflow::normalize_archive_name(&self.archive_name.get_untracked());
        let editing = !self.editor_mode.get_untracked().is_new();
        self.run(t!("compressing-files"), async move {
            let (volumes, reopened) = background::build_archive(
                &self.draft_files.get_untracked(),
                &name,
                self.alignment.get_untracked(),
                self.random_access.get_untracked(),
                self.dz_options.get_untracked(),
            )
            .await?;
            let count = volumes.len();
            self.busy.set(Some(t!("saving-archive")));
            if let Err(error) = platform::save_archive_volumes(&name, volumes).await {
                let _ = background::close_archive(reopened.session_id).await;
                return Err(error);
            }
            self.replace(reopened).await;
            batch(|| self.reset_editor());
            let key = match (editing, count > 1) {
                (true, true) => "saved-split-archive",
                (true, false) => "saved-archive",
                (false, true) => "created-split-archive",
                (false, false) => "created-archive",
            };
            self.notice(true, t!(key, name = name, count = count));
            Ok(())
        });
    }

    pub fn save_logs(self) {
        let text = self.logs.text();
        self.run(t!("export-log"), async move {
            let path = platform::save_bytes("dzip-archive.log", text.into_bytes()).await?;
            event!(Level::Info, "Log exported: {path}");
            Ok(())
        });
    }
}
