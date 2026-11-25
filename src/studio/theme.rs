use eframe::egui::{self, Color32, FontId, Frame, Stroke, TextStyle, vec2};

pub(crate) const INK: Color32 = Color32::from_rgb(230, 237, 248);
pub(crate) const MUTED: Color32 = Color32::from_rgb(164, 178, 201);
pub(crate) const ACCENT: Color32 = Color32::from_rgb(161, 176, 255);
pub(crate) const HIGH: Color32 = Color32::from_rgb(98, 222, 190);
pub(crate) const ANALOG: Color32 = Color32::from_rgb(209, 173, 255);
pub(crate) const OUTPUT: Color32 = Color32::from_rgb(255, 194, 129);
pub(crate) const BORDER: Color32 = Color32::from_rgb(57, 70, 93);
pub(crate) const PAPER: Color32 = Color32::from_rgb(15, 21, 32);
pub(crate) const DIAGRAM_PAPER: Color32 = Color32::WHITE;
pub(crate) const DIAGRAM_GRID: Color32 = Color32::from_rgb(207, 216, 230);
pub(crate) const DIAGRAM_MUTED: Color32 = Color32::from_rgb(78, 92, 115);
pub(crate) const CARD: Color32 = Color32::from_rgb(30, 40, 57);
pub(crate) const SHELL: Color32 = Color32::from_rgb(23, 31, 45);
pub(crate) const DARK: Color32 = Color32::from_rgb(12, 17, 27);
pub(crate) const SELECTED: Color32 = Color32::from_rgb(47, 58, 103);
pub(crate) const LIVE: Color32 = Color32::from_rgb(24, 60, 58);
pub(crate) const WIRE: Color32 = Color32::from_rgb(25, 30, 36);

pub(crate) fn style(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::Dark);
    let mut style = (*ctx.style()).clone();
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = SHELL;
    visuals.window_fill = SHELL;
    visuals.window_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.extreme_bg_color = PAPER;
    visuals.widgets.noninteractive.fg_stroke.color = MUTED;
    for (widget, fill, outline) in [
        (&mut visuals.widgets.inactive, CARD, BORDER),
        (&mut visuals.widgets.hovered, SELECTED, ACCENT),
        (&mut visuals.widgets.active, SELECTED, ACCENT),
        (&mut visuals.widgets.open, SELECTED, ACCENT),
    ] {
        widget.bg_fill = fill;
        widget.weak_bg_fill = fill;
        widget.bg_stroke = Stroke::new(1.0_f32, outline);
        widget.fg_stroke.color = INK;
        widget.corner_radius = 6.into();
    }
    visuals.selection.bg_fill = SELECTED;
    visuals.selection.stroke = Stroke::new(1.0_f32, ACCENT);
    style.visuals = visuals;
    style
        .text_styles
        .insert(TextStyle::Body, FontId::proportional(14.0));
    style.spacing.item_spacing = vec2(8.0, 5.0);
    style.spacing.button_padding = vec2(10.0, 5.0);
    ctx.set_style_of(egui::Theme::Dark, style);
}

pub(super) fn panel(fill: Color32, horizontal: i8, vertical: i8) -> Frame {
    Frame::new()
        .fill(fill)
        .inner_margin(egui::Margin::symmetric(horizontal, vertical))
}
