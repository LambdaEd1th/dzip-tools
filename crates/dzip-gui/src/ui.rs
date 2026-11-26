//! Shared Native Shell hierarchy for the archive manager.
use crate::{
    app_i18n, icons::IconName as I, model::WorkspacePage, preferences, runtime::Workspace,
    widgets::*, workspace_ui,
};
use nsh_core::element::input;
use nsh_core::prelude::*;
mod dialogs;

#[derive(Clone, Copy)]
struct Shell {
    ws: Workspace,
    theme: Signal<Appearance>,
    selected_theme_radio: ElementRef,
    settings: Signal<bool>,
    logs: Signal<bool>,
    settings_opener: ElementRef,
    logs_opener: ElementRef,
}

pub(crate) fn shell() -> impl IntoElement {
    app_i18n::initialize();
    let theme = match preferences::read_theme().as_deref() {
        Some("light") => Appearance::Light,
        Some("dark") => Appearance::Dark,
        _ => Appearance::System,
    };
    if theme != Appearance::System {
        after_answer(move || set_color_scheme(theme));
    }
    let ws = Workspace::new();
    let s = Shell {
        ws,
        theme: signal(theme),
        selected_theme_radio: element_ref(),
        settings: signal(false),
        logs: signal(false),
        settings_opener: element_ref(),
        logs_opener: element_ref(),
    };
    // Changing appearance rebuilds the palette and its dialog. Keep the
    // selected radio focused so NSH can handle the next arrow key too.
    let previous_theme = stored(theme);
    effect(move || {
        let current = s.theme.get();
        if previous_theme.replace(current) != current && s.settings.get_untracked() {
            after_answer(move || s.selected_theme_radio.focus());
        }
    });
    // On the Web host, dismissing an inner Dialog while another remains
    // does not restore the opener. Return focus after the modal is removed.
    for (shown, opener) in [(s.settings, s.settings_opener), (s.logs, s.logs_opener)] {
        let was_open = stored(false);
        effect(move || {
            let open = shown.get();
            if was_open.replace(open) && !open {
                after_answer(move || opener.focus());
            }
        });
    }
    ws.attach_drop(view().flex_1().min_h(px(0.)).child(dynamic(move || {
        let viewport = window();
        let p = Palette::new(match s.theme.get() {
            Appearance::Dark => true,
            Appearance::Light => false,
            _ => dark(),
        });
        layout(s, p, viewport.width.0, viewport.height.0)
    })))
}

fn layout(s: Shell, p: Palette, w: f32, h: f32) -> View {
    let phone = w <= 760.;
    let sidebar = if phone {
        0.
    } else if w <= 1120. {
        92.
    } else {
        236.
    };
    let ws = s.ws;
    let margin = if phone {
        if w <= 420. { 9. } else { 12. }
    } else {
        5.
    };
    let mut body = view()
        .flex_1()
        .min_h(px(0.))
        .ml(px(sidebar))
        .pt(px(if phone { 0. } else { 14. }))
        .pr(px(if phone { 0. } else { 18. }))
        .child(dynamic(move || topbar(s, p, w)));
    // A new workspace starts at its heading. Do not carry a scrolled
    // editor's offset over to an archive (or reset the view while typing).
    body = body.child(dynamic(move || {
        let page = ws.page.get();
        scroll().flex_1().min_h(px(0.)).child(
            view()
                .w_full()
                .max_w(px(1480.))
                .self_center()
                .px(px(margin))
                .pb(px(if phone { 98. } else { 32. }))
                .pt(move || {
                    px(if phone {
                        if page == WorkspacePage::Browse && ws.archive.with(Option::is_some) {
                            75.
                        } else {
                            24.
                        }
                    } else {
                        38.
                    })
                })
                .child(dynamic(move || match page {
                    WorkspacePage::Editor => workspace_ui::editor(ws, p, w),
                    WorkspacePage::Browse if ws.archive.with(Option::is_some) => {
                        workspace_ui::browser(ws, p, w)
                    }
                    WorkspacePage::Browse => home(ws, p, w, h),
                })),
        )
    }));
    let mut root = view()
        .flex_1()
        .min_h(px(0.))
        .overflow_hidden()
        .bg_linear(deg(135.))
        .gradient_from(rgb(p.page))
        .gradient_to(rgb(p.page_end));
    for (x, y, size, color) in [
        (
            if phone { w - 300. } else { w * 0.92 - 430. },
            -140.,
            430.,
            0x33ccbb26,
        ),
        (w - 220., h * 0.92 - 340., 340., 0xff78a81c),
        (w * 0.3, h - 270., 420., 0x6aa9ff17),
    ] {
        let ambient = view().absolute();
        #[cfg(target_arch = "wasm32")]
        let ambient = ambient
            .left(px(x))
            .top(px(y))
            .w(px(size))
            .h(px(size))
            .rounded_full()
            .bg(rgba(color))
            .blur(px(80.));
        #[cfg(not(target_arch = "wasm32"))]
        let ambient = ambient
            // The native renderers don't all filter a flat View. Paint
            // the falloff rather than relying on a CSS-style layer blur.
            .left(px(x - 130.))
            .top(px(y - 130.))
            .w(px(size + 260.))
            .h(px(size + 260.))
            .bg_radial()
            .gradient_from(rgba(color))
            .gradient_via(rgba((color & 0xffffff00) | ((color & 255) / 3)))
            .gradient_to(rgba(color & 0xffffff00));
        root = root.child(ambient.opacity(if p.dark { 0.42 } else { 0.74 }));
    }
    root.child(body)
        .child(dynamic(move || navigation(ws, p, w)))
        .child(dynamic(move || {
            s.settings.get().then(|| dialogs::settings(s, p, w, h))
        }))
        .child(dynamic(move || {
            s.logs.get().then(|| dialogs::logs(s, p, w, h))
        }))
        .child(dynamic(move || {
            ws.dragging.get().then(|| {
                let browsing = ws.page.get() == WorkspacePage::Browse;
                card(p, 19.)
                    .absolute()
                    .top(px(20.))
                    .self_center()
                    .p(px(10.))
                    .flex_row()
                    .items_center()
                    .gap(px(10.))
                    .child(icon(I::CloudUpload, 20., p.accent))
                    .child(
                        view()
                            .child(label(
                                t!(if browsing {
                                    "drop-open-title"
                                } else {
                                    "drop-active-title"
                                }),
                                13.,
                                700,
                                p.ink,
                            ))
                            .child(label(
                                t!(if browsing {
                                    "drop-open-hint"
                                } else {
                                    "drop-active-hint"
                                }),
                                11.,
                                400,
                                p.muted,
                            )),
                    )
            })
        }))
        .child(dynamic(move || {
            ws.busy.get().map(|msg| {
                view()
                    .absolute()
                    .inset_0()
                    .bg(rgba(0x101d2633))
                    .backdrop_blur(px(5.))
                    .items_center()
                    .justify_center()
                    .child(
                        card(p, 22.)
                            .min_w(px(230.))
                            .px(px(28.))
                            .py(px(25.))
                            .items_center()
                            .child(
                                view()
                                    .w(px(38.))
                                    .h(px(38.))
                                    .mb(px(14.))
                                    .rounded_full()
                                    .border(px(3.))
                                    .border_color(rgba(0x33ccbb2e))
                                    .border_t_color(rgb(p.accent))
                                    .animate_spin()
                                    .role(AccessibilityRole::Progressbar),
                            )
                            .child(label(msg, 12., 700, p.ink))
                            .child(label(t!("busy-large-files"), 11., 400, p.muted).mt(px(4.))),
                    )
            })
        }))
        .child(dynamic(move || {
            ws.toast.get().map(|(ok, msg)| {
                let dismiss = move || ws.toast.set(None);
                nsh_base::toast()
                    .transition_status(nsh_base::ToastTransitionStatus::Present)
                    .accessibility_label(msg.clone())
                    .on_dismiss_request(dismiss)
                    .absolute()
                    .bottom(px(if phone { 84. } else { 24. }))
                    .right(px(if phone { 14. } else { 24. }))
                    .max_w(px((w - 32.).min(390.)))
                    .child(
                        button(t!("dismiss-notification"), dismiss)
                            .min_h(px(52.))
                            .px(px(12.))
                            .py(px(9.))
                            .rounded(px(16.))
                            .bg(rgb(p.surface_solid))
                            .shadow([shadow(18., 45., 0x1f364333, false)])
                            .border_1()
                            .border_color(rgb(p.line))
                            .gap(px(9.))
                            .child(icon(
                                if ok { I::CheckCircle } else { I::AlertCircle },
                                21.,
                                if ok { p.accent_text } else { p.danger },
                            ))
                            .child(label(msg, 11., 400, p.body))
                            .child(icon(I::X, 17., p.soft)),
                    )
            })
        }))
}

fn navigation(ws: Workspace, p: Palette, w: f32) -> View {
    let phone = w <= 760.;
    let compact = !phone && w <= 1120.;
    let mut nav = if phone {
        card(p, 22.)
            .absolute()
            .bottom(px(8.))
            .left(px(10.))
            .right(px(10.))
            .h(px(64.))
            .flex_row()
            .px(px(8.))
            .py(px(6.))
            .gap(px(5.))
    } else {
        card(p, 28.)
            .absolute()
            .top(px(14.))
            .bottom(px(14.))
            .left(px(14.))
            .w(px(if compact { 66. } else { 208. }))
            .pt(px(23.))
            .px(px(if compact { 8. } else { 14. }))
            .gap(px(7.))
    };
    if !phone {
        let mut brand = view()
            .flex_row()
            .items_center()
            .gap(px(12.))
            .mb(px(17.))
            .px(px(if compact { 0. } else { 8. }));
        if compact {
            brand = brand.justify_center();
        }
        brand = brand.child(
            view()
                .w(px(42.))
                .h(px(42.))
                .items_center()
                .justify_center()
                .rounded(px(14.))
                .overflow_hidden()
                .bg_linear(deg(145.))
                .gradient_from(rgb(0x59dfcf))
                .gradient_to(rgb(0x25b8a8))
                .shadow([
                    shadow(8., 22., 0x33ccbb47, false),
                    shadow(1., 0., 0xffffff73, true),
                ])
                .child(
                    view()
                        .absolute()
                        .top(px(-14.))
                        .left(px(-7.))
                        .w(px(30.))
                        .h(px(55.))
                        .rotate(deg(35.))
                        .bg(rgba(0xffffff2e)),
                )
                .child(icon(I::Archive, 22., 0xffffff)),
        );
        if !compact {
            brand = brand.child(
                view()
                    .child(label("Dzip", 21., 750, p.ink).letter_spacing(px(-0.84)))
                    .child(label("ARCHIVE", 10., 800, p.accent_text).letter_spacing(px(2.4))),
            );
        }
        nav = nav.child(brand);
    }
    for (page, symbol, key, hint) in [
        (
            WorkspacePage::Browse,
            I::FolderOpen,
            "nav-archive",
            "nav-archive-hint",
        ),
        (
            WorkspacePage::Editor,
            I::PackagePlus,
            "nav-editor",
            "nav-editor-hint",
        ),
    ] {
        let active = ws.page.get() == page;
        let color = if active { p.accent_text } else { p.muted };
        let title = t!(key);
        let mut b = button(title.clone(), move || ws.page.set(page))
            .hint(t!(hint))
            .rounded(px(if phone { 16. } else { 18. }))
            .gap(px(if phone { 5. } else { 7. }));
        if phone {
            b = b.flex_1();
        } else {
            b = b
                .h(px(58.))
                .transition_transform()
                .hover(|s| s.translate_x(px(2.)));
            if !compact {
                b = b.justify_start().pl(px(10.));
            }
        }
        if active {
            b = b
                .bg_linear(deg(135.))
                .gradient_from(rgba(0x33ccbb29))
                .gradient_to(rgba(0x33ccbb14))
                .shadow([shadow(7., 20., 0x33ccbb14, false)])
                .border_1()
                .border_color(rgba(0x33ccbb24))
                .child(
                    if phone {
                        view().absolute().bottom(px(-7.)).w(relative(0.4)).h(px(3.))
                    } else {
                        view()
                            .absolute()
                            .left(px(if compact { -9. } else { -15. }))
                            .w(px(3.))
                            .h(px(24.))
                    }
                    .rounded_full()
                    .bg(rgb(p.accent)),
                );
        }
        b = b.child(
            view()
                .w(px(if phone { 29. } else { 38. }))
                .h(px(if phone { 29. } else { 38. }))
                .items_center()
                .justify_center()
                .rounded(px(12.))
                .bg(rgba(if active { 0x33ccbb21 } else { 0 }))
                .child(icon(symbol, if phone { 17. } else { 20. }, color)),
        );
        if !compact {
            let mut copy = view().child(label(title, if phone { 11. } else { 14. }, 680, color));
            if !phone {
                copy = copy.child(label(t!(hint), 11., 400, p.soft));
            }
            b = b.child(copy);
        }
        nav = nav.child(b);
    }
    nav
}

fn topbar(s: Shell, p: Palette, w: f32) -> View {
    let ws = s.ws;
    let phone = w <= 760.;
    let browse = ws.page.get() == WorkspacePage::Browse;
    let mut bar = card(p, if phone { 20. } else { 24. })
        .flex_shrink_0()
        .h(px(if phone { 67. } else { 72. }))
        .flex_row()
        .items_center()
        .gap(px(8.))
        .pl(px(if phone { 15. } else { 22. }))
        .pr(px(if phone { 11. } else { 15. }))
        .z_index(5);
    if phone {
        bar = bar
            .rounded_t(px(0.))
            .border_t(px(0.))
            .border_l(px(0.))
            .border_r(px(0.));
    }
    let mut crumb = view().flex_1().min_w(px(0.));
    if !phone {
        crumb = crumb.child(
            label("WORKSPACE", 10., 800, p.soft)
                .letter_spacing(px(1.8))
                .mb(px(4.)),
        );
    }
    let mut line = view().flex_row().items_center().gap(px(9.)).child(label(
        t!(if browse {
            "page-archive-manager"
        } else {
            "page-archive-editor"
        }),
        15.,
        720,
        p.ink,
    ));
    if !phone {
        line = line
            .child(label("/", 16., 400, p.line))
            .child(dynamic(move || {
                label(
                    if browse {
                        ws.archive
                            .with(|a| a.as_ref().map(|a| a.name.clone()))
                            .unwrap_or_else(|| t!("no-archive-open"))
                    } else {
                        ws.archive_name.get()
                    },
                    12.,
                    400,
                    p.muted,
                )
            }));
    }
    bar = bar.child(crumb.child(line));
    if browse && ws.archive.with(Option::is_some) {
        let mut search = view()
            .flex_row()
            .items_center()
            .gap(px(8.))
            .px(px(12.))
            .h(px(42.))
            .rounded(px(14.))
            .bg(rgb(p.soft_bg))
            .border_1()
            .border_color(rgb(p.line));
        if phone {
            search = search.absolute().top(px(73.)).left(px(12.)).right(px(12.));
        } else {
            search = search.w(px((w * 0.2).clamp(170., 270.)));
        }
        bar = bar.child(
            search
                .child(icon(I::Search, 17., p.muted))
                .child(
                    input()
                        .flex_1()
                        .min_w(px(0.))
                        .label(t!("search-archive"))
                        .placeholder(t!("search-placeholder"))
                        .value(move || ws.search.get())
                        .on_change(move |v: &str| ws.search.set(v.to_owned()))
                        .text_size(px(13.))
                        .text_color(rgb(p.body)),
                )
                .child(dynamic(move || {
                    (!ws.search.with(String::is_empty)).then(|| {
                        button(t!("clear-search"), move || ws.search.set(String::new()))
                            .child(icon(I::X, 14., p.muted))
                    })
                })),
        );
    }
    bar.child(
        button(t!("open-settings"), move || s.settings.set(true))
            .element_ref(&s.settings_opener)
            .focusable(true)
            .w(px(42.))
            .h(px(42.))
            .rounded(px(15.))
            .border_1()
            .border_color(rgb(p.line))
            .bg(rgb(p.soft_bg))
            .child(icon(I::Settings, 20., p.muted)),
    )
}

fn home(ws: Workspace, p: Palette, w: f32, h: f32) -> View {
    let phone = w <= 760.;
    let mut visual = view().w(px(176.)).h(px(150.)).mb(px(24.)).items_center();
    for (top, angle, alpha) in [(34., -12., 0.55), (28., 9., 0.75), (23., 0., 1.)] {
        let mut layer = view()
            .absolute()
            .top(px(top))
            .left(px(36.))
            .w(px(104.))
            .h(px(82.))
            .rounded(px(24.))
            .bg(rgb(p.surface_solid))
            .border_1()
            .border_color(rgba(0x33ccbb26))
            .shadow([shadow(14., 35., 0x285b671a, false)])
            .opacity(alpha)
            .rotate(deg(angle));
        if angle == 0. {
            layer = layer
                .bg_linear(deg(145.))
                .gradient_from(rgb(p.surface_solid))
                .gradient_to(rgba(0x33ccbb1a))
                .items_center()
                .justify_center()
                .gap(px(5.))
                .child(icon(I::Archive, 40., p.accent_text))
                .child(label("DZ", 10., 900, p.accent_text).letter_spacing(px(1.8)));
        }
        visual = visual.child(layer);
    }
    for (x, y, d, c) in [
        (147., 14., 7., p.accent),
        (16., 75., 5., 0xff78a8),
        (164., 121., 4., p.amber),
    ] {
        visual = visual.child(
            view()
                .absolute()
                .left(px(x))
                .top(px(y))
                .w(px(d))
                .h(px(d))
                .rounded(px(2.))
                .rotate(deg(45.))
                .bg(rgb(c)),
        );
    }
    let mut actions = view().gap(px(9.)).items_center().mt(px(23.));
    if phone {
        actions = actions.w_full();
        visual = visual.scale(0.9);
    } else {
        actions = actions.flex_row();
    }
    for (primary, key, symbol) in [
        (true, "choose-dz-files", I::FolderOpen),
        (false, "create-new-archive", I::PackagePlus),
    ] {
        let title = t!(key);
        let color = if primary { 0xffffff } else { p.body };
        let mut b = button(title.clone(), move || {
            if primary { ws.open() } else { ws.new_archive() }
        })
        .h(px(48.))
        .px(px(20.))
        .gap(px(8.))
        .rounded(px(16.))
        .transition_transform()
        .hover(|s| s.translate_y(px(-1.)))
        .border_1()
        .border_color(rgb(if primary { p.accent } else { p.line }))
        .bg(rgb(p.soft_bg));
        if primary {
            b = b
                .bg_linear(deg(135.))
                .gradient_from(rgb(0x54dccc))
                .gradient_to(rgb(0x2ebcac))
                .shadow([
                    shadow(9., 24., 0x33ccbb3d, false),
                    shadow(1., 0., 0xffffff59, true),
                ]);
        } else {
            b = b.shadow([shadow(
                8.,
                28.,
                if p.dark { 0x00000033 } else { 0x2b4b5f12 },
                false,
            )]);
        }
        if phone {
            b = b.w_full().max_w(px(330.));
        }
        actions = actions.child(
            b.child(icon(symbol, 19., color))
                .child(label(title, 13., 680, color)),
        );
    }
    let heading = (w * 0.04).clamp(26., 38.);
    card(p, if phone { 20. } else { 24. })
        .shadow([
            shadow(8., 28., if p.dark { 0x00000033 } else { 0x2b4b5f12 }, false),
            shadow(1., 0., if p.dark { 0xffffff09 } else { 0xffffffb8 }, true),
        ])
        .min_h(px((h - if phone { 126. } else { 154. }).max(0.)))
        .items_center()
        .justify_center()
        .px(px(if phone { 18. } else { 24. }))
        .py(px(if phone { 34. } else { 46. }))
        .child(visual)
        .child(
            label(t!("empty-open-title"), heading, 760, p.ink)
                .text_center()
                .letter_spacing(px(-heading * 0.045)),
        )
        .child(
            label(t!("empty-open-description"), 12., 400, p.muted)
                .text_center()
                .mt(px(8.))
                .max_w(px(490.))
                .line_height(px(20.4)),
        )
        .child(actions)
}
