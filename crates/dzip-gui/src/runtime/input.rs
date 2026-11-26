use super::*;

impl Workspace {
    pub fn open(self) {
        self.pick(WorkspacePage::Browse, false);
    }
    pub fn add_files(self) {
        self.pick(WorkspacePage::Editor, false);
    }
    pub fn add_folder(self) {
        self.pick(WorkspacePage::Editor, true);
    }

    fn pick(self, page: WorkspacePage, directory: bool) {
        if self.busy.get_untracked().is_some() {
            return;
        }
        let status = t!(if page == WorkspacePage::Browse {
            "reading-archive"
        } else if directory {
            "add-folder"
        } else {
            "add-files"
        });
        #[cfg(target_arch = "wasm32")]
        let selection = super::web_files::pick_selection("", directory, true);
        #[cfg(not(target_arch = "wasm32"))]
        let selection = {
            use nsh_core::file_dialog::OpenDialog;
            pick_paths(
                if directory {
                    OpenDialog::folders()
                } else {
                    OpenDialog::files()
                }
                .multiple(true),
            )
        };
        self.run(status, async move {
            #[cfg(target_arch = "wasm32")]
            let files = read_web_files(
                selection?
                    .await
                    .into_iter()
                    .map(super::web_files::named_file),
            )
            .await?;
            #[cfg(not(target_arch = "wasm32"))]
            let files = read_paths(selection.await?.unwrap_or_default()).await?;
            self.import_files(files, page).await
        });
    }

    pub fn attach_drop(self, view: View) -> View {
        #[cfg(not(target_arch = "wasm32"))]
        return view
            .on_file_drag_enter(move || self.dragging.set(self.busy.get_untracked().is_none()))
            .on_file_drag_leave(move || self.dragging.set(false))
            .on_file_drop(move |event: &FileDropEvent| {
                self.dragging.set(false);
                let paths = event
                    .uris
                    .iter()
                    .filter_map(|uri| url::Url::parse(uri).ok()?.to_file_path().ok())
                    .collect();
                let page = self.page.get_untracked();
                self.run(t!("reading-archive"), async move {
                    self.import_files(read_paths(paths).await?, page).await
                });
            });
        #[cfg(target_arch = "wasm32")]
        {
            use super::web_files::{FileDrag, drops};
            use futures_util::StreamExt;
            match drops("body", true) {
                Ok(mut events) => {
                    drop(spawn(async move {
                        while let Some(event) = events.next().await {
                            match event {
                                FileDrag::Hovered(on) => {
                                    self.dragging.set(on && self.busy.get_untracked().is_none())
                                }
                                FileDrag::Dropped(files) => {
                                    self.dragging.set(false);
                                    let page = self.page.get_untracked();
                                    self.run(t!("reading-archive"), async move {
                                        let files = read_web_files(files.named().await?).await?;
                                        self.import_files(files, page).await
                                    });
                                }
                            }
                        }
                    }));
                }
                Err(message) => event!(Level::Error, "{message}"),
            }
            view
        }
    }
}

#[cfg(target_arch = "wasm32")]
async fn read_web_files(
    files: impl IntoIterator<Item = (String, web_sys::File)>,
) -> Result<Vec<DroppedFile>, String> {
    let mut dropped = Vec::new();
    for (path, file) in files {
        let bytes = wasm_bindgen_futures::JsFuture::from(file.array_buffer())
            .await
            .map_err(super::web_files::js_error)?;
        dropped.push(DroppedFile {
            path,
            bytes: js_sys::Uint8Array::new(&bytes).to_vec(),
        });
    }
    Ok(dropped)
}

#[cfg(not(target_arch = "wasm32"))]
async fn read_paths(paths: Vec<std::path::PathBuf>) -> Result<Vec<DroppedFile>, String> {
    crate::task::run_cpu_task(move || {
        let paths = paths
            .into_iter()
            .map(|path| {
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                (path, name)
            })
            .collect();
        crate::file_drop::read_native_dropped_paths(paths)
    })
    .await?
}
