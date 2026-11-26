pub(crate) use crate::icons::icon;
use nsh_core::prelude::*;

#[derive(Clone, Copy)]
pub(crate) struct Palette {
    pub dark: bool,
    pub page: u32,
    pub page_end: u32,
    pub surface: u32,
    pub surface_solid: u32,
    pub soft_bg: u32,
    pub control: u32,
    pub line: u32,
    pub ink: u32,
    pub body: u32,
    pub muted: u32,
    pub soft: u32,
    pub accent: u32,
    pub accent_text: u32,
    pub amber: u32,
    pub danger: u32,
}

impl Palette {
    pub fn new(dark: bool) -> Self {
        let color = |light, night| if dark { night } else { light };
        Self {
            dark,
            page: color(0xfdfefe, 0x07121b),
            page_end: color(0xedf9f7, 0x0d2027),
            surface: color(0xffffff, 0x101d28),
            surface_solid: color(0xffffff, 0x122532),
            soft_bg: color(0xf7fafc, 0x182834),
            control: color(0xedf3f4, 0x1b3440),
            line: color(0xe1e8eb, 0x283c46),
            ink: color(0x263847, 0xf1f8fa),
            body: color(0x435666, 0xd0dfe4),
            muted: color(0x758897, 0x91a8b1),
            soft: color(0xa2b0ba, 0x627c87),
            accent: 0x33ccbb,
            accent_text: 0x27aa9c,
            amber: 0xf7b94b,
            danger: color(0xeb5d76, 0xff7890),
        }
    }
}

pub(crate) fn label(value: impl Into<SharedString>, size: f32, weight: u16, color: u32) -> Text {
    text(value.into())
        .text_size(px(size))
        .line_height(px(size * 1.25))
        .font_weight(FontWeight(weight))
        .text_color(rgb(color))
        .min_w(px(0.))
}

pub(crate) fn card(p: Palette, radius: f32) -> View {
    view()
        .min_w(px(0.))
        .rounded(px(radius))
        .border_1()
        .border_color(if p.dark {
            rgba(0x8fb2bf2b)
        } else {
            rgba(0xffffffc7)
        })
        .bg(rgba((p.surface << 8) | if p.dark { 184 } else { 194 }))
        .shadow([
            shadow(
                18.,
                55.,
                if p.dark { 0x00000047 } else { 0x2b4b5f17 },
                false,
            ),
            shadow(3., 12., if p.dark { 0x0000002e } else { 0x2b4b5f0d }, false),
            shadow(1., 0., if p.dark { 0xffffff09 } else { 0xffffffbf }, true),
        ])
        .backdrop_blur(px(30.))
        .backdrop_saturate(1.75)
}

pub(crate) fn shadow(y: f32, blur: f32, color: u32, inset: bool) -> BoxShadow {
    BoxShadow {
        offset_x: px(0.),
        offset_y: px(y),
        blur: px(blur),
        spread: px(0.),
        color: rgba(color).into(),
        inset,
    }
}

pub(crate) fn button(title: impl Into<SharedString>, action: impl Fn() + 'static) -> View {
    view()
        .flex_row()
        .items_center()
        .justify_center()
        .cursor(Cursor::Pointer)
        .role(AccessibilityRole::Button)
        .label(title.into())
        .active(|s| s.opacity(0.8))
        .on_press(action)
}
