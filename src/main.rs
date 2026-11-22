#![forbid(unsafe_code)]
// Release builds on Windows are GUI apps (no console window).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod esi;
mod eve;
mod m3;
mod store;
mod theme;

use egui::{FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

/// The system UI font at a given weight: Segoe UI on Windows.
#[cfg(windows)]
fn sys_font(weight: u32) -> Option<Vec<u8>> {
    let dir = std::path::PathBuf::from(std::env::var_os("WINDIR")?).join("Fonts");
    let files: &[&str] = if weight >= 100 {
        &["seguisb.ttf", "segoeuib.ttf"]
    } else {
        &["segoeui.ttf"]
    };
    files.iter().find_map(|f| std::fs::read(dir.join(f)).ok())
}

/// Ask fontconfig for the user's sans-serif font at a given weight.
#[cfg(not(windows))]
fn sys_font(weight: u32) -> Option<Vec<u8>> {
    let out = std::process::Command::new("fc-match")
        .args(["-f", "%{file}", &format!("sans-serif:weight={weight}")])
        .output()
        .ok()?;
    let path = String::from_utf8(out.stdout).ok()?;
    std::fs::read(path.trim()).ok()
}

fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let mut medium = fonts.families[&FontFamily::Proportional].clone();
    if let Some(b) = sys_font(80) {
        fonts
            .font_data
            .insert("sys-regular".into(), Arc::new(FontData::from_owned(b)));
        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(0, "sys-regular".into());
        medium.insert(0, "sys-regular".into());
    }
    if let Some(b) = sys_font(100) {
        fonts
            .font_data
            .insert("sys-medium".into(), Arc::new(FontData::from_owned(b)));
        medium.insert(0, "sys-medium".into());
    }
    fonts
        .families
        .insert(FontFamily::Name("medium".into()), medium);
    ctx.set_fonts(fonts);
    egui_material_icons::initialize(ctx);
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_app_id(store::APP_ID)
            .with_title("EVE Settings Manager")
            .with_inner_size([1120.0, 720.0])
            .with_min_inner_size([900.0, 580.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!(
                    "../assets/eve-settings-manager.png"
                ))
                .unwrap_or_default(),
            ),
        ..Default::default()
    };
    eframe::run_native(
        "EVE Settings Manager",
        options,
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            Ok(Box::new(app::App::new(cc)))
        }),
    )
}
