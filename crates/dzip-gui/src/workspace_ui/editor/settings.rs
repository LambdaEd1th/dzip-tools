use super::*;

pub(super) fn settings(ws: Workspace, p: Palette, w: f32) -> View {
    let advanced = signal(false);
    let has_dz = computed(move || {
        ws.draft_files.with(|fs| {
            fs.iter().any(|f| f.compression == CompressionChoice::Dz)
                || fs.is_empty() && ws.compression.get() == CompressionChoice::Dz
        })
    });
    let heading = row()
        .gap(px(11.))
        .mb(px(20.))
        .child(badge(I::Sliders, p))
        .child(pair(t!("archive-settings"), t!("per-file-hint"), p));
    let count = if w <= 920. && w > 760. { 3 } else { 2 };
    let mut grid = nsh_base::radio_group()
        .accessibility_label(t!("default-algorithm"))
        .flex_col()
        .gap(px(7.));
    for chunk in CompressionChoice::ALL.chunks(count) {
        let mut r = row().gap(px(7.));
        for &method in chunk {
            let key = method_key(method);
            let description = t!(&format!("{key}-description"));
            let tip = card(p, 8.)
                .p(px(6.))
                .child(label(description.clone(), 11., 500, p.ink));
            let active = move || ws.compression.get() == method;
            let v = nsh_base::radio()
                .accessibility_label(t!(key))
                .checked(active)
                .on_change(move |_: &bool| ws.compression.set(method))
                .flex_row()
                .items_center()
                .cursor(Cursor::Pointer)
                .active(|s| s.opacity(0.8))
                .min_w(px(0.))
                .flex_1()
                .h(px(58.))
                .justify_start()
                .px(px(9.))
                .gap(px(8.))
                .rounded(px(13.))
                .border_1()
                .border_color(move || {
                    if active() {
                        rgba(0x33ccbb96)
                    } else {
                        rgb(p.line)
                    }
                })
                .bg(move || {
                    if active() {
                        rgba(0x33ccbb16)
                    } else {
                        rgb(p.soft_bg)
                    }
                })
                .child(
                    view()
                        .w(px(13.))
                        .h(px(13.))
                        .rounded(px(7.))
                        .border_1()
                        .border_color(move || rgb(if active() { p.accent } else { p.line }))
                        .bg(move || if active() { rgb(p.accent) } else { rgba(0) })
                        .items_center()
                        .justify_center()
                        .child(when(active, || {
                            view().w(px(5.)).h(px(5.)).rounded(px(3.)).bg(rgb(0xffffff))
                        })),
                )
                .child(
                    view()
                        .min_w(px(0.))
                        .flex_1()
                        .child(label(t!(key), 11., 700, p.ink).truncate())
                        .child(label(description, 10., 400, p.soft).truncate()),
                )
                .child(nsh_base::tooltip().child(tip));
            r = r.child(v);
        }
        grid = grid.child(r);
    }
    card(p, if w <= 760. { 19. } else { 24. })
        .flex_col()
        .p(px(if w <= 760. { 16. } else { 19. }))
        .child(heading)
        .child(caption(t!("archive-name"), p))
        .child(
            field(p)
                .h(px(43.))
                .px(px(12.))
                .gap(px(9.))
                .child(icon(I::Archive, 16., p.muted))
                .child(
                    input()
                        .label(t!("archive-name"))
                        .value(move || ws.archive_name.get())
                        .on_change(move |v: &str| ws.archive_name.set(v.to_owned()))
                        .text_size(px(11.))
                        .text_color(rgb(p.ink))
                        .flex_1()
                        .min_w(px(0.)),
                ),
        )
        .child(
            row()
                .justify_between()
                .mt(px(15.))
                .mb(px(6.))
                .gap(px(4.))
                .child(label(t!("default-algorithm"), 11., 650, p.muted).flex_1())
                .child(when(
                    move || !ws.draft_files.with(Vec::is_empty),
                    move || {
                        button(t!("apply-all"), move || {
                            let method = ws.compression.get_untracked();
                            ws.draft_files.update(|fs| {
                                for f in fs {
                                    f.replace_compression(method)
                                }
                            });
                        })
                        .p(px(5.))
                        .rounded(px(8.))
                        .bg(rgba(0x33ccbb17))
                        .child(label(
                            t!("apply-all"),
                            10.,
                            650,
                            p.accent_text,
                        ))
                    },
                )),
        )
        .child(grid)
        .child(view().h(px(1.)).my(px(17.)).bg(rgb(p.line)))
        .child(caption(t!("data-alignment"), p))
        .child(dynamic(move || {
            let value = ws.alignment.get();
            let alignment = |v| {
                t!(match v {
                    512 => "alignment-512",
                    2048 => "alignment-2048",
                    4096 => "alignment-4096",
                    _ => "alignment-none",
                })
            };
            choice(
                t!("choose-alignment"),
                alignment(value),
                value.to_string(),
                [0, 512, 2048, 4096]
                    .map(|v| SelectItem::new(v.to_string(), alignment(v)))
                    .to_vec(),
                p,
                43.,
                move |v| {
                    if let Ok(v) = v.parse() {
                        ws.alignment.set(v)
                    }
                },
            )
        }))
        .child(toggle(
            p,
            "random-access",
            "random-access-description",
            move || ws.random_access.get(),
            || false,
            move |on| {
                ws.random_access.set(on);
                ws.draft_files.update(|fs| {
                    for f in fs {
                        f.preserved_segments = None
                    }
                });
            },
        ))
        .child(toggle(
            p,
            "common-buffer",
            "common-buffer-description",
            move || ws.dz_options.get().use_combuf,
            move || !has_dz.get(),
            move |on| {
                ws.dz_options.update(|dz| {
                    dz.use_combuf = on;
                    if on {
                        dz.combuf_static_tables = true;
                        dz.ref_length_table_size = dz.ref_length_table_size.max(1);
                        dz.ref_length_tables = dz.ref_length_tables.max(1);
                        dz.ref_offset_table_size = dz.ref_offset_table_size.max(1);
                        dz.ref_offset_tables = dz.ref_offset_tables.max(1);
                        dz.big_min_match = dz.big_min_match.max(1);
                    }
                })
            },
        ))
        .child(
            button(t!("dz-advanced"), move || advanced.update(|v| *v = !*v))
                .mt(px(13.))
                .p(px(12.))
                .gap(px(8.))
                .bg(rgba(0x33ccbb08))
                .border_1()
                .border_color(rgb(p.line))
                .rounded(px(14.))
                .disabled(move || !has_dz.get())
                .opacity(move || if has_dz.get() { 1.0_f32 } else { 0.45 })
                .accessibility_state(move || AccessibilityState {
                    expanded: Some(advanced.get() && has_dz.get()),
                    ..Default::default()
                })
                .child(
                    view()
                        .flex_1()
                        .min_w(px(0.))
                        .gap(px(3.))
                        .child(label(t!("dz-advanced"), 12., 700, p.ink))
                        .child(label(t!("dz-advanced-description"), 10., 400, p.soft)),
                )
                .child(icon(I::ChevronDown, 15., p.muted)),
        )
        .child(when(
            move || has_dz.get() && advanced.get(),
            move || parameters(ws, p),
        ))
        .child(
            row()
                .items_start()
                .gap(px(9.))
                .mt(px(16.))
                .p(px(10.))
                .rounded(px(12.))
                .bg(rgba(0x33ccbb0e))
                .child(icon(I::ShieldCheck, 17., p.accent_text))
                .child(pair(
                    t!("local-processing"),
                    t!(if cfg!(target_arch = "wasm32") {
                        "local-processing-web"
                    } else {
                        "local-processing-desktop"
                    }),
                    p,
                )),
        )
}
fn caption(s: String, p: Palette) -> Text {
    label(s, 11., 650, p.muted).ml(px(2.)).mb(px(6.))
}
fn toggle(
    p: Palette,
    key: &str,
    hint: &str,
    on: impl Fn() -> bool + Copy + 'static,
    disabled: impl Fn() -> bool + Copy + 'static,
    set: impl Fn(bool) + 'static,
) -> impl IntoElement {
    let title = t!(key);
    let description = t!(hint);
    nsh_base::switch()
        .checked(on)
        .disabled(disabled)
        .accessibility_label(title.clone())
        .accessibility_hint(description.clone())
        .on_change(move |checked: &bool| set(*checked))
        // NSH puts its track before the children; keep our label on the left.
        .app_class(format!(
            "flex-row-reverse items-center gap-[8px] py-[7px] border-b border-[#{:06x}] cursor-pointer active:opacity-80 disabled:opacity-45",
            p.line
        ))
        .track_class(format!(
            "flex-row shrink-0 w-[34px] h-[20px] p-[2px] rounded-[12px] border border-[#{:06x}] bg-[#{:06x}] checked:bg-[#{:06x}]",
            p.line, p.control, p.accent
        ))
        .thumb_class("w-[14px] h-[14px] rounded-[7px] bg-white shadow-sm checked:ml-[14px]")
        .child(
            view()
                .flex_1()
                .min_w(px(0.))
                .gap(px(1.))
                .child(label(title, 11., 650, p.ink))
                .child(label(description, 10., 400, p.soft)),
        )
}
fn parameters(ws: Workspace, p: Palette) -> View {
    let dz = ws.dz_options;
    let common = move || dz.get().use_combuf;
    let min_ref = move || i64::from(common());
    // Numeric inputs keep editing and stepping semantics from the native
    // NumberInput, including blur/Enter clamping and keyboard accessibility.
    macro_rules! param {
        ($name:literal,$field:ident,$hint:expr,$min:expr,$max:expr,$disabled:expr) => {
            parameter(
                p,
                $name,
                $hint,
                move || i64::from(dz.get().$field),
                $min,
                $max,
                $disabled,
                move |val| dz.update(|opts| opts.$field = val as _),
            )
        };
    }
    view()
        .flex_col()
        .mt(px(8.))
        .p(px(9.))
        .gap(px(6.))
        .rounded(px(12.))
        .border_1()
        .border_color(rgb(p.line))
        .child(
            row()
                .justify_between()
                .gap(px(6.))
                .child(label(t!("encoding-references"), 11., 700, p.ink).flex_1())
                .child(
                    button(t!("reset-defaults"), move || {
                        dz.set(DzCompressionOptions::default())
                    })
                    .child(label(
                        t!("reset-defaults"),
                        10.,
                        600,
                        p.accent_text,
                    )),
                ),
        )
        .child(toggle(
            p,
            "preprocess-analysis",
            "preprocess-description",
            move || dz.get().preprocess,
            move || !common(),
            move |v| dz.update(|o| o.preprocess = v),
        ))
        .child(toggle(
            p,
            "combuf-static-tables",
            "combuf-static-description",
            move || dz.get().combuf_static_tables,
            common,
            move |v| dz.update(|o| o.combuf_static_tables = v),
        ))
        .child(parameter_grid(vec![
            param!(
                "max_mem_usage",
                max_mem_usage,
                t!("minus-one-unlimited"),
                || -1,
                i32::MAX as i64,
                || false
            ),
            param!(
                "trim_reference_factor",
                trim_reference_factor,
                t!("common-buffer-only"),
                || i32::MIN as i64,
                i32::MAX as i64,
                move || !common()
            ),
            param!(
                "max_common_match",
                max_common_match,
                t!("zero-unlimited"),
                || 0,
                u32::MAX as i64,
                move || !common()
            ),
        ]))
        .child(label(t!("range-model"), 11., 700, p.ink).mt(px(6.)))
        .child(parameter_grid(vec![
            param!(
                "WinSize",
                win_size,
                t!("stored-archive-settings"),
                || 0,
                30,
                || false
            ),
            param!(
                "OffsetTableSize",
                offset_table_size,
                "1–15".into(),
                || 1,
                15,
                || false
            ),
            param!(
                "OffsetTables",
                offset_tables,
                "1–255".into(),
                || 1,
                255,
                || false
            ),
            param!(
                "OffsetContexts",
                offset_contexts,
                "1–8".into(),
                || 1,
                8,
                || false
            ),
            param!(
                "RefLengthTableSize",
                ref_length_table_size,
                "0–15".into(),
                min_ref,
                15,
                || false
            ),
            param!(
                "RefLengthTables",
                ref_length_tables,
                "0–255".into(),
                min_ref,
                255,
                || false
            ),
            param!(
                "RefOffsetTableSize",
                ref_offset_table_size,
                "0–15".into(),
                min_ref,
                15,
                || false
            ),
            param!(
                "RefOffsetTables",
                ref_offset_tables,
                "0–255".into(),
                min_ref,
                255,
                || false
            ),
            param!(
                "BigMinMatch",
                big_min_match,
                "0–255".into(),
                min_ref,
                255,
                || false
            ),
        ]))
}
fn parameter_grid(fields: Vec<View>) -> View {
    let mut fields = fields.into_iter();
    let mut grid = view().flex_col().gap(px(8.));
    while let Some(first) = fields.next() {
        grid = grid.child(
            row()
                .gap(px(8.))
                .items_stretch()
                .child(first.flex_1())
                .child(fields.next().unwrap_or_else(view).flex_1()),
        );
    }
    grid
}
#[allow(clippy::too_many_arguments)]
fn parameter(
    p: Palette,
    name: &'static str,
    hint: String,
    value: impl Fn() -> i64 + 'static,
    min: impl Fn() -> i64 + Copy + 'static,
    max: i64,
    disabled: impl Fn() -> bool + Copy + 'static,
    change: impl Fn(i64) + 'static,
) -> View {
    view()
        .min_w(px(0.))
        .flex_col()
        .gap(px(5.))
        .rounded(px(11.))
        .border_1()
        .border_color(rgb(p.line))
        .bg(rgb(p.soft_bg))
        .p(px(8.))
        .opacity(move || if disabled() { 0.45_f32 } else { 1. })
        .child(label(name, 11., 650, p.ink))
        .child(
            field(p).bg(rgb(p.surface_solid)).px(px(4.)).child(
                nsh_base::number_input()
                    .accessibility_label(name)
                    .value(move || Some(value() as f64))
                    .min(move || Some(min() as f64))
                    .max(max as f64)
                    .disabled(disabled)
                    .controls_right(true)
                    .input_class(format!(
                        "min-w-0 flex-1 h-[29px] text-[12px] text-[#{:06x}]",
                        p.ink
                    ))
                    .increment_button(label("▴", 10., 650, p.muted))
                    .decrement_button(label("▾", 10., 650, p.muted))
                    .on_change(move |v: &Option<f64>| {
                        if let Some(v) = v {
                            let n = *v as i64;
                            if (min()..=max).contains(&n) {
                                change(n);
                            }
                        }
                    }),
            ),
        )
        .child(label(hint, 10., 400, p.soft).min_h(px(25.)))
}
