//! Archive overview, navigable directory listing and file inspector.

use std::collections::HashSet;

use crate::{
    browser::{self, FolderView},
    icons::{IconName as I, icon_for_file},
    model::{EntryView, LoadedArchive, human_size, ratio_percent},
    runtime::Workspace,
    widgets::*,
};
use nsh_core::Checked;
use nsh_core::prelude::*;

pub(crate) fn browser(ws: Workspace, p: Palette, w: f32) -> View {
    let Some(archive) = ws.archive.get() else {
        return view();
    };
    let split = w > 920.;
    let mut columns = view().gap(px(14.));
    if split {
        columns = columns.flex_row().items_start();
    }
    let table_archive = archive.clone();
    let detail_archive = archive.clone();
    let mut left = view().min_w(px(0.));
    let mut right = view().min_w(px(0.));
    if split {
        left = left.flex_1();
        right = right
            .w(px(if w <= 1120. { 244. } else { 272. }))
            .flex_shrink_0();
    }
    let title = archive.name.clone();
    view()
        .child(dynamic(move || heading(ws, p, w, &title)))
        .child(stats(&archive, p, w))
        .child(
            columns
                .child(left.child(dynamic(move || table(ws, p, w, &table_archive))))
                .child(right.child(dynamic(move || {
                    let focused = ws
                        .focused_entry
                        .get()
                        .and_then(|id| detail_archive.entries.iter().find(|e| e.id == id));
                    inspector(p, w, focused)
                }))),
        )
}

fn heading(ws: Workspace, p: Palette, w: f32, name: &str) -> View {
    let phone = w <= 760.;
    let small = w <= 420.;
    let title_size = if phone {
        28.
    } else {
        (w * 0.03).clamp(27., 40.)
    };
    let mut head = view()
        .mx(px(if phone { 2. } else { 9. }))
        .mb(px(if phone { 20. } else { 25. }))
        .gap(px(if phone { 16. } else { 24. }));
    // There is insufficient horizontal room for all four actions and the description on a tablet.
    let stacked = w < 1200.;
    if !stacked {
        head = head.flex_row().items_end().justify_between();
    }
    let mut intro = view().min_w(px(0.));
    if !stacked {
        intro = intro.flex_1();
    }
    intro = intro
        .child(label(t!("archive-overview"), 10., 800, p.accent_text).letter_spacing(px(1.8)))
        .child(
            label(name.to_owned(), title_size, 760, p.ink)
                .truncate()
                .letter_spacing(px(-0.045 * title_size))
                .line_height(px(title_size * 1.05))
                .mt(px(if phone { 6. } else { 7. }))
                .mb(px(2.)),
        )
        .child(label(t!("archive-overview-description"), 12., 400, p.muted).line_height(px(18.6)));
    let n = ws.selected.with(HashSet::len);
    let mut actions = view()
        .flex_row()
        .items_center()
        .gap(px(if small { 6. } else { 9. }));
    for (idx, key, symbol) in [
        (0, "edit-archive", I::Pencil),
        (1, "extract-selected", I::Download),
        (2, "extract-all", I::ArchiveRestore),
        (3, "close-archive", I::X),
    ] {
        let primary = idx == 2;
        let disabled = idx == 1 && n == 0;
        let mut caption = t!(key);
        if idx == 1 && n > 0 {
            caption = format!("{caption} ({n})");
        }
        let color = if primary { 0xffffff } else { p.body };
        let mut action = button(caption.clone(), move || match idx {
            0 => ws.edit(),
            1 => ws.extract(false),
            2 => ws.extract(true),
            _ => ws.close(),
        })
        .h(px(42.))
        .min_w(px(0.))
        .rounded(px(14.))
        .gap(px(if small { 5. } else { 8. }))
        .border_1()
        .border_color(rgb(if primary { p.accent } else { p.line }))
        .bg(rgb(p.soft_bg))
        .disabled(disabled)
        .opacity(if disabled { 0.45 } else { 1. });
        if primary {
            action = action
                .bg_linear(deg(135.))
                .gradient_from(rgb(0x50dccc))
                .gradient_to(rgb(0x29bba9));
        }
        if !phone {
            action = action.px(px(15.));
        } else if idx == 3 && small {
            action = action.w(px(42.));
        } else {
            action = action.flex_1();
        }
        action = action.child(icon(symbol, 17., color));
        if !(idx == 3 && small) {
            action =
                action.child(label(caption, if small { 11. } else { 12. }, 650, color).truncate());
        }
        actions = actions.child(action);
    }
    head.child(intro).child(actions)
}

fn stats(a: &LoadedArchive, p: Palette, w: f32) -> View {
    let phone = w <= 760.;
    let compact = w <= 420.;
    let gap = if phone { 8. } else { 13. };
    let size = human_size(a.source_size);
    let size = if a.source_complete {
        size
    } else {
        format!("{size} ({}/{})", a.loaded_volume_count, a.volume_count)
    };
    let ratio = if a.source_complete {
        format!("{}%", ratio_percent(a.source_size, a.unpacked_size))
    } else {
        t!("pending-generation")
    };
    let tiles = [
        (
            "files",
            I::Files,
            a.entries.len().to_string(),
            t!("chunk-count", count = a.chunk_count),
            p.accent_text,
            0x33ccbb1f,
        ),
        (
            "archive-size",
            I::HardDrive,
            size,
            t!("volume-count", count = a.volume_count),
            0x4e8ee8,
            0x6aa9ff21,
        ),
        (
            "unpacked",
            I::Expand,
            human_size(a.unpacked_size),
            t!("original-total"),
            0xe65e91,
            0xff78a821,
        ),
        (
            "compression-ratio",
            I::Gauge,
            ratio,
            t!("ratio-note"),
            0xc98c20,
            0xf7b94b26,
        ),
    ];
    let mut out = view().gap(px(gap)).mb(px(14.));
    for row in tiles.chunks(if w <= 920. { 2 } else { 4 }) {
        let mut line = view().flex_row().gap(px(gap));
        for (key, symbol, value, note, color, bg) in row {
            let mut copy = view()
                .flex_1()
                .min_w(px(0.))
                .child(label(t!(key), 11., 600, p.muted))
                .child(
                    label(
                        value.clone(),
                        if compact {
                            15.
                        } else if phone {
                            17.
                        } else {
                            21.
                        },
                        730,
                        p.ink,
                    )
                    .truncate()
                    .mt(px(2.)),
                );
            if !compact {
                copy = copy.child(label(note.clone(), 10., 400, p.soft));
            }
            line = line.child(
                card(p, if phone { 18. } else { 20. })
                    .flex_1()
                    .overflow_hidden()
                    .min_h(px(if compact {
                        80.
                    } else if phone {
                        91.
                    } else {
                        108.
                    }))
                    .flex_row()
                    .items_center()
                    .gap(px(if phone { 9. } else { 13. }))
                    .p(px(if compact {
                        11.
                    } else if phone {
                        13.
                    } else {
                        18.
                    }))
                    .child(
                        view()
                            .absolute()
                            .top(px(-36.))
                            .right(px(-30.))
                            .w(px(105.))
                            .h(px(105.))
                            .rounded_full()
                            .bg(rgba((p.ink << 8) | 14)),
                    )
                    .child(icon_box(
                        *symbol,
                        if compact {
                            31.
                        } else if phone {
                            35.
                        } else {
                            43.
                        },
                        20.,
                        if phone { 12. } else { 15. },
                        *color,
                        *bg,
                    ))
                    .child(copy),
            );
        }
        out = out.child(line);
    }
    out
}

fn icon_box(symbol: I, size: f32, glyph: f32, radius: f32, color: u32, bg: u32) -> View {
    view()
        .w(px(size))
        .h(px(size))
        .flex_shrink_0()
        .rounded(px(radius))
        .items_center()
        .justify_center()
        .bg(rgba(bg))
        .child(icon(symbol, glyph, color))
}

fn select_state(selected: &HashSet<usize>, ids: &[usize]) -> Checked {
    let n = ids.iter().filter(|id| selected.contains(id)).count();
    if n == 0 {
        Checked::False
    } else if n == ids.len() {
        Checked::True
    } else {
        Checked::Mixed
    }
}

fn toggle_group(selected: &mut HashSet<usize>, ids: &[usize]) {
    if select_state(selected, ids) == Checked::True {
        for id in ids {
            selected.remove(id);
        }
    } else {
        selected.extend(ids.iter().copied());
    }
}

fn checkbox(
    ws: Workspace,
    p: Palette,
    title: String,
    ids: Vec<usize>,
    selected: &HashSet<usize>,
    slot: f32,
) -> impl IntoElement {
    let state = select_state(selected, &ids);
    let control = nsh_base::checkbox()
        .accessibility_label(title)
        .checked(state == Checked::True)
        .indeterminate(state == Checked::Mixed)
        .disabled(ids.is_empty())
        .on_change(move |_: &bool| ws.selected.update(|s| toggle_group(s, &ids)))
        .flex_row()
        .items_center()
        .justify_center()
        .cursor(Cursor::Pointer)
        .active(|s| s.opacity(0.8))
        .w(px(slot))
        .min_h(px(34.))
        .flex_shrink_0()
        .indicator_style(|s| {
            s.size(px(15.))
                .mr(px(if slot > 30. { 4. } else { 0. }))
                .rounded(px(3.))
                .border_1()
                .border_color(rgb(if state == Checked::False {
                    p.muted
                } else {
                    p.accent_text
                }))
                .items_center()
                .justify_center()
                .bg(rgb(if state == Checked::True {
                    p.accent_text
                } else {
                    p.surface_solid
                }))
        });
    match state {
        Checked::True => control.indicator(icon(I::Check, 12., 0xffffff)),
        Checked::Mixed => control.indicator(view().w(px(7.)).h(px(2.)).bg(rgb(p.accent_text))),
        Checked::False => control,
    }
}

fn navigate(ws: Workspace, target: String) {
    batch(|| {
        ws.browse_path.set(target);
        ws.focused_entry.set(None);
    });
}

fn breadcrumb_bar(ws: Workspace, p: Palette, dir: &str, search: bool) -> View {
    let parent = browser::parent_browser_directory(dir);
    let disabled = dir.is_empty() || search;
    let back = button(t!("go-parent-folder"), move || navigate(ws, parent.clone()))
        .w(px(32.))
        .h(px(32.))
        .flex_shrink_0()
        .rounded(px(10.))
        .border_1()
        .border_color(rgb(p.line))
        .bg(rgb(p.soft_bg))
        .disabled(disabled)
        .opacity(if disabled { 0.38 } else { 1. })
        .child(icon(I::ArrowLeft, 16., p.muted));
    let root = t!(if search { "search-results" } else { "root" });
    let active = dir.is_empty() || search;
    let root_color = if active { p.accent_text } else { p.muted };
    let mut crumbs = view().flex_row().items_center().gap(px(2.)).child(
        button(root.clone(), move || navigate(ws, String::new()))
            .disabled(search)
            .h(px(30.))
            .px(px(8.))
            .gap(px(6.))
            .rounded(px(9.))
            .bg(rgba(if active { 0x33ccbb14 } else { 0 }))
            .child(icon(I::Home, 14., root_color))
            .child(label(root, 11., 620, root_color)),
    );
    if !search {
        for crumb in browser::browser_breadcrumbs(dir) {
            let active = crumb.path == dir;
            crumbs = crumbs.child(icon(I::ChevronRight, 13., p.soft)).child(
                button(crumb.name.clone(), move || navigate(ws, crumb.path.clone()))
                    .h(px(30.))
                    .px(px(8.))
                    .rounded(px(9.))
                    .bg(rgba(if active { 0x33ccbb14 } else { 0 }))
                    .child(label(
                        crumb.name,
                        11.,
                        620,
                        if active { p.accent_text } else { p.muted },
                    )),
            );
        }
    }
    view()
        .h(px(48.))
        .flex_row()
        .items_center()
        .gap(px(8.))
        .px(px(16.))
        .border_b_1()
        .border_color(rgb(p.line))
        .bg(rgba(0x33ccbb06))
        .child(back)
        .child(
            scroll()
                .horizontal()
                .scrollbar(false)
                .flex_1()
                .min_w(px(0.))
                .label(t!("archive-path"))
                .child(crumbs),
        )
}

fn table(ws: Workspace, p: Palette, w: f32, a: &LoadedArchive) -> View {
    let phone = w <= 760.;
    let dir = ws.browse_path.get();
    let q = ws.search.with(|s| s.trim().to_lowercase());
    let search = !q.is_empty();
    let (folders, files) = browser::build_browser_listing(&a.entries, &dir, &q);
    let selected = ws.selected.get();
    let focused = ws.focused_entry.get();
    let selectable = if search {
        files.iter().map(|f| f.id).collect()
    } else {
        browser::entry_ids_in_directory(&a.entries, &dir)
    };
    let (folder_count, file_count) = (folders.len(), files.len());
    let summary = t!(
        if search {
            "contents-search-summary"
        } else {
            "contents-summary"
        },
        folders = folder_count,
        files = file_count,
    );
    let mut toolbar = view()
        .min_h(px(if phone { 68. } else { 76. }))
        .flex_row()
        .items_center()
        .gap(px(11.))
        .px(px(if phone { 14. } else { 18. }))
        .border_b_1()
        .border_color(rgb(p.line))
        .child(icon_box(
            I::ListTree,
            38.,
            18.,
            13.,
            p.accent_text,
            0x33ccbb1a,
        ))
        .child(
            view()
                .flex_1()
                .min_w(px(0.))
                .child(label(t!("archive-contents"), 13., 700, p.ink))
                .child(label(summary, 11., 400, p.soft).mt(px(2.))),
        );
    if !selected.is_empty() {
        let mut selection = view()
            .flex_row()
            .items_center()
            .gap(px(6.))
            .px(px(8.))
            .min_h(px(34.))
            .rounded(px(12.))
            .bg(rgba(0x33ccbb14));
        if w > 420. {
            selection = selection.child(label(
                t!("selected-summary", count = selected.len()),
                11.,
                500,
                p.accent_text,
            ));
        }
        if !phone {
            let size = a
                .entries
                .iter()
                .filter(|e| selected.contains(&e.id))
                .fold(0u64, |n, e| n.saturating_add(e.size));
            selection = selection.child(label(human_size(size), 11., 700, p.accent_text));
        }
        let clear = t!("clear");
        toolbar = toolbar.child(
            selection.child(
                button(clear.clone(), move || ws.selected.update(HashSet::clear))
                    .rounded(px(7.))
                    .bg(rgb(p.surface_solid))
                    .p(px(4.))
                    .child(label(clear, 10., 500, p.muted)),
            ),
        );
    }
    let mut panel = card(p, if phone { 20. } else { 24. })
        .overflow_hidden()
        .child(toolbar)
        .child(breadcrumb_bar(ws, p, &dir, search));
    let mut head = view()
        .h(px(40.))
        .flex_row()
        .items_center()
        .px(px(17.))
        .bg(rgb(p.soft_bg))
        .border_b_1()
        .border_color(rgb(p.line))
        .child(checkbox(
            ws,
            p,
            t!("select-visible"),
            selectable,
            &selected,
            if phone { 30. } else { 34. },
        ));
    let keys = [
        "name",
        "original-size",
        "packed-size",
        "algorithm",
        "volume",
    ];
    for (i, key) in keys.iter().enumerate().take(if phone { 1 } else { 5 }) {
        head = head.child(
            column(i)
                .child(label(t!(key).to_uppercase(), 11., 700, p.soft).letter_spacing(px(0.44))),
        );
    }
    if !phone {
        head = head.child(view().w(px(34.)));
    }
    panel = panel.child(head);
    if folders.is_empty() && files.is_empty() {
        panel = panel.child(
            view()
                .min_h(px(260.))
                .items_center()
                .justify_center()
                .child(icon(
                    if search { I::SearchX } else { I::FolderOpen },
                    32.,
                    p.soft,
                ))
                .child(
                    label(
                        t!(if search { "no-matches" } else { "empty-folder" }),
                        12.,
                        700,
                        p.body,
                    )
                    .mt(px(12.)),
                )
                .child(
                    label(
                        t!(if search {
                            "no-matches-hint"
                        } else {
                            "empty-folder-hint"
                        }),
                        11.,
                        400,
                        p.soft,
                    )
                    .mt(px(4.)),
                ),
        );
    }
    for f in folders {
        panel = panel.child(list_row(ws, p, w, Row::Folder(f), &selected, focused));
    }
    for e in files {
        panel = panel.child(list_row(ws, p, w, Row::File(e), &selected, focused));
    }
    let footer_text = if search {
        t!(
            "search-summary",
            visible = file_count,
            total = a.entries.len(),
        )
    } else {
        t!(
            "directory-summary",
            folders = folder_count,
            files = file_count
        )
    };
    let mut footer = view()
        .min_h(px(if phone { 40. } else { 44. }))
        .flex_row()
        .items_center()
        .justify_between()
        .bg(rgb(p.soft_bg))
        .px(px(18.))
        .child(label(footer_text, 11., 400, p.soft));
    if !phone {
        footer = footer.child(
            view()
                .flex_row()
                .items_center()
                .gap(px(5.))
                .child(icon(
                    if search { I::Search } else { I::MousePointer },
                    14.,
                    p.soft,
                ))
                .child(label(
                    t!(if search {
                        "searching-archive"
                    } else {
                        "browse-hint"
                    }),
                    11.,
                    400,
                    p.soft,
                )),
        );
    }
    panel.child(footer)
}

fn column(i: usize) -> View {
    // Starts and proportions match the grid used by both the header and the rows.
    view()
        .w(px(0.))
        .flex_grow([1.82, 0.48, 0.48, 0.62, 0.62][i])
        .min_w(px(0.))
        .pr(px(8.))
}

fn folder_name(e: &EntryView) -> String {
    if e.folder.is_empty() || e.folder == "根目录" {
        t!("root")
    } else {
        e.folder.clone()
    }
}

fn volume(value: u16) -> String {
    if value == 0 {
        t!("main-volume")
    } else {
        t!("volume-value", number = value)
    }
}

fn packed_size(value: Option<u64>) -> String {
    value
        .map(human_size)
        .unwrap_or_else(|| t!("pending-generation"))
}

enum Row {
    Folder(FolderView),
    File(EntryView),
}

fn pill(value: String, p: Palette, folder: bool) -> View {
    view()
        .min_w(px(0.))
        .min_h(px(34.))
        .px(px(8.))
        .rounded(px(10.))
        .justify_center()
        .border_1()
        .border_color(rgba(if folder { 0xf7b94b33 } else { 0x33ccbb24 }))
        .bg(rgba(if folder { 0xf7b94b17 } else { 0x33ccbb12 }))
        .child(
            label(
                value,
                11.,
                720,
                if folder { 0xc48522 } else { p.accent_text },
            )
            .truncate(),
        )
}

fn list_row(
    ws: Workspace,
    p: Palette,
    w: f32,
    row: Row,
    selected: &HashSet<usize>,
    focused: Option<usize>,
) -> View {
    let phone = w <= 760.;
    let folder = matches!(row, Row::Folder(_));
    let (
        name,
        subtitle,
        original,
        packed,
        method,
        volume_label,
        ids,
        active,
        select_label,
        open_label,
        symbol,
    ) = match &row {
        Row::Folder(f) => (
            f.name.clone(),
            t!("folder-file-count", count = f.file_count),
            f.size,
            f.packed_size,
            t!("folder"),
            None,
            f.entry_ids.clone(),
            false,
            t!("select-folder", name = f.name),
            t!("open-folder", name = f.name),
            I::Folder,
        ),
        Row::File(e) => (
            e.name.clone(),
            folder_name(e),
            e.size,
            e.packed_size,
            e.compression.to_string().to_uppercase(),
            Some(volume(e.volume)),
            vec![e.id],
            focused == Some(e.id),
            t!("select-file", name = e.path),
            e.path.clone(),
            icon_for_file(&e.name),
        ),
    };
    let name_cell = view()
        .min_w(px(0.))
        .flex_row()
        .items_center()
        .gap(px(11.))
        .child(icon_box(
            symbol,
            35.,
            19.,
            11.,
            if folder { 0xe6a83f } else { p.accent_text },
            if folder { 0xf7b94b1f } else { 0x33ccbb17 },
        ))
        .child(
            view()
                .flex_1()
                .min_w(px(0.))
                .child(label(name, 13., 640, p.ink).truncate())
                .child(label(subtitle, 11., 400, p.soft).truncate().mt(px(2.))),
        );
    let mut row_content = button(open_label, move || match &row {
        Row::Folder(f) => navigate(ws, f.path.clone()),
        Row::File(e) => ws.focused_entry.set(Some(e.id)),
    })
    .flex_1()
    .min_w(px(0.))
    .justify_start();
    let first = human_size(original);
    let second = packed_size(packed);
    if phone {
        let sizes =
            view()
                .flex_row()
                .min_h(px(22.))
                .child(view().flex_1().child(
                    label(format!("{} {first}", t!("original")), 10., 400, p.muted).truncate(),
                ))
                .child(view().flex_1().child(
                    label(format!("{} {second}", t!("packed")), 10., 400, p.muted).truncate(),
                ));
        let mut second_col = view().flex_1().min_w(px(0.));
        if let Some(v) = volume_label {
            second_col = second_col.child(pill(v, p, false));
        }
        row_content = row_content
            .flex_col()
            .items_stretch()
            .py(px(5.))
            .pr(px(39.))
            .child(view().min_h(px(42.)).justify_center().child(name_cell))
            .child(sizes)
            .child(
                view()
                    .flex_row()
                    .items_center()
                    .min_h(px(38.))
                    .gap(px(9.))
                    .child(view().flex_1().min_w(px(0.)).child(pill(method, p, folder)))
                    .child(second_col),
            );
    } else {
        row_content = row_content
            .min_h(px(61.))
            .child(column(0).child(name_cell))
            .child(column(1).child(label(first, 12., 400, p.muted)))
            .child(column(2).child(label(second, 12., 400, p.muted)))
            .child(column(3).child(pill(method, p, folder)));
        let mut col = column(4);
        if let Some(v) = volume_label {
            col = col.child(pill(v, p, false));
        }
        row_content = row_content.child(col).child(view().w(px(34.)));
    }
    let mut line = view()
        .flex_row()
        .items_center()
        .px(px(if w <= 420. {
            10.
        } else if phone {
            12.
        } else {
            17.
        }))
        .border_b_1()
        .border_color(rgb(p.line))
        .hover(|s| s.bg(rgba(0x33ccbb0e)));
    if active {
        line = line.bg(rgba(0x33ccbb0e)).child(
            view()
                .absolute()
                .top(px(0.))
                .bottom(px(0.))
                .left(px(0.))
                .w(px(3.))
                .bg(rgb(p.accent)),
        );
    }
    line.child(checkbox(
        ws,
        p,
        select_label,
        ids,
        selected,
        if phone { 30. } else { 34. },
    ))
    .child(row_content)
}

fn inspector(p: Palette, w: f32, e: Option<&EntryView>) -> View {
    let phone = w <= 760.;
    let mut panel = card(p, if phone { 20. } else { 24. })
        .p(px(if phone { 18. } else { 21. }))
        .overflow_hidden();
    if w > 920. {
        panel = panel.min_h(px(445.));
    }
    let Some(e) = e else {
        return panel.child(
            view()
                .min_h(px(if w > 920. { 390. } else { 160. }))
                .items_center()
                .justify_center()
                .child(icon(I::MousePointer, 28., p.soft))
                .child(label(t!("select-entry"), 12., 700, p.body).mt(px(12.)))
                .child(label(t!("details-hint"), 11., 400, p.soft).mt(px(3.))),
        );
    };
    let eyebrow = label(t!("file-details"), 10., 800, p.soft).letter_spacing(px(1.8));
    let glyph = icon_box(
        icon_for_file(&e.name),
        if phone { 52. } else { 61. },
        28.,
        if phone { 17. } else { 20. },
        p.accent_text,
        0x33ccbb1a,
    )
    .border_1()
    .border_color(rgba(0x33ccbb1f));
    let name = label(e.name.clone(), 15., 700, p.ink).truncate();
    let folder = label(folder_name(e), 11., 400, p.soft)
        .mt(px(3.))
        .truncate();
    let header = if phone {
        view()
            .flex_row()
            .items_center()
            .gap(px(12.))
            .child(glyph)
            .child(
                view()
                    .min_w(px(0.))
                    .flex_1()
                    .child(eyebrow)
                    .child(name)
                    .child(folder),
            )
    } else {
        view()
            .items_center()
            .child(eyebrow)
            .child(glyph.mt(px(16.)).mb(px(10.)))
            .child(name)
            .child(folder)
    };
    let ratio = e.packed_size.map(|n| ratio_percent(n, e.size));
    let value = ratio.map_or_else(|| t!("pending-generation"), |n| format!("{n}%"));
    let saved = ratio.map_or_else(
        || t!("pending-generation"),
        |n| t!("saved-space", percent = 100u64.saturating_sub(n)),
    );
    panel = panel.child(header).child(
        view()
            .p(px(14.))
            .mt(px(21.))
            .mb(px(15.))
            .rounded(px(15.))
            .border_1()
            .border_color(rgb(p.line))
            .bg(rgb(p.soft_bg))
            .child(
                view()
                    .flex_row()
                    .justify_between()
                    .child(label(t!("compression-ratio"), 11., 400, p.muted))
                    .child(label(value, 11., 700, p.accent_text)),
            )
            .child(
                view()
                    .h(px(6.))
                    .rounded_full()
                    .bg(rgb(p.control))
                    .mt(px(9.))
                    .mb(px(7.))
                    .child(
                        view()
                            .w(relative(ratio.unwrap_or_default().min(100) as f32 / 100.))
                            .h_full()
                            .rounded_full()
                            .bg(rgb(p.accent)),
                    ),
            )
            .child(label(saved, 10., 400, p.soft)),
    );
    for (key, value) in [
        ("original-size", human_size(e.size)),
        ("packed-size", packed_size(e.packed_size)),
        (
            "compression-algorithm",
            e.compression.to_string().to_uppercase(),
        ),
        ("chunks", e.chunks.to_string()),
        ("volume", volume(e.volume)),
    ] {
        let mut item = view()
            .flex_row()
            .min_h(px(36.))
            .items_center()
            .justify_between()
            .gap(px(10.))
            .border_b_1()
            .border_color(rgb(p.line))
            .child(label(t!(key), 11., 400, p.muted));
        item = if key == "compression-algorithm" {
            item.child(
                view()
                    .min_w(px(40.))
                    .min_h(px(22.))
                    .items_center()
                    .justify_center()
                    .rounded(px(8.))
                    .bg(rgba(0x33ccbb14))
                    .border_1()
                    .border_color(rgba(0x33ccbb24))
                    .child(label(value, 10., 750, p.accent_text)),
            )
        } else {
            item.child(label(value, 11., 650, p.ink))
        };
        panel = panel.child(item);
    }
    panel.child(
        view()
            .mt(px(14.))
            .p(px(11.))
            .rounded(px(12.))
            .bg(rgb(p.soft_bg))
            .child(label(t!("archive-path"), 10., 400, p.soft).mb(px(5.)))
            .child(label(e.path.clone(), 10., 400, p.body).font_family("monospace")),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partial_folder_and_search_selections_preserve_unrelated_entries() {
        let mut s = HashSet::from([1, 9]);
        assert_eq!(select_state(&s, &[1, 2]), Checked::Mixed);
        toggle_group(&mut s, &[1, 2]);
        assert_eq!(s, HashSet::from([1, 2, 9]));
        assert_eq!(select_state(&s, &[1, 2]), Checked::True);
        toggle_group(&mut s, &[1, 2]);
        assert_eq!(s, HashSet::from([9]));
        assert_eq!(select_state(&s, &[]), Checked::False);
        toggle_group(&mut s, &[]);
        assert_eq!(s, HashSet::from([9]));
    }
}
