use super::*;

const AUTHOR_URL: &str = "https://space.bilibili.com/8217621";
const REPO_URL: &str = "https://github.com/LambdaEd1th/dzip-tools";

fn edge(p: Palette) -> Rgba {
    rgba(if p.dark { 0x95b8c521 } else { 0x6b829424 })
}

fn shadow(y: f32, blur: f32, spread: f32, color: u32, inset: bool) -> BoxShadow {
    BoxShadow {
        offset_y: px(y),
        blur: px(blur),
        spread: px(spread),
        color: rgba(color).into(),
        inset,
        ..Default::default()
    }
}

fn control_surface(p: Palette, b: View) -> View {
    b.min_h(px(44.))
        .flex_shrink_0()
        .px(px(12.))
        .gap(px(9.))
        .rounded(px(13.))
        .border_1()
        .border_color(edge(p))
        .bg(rgba((p.soft_bg << 8) | if p.dark { 191 } else { 199 }))
}

fn control(p: Palette, b: View) -> View {
    control_surface(p, b).hover(|style| style.border_color(rgba(0x33ccbb4d)).bg(rgba(0x33ccbb13)))
}

fn dialog_frame(title: String, panel: View, close: impl Fn() + Copy + 'static) -> Modal {
    nsh_base::dialog()
        .label(title)
        .animation(ModalAnimation::None)
        .on_cancel(close)
        .child(
            view()
                .flex_1()
                .items_center()
                .justify_center()
                .child(
                    view()
                        .absolute()
                        .inset_0()
                        .bg(rgba(0x0a141c57))
                        .backdrop_blur(px(8.))
                        .focusable(false)
                        .accessibility_elements_hidden(true)
                        .on_press(close),
                )
                .child(panel),
        )
}

fn panel(p: Palette, width: f32, height: f32, small: bool) -> View {
    view()
        .w(px(width))
        .h(px(height))
        .rounded(px(if small { 23. } else { 27. }))
        .overflow_hidden()
        .border_1()
        .border_color(rgba(if p.dark { 0x94a3b838 } else { 0xffffff73 }))
        .bg(rgba(if p.dark { 0x0f172aad } else { 0xffffffb8 }))
        .bg_linear(deg(135.))
        .gradient_from(rgba(if p.dark { 0xffffff12 } else { 0xffffff4d }))
        .gradient_via(rgba(0xffffff00))
        .gradient_via_position(percentage(if p.dark { 0.38 } else { 0.34 }))
        .gradient_to(rgba(if p.dark { 0xffffff06 } else { 0xffffff1f }))
        .backdrop_blur(px(28.))
        .backdrop_saturate(if p.dark { 2.2 } else { 1.9 })
        .shadow([
            shadow(
                25.,
                50.,
                -12.,
                if p.dark { 0x0000006b } else { 0x0000002e },
                false,
            ),
            shadow(
                0.,
                0.,
                1.,
                if p.dark { 0xffffff0d } else { 0xffffff80 },
                true,
            ),
            shadow(
                1.,
                0.,
                0.,
                if p.dark { 0xffffff14 } else { 0xffffffb3 },
                true,
            ),
            shadow(-1., 0., 0., if p.dark { 0 } else { 0xffffff4d }, true),
        ])
}

fn header(
    p: Palette,
    symbol: I,
    title: String,
    subtitle: String,
    close_label: String,
    close: impl Fn() + 'static,
) -> View {
    view()
        .h(px(68.))
        .flex_shrink_0()
        .flex_row()
        .items_center()
        .pl(px(19.))
        .pr(px(15.))
        .gap(px(11.))
        .border_b_1()
        .border_color(edge(p))
        .bg_linear(deg(90.))
        .gradient_from(rgba(0x33ccbb1c))
        .gradient_to(rgba(0x33ccbb00))
        .gradient_to_position(percentage(0.72))
        .child(
            view()
                .w(px(36.))
                .h(px(36.))
                .flex_shrink_0()
                .items_center()
                .justify_center()
                .rounded(px(12.))
                .bg(rgba(0x33ccbb1f))
                .child(icon(
                    symbol,
                    if symbol == I::ScrollText { 17. } else { 16. },
                    p.accent_text,
                )),
        )
        .child(
            view()
                .flex_1()
                .min_w(px(0.))
                .gap(px(2.))
                .child(label(title, 16., 730, p.ink))
                .child(label(subtitle, 12., 400, p.soft)),
        )
        .child(
            button(close_label, close)
                .w(px(32.))
                .h(px(32.))
                .rounded(px(10.))
                .hover(move |style| style.bg(rgb(p.soft_bg)))
                .child(icon(I::X, 16., p.muted)),
        )
}

fn section(p: Palette, symbol: I, title: String, separated: bool) -> View {
    let mut section = view().flex_shrink_0();
    if separated {
        section = section.pt(px(18.)).border_t_1().border_color(edge(p));
    }
    section.child(
        view()
            .mx(px(2.))
            .mb(px(9.))
            .flex_row()
            .items_center()
            .gap(px(8.))
            .child(
                view()
                    .w(px(26.))
                    .h(px(26.))
                    .rounded(px(9.))
                    .bg(rgba(0x33ccbb1a))
                    .items_center()
                    .justify_center()
                    .child(icon(symbol, 16., p.accent_text)),
            )
            .child(label(title, 13., 720, p.muted)),
    )
}

fn themes(s: Shell, p: Palette, small: bool) -> View {
    let mut segments = nsh_base::radio_group()
        .accessibility_label(t!("appearance-mode"))
        .flex_row()
        .gap(px(4.))
        .p(px(4.))
        .rounded(px(14.))
        .bg(rgb(p.control));
    for (theme, code, key, symbol) in [
        (
            Appearance::System,
            "system",
            "appearance-system",
            I::Monitor,
        ),
        (Appearance::Light, "light", "appearance-light", I::Sun),
        (Appearance::Dark, "dark", "appearance-dark", I::Moon),
    ] {
        let active = s.theme.get() == theme;
        let color = if active { 0xffffff } else { p.muted };
        let title = t!(key);
        let mut b = nsh_base::radio()
            .accessibility_label(title.clone())
            .checked(move || s.theme.get() == theme)
            .on_change(move |_: &bool| {
                if let Err(error) = preferences::save_theme(code) {
                    crate::session_log::event!(log::Level::Error, "{error}");
                }
                set_color_scheme(theme);
                s.theme.set(theme);
            })
            .flex_row()
            .items_center()
            .justify_center()
            .cursor(Cursor::Pointer)
            .active(|s| s.opacity(0.8))
            .flex_1()
            .min_w(px(0.))
            .h(px(39.))
            .gap(px(if small { 5. } else { 7. }))
            .rounded(px(11.));
        if active {
            b = b
                .element_ref(&s.selected_theme_radio)
                .bg(rgb(p.accent))
                .shadow([shadow(5., 13., 0., 0x33ccbb38, false)]);
        }
        segments = segments.child(b.child(icon(symbol, 15., color)).child(label(
            title,
            if small { 12. } else { 13. },
            690,
            color,
        )));
    }
    section(p, I::Monitor, t!("appearance-mode"), false).child(segments)
}

fn languages(p: Palette) -> View {
    let title = t!("interface-language");
    let current = app_i18n::current_language();
    let chosen = stored(current.clone());
    let items = available_locales()
        .into_iter()
        .map(|tag| {
            let name = t!("language-self", locale = tag);
            nsh_base::SelectItem::new(tag, name)
        })
        .collect::<Vec<_>>();
    let expanded = signal(false);
    let trigger = control(p, view().flex_row().items_center())
        .w_full()
        .justify_between()
        .cursor(Cursor::Pointer)
        .child(label(t!("language-self"), 13., 650, p.body))
        .child(
            icon(I::ChevronDown, 16., p.body)
                .rotate(move || deg(if expanded.get() { 180. } else { 0. })),
        );
    section(p, I::Languages, title.clone(), false).child(
        nsh_base::select()
            .accessibility_label(title)
            .items(items)
            .value(nsh_base::SelectValue::from(current.clone()))
            .trigger(trigger)
            .cover(false)
            .offset(px(4.))
            .popup_style(move |style| {
                style
                    .rounded(px(13.))
                    .border_1()
                    .border_color(edge(p))
                    .bg(rgb(p.surface_solid))
                    .shadow_lg()
            })
            .list_style(|style| style.p(px(5.)).max_h(px(245.)))
            .item_class("rounded-[9px] focus:bg-[#33ccbb19] selected:bg-[#33ccbb19]")
            .item_style(|style| {
                style
                    .min_h(px(38.))
                    .px(px(10.))
                    .justify_center()
                    .cursor(Cursor::Pointer)
            })
            .render_item(move |item: &nsh_base::SelectEntry| {
                let active = item.value.as_ref() == current.as_str();
                view()
                    .flex_row()
                    .items_center()
                    .justify_between()
                    .child(label(
                        item.title.clone(),
                        13.,
                        630,
                        if active { p.accent_text } else { p.muted },
                    ))
                    .children(active.then(|| icon(I::Check, 15., p.accent_text)))
            })
            .on_open_change(move |event: &nsh_base::SelectEvent| {
                if let nsh_base::SelectEvent::OpenChange(open, reason) = event {
                    expanded.set(*open);
                    // Select reports a choice even if the value didn't change:
                    // picking the system language explicitly must pin it too.
                    if !open && *reason == nsh_base::OpenChangeReason::Select {
                        choose_language(&chosen.get());
                    }
                }
            })
            .on_change(move |value: &Option<SharedString>| {
                if let Some(tag) = value {
                    chosen.replace(tag.to_string());
                    if !expanded.get_untracked() {
                        choose_language(tag); // Closed-trigger typeahead.
                    }
                }
            }),
    )
}

fn choose_language(tag: &str) {
    let Some(locale) = nsh_core::platform::Locale::parse(tag) else {
        return;
    };
    if let Err(error) = preferences::save_locale(locale.as_str()) {
        crate::session_log::event!(log::Level::Error, "{error}");
    }
    let name = t!("language-self", locale = tag);
    set_locale(Some(locale));
    crate::session_log::event!(log::Level::Info, "Interface language: {name}");
}

fn about(p: Palette) -> View {
    let mut details = control_surface(p, view()).py(px(5.)).gap(px(2.));
    for (index, (key, value)) in [
        ("version", env!("CARGO_PKG_VERSION")),
        ("license", env!("CARGO_PKG_LICENSE")),
        ("author", "LambdaEd1th"),
    ]
    .into_iter()
    .enumerate()
    {
        let mut row = view()
            .h(px(34.))
            .flex_row()
            .items_center()
            .justify_between()
            .gap(px(12.));
        if index != 0 {
            row = row.border_t_1().border_color(edge(p));
        }
        row = row.child(label(t!(key), 12., 400, p.muted));
        if key == "author" {
            row = row.child(
                nsh_base::link()
                    .href(AUTHOR_URL)
                    .accessibility_label(value)
                    .child(label(value, 12., 670, p.accent_text)),
            );
        } else {
            row = row.child(label(value, 12., 670, p.ink));
        }
        details = details.child(row);
    }
    section(p, I::Info, t!("about"), true).child(details).child(
        nsh_base::link()
            .href(REPO_URL)
            .accessibility_label("GitHub")
            .child(
                control(p, view().flex_row().items_center().justify_center())
                    .mt(px(10.))
                    .gap(px(8.))
                    .child(icon(I::Github, 17., p.ink))
                    .child(label("GitHub", 13., 700, p.ink)),
            ),
    )
}

pub(super) fn settings(s: Shell, p: Palette, w: f32, h: f32) -> Modal {
    let small = w <= 420.;
    let inset = if small { 9. } else { 16. };
    let pad = if small { 16. } else { 19. };
    let close = move || s.settings.set(false);
    let view_logs = t!("view-application-logs");
    let log_section = section(p, I::ScrollText, t!("logs"), true).child(
        control(p, button(view_logs.clone(), move || s.logs.set(true)))
            .element_ref(&s.logs_opener)
            .focusable(true)
            .justify_start()
            .child(icon(I::ScrollText, 17., p.accent_text))
            .child(label(view_logs, 13., 650, p.body)),
    );
    let content = view()
        .w_full()
        .p(px(pad))
        .gap(px(19.))
        .child(themes(s, p, small))
        .child(languages(p))
        .child(log_section)
        .child(about(p));
    let fit = if small { 644. } else { 650. };
    let dialog = panel(
        p,
        (w - inset * 2.).clamp(1., 440.),
        (h - inset * 2.).clamp(1., fit),
        small,
    )
    .child(header(
        p,
        I::Settings,
        t!("settings"),
        t!("settings-subtitle"),
        t!("close-settings"),
        close,
    ))
    .child(scroll().flex_1().min_h(px(0.)).child(content));
    dialog_frame(t!("settings"), dialog, close)
}

pub(super) fn logs(s: Shell, p: Palette, w: f32, h: f32) -> Modal {
    let small = w <= 420.;
    let inset = if small { 9. } else { 16. };
    let area = view()
        .flex_1()
        .min_h(px(0.))
        .px(px(if small { 13. } else { 19. }))
        .py(px(if small { 13. } else { 16. }))
        .child(
            nsh_base::input()
                .flex_1()
                .min_h(px(0.))
                .multiline(true)
                .readonly(true)
                .auto_correct(false)
                .value(move || s.ws.logs.text())
                .placeholder(t!("no-log-entries"))
                .placeholder_color(rgb(p.muted))
                .text_color(rgb(p.body))
                .font_family("ui-monospace, Menlo, Consolas, monospace")
                .text_size(px(12.))
                .line_height(px(19.8))
                .px(px(14.))
                .py(px(13.))
                .rounded(px(13.))
                .border_1()
                .border_color(rgba(if p.dark { 0x95b8c53d } else { 0x59748738 }))
                .bg(rgba(if p.dark { 0x050c125c } else { 0x0a181f0f }))
                .label(t!("log-content")),
        );
    let mut actions = view()
        .flex_shrink_0()
        .flex_row()
        .justify_end()
        .gap(px(8.))
        .px(px(if small { 13. } else { 19. }))
        .pt(px(if small { 11. } else { 12. }))
        .pb(px(if small { 13. } else { 16. }))
        .border_t_1()
        .border_color(edge(p));
    for (export, key, symbol) in [
        (false, "clear", I::Trash),
        (true, "export-log", I::Download),
    ] {
        let color = if export { 0xffffff } else { p.body };
        let title = t!(key);
        let mut b = control(
            p,
            button(title.clone(), move || {
                if export {
                    s.ws.save_logs();
                } else {
                    s.ws.logs.clear();
                }
            }),
        )
        .min_h(px(38.))
        .px(px(13.))
        .gap(px(8.));
        if export {
            b = b
                .bg_linear(deg(135.))
                .gradient_from(rgb(0x54dccc))
                .gradient_to(rgb(0x2ebcac))
                .border_color(rgba(0x33ccbbbf))
                .shadow([
                    shadow(9., 24., 0., 0x33ccbb3d, false),
                    shadow(1., 0., 0., 0xffffff59, true),
                ]);
        } else {
            b = b.shadow([
                shadow(
                    8.,
                    28.,
                    0.,
                    if p.dark { 0x0000002e } else { 0x2b4b5f12 },
                    false,
                ),
                shadow(
                    1.,
                    0.,
                    0.,
                    if p.dark { 0xffffff09 } else { 0xffffffb8 },
                    true,
                ),
            ]);
        }
        actions = actions.child(
            b.child(icon(symbol, 16., color))
                .child(label(title, 11., 680, color)),
        );
    }
    let close = move || s.logs.set(false);
    let fit = if small { 419. } else { 469. };
    let dialog = panel(
        p,
        (w - inset * 2.).clamp(1., 650.),
        (h - inset * 2.).clamp(1., fit),
        small,
    )
    .child(header(
        p,
        I::ScrollText,
        t!("application-logs"),
        t!("logs-subtitle"),
        t!("close-logs"),
        close,
    ))
    .child(area)
    .child(actions);
    dialog_frame(t!("application-logs"), dialog, close)
}
