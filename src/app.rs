use crate::esi;
use crate::eve::{self, Backup, BackupKind, Esi, FileKind, Profile, Server, SettingFile};
use crate::m3::{self, Kind, font, font_m, icon_font};
use crate::store::{self, Store};
use crate::theme::{self, Choice, Custom, Faction, Mode, Scheme, Style, ThemeSource, layer};
use egui::{
    Align, Align2, Color32, CornerRadius, Frame, Layout, Margin, RichText, ScrollArea, Sense,
    Stroke, StrokeKind, Ui, UiBuilder, pos2, vec2,
};
use egui_material_icons::icons::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant, SystemTime};

enum NameAction {
    NewProfile,
    Rename(String),
    Duplicate(String),
    BackupProfile,
    BackupFile(SettingFile),
}

enum ConfirmAction {
    DeleteProfile(String),
    Sync(PathBuf, Vec<PathBuf>),
    Restore(Backup),
    DeleteBackup(Backup),
    OverwriteBackup(String),
}

enum Dialog {
    Name {
        action: NameAction,
        value: String,
        error: Option<String>,
    },
    Confirm {
        title: String,
        body: String,
        suggest_backup: bool,
        danger: bool,
        action: ConfirmAction,
    },
    Sync {
        src: SettingFile,
        targets: Vec<(SettingFile, bool)>,
    },
    Note {
        id: String,
        text: String,
    },
    Info,
    Backups,
    Appearance {
        hex: String,
    },
}

pub struct App {
    store: Store,
    theme_src: ThemeSource,
    matugen_installed: bool,
    scheme: Scheme,
    scheme_key: Option<(Choice, Custom, Mode)>,

    roots: Vec<PathBuf>,
    root: Option<PathBuf>,
    servers: Vec<Server>,
    server: Option<usize>,
    status: HashMap<Esi, Option<bool>>,
    profiles: Vec<Profile>,
    profile: Option<String>,
    files: Vec<SettingFile>,
    tab: FileKind,
    backups: Vec<Backup>,

    dialog: Option<Dialog>,
    snack: Option<(String, Instant)>,
    esi: esi::Client,
    folder_rx: Option<Receiver<Option<PathBuf>>>,
    last_scan: Instant,
}

fn fmt_time(t: SystemTime) -> String {
    chrono::DateTime::<chrono::Local>::from(t)
        .format("%b %-d, %Y  %H:%M")
        .to_string()
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let store = Store::load();
        let colors = store::matugen_colors_path();
        let mut app = App {
            matugen_installed: theme::matugen_available(&colors),
            theme_src: ThemeSource::new(colors),
            scheme: theme::BASELINE_DARK,
            scheme_key: None,
            store,
            roots: vec![],
            root: None,
            servers: vec![],
            server: None,
            status: HashMap::new(),
            profiles: vec![],
            profile: None,
            files: vec![],
            tab: FileKind::Char,
            backups: eve::list_backups(),
            dialog: None,
            snack: None,
            esi: esi::Client::new(cc.egui_ctx.clone()),
            folder_rx: None,
            last_scan: Instant::now(),
        };
        app.update_scheme(&cc.egui_ctx);
        app.roots = eve::find_roots();
        let custom = app.store.custom_eve_folder.clone();
        if !custom.is_empty() && Path::new(&custom).is_dir() {
            app.set_folder(PathBuf::from(custom), false);
        } else if let Some(r) = app.roots.first().cloned() {
            app.set_folder(r, false);
        }
        #[cfg(debug_assertions)]
        app.demo_view();
        app
    }

    /// Debug builds only: open a given view at start-up, for README screenshots.
    /// `ESM_DEMO_VIEW=accounts|appearance|sync|backups|confirm`
    #[cfg(debug_assertions)]
    fn demo_view(&mut self) {
        let Ok(view) = std::env::var("ESM_DEMO_VIEW") else {
            return;
        };
        let chars: Vec<SettingFile> = self
            .files
            .iter()
            .filter(|f| f.kind == FileKind::Char)
            .cloned()
            .collect();
        match view.as_str() {
            "accounts" => self.tab = FileKind::User,
            "appearance" => self.open_appearance(),
            "backups" => self.dialog = Some(Dialog::Backups),
            "sync" if !chars.is_empty() => {
                let targets = chars[1..]
                    .iter()
                    .enumerate()
                    .map(|(i, f)| (f.clone(), i < 2))
                    .collect();
                self.dialog = Some(Dialog::Sync {
                    src: chars[0].clone(),
                    targets,
                });
            }
            "confirm" => {
                self.dialog = Some(Dialog::Confirm {
                    title: "Overwrite settings?".into(),
                    body: "This will overwrite 2 file(s) with the selected settings.".into(),
                    suggest_backup: true,
                    danger: false,
                    action: ConfirmAction::Sync(PathBuf::new(), vec![]),
                });
            }
            _ => {}
        }
    }

    // ----- theme ---------------------------------------------------------

    fn choice(&self) -> Choice {
        self.store.theme.unwrap_or(if self.theme_src.from_matugen {
            Choice::Matugen
        } else {
            Choice::Faction(Faction::Caldari)
        })
    }

    fn update_scheme(&mut self, ctx: &egui::Context) {
        let key = (
            self.choice(),
            self.store.custom_theme,
            self.store.theme_mode,
        );
        if self.scheme_key != Some(key) {
            self.scheme = self.theme_src.resolve(key.0, &key.1, key.2);
            theme::apply(ctx, &self.scheme);
            self.scheme_key = Some(key);
        }
    }

    // ----- data ----------------------------------------------------------

    fn toast(&mut self, msg: impl Into<String>) {
        self.snack = Some((msg.into(), Instant::now()));
    }

    fn report<T>(&mut self, r: anyhow::Result<T>, ok: impl FnOnce(T) -> String) {
        match r {
            Ok(v) => {
                let m = ok(v);
                self.toast(m)
            }
            Err(e) => self.toast(format!("Error: {e:#}")),
        }
    }

    fn set_folder(&mut self, picked: PathBuf, remember: bool) {
        let (root, server) = eve::resolve_picked(&picked);
        if remember {
            self.store.custom_eve_folder = root.to_string_lossy().to_string();
            self.store.save();
        }
        self.servers = eve::detect_servers(&root);
        self.root = Some(root);
        let want = server
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| self.store.last_active_server.clone());
        let idx = self
            .servers
            .iter()
            .position(|s| s.path.to_string_lossy() == want)
            .or((!self.servers.is_empty()).then_some(0));
        for s in self.servers.clone() {
            if !self.status.contains_key(&s.esi) {
                self.status.insert(s.esi, None);
                if s.esi == Esi::Other {
                    self.status.insert(s.esi, Some(false));
                } else {
                    self.esi.status(s.esi);
                }
            }
        }
        match idx {
            Some(i) => self.select_server(i),
            None => {
                self.server = None;
                self.profiles.clear();
                self.files.clear();
            }
        }
    }

    fn current_server(&self) -> Option<&Server> {
        self.server.and_then(|i| self.servers.get(i))
    }

    fn select_server(&mut self, i: usize) {
        self.server = Some(i);
        let path = self.servers[i].path.clone();
        self.store.last_active_server = path.to_string_lossy().to_string();
        self.store.save();
        self.profiles = eve::list_profiles(&path);
        let remembered = self
            .store
            .server_profiles
            .get(&*path.to_string_lossy())
            .cloned();
        self.profile = remembered
            .filter(|n| self.profiles.iter().any(|p| &p.name == n))
            .or_else(|| {
                self.profiles
                    .iter()
                    .find(|p| p.name == "Default")
                    .map(|p| p.name.clone())
            })
            .or_else(|| self.profiles.first().map(|p| p.name.clone()));
        self.load_files();
    }

    fn select_profile(&mut self, name: Option<String>) {
        self.profile = name.clone();
        if let (Some(s), Some(n)) = (self.current_server(), name) {
            let key = s.path.to_string_lossy().to_string();
            self.store.server_profiles.insert(key, n);
            self.store.save();
        }
        self.load_files();
    }

    fn profile_path(&self) -> Option<PathBuf> {
        let s = self.current_server()?;
        let n = self.profile.as_ref()?;
        Some(s.path.join(format!("settings_{n}")))
    }

    fn load_files(&mut self) {
        self.files = self
            .profile_path()
            .map(|p| eve::list_settings(&p))
            .unwrap_or_default();
        let missing: Vec<String> = self
            .files
            .iter()
            .filter(|f| f.kind == FileKind::Char && !self.store.char_names.contains_key(&f.id))
            .map(|f| f.id.clone())
            .collect();
        if let Some(s) = self.current_server() {
            self.esi.names(s.esi, missing);
        }
    }

    fn reload_profiles(&mut self) {
        if let Some(i) = self.server {
            let keep = self.profile.clone();
            self.profiles = eve::list_profiles(&self.servers[i].path);
            let p = keep
                .filter(|n| self.profiles.iter().any(|p| &p.name == n))
                .or_else(|| self.profiles.first().map(|p| p.name.clone()));
            self.select_profile(p);
        }
    }

    fn rescan(&mut self) {
        if let Some(r) = self.root.clone() {
            let srv = self.current_server().map(|s| s.path.clone());
            self.servers = eve::detect_servers(&r);
            self.server = srv.and_then(|p| self.servers.iter().position(|s| s.path == p));
            if self.server.is_some() {
                self.reload_profiles();
            }
        }
        self.backups = eve::list_backups();
    }

    fn char_name(&self, id: &str) -> Option<String> {
        self.store.char_names.get(id).cloned()
    }

    fn file_label(&self, f: &SettingFile) -> String {
        match f.kind {
            FileKind::Char => self
                .char_name(&f.id)
                .unwrap_or_else(|| format!("Character {}", f.id)),
            FileKind::User => {
                let note = self
                    .store
                    .descriptions
                    .get(&f.id)
                    .cloned()
                    .unwrap_or_default();
                if note.is_empty() {
                    format!("Account {}", f.id)
                } else {
                    note
                }
            }
        }
    }

    fn pick_folder(&mut self) {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(
                rfd::FileDialog::new()
                    .set_title("Select EVE Settings Folder")
                    .pick_folder(),
            );
        });
        self.folder_rx = Some(rx);
    }

    fn tick(&mut self, ctx: &egui::Context) {
        while let Ok(r) = self.esi.rx.try_recv() {
            match r {
                esi::Reply::Status(e, ok) => {
                    self.status.insert(e, Some(ok));
                }
                esi::Reply::Names(m) => {
                    self.store.char_names.extend(m);
                    self.store.save();
                }
            }
        }
        if let Some(rx) = &self.folder_rx
            && let Ok(res) = rx.try_recv()
        {
            self.folder_rx = None;
            if let Some(p) = res {
                self.set_folder(p, true);
            }
        }
        if self.theme_src.poll() {
            self.matugen_installed = theme::matugen_available(&self.theme_src.path);
            self.scheme_key = None;
        }
        self.update_scheme(ctx);
        // Pick up changes made by the game client while we're open.
        if self.last_scan.elapsed() > Duration::from_secs(3) && self.dialog.is_none() {
            self.last_scan = Instant::now();
            if let Some(p) = self.profile_path() {
                let new = eve::list_settings(&p);
                if new.len() != self.files.len()
                    || new
                        .iter()
                        .zip(&self.files)
                        .any(|(a, b)| a.path != b.path || a.modified != b.modified)
                {
                    self.load_files();
                }
            }
        }
        ctx.request_repaint_after(Duration::from_secs(1));
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.tick(&ctx);
        let s = self.scheme;

        // M3 responsive layout: drawer on wide windows, rail when tiled narrow.
        let narrow = ctx.content_rect().width() < 980.0;
        egui::Panel::left("nav")
            .exact_size(if narrow { 88.0 } else { 292.0 })
            .resizable(false)
            .show_separator_line(false)
            .frame(
                Frame::new()
                    .fill(s.surface_container_low)
                    .inner_margin(Margin::symmetric(if narrow { 4 } else { 12 }, 16)),
            )
            .show(ui, |ui| {
                if narrow {
                    self.rail(ui)
                } else {
                    self.sidebar(ui)
                }
            });

        egui::CentralPanel::default()
            .frame(Frame::new().fill(s.surface).inner_margin(Margin {
                left: 28,
                right: 28,
                top: 20,
                bottom: 16,
            }))
            .show(ui, |ui| self.main(ui));

        if let Some(mut d) = self.dialog.take()
            && self.show_dialog(&ctx, &mut d)
            && self.dialog.is_none()
        {
            self.dialog = Some(d);
        }
        self.snackbar(&ctx);
    }
}

// ---------------------------------------------------------------------------
// Sidebar
// ---------------------------------------------------------------------------

impl App {
    fn rail(&mut self, ui: &mut Ui) {
        let s = self.scheme;
        ui.spacing_mut().item_spacing.y = 4.0;
        ui.with_layout(Layout::bottom_up(Align::Center), |ui| {
            if m3::rail_item(ui, &s, ICON_INFO, "About", None, false).clicked() {
                self.dialog = Some(Dialog::Info);
            }
            if m3::rail_item(ui, &s, ICON_PALETTE, "Theme", None, false).clicked() {
                self.open_appearance();
            }
            if m3::rail_item(ui, &s, ICON_FOLDER_OPEN, "Folder", None, false).clicked() {
                self.pick_folder();
            }
            ui.with_layout(Layout::top_down(Align::Center), |ui| {
                let (r, _) = ui.allocate_exact_size(vec2(56.0, 56.0), Sense::hover());
                ui.painter()
                    .rect_filled(r, CornerRadius::same(16), s.primary_container);
                ui.painter().text(
                    r.center(),
                    Align2::CENTER_CENTER,
                    ICON_ROCKET_LAUNCH.codepoint,
                    icon_font(ICON_ROCKET_LAUNCH, 26.0),
                    s.on_primary_container,
                );
                ui.add_space(16.0);
                let mut pick = None;
                for (i, srv) in self.servers.iter().enumerate() {
                    let dot = match self.status.get(&srv.esi).copied().flatten() {
                        _ if srv.esi == Esi::Other => s.outline,
                        Some(true) => Color32::from_rgb(0x4c, 0xc3, 0x6b),
                        Some(false) => s.error,
                        None => s.outline,
                    };
                    if m3::rail_item(
                        ui,
                        &s,
                        ICON_DNS,
                        &srv.display,
                        Some(dot),
                        self.server == Some(i),
                    )
                    .clicked()
                    {
                        pick = Some(i);
                    }
                }
                if let Some(i) = pick {
                    self.select_server(i);
                }
                if m3::rail_item(
                    ui,
                    &s,
                    ICON_HISTORY,
                    &format!("Backups ({})", self.backups.len()),
                    None,
                    false,
                )
                .clicked()
                {
                    self.dialog = Some(Dialog::Backups);
                }
            });
        });
    }

    fn sidebar(&mut self, ui: &mut Ui) {
        let s = self.scheme;
        ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
            ui.add_space(4.0);
            if m3::nav_item(ui, &s, ICON_INFO, "About & paths", None, false).clicked() {
                self.dialog = Some(Dialog::Info);
            }
            if m3::nav_item(ui, &s, ICON_PALETTE, "Appearance", None, false).clicked() {
                self.open_appearance();
            }
            if m3::nav_item(
                ui,
                &s,
                ICON_FOLDER_OPEN,
                "Select settings folder",
                None,
                false,
            )
            .clicked()
            {
                self.pick_folder();
            }
            ui.add_space(4.0);
            divider(ui, &s);

            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    let (r, _) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::hover());
                    ui.painter()
                        .rect_filled(r, CornerRadius::same(12), s.primary_container);
                    ui.painter().text(
                        r.center(),
                        Align2::CENTER_CENTER,
                        ICON_ROCKET_LAUNCH.codepoint,
                        icon_font(ICON_ROCKET_LAUNCH, 24.0),
                        s.on_primary_container,
                    );
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.label(
                            RichText::new("EVE Settings")
                                .font(font_m(18.0))
                                .color(s.on_surface),
                        );
                        ui.label(
                            RichText::new("Manager")
                                .font(font(13.0))
                                .color(s.on_surface_variant),
                        );
                    });
                });
                ui.add_space(16.0);
                section(ui, &s, "Servers");

                let mut pick = None;
                for (i, srv) in self.servers.iter().enumerate() {
                    let (dot, text) = match self.status.get(&srv.esi).copied().flatten() {
                        _ if srv.esi == Esi::Other => (s.outline, "No ESI"),
                        Some(true) => (Color32::from_rgb(0x4c, 0xc3, 0x6b), "Online"),
                        Some(false) => (s.error, "Offline"),
                        None => (s.outline, "Checking…"),
                    };
                    if m3::nav_item(
                        ui,
                        &s,
                        ICON_DNS,
                        &srv.display,
                        Some((dot, text)),
                        self.server == Some(i),
                    )
                    .on_hover_text(srv.path.display().to_string())
                    .clicked()
                    {
                        pick = Some(i);
                    }
                }
                if self.servers.is_empty() {
                    ui.add_space(4.0);
                    ui.label(RichText::new("    No servers found").color(s.on_surface_variant));
                }
                if let Some(i) = pick {
                    self.select_server(i);
                }

                ui.add_space(12.0);
                divider(ui, &s);
                section(ui, &s, "Backups");
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| self.backup_list(ui));
            });
        });
    }

    fn backup_list(&mut self, ui: &mut Ui) {
        let s = self.scheme;
        if self.backups.is_empty() {
            ui.label(RichText::new("    No backups yet").color(s.on_surface_variant));
            return;
        }
        let mut act: Option<(u8, Backup)> = None;
        for b in &self.backups {
            let w = ui.available_width();
            let (rect, resp) = ui.allocate_exact_size(vec2(w, 60.0), Sense::hover());
            let hovered = resp.hovered() || ui.rect_contains_pointer(rect);
            if hovered {
                ui.painter().rect_filled(
                    rect,
                    CornerRadius::same(16),
                    s.on_surface.gamma_multiply(0.06),
                );
            }
            let (icon, sub) = match b.kind {
                BackupKind::Folder => (
                    ICON_FOLDER_ZIP,
                    format!("{} files · {}", b.file_count, fmt_time(b.created)),
                ),
                BackupKind::File { .. } => (
                    ICON_DESCRIPTION,
                    format!("single file · {}", fmt_time(b.created)),
                ),
            };
            let p = ui.painter();
            p.text(
                pos2(rect.left() + 28.0, rect.center().y),
                Align2::CENTER_CENTER,
                icon.codepoint,
                icon_font(icon, 22.0),
                s.on_surface_variant,
            );
            let text_w = rect.width() - 52.0 - if hovered { 108.0 } else { 8.0 };
            let g = one_line(ui, &b.display, font_m(14.0), s.on_surface, text_w);
            let p = ui.painter();
            p.galley(pos2(rect.left() + 52.0, rect.top() + 11.0), g, s.on_surface);
            p.text(
                pos2(rect.left() + 52.0, rect.top() + 32.0),
                Align2::LEFT_TOP,
                sub,
                font(12.0),
                s.on_surface_variant,
            );
            if hovered {
                let r = egui::Rect::from_min_max(pos2(rect.right() - 108.0, rect.top()), rect.max);
                let mut c = ui.new_child(
                    UiBuilder::new()
                        .max_rect(r)
                        .layout(Layout::right_to_left(Align::Center)),
                );
                c.spacing_mut().item_spacing.x = 2.0;
                if m3::small_icon_button(&mut c, &s, ICON_DELETE, "Delete backup").clicked() {
                    act = Some((2, b.clone()));
                }
                if m3::small_icon_button(&mut c, &s, ICON_FOLDER_OPEN, "Show in folder").clicked() {
                    act = Some((1, b.clone()));
                }
                if m3::small_icon_button(
                    &mut c,
                    &s,
                    ICON_SETTINGS_BACKUP_RESTORE,
                    "Put back to profile",
                )
                .clicked()
                {
                    act = Some((0, b.clone()));
                }
            }
        }
        match act {
            Some((0, b)) => {
                if self.profile_path().is_none() {
                    self.toast("Select a profile first");
                    return;
                }
                let body = match &b.kind {
                    BackupKind::Folder => format!(
                        "All files from \"{}\" will overwrite the current profile.",
                        b.display
                    ),
                    BackupKind::File { target } => format!(
                        "This will overwrite \"{target}\" in the current profile with the backup."
                    ),
                };
                self.dialog = Some(Dialog::Confirm {
                    title: "Overwrite settings?".into(),
                    body,
                    suggest_backup: true,
                    danger: false,
                    action: ConfirmAction::Restore(b),
                });
            }
            Some((1, b)) => {
                let target = if b.path.is_dir() {
                    b.path.clone()
                } else {
                    b.path.parent().unwrap_or(&b.path).to_path_buf()
                };
                let _ = open::that_detached(target);
            }
            Some((_, b)) => {
                self.dialog = Some(Dialog::Confirm {
                    title: "Delete backup?".into(),
                    body: format!("This will permanently delete the backup \"{}\".", b.display),
                    suggest_backup: false,
                    danger: true,
                    action: ConfirmAction::DeleteBackup(b),
                });
            }
            None => {}
        }
    }
}

/// Lay out text on one line, ellipsised to fit `width`.
fn one_line(
    ui: &Ui,
    text: &str,
    font: egui::FontId,
    color: Color32,
    width: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::simple_singleline(text.to_owned(), font, color);
    job.wrap = egui::text::TextWrapping::truncate_at_width(width.max(20.0));
    ui.painter().layout_job(job)
}

fn section(ui: &mut Ui, s: &Scheme, t: &str) {
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        ui.label(
            RichText::new(t)
                .font(font_m(14.0))
                .color(s.on_surface_variant),
        );
    });
    ui.add_space(2.0);
}

fn divider(ui: &mut Ui, s: &Scheme) {
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, 9.0), Sense::hover());
    ui.painter().hline(
        r.x_range().shrink(16.0),
        r.center().y,
        Stroke::new(1.0, s.outline_variant),
    );
}

// ---------------------------------------------------------------------------
// Main content
// ---------------------------------------------------------------------------

impl App {
    fn main(&mut self, ui: &mut Ui) {
        let s = self.scheme;
        let Some(server) = self.current_server().cloned() else {
            self.empty_state(ui);
            return;
        };

        // Top app bar
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(
                    RichText::new(&server.display)
                        .font(FontId28())
                        .color(s.on_surface),
                );
                ui.label(
                    RichText::new(server.name.clone())
                        .font(font(13.0))
                        .color(s.on_surface_variant),
                )
                .on_hover_text(server.path.display().to_string());
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let mode_icon = match self.store.theme_mode {
                    Mode::Auto => ICON_BRIGHTNESS_AUTO,
                    Mode::Light => ICON_LIGHT_MODE,
                    Mode::Dark => ICON_DARK_MODE,
                };
                if m3::icon_button(ui, &s, mode_icon, "Light / dark / auto").clicked() {
                    self.store.theme_mode = self.store.theme_mode.next();
                    self.store.save();
                }
                if m3::icon_button(ui, &s, ICON_PALETTE, "Appearance").clicked() {
                    self.open_appearance();
                }
                if m3::icon_button(ui, &s, ICON_REFRESH, "Rescan").clicked() {
                    self.rescan();
                    self.toast("Rescanned settings folder");
                }
                let has_profile = self.profile_path().is_some();
                if m3::icon_button(ui, &s, ICON_FOLDER_OPEN, "Open profile folder").clicked()
                    && let Some(p) = self.profile_path()
                {
                    let _ = open::that_detached(p);
                }
                ui.add_space(4.0);
                let narrow = ui.available_width() < 420.0;
                let clicked = if narrow {
                    has_profile
                        && m3::icon_button_colored(
                            ui,
                            &s,
                            ICON_BACKUP,
                            "Backup profile",
                            s.on_secondary_container,
                            true,
                        )
                        .clicked()
                } else {
                    m3::button_enabled(
                        ui,
                        &s,
                        Kind::Filled,
                        Some(ICON_BACKUP),
                        "Backup profile",
                        has_profile,
                    )
                    .clicked()
                };
                if clicked {
                    let name = format!(
                        "{} - {} - {}",
                        server.display,
                        self.profile.clone().unwrap_or_default(),
                        chrono::Local::now().format("%Y-%m-%d %H%M")
                    );
                    self.dialog = Some(Dialog::Name {
                        action: NameAction::BackupProfile,
                        value: name,
                        error: None,
                    });
                }
            });
        });
        ui.add_space(16.0);

        // Profiles
        m3::label_small(ui, &s, "Profiles");
        ui.horizontal_wrapped(|ui| {
            let mut pick = None;
            for p in &self.profiles {
                if m3::filter_chip(ui, &s, &p.name, self.profile.as_deref() == Some(&p.name)).clicked() {
                    pick = Some(p.name.clone());
                }
            }
            if let Some(p) = pick {
                self.select_profile(Some(p));
            }
            if m3::small_icon_button(ui, &s, ICON_ADD, "New profile").clicked() {
                self.dialog = Some(Dialog::Name { action: NameAction::NewProfile, value: String::new(), error: None });
            }
            if let Some(cur) = self.profile.clone() {
                ui.add_space(8.0);
                if m3::small_icon_button(ui, &s, ICON_EDIT, "Rename profile").clicked() {
                    self.dialog = Some(Dialog::Name { action: NameAction::Rename(cur.clone()), value: cur.clone(), error: None });
                }
                if m3::small_icon_button(ui, &s, ICON_CONTENT_COPY, "Duplicate profile").clicked() {
                    self.dialog = Some(Dialog::Name {
                        action: NameAction::Duplicate(cur.clone()),
                        value: format!("Copy of {cur}"),
                        error: None,
                    });
                }
                if m3::small_icon_button(ui, &s, ICON_DELETE, "Delete profile").clicked() {
                    if self.profiles.len() <= 1 {
                        self.toast("Cannot delete the only profile");
                    } else {
                        self.dialog = Some(Dialog::Confirm {
                            title: "Delete profile?".into(),
                            body: format!("This will permanently delete \"{cur}\" and all its settings files. This cannot be undone."),
                            suggest_backup: true,
                            danger: true,
                            action: ConfirmAction::DeleteProfile(cur),
                        });
                    }
                }
            }
        });
        ui.add_space(16.0);

        if self.profiles.is_empty() {
            centered_message(
                ui,
                &s,
                ICON_FOLDER_OFF,
                "No profiles yet",
                "Click + above to create your first profile",
            );
            return;
        }

        let nc = self
            .files
            .iter()
            .filter(|f| f.kind == FileKind::Char)
            .count();
        let nu = self.files.len() - nc;
        let items = [format!("Characters  {nc}"), format!("Accounts  {nu}")];
        let sel = if self.tab == FileKind::Char { 0 } else { 1 };
        if let Some(i) = m3::segmented(ui, &s, &items, sel) {
            self.tab = if i == 0 {
                FileKind::Char
            } else {
                FileKind::User
            };
        }
        ui.add_space(12.0);

        m3::card(ui, &s, |ui| {
            ui.set_min_height(ui.available_height());
            self.table(ui);
        });
    }

    fn table(&mut self, ui: &mut Ui) {
        let s = self.scheme;
        let kind = self.tab;
        let rows: Vec<SettingFile> = self
            .files
            .iter()
            .filter(|f| f.kind == kind)
            .cloned()
            .collect();
        let w = ui.available_width();
        let act_w = 96.0;
        let id_w = if w < 640.0 { 110.0 } else { 130.0 };
        let show_mod = w >= 560.0;
        let mod_w = if !show_mod {
            0.0
        } else if w < 640.0 {
            150.0
        } else {
            170.0
        };
        let name_w = (w - act_w - id_w - mod_w - 16.0).max(120.0);
        let cols = |x0: f32| [x0 + 16.0, x0 + 16.0 + name_w, x0 + 16.0 + name_w + id_w];

        // header
        let (hr, _) = ui.allocate_exact_size(vec2(w, 40.0), Sense::hover());
        let c = cols(hr.left());
        let first = if kind == FileKind::Char {
            "Character"
        } else {
            "Account"
        };
        for (x, t) in c
            .iter()
            .zip([first, "ID", if show_mod { "Modified" } else { "" }])
        {
            ui.painter().text(
                pos2(*x, hr.center().y),
                Align2::LEFT_CENTER,
                t,
                font_m(13.0),
                s.on_surface_variant,
            );
        }
        ui.painter().hline(
            hr.x_range(),
            hr.bottom(),
            Stroke::new(1.0, s.outline_variant),
        );

        if rows.is_empty() {
            ui.add_space(24.0);
            let msg = if kind == FileKind::Char {
                "No character files found"
            } else {
                "No account files found"
            };
            ui.vertical_centered(|ui| ui.label(RichText::new(msg).color(s.on_surface_variant)));
            return;
        }

        let mut action: Option<(u8, SettingFile)> = None;
        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for f in &rows {
                    let (rect, _) = ui.allocate_exact_size(vec2(w, 60.0), Sense::hover());
                    let hovered = ui.rect_contains_pointer(rect);
                    if hovered {
                        ui.painter().rect_filled(
                            rect,
                            CornerRadius::same(12),
                            s.on_surface.gamma_multiply(0.06),
                        );
                    }
                    let c = cols(rect.left());
                    let p = ui.painter();
                    // avatar
                    let label = self.file_label(f);
                    let resolved = match kind {
                        FileKind::Char => self.char_name(&f.id).is_some(),
                        FileKind::User => self
                            .store
                            .descriptions
                            .get(&f.id)
                            .is_some_and(|n| !n.is_empty()),
                    };
                    let av = pos2(c[0] + 20.0, rect.center().y);
                    let (abg, afg) = if kind == FileKind::Char {
                        (s.primary_container, s.on_primary_container)
                    } else {
                        (s.tertiary_container, s.on_tertiary_container)
                    };
                    p.circle_filled(av, 20.0, abg);
                    if kind == FileKind::Char && resolved {
                        let ch = label
                            .chars()
                            .next()
                            .unwrap_or('?')
                            .to_uppercase()
                            .to_string();
                        p.text(av, Align2::CENTER_CENTER, ch, font_m(16.0), afg);
                    } else {
                        let ic = if kind == FileKind::Char {
                            ICON_PERSON
                        } else {
                            ICON_ACCOUNT_CIRCLE
                        };
                        p.text(
                            av,
                            Align2::CENTER_CENTER,
                            ic.codepoint,
                            icon_font(ic, 22.0),
                            afg,
                        );
                    }
                    let g = one_line(
                        ui,
                        &label,
                        font(15.0),
                        if resolved {
                            s.on_surface
                        } else {
                            s.on_surface_variant
                        },
                        name_w - 60.0,
                    );
                    p.galley(
                        pos2(c[0] + 52.0, rect.center().y - g.size().y / 2.0),
                        g,
                        s.on_surface,
                    );
                    p.text(
                        pos2(c[1], rect.center().y),
                        Align2::LEFT_CENTER,
                        &f.id,
                        FontIdMono(),
                        s.on_surface_variant,
                    );
                    if show_mod {
                        p.text(
                            pos2(c[2], rect.center().y),
                            Align2::LEFT_CENTER,
                            fmt_time(f.modified),
                            font(13.0),
                            s.on_surface_variant,
                        );
                    }

                    let ar = egui::Rect::from_min_max(
                        pos2(rect.right() - act_w - 8.0, rect.top()),
                        pos2(rect.right() - 8.0, rect.bottom()),
                    );
                    let mut cu = ui.new_child(
                        UiBuilder::new()
                            .max_rect(ar)
                            .layout(Layout::right_to_left(Align::Center)),
                    );
                    cu.spacing_mut().item_spacing.x = 0.0;
                    if m3::small_icon_button(&mut cu, &s, ICON_SAVE, "Back up this file").clicked()
                    {
                        action = Some((1, f.clone()));
                    }
                    let tip = if kind == FileKind::Char {
                        "Sync to other characters"
                    } else {
                        "Sync to other accounts"
                    };
                    if m3::small_icon_button(&mut cu, &s, ICON_SYNC, tip).clicked() {
                        action = Some((0, f.clone()));
                    }
                    if kind == FileKind::User
                        && m3::small_icon_button(&mut cu, &s, ICON_EDIT_NOTE, "Edit note").clicked()
                    {
                        action = Some((2, f.clone()));
                    }
                }
            });

        match action {
            Some((0, f)) => {
                let targets = rows
                    .iter()
                    .filter(|o| o.path != f.path)
                    .map(|o| (o.clone(), false))
                    .collect();
                self.dialog = Some(Dialog::Sync { src: f, targets });
            }
            Some((1, f)) => {
                let value = format!(
                    "{} — {}",
                    self.file_label(&f),
                    chrono::Local::now().format("%Y-%m-%d %H:%M")
                );
                self.dialog = Some(Dialog::Name {
                    action: NameAction::BackupFile(f),
                    value,
                    error: None,
                });
            }
            Some((_, f)) => {
                let text = self
                    .store
                    .descriptions
                    .get(&f.id)
                    .cloned()
                    .unwrap_or_default();
                self.dialog = Some(Dialog::Note { id: f.id, text });
            }
            None => {}
        }
    }

    fn empty_state(&mut self, ui: &mut Ui) {
        let s = self.scheme;
        ui.add_space(ui.available_height() * 0.22);
        ui.vertical_centered(|ui| {
            let (r, _) = ui.allocate_exact_size(vec2(96.0, 96.0), Sense::hover());
            ui.painter().circle_filled(r.center(), 48.0, s.secondary_container);
            ui.painter().text(r.center(), Align2::CENTER_CENTER, ICON_FOLDER_OFF.codepoint, icon_font(ICON_FOLDER_OFF, 48.0), s.on_secondary_container);
            ui.add_space(16.0);
            ui.label(RichText::new("EVE settings folder not found").font(font(24.0)).color(s.on_surface));
            ui.label(RichText::new("Make sure EVE Online has been launched once (Steam/Proton, Lutris, Bottles or Wine), or pick the CCP/EVE folder manually.").color(s.on_surface_variant));
            ui.add_space(16.0);
            if m3::button(ui, &s, Kind::Filled, Some(ICON_FOLDER_OPEN), "Select settings folder").clicked() {
                self.pick_folder();
            }
            ui.add_space(24.0);
            ui.label(RichText::new("Searched locations").font(font_m(13.0)).color(s.on_surface_variant));
            for h in eve::PATTERN_HINTS {
                ui.label(RichText::new(*h).font(FontIdMono()).color(s.on_surface_variant));
            }
        });
    }
}

#[allow(non_snake_case)]
fn FontId28() -> egui::FontId {
    font(28.0)
}
#[allow(non_snake_case)]
fn FontIdMono() -> egui::FontId {
    egui::FontId::monospace(13.0)
}

fn centered_message(
    ui: &mut Ui,
    s: &Scheme,
    icon: egui_material_icons::MaterialIcon,
    title: &str,
    sub: &str,
) {
    ui.add_space(48.0);
    ui.vertical_centered(|ui| {
        ui.label(
            RichText::new(icon.codepoint)
                .font(icon_font(icon, 48.0))
                .color(s.on_surface_variant),
        );
        ui.label(RichText::new(title).font(font(20.0)).color(s.on_surface));
        ui.label(RichText::new(sub).color(s.on_surface_variant));
    });
}

// ---------------------------------------------------------------------------
// Dialogs
// ---------------------------------------------------------------------------

impl App {
    fn open_appearance(&mut self) {
        self.dialog = Some(Dialog::Appearance {
            hex: theme::to_hex(self.store.custom_theme.seed),
        });
    }

    /// Returns true to keep the dialog open.
    fn show_dialog(&mut self, ctx: &egui::Context, d: &mut Dialog) -> bool {
        let s = self.scheme;
        let width = match d {
            Dialog::Appearance { .. } => 560.0_f32,
            Dialog::Info | Dialog::Sync { .. } | Dialog::Backups => 520.0,
            _ => 400.0,
        };
        let width = width.min(ctx.content_rect().width() - 96.0).max(280.0);
        let mut keep = true;
        let resp = egui::Modal::new(egui::Id::unique("dialog"))
            .frame(m3::dialog_frame(&s))
            .backdrop_color(s.scrim.gamma_multiply(0.4))
            .show(ctx, |ui| {
                ui.set_width(width);
                keep = self.dialog_body(ui, d);
            });
        if resp.should_close() {
            keep = false;
        }
        keep
    }

    fn title(ui: &mut Ui, s: &Scheme, t: &str) {
        ui.label(RichText::new(t).font(font(24.0)).color(s.on_surface));
        ui.add_space(8.0);
    }

    fn actions(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
        ui.add_space(16.0);
        ui.with_layout(Layout::right_to_left(Align::Center), add);
    }

    fn dialog_body(&mut self, ui: &mut Ui, d: &mut Dialog) -> bool {
        let s = self.scheme;
        let mut keep = true;
        match d {
            Dialog::Name {
                action,
                value,
                error,
            } => {
                let (title, label, ok) = match action {
                    NameAction::NewProfile => ("New profile", "Profile name", "Create"),
                    NameAction::Rename(_) => ("Rename profile", "Profile name", "Rename"),
                    NameAction::Duplicate(_) => ("Duplicate profile", "Profile name", "Duplicate"),
                    NameAction::BackupProfile => ("Save backup", "Backup name", "Save"),
                    NameAction::BackupFile(_) => ("Back up file", "Backup name", "Save"),
                };
                Self::title(ui, &s, title);
                let r = m3::text_field(ui, &s, value, label);
                if !r.has_focus() && error.is_none() && ui.memory(|m| m.focused().is_none()) {
                    r.request_focus();
                }
                let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if let Some(e) = error {
                    ui.label(RichText::new(e.as_str()).font(font(12.0)).color(s.error));
                }
                let mut submit = enter;
                Self::actions(ui, |ui| {
                    submit |= m3::button(ui, &s, Kind::Text, None, ok).clicked();
                    if m3::button(ui, &s, Kind::Text, None, "Cancel").clicked() {
                        keep = false;
                    }
                });
                if submit {
                    let v = value.trim().to_string();
                    let server = self.current_server().map(|s| s.path.clone());
                    let res: anyhow::Result<String> = (|| {
                        eve::validate_name(&v)?;
                        let server = server.ok_or_else(|| anyhow::anyhow!("No server selected"))?;
                        match action {
                            NameAction::NewProfile => {
                                eve::create_profile(&server, &v)?;
                                Ok(format!("Created profile \"{v}\""))
                            }
                            NameAction::Rename(old) => {
                                eve::rename_profile(&server, old, &v)?;
                                Ok(format!("Renamed to \"{v}\""))
                            }
                            NameAction::Duplicate(src) => {
                                eve::duplicate_profile(&server, src, &v)?;
                                Ok(format!("Duplicated as \"{v}\""))
                            }
                            NameAction::BackupProfile => {
                                if eve::folder_backup_exists(&v) {
                                    anyhow::bail!("__exists");
                                }
                                let p = self
                                    .profile_path()
                                    .ok_or_else(|| anyhow::anyhow!("Select a profile"))?;
                                let n = eve::backup_profile(&p, &v)?;
                                Ok(format!("Backed up {n} files"))
                            }
                            NameAction::BackupFile(f) => {
                                eve::backup_file(f, &v)?;
                                Ok("File backed up".to_string())
                            }
                        }
                    })();
                    match res {
                        Ok(msg) => {
                            keep = false;
                            self.toast(msg);
                            match action {
                                NameAction::NewProfile
                                | NameAction::Rename(_)
                                | NameAction::Duplicate(_) => {
                                    self.reload_profiles();
                                    self.select_profile(Some(v));
                                }
                                _ => self.backups = eve::list_backups(),
                            }
                        }
                        Err(e) if e.to_string() == "__exists" => {
                            keep = false;
                            self.dialog = Some(Dialog::Confirm {
                                title: "Backup already exists".into(),
                                body: format!(
                                    "A backup named \"{v}\" already exists. Overwrite it?"
                                ),
                                suggest_backup: false,
                                danger: false,
                                action: ConfirmAction::OverwriteBackup(v),
                            });
                        }
                        Err(e) => *error = Some(format!("{e:#}")),
                    }
                }
            }

            Dialog::Confirm {
                title,
                body,
                suggest_backup,
                danger,
                ..
            } => {
                Self::title(ui, &s, title);
                ui.label(RichText::new(body.as_str()).color(s.on_surface_variant));
                if *suggest_backup {
                    ui.add_space(8.0);
                    Frame::new().fill(s.tertiary_container).corner_radius(CornerRadius::same(12)).inner_margin(Margin::same(12)).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(ICON_LIGHTBULB.codepoint).font(icon_font(ICON_LIGHTBULB, 20.0)).color(s.on_tertiary_container));
                            ui.label(RichText::new("Consider creating a backup first to avoid losing your current settings.").color(s.on_tertiary_container));
                        });
                    });
                }
                let mut go = false;
                let danger = *danger;
                Self::actions(ui, |ui| {
                    go = m3::button(
                        ui,
                        &s,
                        if danger { Kind::Danger } else { Kind::Filled },
                        None,
                        "Proceed",
                    )
                    .clicked();
                    if m3::button(ui, &s, Kind::Text, None, "Cancel").clicked() {
                        keep = false;
                    }
                });
                if go {
                    keep = false;
                    let Dialog::Confirm { action, .. } = std::mem::replace(d, Dialog::Info) else {
                        unreachable!()
                    };
                    self.run_confirm(action);
                }
            }

            Dialog::Sync { src, targets } => {
                Self::title(ui, &s, "Sync to…");
                ui.label(
                    RichText::new(format!(
                        "Copy the settings of {} onto:",
                        self.file_label(src)
                    ))
                    .color(s.on_surface_variant),
                );
                ui.add_space(4.0);
                if targets.is_empty() {
                    ui.label(
                        RichText::new("No other files to sync to.").color(s.on_surface_variant),
                    );
                } else {
                    let mut all = targets.iter().all(|t| t.1);
                    if m3::checkbox(ui, &s, &mut all, "Select all").changed() {
                        targets.iter_mut().for_each(|t| t.1 = all);
                    }
                    divider(ui, &s);
                    ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                        for (f, on) in targets.iter_mut() {
                            let label = format!("{}   ·   {}", self.file_label(f), f.id);
                            m3::checkbox(ui, &s, on, &label);
                        }
                    });
                }
                let n = targets.iter().filter(|t| t.1).count();
                let mut go = false;
                Self::actions(ui, |ui| {
                    go = m3::button_enabled(
                        ui,
                        &s,
                        Kind::Filled,
                        Some(ICON_SYNC),
                        &format!("Sync {n}"),
                        n > 0,
                    )
                    .clicked();
                    if m3::button(ui, &s, Kind::Text, None, "Cancel").clicked() {
                        keep = false;
                    }
                });
                if go {
                    keep = false;
                    let paths: Vec<PathBuf> = targets
                        .iter()
                        .filter(|t| t.1)
                        .map(|t| t.0.path.clone())
                        .collect();
                    self.dialog = Some(Dialog::Confirm {
                        title: "Overwrite settings?".into(),
                        body: format!(
                            "This will overwrite {n} file(s) with the selected settings."
                        ),
                        suggest_backup: true,
                        danger: false,
                        action: ConfirmAction::Sync(src.path.clone(), paths),
                    });
                }
            }

            Dialog::Note { id, text } => {
                Self::title(ui, &s, "Account note");
                ui.label(RichText::new(format!("Account {id}")).color(s.on_surface_variant));
                ui.add_space(4.0);
                let r = m3::text_field(ui, &s, text, "Note");
                let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                let mut save = enter;
                let mut clear = false;
                Self::actions(ui, |ui| {
                    save |= m3::button(ui, &s, Kind::Text, None, "Save").clicked();
                    if m3::button(ui, &s, Kind::Text, None, "Cancel").clicked() {
                        keep = false;
                    }
                    clear = m3::button(ui, &s, Kind::Text, None, "Clear note").clicked();
                });
                if save || clear {
                    let t = if clear {
                        String::new()
                    } else {
                        text.trim().to_string()
                    };
                    if t.is_empty() {
                        self.store.descriptions.remove(id.as_str());
                    } else {
                        self.store.descriptions.insert(id.clone(), t);
                    }
                    self.store.save();
                    keep = false;
                }
            }

            Dialog::Info => {
                Self::title(ui, &s, "About & paths");
                m3::label_small(ui, &s, "Settings folder");
                let cur = self
                    .root
                    .as_ref()
                    .map(|r| r.display().to_string())
                    .unwrap_or_else(|| "Not found".into());
                ui.label(RichText::new(cur).font(FontIdMono()).color(s.on_surface));
                if self.roots.len() > 1 {
                    ui.add_space(6.0);
                    m3::label_small(ui, &s, "Other detected EVE folders");
                    let mut pick = None;
                    for r in &self.roots {
                        if Some(r) != self.root.as_ref()
                            && m3::nav_item(
                                ui,
                                &s,
                                ICON_FOLDER,
                                &r.display().to_string(),
                                None,
                                false,
                            )
                            .clicked()
                        {
                            pick = Some(r.clone());
                        }
                    }
                    if let Some(r) = pick {
                        self.set_folder(r, true);
                    }
                }
                ui.add_space(8.0);
                ui.label(RichText::new("Found automatically in Proton / Wine prefixes. You can pick the top-level CCP/EVE folder or a server subfolder.").color(s.on_surface_variant));
                ui.add_space(12.0);
                m3::label_small(ui, &s, "Local data");
                ui.label(RichText::new("Backups, notes and preferences are stored locally. Only character IDs are sent to EVE's public ESI to look up names and server status.").color(s.on_surface_variant));
                for (k, v) in [
                    ("Config", store::config_dir()),
                    ("Backups", eve::backups_dir()),
                    ("Matugen colours", store::matugen_colors_path()),
                ] {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("{k}:"))
                                .font(font_m(13.0))
                                .color(s.on_surface_variant),
                        );
                        ui.label(
                            RichText::new(v.display().to_string())
                                .font(FontIdMono())
                                .color(s.on_surface),
                        );
                    });
                }
                Self::actions(ui, |ui| {
                    if m3::button(ui, &s, Kind::Text, None, "Close").clicked() {
                        keep = false;
                    }
                    if m3::button(
                        ui,
                        &s,
                        Kind::Text,
                        Some(ICON_FOLDER_OPEN),
                        "Open data folder",
                    )
                    .clicked()
                    {
                        let _ = std::fs::create_dir_all(store::data_dir());
                        let _ = open::that_detached(store::data_dir());
                    }
                });
            }

            Dialog::Backups => {
                Self::title(ui, &s, "Backups");
                ScrollArea::vertical()
                    .max_height(420.0)
                    .show(ui, |ui| self.backup_list(ui));
                Self::actions(ui, |ui| {
                    if m3::button(ui, &s, Kind::Text, None, "Close").clicked() {
                        keep = false;
                    }
                });
            }

            Dialog::Appearance { hex } => keep = self.appearance(ui, hex),
        }
        keep
    }

    fn run_confirm(&mut self, action: ConfirmAction) {
        match action {
            ConfirmAction::DeleteProfile(name) => {
                if let Some(srv) = self.current_server().map(|s| s.path.clone()) {
                    let r = eve::delete_profile(&srv, &name);
                    self.report(r, |_| format!("Deleted profile \"{name}\""));
                    self.profile = None;
                    self.reload_profiles();
                }
            }
            ConfirmAction::Sync(src, targets) => {
                let r = eve::copy_settings(&src, &targets);
                self.report(r, |n| format!("Synced settings to {n} file(s)"));
                self.load_files();
            }
            ConfirmAction::Restore(b) => {
                if let Some(p) = self.profile_path() {
                    let r = eve::restore_backup(&b, &p);
                    self.report(r, |n| {
                        format!("Restored {n} file(s) from \"{}\"", b.display)
                    });
                    self.load_files();
                }
            }
            ConfirmAction::DeleteBackup(b) => {
                let r = eve::delete_backup(&b);
                self.report(r, |_| "Backup deleted".to_string());
                self.backups = eve::list_backups();
            }
            ConfirmAction::OverwriteBackup(name) => {
                if let Some(p) = self.profile_path() {
                    let r = eve::backup_profile(&p, &name);
                    self.report(r, |n| format!("Backed up {n} files"));
                    self.backups = eve::list_backups();
                }
            }
        }
    }

    fn appearance(&mut self, ui: &mut Ui, hex: &mut String) -> bool {
        let s = self.scheme;
        let mut keep = true;
        let choice = self.choice();
        let mut new_choice = None;
        Self::title(ui, &s, "Appearance");

        m3::label_small(ui, &s, "Brightness");
        let modes = ["Auto".to_string(), "Light".to_string(), "Dark".to_string()];
        let sel = match self.store.theme_mode {
            Mode::Auto => 0,
            Mode::Light => 1,
            Mode::Dark => 2,
        };
        if let Some(i) = m3::segmented(ui, &s, &modes, sel) {
            self.store.theme_mode = [Mode::Auto, Mode::Light, Mode::Dark][i];
            self.store.save();
        }
        ui.add_space(12.0);

        // Matugen
        m3::label_small(ui, &s, "Wallpaper colours");
        let mg_ok = self.theme_src.from_matugen;
        let (mg_sub, mg_enabled) = if mg_ok {
            (
                "Follows your wallpaper via matugen · updates live".to_string(),
                true,
            )
        } else if self.matugen_installed {
            (
                "matugen found, but the app template isn't set up yet — run install.sh".to_string(),
                false,
            )
        } else {
            (
                "matugen isn't installed — pick a faction or custom theme below".to_string(),
                false,
            )
        };
        let preview = if mg_ok {
            self.theme_src.scheme(self.store.theme_mode)
        } else {
            s
        };
        if theme_card(
            ui,
            &s,
            choice == Choice::Matugen && mg_ok,
            &preview,
            "Matugen",
            &mg_sub,
            mg_enabled,
            ICON_WALLPAPER,
        )
        .clicked()
            && mg_enabled
        {
            new_choice = Some(Choice::Matugen);
        }
        ui.add_space(12.0);

        // Factions
        m3::label_small(ui, &s, "EVE factions");
        let dark = self.store.theme_mode != Mode::Light;
        for pair in Faction::ALL.chunks(2) {
            ui.columns(2, |cols| {
                for (col, f) in cols.iter_mut().zip(pair) {
                    let (l, d) = theme::generate(f.seed(), Style::Fidelity);
                    let pv = if dark { d } else { l };
                    if theme_card(
                        col,
                        &s,
                        choice == Choice::Faction(*f),
                        &pv,
                        f.name(),
                        f.blurb(),
                        true,
                        ICON_SHIELD,
                    )
                    .clicked()
                    {
                        new_choice = Some(Choice::Faction(*f));
                    }
                }
            });
        }
        ui.add_space(12.0);

        // Custom
        m3::label_small(ui, &s, "Custom");
        let (l, d) = theme::generate(self.store.custom_theme.seed, self.store.custom_theme.style);
        let pv = if dark { d } else { l };
        if theme_card(
            ui,
            &s,
            choice == Choice::Custom,
            &pv,
            "Custom theme",
            "Your own seed colour and palette style",
            true,
            ICON_COLORIZE,
        )
        .clicked()
        {
            new_choice = Some(Choice::Custom);
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            let mut c = self.store.custom_theme.seed;
            let r = egui::color_picker::color_edit_button_srgba(
                ui,
                &mut c,
                egui::color_picker::Alpha::Opaque,
            );
            if r.changed() {
                self.store.custom_theme.seed = c;
                *hex = theme::to_hex(c);
                new_choice = Some(Choice::Custom);
            }
            let te = ui.add(
                egui::TextEdit::singleline(hex)
                    .desired_width(90.0)
                    .font(FontIdMono())
                    .margin(Margin::symmetric(10, 8)),
            );
            if te.changed()
                && let Some(c) = theme::parse_hex_pub(hex)
            {
                self.store.custom_theme.seed = c;
                new_choice = Some(Choice::Custom);
            }
            ui.label(RichText::new("Seed colour").color(s.on_surface_variant));
        });
        ui.horizontal_wrapped(|ui| {
            for st in Style::ALL {
                if m3::filter_chip(ui, &s, st.name(), self.store.custom_theme.style == st).clicked()
                {
                    self.store.custom_theme.style = st;
                    new_choice = Some(Choice::Custom);
                }
            }
        });

        if let Some(c) = new_choice {
            self.store.theme = Some(c);
            self.store.save();
        }

        Self::actions(ui, |ui| {
            if m3::button(ui, &s, Kind::Text, None, "Done").clicked() {
                keep = false;
            }
        });
        keep
    }

    fn snackbar(&mut self, ctx: &egui::Context) {
        let s = self.scheme;
        let Some((msg, t)) = &self.snack else { return };
        if t.elapsed() > Duration::from_secs(4) {
            self.snack = None;
            return;
        }
        let msg = msg.clone();
        egui::Area::new(egui::Id::unique("snackbar"))
            .anchor(Align2::CENTER_BOTTOM, vec2(0.0, -24.0))
            .order(egui::Order::Tooltip)
            .show(ctx, |ui| {
                Frame::new()
                    .fill(s.inverse_surface)
                    .corner_radius(CornerRadius::same(4))
                    .inner_margin(Margin::symmetric(16, 14))
                    .shadow(egui::Shadow {
                        offset: [0, 3],
                        blur: 8,
                        spread: 0,
                        color: Color32::from_black_alpha(50),
                    })
                    .show(ui, |ui| {
                        ui.label(RichText::new(msg).color(s.inverse_on_surface));
                    });
            });
        ctx.request_repaint_after(Duration::from_millis(250));
    }
}

/// Selectable theme option with a three-colour swatch.
#[allow(clippy::too_many_arguments)]
fn theme_card(
    ui: &mut Ui,
    s: &Scheme,
    selected: bool,
    preview: &Scheme,
    title: &str,
    sub: &str,
    enabled: bool,
    icon: egui_material_icons::MaterialIcon,
) -> egui::Response {
    let w = ui.available_width();
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 64.0), sense);
    let p = ui.painter();
    let cr = CornerRadius::same(16);
    let hover = enabled && resp.hovered();
    let bg = if selected {
        s.secondary_container
    } else {
        s.surface_container
    };
    p.rect_filled(
        rect,
        cr,
        if hover {
            layer(bg, s.on_surface, 0.08)
        } else {
            bg
        },
    );
    if selected {
        p.rect_stroke(rect, cr, Stroke::new(2.0, s.primary), StrokeKind::Inside);
    }
    // swatch: big primary half + secondary/tertiary quarters, like Android's picker
    let c = pos2(rect.left() + 32.0, rect.center().y);
    let r = 18.0;
    p.circle_filled(c, r + 2.0, preview.surface_container_highest);
    p.circle_filled(c, r, preview.primary);
    let q = egui::Rect::from_min_max(pos2(c.x, c.y), pos2(c.x + r, c.y + r));
    p.rect_filled(
        q,
        CornerRadius {
            se: r as u8,
            ..Default::default()
        },
        preview.tertiary_container,
    );
    let q2 = egui::Rect::from_min_max(pos2(c.x - r, c.y), pos2(c.x, c.y + r));
    p.rect_filled(
        q2,
        CornerRadius {
            sw: r as u8,
            ..Default::default()
        },
        preview.secondary_container,
    );
    let fg = if enabled {
        s.on_surface
    } else {
        s.on_surface.gamma_multiply(0.5)
    };
    let sub_fg = if enabled {
        s.on_surface_variant
    } else {
        s.on_surface_variant.gamma_multiply(0.7)
    };
    let tx = rect.left() + 64.0;
    p.text(
        pos2(tx, rect.top() + 14.0),
        Align2::LEFT_TOP,
        title,
        font_m(15.0),
        fg,
    );
    let g = p.layout(sub.to_owned(), font(12.0), sub_fg, rect.right() - tx - 44.0);
    p.galley(pos2(tx, rect.top() + 34.0), g, sub_fg);
    let ic = if selected { ICON_CHECK_CIRCLE } else { icon };
    p.text(
        pos2(rect.right() - 22.0, rect.center().y),
        Align2::CENTER_CENTER,
        ic.codepoint,
        icon_font(ic, 22.0),
        if selected { s.primary } else { sub_fg },
    );
    if enabled {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    }
}
