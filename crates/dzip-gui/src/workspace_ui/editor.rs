//! Create/edit workspace. Inputs and keyed file rows stay mounted as their
//! values change, so typing a name does not replace the focused text field.

use crate::{
    browser,
    icons::{IconName as I, icon_for_file},
    model::{CompressionChoice, DraftFile, DzCompressionOptions, human_size},
    runtime::Workspace,
    widgets::*,
};
use nsh_base::{SelectItem, SelectValue};
use nsh_core::element::input;
use nsh_core::prelude::*;

mod settings;

pub(crate) fn editor(ws: Workspace, p: Palette, w: f32) -> View {
    let phone = w <= 760.;
    let title_size = if phone {
        28.
    } else {
        (w * 0.03).clamp(27., 40.)
    };
    let editing = ws.editor_mode.get().source_name().map(str::to_owned);
    let title = editing
        .as_ref()
        .map_or_else(|| t!("create-title"), |n| t!("edit-title", name = n));
    let size_action = |v: View| if phone { v.flex_1().px(px(10.)) } else { v };
    let mut actions = row().gap(px(if w <= 420. { 6. } else { 9. }));
    if editing.is_some() {
        actions = actions.child(size_action(action(
            t!("create-new-archive"),
            I::PackagePlus,
            p,
            false,
            false,
            move || ws.new_archive(),
        )));
    }
    let save = action(
        t!(if editing.is_some() {
            "save-archive"
        } else {
            "create-and-save"
        }),
        I::PackageCheck,
        p,
        true,
        false,
        move || ws.save(),
    )
    .disabled(move || ws.draft_files.with(Vec::is_empty))
    .opacity(move || {
        if ws.draft_files.with(Vec::is_empty) {
            0.4
        } else {
            1.0_f32
        }
    });
    let heading = (if phone {
        view().flex_col().gap(px(16.))
    } else {
        row().items_end().gap(px(24.))
    })
    .mx(px(if phone { 2. } else { 9. }))
    .mb(px(if phone { 20. } else { 25. }))
    .child(
        view()
            .flex_col()
            .min_w(px(0.))
            .flex_grow(1.)
            .child(
                label(
                    t!(if editing.is_some() {
                        "edit-archive-mode"
                    } else {
                        "new-archive"
                    }),
                    10.,
                    700,
                    p.accent_text,
                )
                .letter_spacing(px(1.5)),
            )
            .child(
                label(title, title_size, 760, p.ink)
                    .letter_spacing(px(-0.045 * title_size))
                    .line_height(px(1.05 * title_size))
                    .mt(px(if phone { 6. } else { 7. })),
            )
            .child(
                label(
                    t!(if editing.is_some() {
                        "edit-description"
                    } else {
                        "create-description"
                    }),
                    12.,
                    400,
                    p.muted,
                )
                .line_height(px(18.6))
                .mt(px(4.)),
            ),
    )
    .child(actions.child(size_action(save)));
    let mut page = view().flex_col().min_w(px(0.)).w_full().child(heading);
    if let Some(n) = editing {
        page = page.child(
            row()
                .gap(px(12.))
                .p(px(13.))
                .mb(px(14.))
                .bg(rgba(0x33ccbb12))
                .rounded(px(17.))
                .border_1()
                .border_color(rgba(0x33ccbb4d))
                .child(badge(I::Pencil, p))
                .child(pair(t!("editing-source", name = n), t!("edit-notice"), p)),
        );
    }
    let side = settings::settings(ws, p, w);
    let panels = if w > 920. {
        view()
            .flex_row()
            .items_start()
            .gap(px(14.))
            .child(
                view()
                    .min_w(px(0.))
                    .flex_1()
                    .child(draft_panel(ws, p, phone)),
            )
            .child(
                side.w(px(if w <= 1100. { 244. } else { 272. }))
                    .flex_shrink_0(),
            )
    } else {
        view()
            .flex_col()
            .gap(px(14.))
            .child(draft_panel(ws, p, phone))
            .child(side)
    };
    page.child(panels)
}

fn row() -> View {
    view().flex_row().items_center().min_w(px(0.))
}
fn field(p: Palette) -> View {
    row()
        .rounded(px(12.))
        .border_1()
        .border_color(rgb(p.line))
        .bg(rgb(p.soft_bg))
}
fn badge(i: I, p: Palette) -> View {
    view()
        .w(px(38.))
        .h(px(38.))
        .flex_shrink_0()
        .rounded(px(13.))
        .items_center()
        .justify_center()
        .bg(rgba(0x33ccbb1a))
        .child(icon(i, 18., p.accent_text))
}
fn pair(a: String, b: String, p: Palette) -> View {
    view()
        .flex_1()
        .min_w(px(0.))
        .flex_col()
        .gap(px(2.))
        .child(label(a, 13., 700, p.ink))
        .child(label(b, 11., 400, p.soft))
}
fn action(
    s: String,
    i: I,
    p: Palette,
    primary: bool,
    compact: bool,
    go: impl Fn() + 'static,
) -> View {
    let color = if primary { 0xffffff } else { p.body };
    let v = button(s.clone(), go)
        .h(px(if compact { 38. } else { 48. }))
        .gap(px(8.))
        .px(px(if compact { 12. } else { 20. }))
        .rounded(px(if compact { 12. } else { 16. }))
        .bg(rgb(if primary { p.accent } else { p.soft_bg }))
        .border_1()
        .border_color(rgb(if primary { p.accent } else { p.line }))
        .child(icon(i, if compact { 15. } else { 18. }, color))
        .child(label(s, if compact { 11. } else { 13. }, 700, color));
    if primary {
        v.bg_linear_to_br()
            .gradient_from(rgb(0x53d9cb))
            .gradient_to(rgb(0x27b9a9))
    } else {
        v
    }
}
fn small_icon(s: String, i: I, p: Palette, go: impl Fn() + 'static) -> View {
    button(s, go)
        .w(px(32.))
        .h(px(32.))
        .rounded(px(10.))
        .child(icon(i, 15., p.muted))
}
fn draft_panel(ws: Workspace, p: Palette, phone: bool) -> View {
    let mut actions = row().gap(px(8.));
    if phone {
        actions = actions
            .child(
                small_icon(t!("add-files"), I::Plus, p, move || ws.add_files())
                    .bg(rgb(p.soft_bg))
                    .border_1()
                    .border_color(rgb(p.line)),
            )
            .child(
                small_icon(t!("add-folder"), I::FolderPlus, p, move || ws.add_folder())
                    .bg(rgb(p.soft_bg))
                    .border_1()
                    .border_color(rgb(p.line)),
            );
    } else {
        actions = actions
            .child(when(
                move || !ws.draft_files.with(Vec::is_empty),
                move || {
                    small_icon(t!("clear"), I::Trash, p, move || ws.clear_draft())
                        .w(px(58.))
                        .gap(px(6.))
                        .px(px(7.))
                        .child(label(t!("clear"), 11., 400, p.muted))
                },
            ))
            .child(action(
                t!("add-files"),
                I::Plus,
                p,
                false,
                true,
                move || ws.add_files(),
            ))
            .child(action(
                t!("add-folder"),
                I::FolderPlus,
                p,
                false,
                true,
                move || ws.add_folder(),
            ));
    }
    let head = row()
        .px(px(if phone { 14. } else { 18. }))
        .min_h(px(if phone { 67. } else { 76. }))
        .gap(px(11.))
        .border_b_1()
        .border_color(rgb(p.line))
        .child(badge(I::Files, p))
        .child(
            view()
                .flex_1()
                .min_w(px(0.))
                .child(label(t!("draft-files"), 13., 700, p.ink))
                .child(dynamic(move || {
                    ws.draft_files.with(|files| {
                        label(
                            t!(
                                "draft-summary",
                                count = files.len(),
                                size = human_size(files.iter().map(|f| f.bytes.len() as u64).sum()),
                            ),
                            11.,
                            400,
                            p.soft,
                        )
                    })
                })),
        )
        .child(actions);
    card(p, if phone { 19. } else { 24. })
        .overflow_hidden()
        .flex_col()
        .child(head)
        .child(when(
            move || ws.draft_files.with(Vec::is_empty),
            move || {
                button(t!("drop-files"), move || ws.add_files())
                    .flex_col()
                    .m(px(if phone { 12. } else { 18. }))
                    .min_h(px(if phone { 350. } else { 480. }))
                    .rounded(px(21.))
                    .border_1()
                    .border_dashed()
                    .border_color(rgba(0x33ccbb69))
                    .bg_linear_to_br()
                    .gradient_from(rgba(0x33ccbb0d))
                    .gradient_to(rgba(0x33ccbb00))
                    .child(
                        view()
                            .w(px(66.))
                            .h(px(66.))
                            .items_center()
                            .justify_center()
                            .rounded(px(22.))
                            .bg(rgba(0x33ccbb1f))
                            .mb(px(15.))
                            .child(icon(I::CloudUpload, 31., p.accent_text)),
                    )
                    .child(label(t!("drop-files"), 14., 700, p.ink))
                    .child(label(t!("drop-files-hint"), 11., 400, p.muted).mt(px(5.)))
                    .child(
                        label(t!("privacy-hint"), 10., 400, p.soft)
                            .text_center()
                            .mt(px(17.))
                            .px(px(10.)),
                    )
            },
        ))
        .child(when(
            move || !ws.draft_files.with(Vec::is_empty),
            move || draft_table(ws, p, phone),
        ))
}
fn draft_table(ws: Workspace, p: Palette, phone: bool) -> View {
    let listing = computed(move || {
        ws.draft_files
            .with(|files| browser::build_draft_browser_listing(files, &ws.draft_path.get()))
    });
    let crumbs = dynamic(move || {
        let path = ws.draft_path.get();
        let parent = browser::parent_browser_directory(&path);
        let mut v = row()
            .gap(px(6.))
            .flex_wrap()
            .px(px(16.))
            .py(px(7.))
            .border_b_1()
            .border_color(rgb(p.line))
            .child(
                small_icon(t!("go-parent-folder"), I::ArrowLeft, p, move || {
                    ws.draft_path.set(parent.clone())
                })
                .disabled(path.is_empty())
                .opacity(if path.is_empty() { 0.4 } else { 1. }),
            )
            .child(
                button(t!("root"), move || ws.draft_path.set(String::new()))
                    .h(px(30.))
                    .px(px(8.))
                    .gap(px(5.))
                    .rounded(px(8.))
                    .bg(rgba(0x33ccbb16))
                    .child(icon(I::Home, 13., p.accent_text))
                    .child(label(t!("root"), 11., 650, p.accent_text)),
            );
        for c in browser::browser_breadcrumbs(&path) {
            v = v.child(icon(I::ChevronRight, 11., p.soft)).child(
                button(c.name.clone(), move || ws.draft_path.set(c.path.clone()))
                    .child(label(c.name, 11., 650, p.body)),
            );
        }
        v
    });
    let mut cols = row()
        .px(px(if phone { 22. } else { 17. }))
        .pl(px(if phone { 46. } else { 52. }))
        .min_h(px(40.))
        .bg(rgb(p.soft_bg))
        .border_b_1()
        .border_color(rgb(p.line))
        .child(label(t!("name").to_uppercase(), 10., 700, p.soft).flex_1());
    if !phone {
        for (k, w) in [
            ("original-size", 77.),
            ("packed-size", 77.),
            ("algorithm", 99.),
            ("volume", 99.),
        ] {
            cols = cols.child(
                view()
                    .w(px(w))
                    .pr(px(10.))
                    .child(label(t!(k).to_uppercase(), 10., 700, p.soft).letter_spacing(px(0.6))),
            );
        }
        cols = cols.child(view().w(px(32.)));
    }
    view()
        .flex_col()
        .child(crumbs)
        .child(cols)
        .child(each(
            move || listing.with(|l| l.0.clone()),
            |f| f.path.clone(),
            move |f, _| dynamic(move || folder_row(ws, p, f.get(), phone)),
        ))
        .child(each(
            move || listing.with(|l| l.1.clone()),
            |f| f.id,
            move |f, _| file_row(ws, p, f, phone),
        ))
        .child(when(
            move || listing.with(|l| l.0.is_empty() && l.1.is_empty()),
            move || {
                view()
                    .p(px(18.))
                    .child(pair(t!("empty-folder"), t!("empty-folder-hint"), p))
            },
        ))
        .child(
            row()
                .p(px(14.))
                .bg(rgb(p.soft_bg))
                .justify_between()
                .child(dynamic(move || {
                    listing.with(|l| {
                        label(
                            t!("directory-summary", folders = l.0.len(), files = l.1.len(),),
                            11.,
                            400,
                            p.soft,
                        )
                    })
                }))
                .children((!phone).then(|| label(t!("draft-browse-hint"), 11., 400, p.soft))),
        )
}
fn size_cells(p: Palette, size: u64, phone: bool) -> View {
    row()
        .child(
            label(
                if phone {
                    format!("{} {}", t!("original"), human_size(size))
                } else {
                    human_size(size)
                },
                if phone { 10. } else { 12. },
                400,
                p.muted,
            )
            .w(px(if phone { 130. } else { 77. })),
        )
        .child(
            label(
                if phone {
                    format!("{} {}", t!("packed"), t!("pending-generation"))
                } else {
                    t!("pending-generation")
                },
                if phone { 10. } else { 12. },
                400,
                p.soft,
            )
            .w(px(if phone { 130. } else { 77. })),
        )
}
fn table_row(phone: bool) -> View {
    (if phone {
        view().flex_col().gap(px(6.))
    } else {
        row().gap(px(0.))
    })
    .min_w(px(0.))
    .pl(px(if phone { 40. } else { 52. }))
    .pr(px(16.))
    .py(px(9.))
    .min_h(px(62.))
    .border_b_1()
}
fn folder_row(ws: Workspace, p: Palette, f: browser::FolderView, phone: bool) -> View {
    let path = f.path.clone();
    let open = t!("open-folder", name = f.name);
    let remove = t!("remove-folder-label", path = f.path);
    let mut name = button(open, move || ws.draft_path.set(path.clone()))
        .justify_start()
        .gap(px(10.))
        .min_w(px(0.))
        .child(
            view()
                .h(px(35.))
                .w(px(35.))
                .rounded(px(11.))
                .items_center()
                .justify_center()
                .bg(rgba(0xf7b94b21))
                .child(icon(I::Folder, 17., p.amber)),
        )
        .child(pair(
            f.name,
            t!("folder-file-count", count = f.file_count),
            p,
        ));
    if !phone {
        name = name.flex_1();
    }
    let tail = row()
        .gap(px(4.))
        .justify_between()
        .child(
            label(t!("folder"), 11., 650, p.amber)
                .w(px(90.))
                .p(px(8.))
                .rounded(px(8.))
                .bg(rgba(0xf7b94b11)),
        )
        .children((!phone).then(|| view().w(px(94.))))
        .child(small_icon(remove, I::X, p, move || {
            ws.draft_files
                .update(|files| browser::remove_draft_folder(files, &f.path))
        }));
    table_row(phone)
        .border_color(rgb(p.line))
        .child(name)
        .child(size_cells(p, f.size, phone))
        .child(tail)
}
fn update_file(ws: Workspace, id: u64, f: impl FnOnce(&mut DraftFile)) {
    ws.draft_files.update(|files| {
        if let Some(file) = files.iter_mut().find(|file| file.id == id) {
            f(file)
        }
    });
}
fn file_row(ws: Workspace, p: Palette, file: ReadSignal<DraftFile>, phone: bool) -> View {
    let f = file.get_untracked();
    let id = f.id;
    let name = signal(browser::draft_file_name(&f.path).to_owned());
    // A name can be temporarily empty when the user selects all and retypes.
    // Commit on leaving the field, so the sorted listing doesn't move or
    // remove the focused row underneath their cursor.
    let commit = move || {
        let next = name.get_untracked();
        if next.trim().is_empty() {
            name.set(file.with_untracked(|f| browser::draft_file_name(&f.path).to_owned()));
        } else {
            update_file(ws, id, |f| {
                f.path = browser::replace_draft_file_name(&f.path, &next)
            });
        }
    };
    let location = f
        .path
        .rsplit_once('/')
        .map(|(p, _)| p.to_owned())
        .unwrap_or_else(|| t!("root"));
    let mut title = row()
        .gap(px(10.))
        .child(
            view()
                .h(px(35.))
                .w(px(35.))
                .rounded(px(11.))
                .items_center()
                .justify_center()
                .bg(rgba(0x33ccbb19))
                .child(dynamic(move || {
                    file.with(|f| icon(icon_for_file(&f.path), 17., p.accent_text))
                })),
        )
        .child(
            view()
                .flex_1()
                .min_w(px(0.))
                .child(
                    input()
                        .label(move || {
                            file.with(|f| {
                                SharedString::from(t!("rename-archive-entry-label", path = f.path))
                            })
                        })
                        .value(move || name.get())
                        .text_size(px(13.))
                        .text_color(rgb(p.ink))
                        .h(px(24.))
                        .w_full()
                        .on_change(move |value: &str| name.set(value.to_owned()))
                        .on_blur(commit)
                        .on_submit(move |_: &str| commit()),
                )
                .child(label(location, 11., 400, p.soft).truncate()),
        );
    if !phone {
        title = title.flex_1();
    }
    let volume = move |v: u16| {
        if v == 0 {
            t!("main-volume")
        } else {
            t!("volume-value", number = v)
        }
    };
    let compression = dynamic(move || {
        let f = file.get();
        let current = method_key(f.compression);
        let caption = t!(current);
        let a = t!("set-compression-for", path = f.path, algorithm = caption);
        choice(
            a,
            caption,
            current.to_owned(),
            CompressionChoice::ALL
                .into_iter()
                .map(|m| SelectItem::new(method_key(m), t!(method_key(m))))
                .collect(),
            p,
            34.,
            move |v| {
                if let Some(method) = CompressionChoice::ALL
                    .into_iter()
                    .find(|m| method_key(*m) == v)
                {
                    update_file(ws, id, |f| f.replace_compression(method));
                }
            },
        )
    });
    let volumes = dynamic(move || {
        let f = file.get();
        let caption = volume(f.volume);
        let a = t!("set-volume-for", path = f.path, volume = caption);
        let max = ws
            .draft_files
            .with(|all| all.iter().map(|f| f.volume).max().unwrap_or(0));
        let mut options: Vec<_> = (0..=max)
            .map(|v| SelectItem::new(v.to_string(), volume(v)))
            .collect();
        if let Some(v) = max.checked_add(1) {
            options.push(SelectItem::new(
                v.to_string(),
                t!("new-volume-option", number = v),
            ));
        }
        choice(
            a,
            caption,
            f.volume.to_string(),
            options,
            p,
            34.,
            move |v| {
                if let Ok(vol) = v.parse() {
                    update_file(ws, id, |f| f.replace_volume(vol));
                }
            },
        )
    });
    let options = row()
        .gap(px(5.))
        .child(
            view()
                .w(px(if phone { 110. } else { 92. }))
                .child(compression),
        )
        .child(view().w(px(if phone { 116. } else { 92. })).child(volumes))
        .child(dynamic(move || {
            small_icon(
                file.with(|f| t!("remove-file-label", path = f.path)),
                I::X,
                p,
                move || ws.draft_files.update(|files| files.retain(|f| f.id != id)),
            )
        }));
    table_row(phone)
        .border_color(rgb(p.line))
        .child(title)
        .child(size_cells(p, f.bytes.len() as u64, phone))
        .child(options)
}
fn method_key(m: CompressionChoice) -> &'static str {
    match m {
        CompressionChoice::Dz => "compression-dz",
        CompressionChoice::Zlib => "compression-zlib",
        CompressionChoice::Bzip => "compression-bzip",
        CompressionChoice::Lzma => "compression-lzma",
        CompressionChoice::Copy => "compression-copy",
        CompressionChoice::Zero => "compression-zero",
    }
}
fn choice(
    a: String,
    title: String,
    value: String,
    items: Vec<SelectItem>,
    p: Palette,
    h: f32,
    change: impl Fn(&str) + 'static,
) -> View {
    let compact = h == 34.;
    let expanded = signal(false);
    let trigger = (if compact {
        row()
            .rounded(px(9.))
            .border_1()
            .border_color(rgba(0x33ccbb4d))
            .bg(rgba(0x33ccbb12))
    } else {
        field(p)
    })
    .w_full()
    .h(px(h))
    .px(px(9.))
    .gap(px(4.))
    .cursor(Cursor::Pointer)
    .child(
        label(
            title,
            12.,
            650,
            if compact { p.accent_text } else { p.body },
        )
        .flex_1()
        .line_height(px(14.)),
    )
    .child(
        icon(
            I::ChevronDown,
            if compact { 11. } else { 14. },
            if compact { p.accent_text } else { p.muted },
        )
        .rotate(move || deg(if expanded.get() { 180. } else { 0. })),
    );
    let selected = value.clone();
    view().min_w(px(0.)).child(
        nsh_base::select()
            .accessibility_label(a)
            .value(SelectValue::from(value))
            .items(items)
            .cover(false)
            .offset(px(4.))
            .popup_style(move |s| {
                s.rounded(px(14.))
                    .border_1()
                    .border_color(rgb(p.line).alpha(0.55))
                    .bg(rgb(p.surface_solid).alpha(0.88))
                    .backdrop_blur(px(22.))
                    .shadow_lg()
            })
            .list_style(|s| s.p(px(5.)).max_h(px(260.)))
            .item_class("rounded-[9px] focus:bg-[#33ccbb13] selected:bg-[#33ccbb21]")
            .item_style(|s| {
                s.min_h(px(34.))
                    .px(px(6.))
                    .justify_center()
                    .cursor(Cursor::Pointer)
            })
            .render_item(move |i: &nsh_base::SelectEntry| {
                let active = i.value.as_ref() == selected.as_str();
                row()
                    .gap(px(6.))
                    .child(
                        view()
                            .w(px(15.))
                            .flex_shrink_0()
                            .children(active.then(|| icon(I::Check, 12., p.accent_text))),
                    )
                    .child(
                        label(
                            i.title.clone(),
                            12.,
                            650,
                            if active { p.accent_text } else { p.muted },
                        )
                        .line_height(px(14.))
                        .flex_1(),
                    )
            })
            .trigger(trigger)
            .on_open_change(move |e: &nsh_base::SelectEvent| {
                if let nsh_base::SelectEvent::OpenChange(on, _) = e {
                    expanded.set(*on);
                }
            })
            .on_change(move |v: &Option<SharedString>| {
                if let Some(v) = v {
                    change(v);
                }
            }),
    )
}
