use std::{
    cell::{Cell, RefCell},
    future::Future,
    rc::Rc,
};

use futures_channel::{mpsc, oneshot};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    DataTransfer, Document, DragEvent, Element, Event, EventTarget, File, FileList,
    FileSystemDirectoryEntry, FileSystemDirectoryReader, FileSystemEntry, FileSystemFileEntry,
    HtmlInputElement,
};

pub(crate) enum FileDrag {
    Hovered(bool),
    Dropped(DroppedFiles),
}

pub(crate) struct DroppedFiles {
    pub(crate) files: Vec<File>,
    entries: Vec<FileSystemEntry>,
}

impl DroppedFiles {
    fn snapshot(transfer: DataTransfer, folders: bool) -> Self {
        let mut entries = Vec::new();
        let files = if folders {
            let items = transfer.items();
            let mut picked = Vec::new();
            for index in 0..items.length() {
                let Some(item) = items.get(index).filter(|item| item.kind() == "file") else {
                    continue;
                };
                // Entries must be captured before the drop event returns.
                if let Ok(Some(entry)) = item.webkit_get_as_entry()
                    && entry.is_directory()
                {
                    entries.push(entry);
                } else if let Ok(Some(file)) = item.get_as_file() {
                    picked.push(file);
                }
            }
            if entries.is_empty() {
                // In particular, WebKit can expose a filesystem entry for a
                // synthetic File whose entry cannot be read. The FileList is
                // authoritative for a drop with no directories.
                files(transfer.files())
            } else {
                picked
            }
        } else {
            files(transfer.files())
        };
        Self { files, entries }
    }

    fn is_empty(&self) -> bool {
        self.files.is_empty() && self.entries.is_empty()
    }

    pub(crate) async fn named(self) -> Result<Vec<(String, File)>, String> {
        let mut files = self.files.into_iter().map(named_file).collect::<Vec<_>>();
        let mut entries = self.entries;
        while let Some(entry) = entries.pop() {
            if entry.is_file() {
                let path = entry.full_path().trim_start_matches('/').to_owned();
                // Chromium and WebKit expose these as interfaces with no
                // reliable constructors. The entry just identified its type.
                let file = entry.unchecked_into::<FileSystemFileEntry>();
                let promise = js_sys::Promise::new(&mut |resolve, reject| {
                    file.file_with_callback_and_callback(&resolve, &reject);
                });
                let file = JsFuture::from(promise)
                    .await
                    .map_err(js_error)?
                    .dyn_into::<File>()
                    .map_err(|_| "Dropped entry is not a file")?;
                files.push((if path.is_empty() { file.name() } else { path }, file));
            } else if entry.is_directory() {
                let reader = entry
                    .unchecked_into::<FileSystemDirectoryEntry>()
                    .create_reader();
                loop {
                    let batch = read_entries(&reader).await?;
                    if batch.is_empty() {
                        break;
                    }
                    entries.extend(batch);
                }
            }
        }
        Ok(files)
    }
}

async fn read_entries(reader: &FileSystemDirectoryReader) -> Result<Vec<FileSystemEntry>, String> {
    let promise = js_sys::Promise::new(&mut |resolve, reject| {
        if let Err(error) = reader.read_entries_with_callback_and_callback(&resolve, &reject) {
            let _ = reject.call1(&JsValue::UNDEFINED, &error);
        }
    });
    let batch = JsFuture::from(promise).await.map_err(js_error)?;
    Ok(js_sys::Array::from(&batch)
        .iter()
        .map(JsCast::unchecked_into)
        .collect())
}

pub(crate) fn named_file(file: File) -> (String, File) {
    let path = js_sys::Reflect::get(file.as_ref(), &JsValue::from_str("webkitRelativePath"))
        .ok()
        .and_then(|path| path.as_string())
        .filter(|path| !path.is_empty())
        .unwrap_or_else(|| file.name());
    (path, file)
}

/// Keep the browser's File handles. Each tool chooses how much of a file to read.
pub(crate) fn pick_selection(
    accept: &str,
    directory: bool,
    multiple: bool,
) -> Result<impl Future<Output = Vec<File>>, String> {
    let document = document()?;
    let input = document
        .create_element("input")
        .map_err(js_error)?
        .dyn_into::<HtmlInputElement>()
        .map_err(|_| "无法创建文件选择器".to_string())?;
    input.set_type("file");
    input.set_multiple(multiple);
    input.set_hidden(true);
    input.set_accept(accept);
    if directory {
        input
            .set_attribute("webkitdirectory", "")
            .map_err(js_error)?;
    }

    let (send, receive) = oneshot::channel();
    let send = Rc::new(RefCell::new(Some(send)));
    let change = Listener::new(input.as_ref(), "change", {
        let input = input.clone();
        let send = send.clone();
        move |_| {
            if let Some(send) = send.borrow_mut().take() {
                let _ = send.send(files(input.files()));
            }
        }
    })?;
    let cancel = Listener::new(input.as_ref(), "cancel", move |_| {
        if let Some(send) = send.borrow_mut().take() {
            let _ = send.send(Vec::new());
        }
    })?;
    document
        .body()
        .ok_or("无法访问文件选择器")?
        .append_child(&input)
        .map_err(js_error)?;
    input.click();
    let input = Picker {
        input,
        _change: change,
        _cancel: cancel,
    };
    Ok(async move {
        let answer = receive.await.unwrap_or_default();
        drop(input);
        answer
    })
}

/// Listen only within the visible workspace. The NSH owner removes the
/// DOM listeners when the workspace leaves the page.
pub(crate) fn drops(
    selector: &'static str,
    folders: bool,
) -> Result<mpsc::UnboundedReceiver<FileDrag>, String> {
    let document = document()?;
    let (send, receive) = mpsc::unbounded();
    let hovered = Rc::new(Cell::new(false));
    let over = Listener::new(document.as_ref(), "dragover", {
        let send = send.clone();
        let hovered = hovered.clone();
        move |event| {
            let Some(drag) = event.dyn_ref::<DragEvent>() else {
                return;
            };
            let in_workspace = within(drag.target(), selector);
            let has_files = drag
                .data_transfer()
                .is_some_and(|data| data.types().includes(&JsValue::from_str("Files"), 0));
            let next = in_workspace && has_files;
            if next {
                drag.prevent_default();
            }
            if hovered.replace(next) != next {
                let _ = send.unbounded_send(FileDrag::Hovered(next));
            }
        }
    })?;
    let leave = Listener::new(document.as_ref(), "dragleave", {
        let send = send.clone();
        let hovered = hovered.clone();
        move |event| {
            let Some(drag) = event.dyn_ref::<DragEvent>() else {
                return;
            };
            if !within(drag.related_target(), selector) && hovered.replace(false) {
                let _ = send.unbounded_send(FileDrag::Hovered(false));
            }
        }
    })?;
    let drop_files = Listener::new(document.as_ref(), "drop", move |event| {
        let Some(drag) = event.dyn_ref::<DragEvent>() else {
            return;
        };
        if hovered.replace(false) {
            let _ = send.unbounded_send(FileDrag::Hovered(false));
        }
        if !within(drag.target(), selector) {
            return;
        }
        let Some(transfer) = drag.data_transfer() else {
            return;
        };
        let selected = DroppedFiles::snapshot(transfer, folders);
        if !selected.is_empty() {
            drag.prevent_default();
            let _ = send.unbounded_send(FileDrag::Dropped(selected));
        }
    })?;
    nsh_core::prelude::cleanup(move || drop((over, leave, drop_files)));
    Ok(receive)
}

fn within(target: Option<EventTarget>, selector: &str) -> bool {
    target
        .and_then(|target| target.dyn_into::<Element>().ok())
        .and_then(|element| element.closest(selector).ok().flatten())
        .is_some()
}

fn files(list: Option<FileList>) -> Vec<File> {
    list.map_or_else(Vec::new, |list| {
        (0..list.length())
            .filter_map(|index| list.get(index))
            .collect()
    })
}

fn document() -> Result<Document, String> {
    web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| "无法访问文件选择器".to_string())
}

pub(crate) fn js_error(error: JsValue) -> String {
    error.as_string().unwrap_or_else(|| format!("{error:?}"))
}

struct Picker {
    input: HtmlInputElement,
    _change: Listener,
    _cancel: Listener,
}

impl Drop for Picker {
    fn drop(&mut self) {
        self.input.remove();
    }
}

struct Listener {
    target: EventTarget,
    event: &'static str,
    handler: Closure<dyn FnMut(Event)>,
}

impl Listener {
    fn new(
        target: &EventTarget,
        event: &'static str,
        handle: impl FnMut(Event) + 'static,
    ) -> Result<Self, String> {
        let handler = Closure::new(handle);
        target
            .add_event_listener_with_callback(event, handler.as_ref().unchecked_ref())
            .map_err(js_error)?;
        Ok(Self {
            target: target.clone(),
            event,
            handler,
        })
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        let _ = self
            .target
            .remove_event_listener_with_callback(self.event, self.handler.as_ref().unchecked_ref());
    }
}
