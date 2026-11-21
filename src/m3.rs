//! Hand-drawn Material 3 components on top of egui.

use crate::theme::{Scheme, layer, medium};
use egui::{
    Align2, Color32, CornerRadius, FontFamily, FontId, Frame, Margin, Rect, Response, Sense,
    Stroke, StrokeKind, Ui, Vec2, pos2, vec2,
};
use egui_material_icons::MaterialIcon;

pub fn font(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}
pub fn font_m(size: f32) -> FontId {
    FontId::new(size, medium())
}
pub fn icon_font(icon: MaterialIcon, size: f32) -> FontId {
    FontId::new(size, icon.font_family())
}

/// Animated hover / press state-layer opacity (M3: 8% hover, 10% pressed).
fn state(ui: &Ui, r: &Response) -> f32 {
    let target = if r.is_pointer_button_down_on() {
        0.10
    } else if r.hovered() {
        0.08
    } else {
        0.0
    };
    ui.ctx().animate_value_with_time(r.id.with("state"), target, 0.12)
}

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Filled,
    Tonal,
    Outlined,
    Text,
    Danger,
}

/// M3 common button (40dp tall, fully rounded).
pub fn button(
    ui: &mut Ui,
    s: &Scheme,
    kind: Kind,
    icon: Option<MaterialIcon>,
    label: &str,
) -> Response {
    button_enabled(ui, s, kind, icon, label, true)
}

pub fn button_enabled(
    ui: &mut Ui,
    s: &Scheme,
    kind: Kind,
    icon: Option<MaterialIcon>,
    label: &str,
    enabled: bool,
) -> Response {
    let (bg, fg) = match kind {
        Kind::Filled => (s.primary, s.on_primary),
        Kind::Tonal => (s.secondary_container, s.on_secondary_container),
        Kind::Outlined | Kind::Text => (Color32::TRANSPARENT, s.primary),
        Kind::Danger => (s.error, s.on_error),
    };
    let galley = ui.painter().layout_no_wrap(label.to_owned(), font_m(14.0), fg);
    let pad_l = if icon.is_some() { 16.0 } else { 24.0 };
    let pad_r = 24.0;
    let pad_l = if kind == Kind::Text { pad_l - 12.0 } else { pad_l };
    let pad_r = if kind == Kind::Text { 12.0 } else { pad_r };
    let icon_w = if icon.is_some() { 18.0 + 8.0 } else { 0.0 };
    let size = vec2(pad_l + icon_w + galley.size().x + pad_r, 40.0);
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(size, sense);
    if ui.is_rect_visible(rect) {
        let a = if enabled { state(ui, &resp) } else { 0.0 };
        let p = ui.painter();
        let cr = CornerRadius::same(20);
        let (bg, fg) = if enabled {
            (bg, fg)
        } else {
            let dis = s.on_surface.gamma_multiply(0.12);
            (
                if bg == Color32::TRANSPARENT { bg } else { dis },
                s.on_surface.gamma_multiply(0.38),
            )
        };
        if bg != Color32::TRANSPARENT {
            p.rect_filled(rect, cr, if a > 0.0 { layer(bg, fg, a) } else { bg });
        } else if a > 0.0 {
            p.rect_filled(rect, cr, fg.gamma_multiply(a));
        }
        if kind == Kind::Outlined {
            p.rect_stroke(rect, cr, Stroke::new(1.0, s.outline), StrokeKind::Inside);
        }
        let mut x = rect.left() + pad_l;
        if let Some(ic) = icon {
            p.text(pos2(x + 9.0, rect.center().y), Align2::CENTER_CENTER, ic.codepoint, icon_font(ic, 18.0), fg);
            x += icon_w;
        }
        p.galley_with_override_text_color(
            pos2(x, rect.center().y - galley.size().y / 2.0),
            galley,
            fg,
        );
    }
    if enabled {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    }
}

/// M3 standard icon button (40dp target, 24dp icon).
pub fn icon_button(ui: &mut Ui, s: &Scheme, icon: MaterialIcon, tip: &str) -> Response {
    icon_button_colored(ui, s, icon, tip, s.on_surface_variant, false)
}

pub fn icon_button_colored(
    ui: &mut Ui,
    s: &Scheme,
    icon: MaterialIcon,
    tip: &str,
    color: Color32,
    filled_tonal: bool,
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(40.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let a = state(ui, &resp);
        let p = ui.painter();
        let (fill, fg) = if filled_tonal {
            (s.secondary_container, s.on_secondary_container)
        } else {
            (Color32::TRANSPARENT, color)
        };
        if fill != Color32::TRANSPARENT {
            p.circle_filled(rect.center(), 20.0, layer(fill, fg, a));
        } else if a > 0.0 {
            p.circle_filled(rect.center(), 20.0, fg.gamma_multiply(a));
        }
        p.text(rect.center(), Align2::CENTER_CENTER, icon.codepoint, icon_font(icon, 22.0), fg);
    }
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    if tip.is_empty() { resp } else { resp.on_hover_text(tip) }
}

/// Small 32dp icon button used inside dense list rows.
pub fn small_icon_button(ui: &mut Ui, s: &Scheme, icon: MaterialIcon, tip: &str) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(32.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let a = state(ui, &resp);
        if a > 0.0 {
            ui.painter().circle_filled(rect.center(), 16.0, s.on_surface_variant.gamma_multiply(a));
        }
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            icon.codepoint,
            icon_font(icon, 20.0),
            s.on_surface_variant,
        );
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).on_hover_text(tip)
}

/// M3 navigation-drawer item: 56dp tall pill, secondary-container when active.
pub fn nav_item(
    ui: &mut Ui,
    s: &Scheme,
    icon: MaterialIcon,
    label: &str,
    badge: Option<(Color32, &str)>,
    selected: bool,
) -> Response {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 48.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let a = state(ui, &resp);
        let p = ui.painter();
        let cr = CornerRadius::same(24);
        let (bg, fg) = if selected {
            (s.secondary_container, s.on_secondary_container)
        } else {
            (Color32::TRANSPARENT, s.on_surface_variant)
        };
        if selected {
            p.rect_filled(rect, cr, layer(bg, fg, a));
        } else if a > 0.0 {
            p.rect_filled(rect, cr, s.on_surface.gamma_multiply(a));
        }
        p.text(pos2(rect.left() + 28.0, rect.center().y), Align2::CENTER_CENTER, icon.codepoint, icon_font(icon, 22.0), fg);
        let text_color = if selected { s.on_secondary_container } else { s.on_surface };
        let max_text = rect.width() - 56.0 - if badge.is_some() { 88.0 } else { 16.0 };
        let galley = ui.painter().layout(label.to_owned(), font_m(14.0), text_color, max_text);
        p.galley(pos2(rect.left() + 52.0, rect.center().y - galley.size().y / 2.0), galley, text_color);
        if let Some((dot, text)) = badge {
            let g = ui.painter().layout_no_wrap(text.to_owned(), font(12.0), s.on_surface_variant);
            let tx = rect.right() - 16.0 - g.size().x;
            p.circle_filled(pos2(tx - 10.0, rect.center().y), 4.0, dot);
            p.galley(pos2(tx, rect.center().y - g.size().y / 2.0), g, s.on_surface_variant);
        }
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// M3 filter chip (32dp, 8dp corners). Shows a check mark when selected.
pub fn filter_chip(ui: &mut Ui, s: &Scheme, label: &str, selected: bool) -> Response {
    let fg = if selected { s.on_secondary_container } else { s.on_surface_variant };
    let galley = ui.painter().layout_no_wrap(label.to_owned(), font_m(14.0), fg);
    let lead = if selected { 8.0 + 18.0 + 8.0 } else { 16.0 };
    let size = vec2(lead + galley.size().x + 16.0, 32.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    if ui.is_rect_visible(rect) {
        let a = state(ui, &resp);
        let p = ui.painter();
        let cr = CornerRadius::same(8);
        if selected {
            p.rect_filled(rect, cr, layer(s.secondary_container, fg, a));
            let ic = egui_material_icons::icons::ICON_CHECK;
            p.text(pos2(rect.left() + 17.0, rect.center().y), Align2::CENTER_CENTER, ic.codepoint, icon_font(ic, 18.0), fg);
        } else {
            if a > 0.0 {
                p.rect_filled(rect, cr, s.on_surface_variant.gamma_multiply(a));
            }
            p.rect_stroke(rect, cr, Stroke::new(1.0, s.outline_variant), StrokeKind::Inside);
        }
        p.galley(pos2(rect.left() + lead, rect.center().y - galley.size().y / 2.0), galley, fg);
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// M3 segmented button row. Returns the index that was clicked, if any.
pub fn segmented(ui: &mut Ui, s: &Scheme, items: &[String], selected: usize) -> Option<usize> {
    let seg_w: Vec<f32> = items
        .iter()
        .map(|t| ui.painter().layout_no_wrap(t.clone(), font_m(14.0), s.on_surface).size().x + 48.0 + 18.0)
        .collect();
    let total: f32 = seg_w.iter().sum();
    let (rect, _) = ui.allocate_exact_size(vec2(total, 40.0), Sense::hover());
    let mut clicked = None;
    let mut x = rect.left();
    for (i, (text, w)) in items.iter().zip(&seg_w).enumerate() {
        let r = Rect::from_min_size(pos2(x, rect.top()), vec2(*w, 40.0));
        let resp = ui.interact(r, ui.scope_id().with(("seg", i)), Sense::click());
        let a = state(ui, &resp);
        let sel = i == selected;
        let cr = CornerRadius {
            nw: if i == 0 { 20 } else { 0 },
            sw: if i == 0 { 20 } else { 0 },
            ne: if i + 1 == items.len() { 20 } else { 0 },
            se: if i + 1 == items.len() { 20 } else { 0 },
        };
        let fg = if sel { s.on_secondary_container } else { s.on_surface };
        let p = ui.painter();
        if sel {
            p.rect_filled(r, cr, layer(s.secondary_container, fg, a));
        } else if a > 0.0 {
            p.rect_filled(r, cr, s.on_surface.gamma_multiply(a));
        }
        p.rect_stroke(r, cr, Stroke::new(1.0, s.outline), StrokeKind::Inside);
        let g = ui.painter().layout_no_wrap(text.clone(), font_m(14.0), fg);
        let content_w = g.size().x + if sel { 26.0 } else { 0.0 };
        let mut cx = r.center().x - content_w / 2.0;
        if sel {
            let ic = egui_material_icons::icons::ICON_CHECK;
            p.text(pos2(cx + 9.0, r.center().y), Align2::CENTER_CENTER, ic.codepoint, icon_font(ic, 18.0), fg);
            cx += 26.0;
        }
        p.galley(pos2(cx, r.center().y - g.size().y / 2.0), g, fg);
        if resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            clicked = Some(i);
        }
        x += w;
    }
    clicked
}

/// M3 checkbox (18dp box inside a 40dp target).
pub fn checkbox(ui: &mut Ui, s: &Scheme, checked: &mut bool, label: &str) -> Response {
    let galley = ui.painter().layout_no_wrap(label.to_owned(), font(14.0), s.on_surface);
    let (rect, mut resp) =
        ui.allocate_exact_size(vec2(40.0 + galley.size().x + 4.0, 40.0), Sense::click());
    if resp.clicked() {
        *checked = !*checked;
        resp.mark_changed();
    }
    let a = state(ui, &resp);
    let c = pos2(rect.left() + 20.0, rect.center().y);
    let p = ui.painter();
    if a > 0.0 {
        p.circle_filled(c, 20.0, (if *checked { s.primary } else { s.on_surface }).gamma_multiply(a));
    }
    let b = Rect::from_center_size(c, Vec2::splat(18.0));
    if *checked {
        p.rect_filled(b, CornerRadius::same(2), s.primary);
        let ic = egui_material_icons::icons::ICON_CHECK;
        p.text(c, Align2::CENTER_CENTER, ic.codepoint, icon_font(ic, 16.0), s.on_primary);
    } else {
        p.rect_stroke(b, CornerRadius::same(2), Stroke::new(2.0, s.on_surface_variant), StrokeKind::Inside);
    }
    p.galley(pos2(rect.left() + 40.0, rect.center().y - galley.size().y / 2.0), galley, s.on_surface);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// M3 outlined text field.
pub fn text_field(ui: &mut Ui, s: &Scheme, value: &mut String, label: &str) -> Response {
    ui.label(egui::RichText::new(label).font(font(12.0)).color(s.primary));
    let resp = Frame::new()
        .stroke(Stroke::new(2.0, s.primary))
        .corner_radius(CornerRadius::same(4))
        .inner_margin(Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(value)
                    .frame(Frame::NONE)
                    .font(font(16.0))
                    .text_color(s.on_surface)
                    .desired_width(f32::INFINITY),
            )
        })
        .inner;
    resp
}

/// Filled card container.
pub fn card<R>(ui: &mut Ui, s: &Scheme, add: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::new()
        .fill(s.surface_container_low)
        .corner_radius(CornerRadius::same(16))
        .inner_margin(Margin::same(8))
        .show(ui, add)
        .inner
}

pub fn label_small(ui: &mut Ui, s: &Scheme, text: &str) {
    ui.label(egui::RichText::new(text).font(font_m(14.0)).color(s.on_surface_variant));
}

pub fn dialog_frame(s: &Scheme) -> Frame {
    Frame::new()
        .fill(s.surface_container_high)
        .corner_radius(CornerRadius::same(28))
        .inner_margin(Margin::same(24))
        .shadow(egui::Shadow {
            offset: [0, 4],
            blur: 16,
            spread: 0,
            color: Color32::from_black_alpha(60),
        })
}

/// M3 navigation rail destination: 56x32 indicator pill with a label underneath.
pub fn rail_item(
    ui: &mut Ui,
    s: &Scheme,
    icon: MaterialIcon,
    label: &str,
    dot: Option<Color32>,
    selected: bool,
) -> Response {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 60.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let a = state(ui, &resp);
        let p = ui.painter();
        let pill = Rect::from_center_size(pos2(rect.center().x, rect.top() + 18.0), vec2(56.0, 32.0));
        let fg = if selected { s.on_secondary_container } else { s.on_surface_variant };
        if selected {
            p.rect_filled(pill, CornerRadius::same(16), layer(s.secondary_container, fg, a));
        } else if a > 0.0 {
            p.rect_filled(pill, CornerRadius::same(16), s.on_surface.gamma_multiply(a));
        }
        p.text(pill.center(), Align2::CENTER_CENTER, icon.codepoint, icon_font(icon, 22.0), fg);
        if let Some(d) = dot {
            p.circle_filled(pos2(pill.right() - 14.0, pill.top() + 8.0), 4.0, d);
        }
        let mut job = egui::text::LayoutJob::simple_singleline(
            label.to_owned(),
            font_m(12.0),
            if selected { s.on_surface } else { s.on_surface_variant },
        );
        job.wrap = egui::text::TextWrapping::truncate_at_width(w - 4.0);
        let g = ui.painter().layout_job(job);
        ui.painter().galley(pos2(rect.center().x - g.size().x / 2.0, rect.top() + 38.0), g, s.on_surface);
    }
    resp.on_hover_text(label).on_hover_cursor(egui::CursorIcon::PointingHand)
}
