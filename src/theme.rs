//! Material 3 colour scheme, loaded from a matugen-generated JSON file and
//! hot-reloaded whenever matugen rewrites it.

use egui::{Color32, CornerRadius, FontFamily, FontId, Margin, Stroke, TextStyle, Visuals};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

macro_rules! scheme {
    ($($field:ident),* $(,)?) => {
        /// Material 3 colour roles (names match matugen / material-color-utilities).
        #[derive(Clone, Copy, Debug)]
        pub struct Scheme { pub dark: bool, $(pub $field: Color32,)* }

        impl Scheme {
            /// Build a scheme from a role → "#rrggbb" map, falling back to `base` for missing roles.
            fn from_map(map: &HashMap<String, String>, base: &Scheme, dark: bool) -> Scheme {
                Scheme {
                    dark,
                    $($field: map.get(stringify!($field)).and_then(|h| parse_hex(h)).unwrap_or(base.$field),)*
                }
            }

            /// Convert a material-color-utilities scheme.
            fn from_mc(m: &material_colors::scheme::Scheme, dark: bool) -> Scheme {
                Scheme { dark, $($field: Color32::from_rgb(m.$field.red, m.$field.green, m.$field.blue),)* }
            }
        }
    };
}

scheme!(
    primary,
    on_primary,
    primary_container,
    on_primary_container,
    secondary,
    on_secondary,
    secondary_container,
    on_secondary_container,
    tertiary,
    on_tertiary,
    tertiary_container,
    on_tertiary_container,
    error,
    on_error,
    error_container,
    on_error_container,
    surface,
    on_surface,
    on_surface_variant,
    surface_container_lowest,
    surface_container_low,
    surface_container,
    surface_container_high,
    surface_container_highest,
    outline,
    outline_variant,
    inverse_surface,
    inverse_on_surface,
    inverse_primary,
    scrim,
);

fn parse_hex(s: &str) -> Option<Color32> {
    let s = s.trim().trim_start_matches('#');
    if s.len() < 6 {
        return None;
    }
    let p = |i: usize| u8::from_str_radix(&s[i..i + 2], 16).ok();
    Some(Color32::from_rgb(p(0)?, p(2)?, p(4)?))
}

const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

/// Material 3 baseline (#6750A4) dark scheme, used until matugen colours exist.
pub const BASELINE_DARK: Scheme = Scheme {
    dark: true,
    primary: rgb(0xD0BCFF),
    on_primary: rgb(0x381E72),
    primary_container: rgb(0x4F378B),
    on_primary_container: rgb(0xEADDFF),
    secondary: rgb(0xCCC2DC),
    on_secondary: rgb(0x332D41),
    secondary_container: rgb(0x4A4458),
    on_secondary_container: rgb(0xE8DEF8),
    tertiary: rgb(0xEFB8C8),
    on_tertiary: rgb(0x492532),
    tertiary_container: rgb(0x633B48),
    on_tertiary_container: rgb(0xFFD8E4),
    error: rgb(0xF2B8B5),
    on_error: rgb(0x601410),
    error_container: rgb(0x8C1D18),
    on_error_container: rgb(0xF9DEDC),
    surface: rgb(0x141218),
    on_surface: rgb(0xE6E0E9),
    on_surface_variant: rgb(0xCAC4D0),
    surface_container_lowest: rgb(0x0F0D13),
    surface_container_low: rgb(0x1D1B20),
    surface_container: rgb(0x211F26),
    surface_container_high: rgb(0x2B2930),
    surface_container_highest: rgb(0x36343B),
    outline: rgb(0x938F99),
    outline_variant: rgb(0x49454F),
    inverse_surface: rgb(0xE6E0E9),
    inverse_on_surface: rgb(0x322F35),
    inverse_primary: rgb(0x6750A4),
    scrim: rgb(0x000000),
};

/// Material 3 baseline light scheme.
pub const BASELINE_LIGHT: Scheme = Scheme {
    dark: false,
    primary: rgb(0x6750A4),
    on_primary: rgb(0xFFFFFF),
    primary_container: rgb(0xEADDFF),
    on_primary_container: rgb(0x4F378B),
    secondary: rgb(0x625B71),
    on_secondary: rgb(0xFFFFFF),
    secondary_container: rgb(0xE8DEF8),
    on_secondary_container: rgb(0x4A4458),
    tertiary: rgb(0x7D5260),
    on_tertiary: rgb(0xFFFFFF),
    tertiary_container: rgb(0xFFD8E4),
    on_tertiary_container: rgb(0x633B48),
    error: rgb(0xB3261E),
    on_error: rgb(0xFFFFFF),
    error_container: rgb(0xF9DEDC),
    on_error_container: rgb(0x8C1D18),
    surface: rgb(0xFEF7FF),
    on_surface: rgb(0x1D1B20),
    on_surface_variant: rgb(0x49454F),
    surface_container_lowest: rgb(0xFFFFFF),
    surface_container_low: rgb(0xF7F2FA),
    surface_container: rgb(0xF3EDF7),
    surface_container_high: rgb(0xECE6F0),
    surface_container_highest: rgb(0xE6E0E9),
    outline: rgb(0x79747E),
    outline_variant: rgb(0xCAC4D0),
    inverse_surface: rgb(0x322F35),
    inverse_on_surface: rgb(0xF5EFF7),
    inverse_primary: rgb(0xD0BCFF),
    scrim: rgb(0x000000),
};

/// Which palette from the matugen file to use.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Whatever mode matugen was last run with (`-m dark` / `-m light`).
    #[default]
    Auto,
    Light,
    Dark,
}

impl Mode {
    pub fn next(self) -> Mode {
        match self {
            Mode::Auto => Mode::Light,
            Mode::Light => Mode::Dark,
            Mode::Dark => Mode::Auto,
        }
    }
}

/// Watches the matugen output file and rebuilds the scheme when it changes.
pub struct ThemeSource {
    pub path: PathBuf,
    mtime: Option<SystemTime>,
    last_check: Instant,
    light: Scheme,
    dark: Scheme,
    default_dark: bool,
    pub from_matugen: bool,
}

impl ThemeSource {
    pub fn new(path: PathBuf) -> Self {
        let mut s = ThemeSource {
            path,
            mtime: None,
            last_check: Instant::now(),
            light: BASELINE_LIGHT,
            dark: BASELINE_DARK,
            default_dark: true,
            from_matugen: false,
        };
        s.reload();
        s
    }

    fn reload(&mut self) {
        self.mtime = std::fs::metadata(&self.path)
            .and_then(|m| m.modified())
            .ok();
        let Ok(text) = std::fs::read_to_string(&self.path) else {
            self.from_matugen = false;
            return;
        };
        let Ok(json) = serde_json::from_str::<Value>(&text) else {
            return; // half-written file; keep the previous scheme and retry next poll
        };
        let section = |k: &str| -> HashMap<String, String> {
            json.get(k)
                .and_then(Value::as_object)
                .map(|o| {
                    o.iter()
                        .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_string())))
                        .collect()
                })
                .unwrap_or_default()
        };
        self.light = Scheme::from_map(&section("light"), &BASELINE_LIGHT, false);
        self.dark = Scheme::from_map(&section("dark"), &BASELINE_DARK, true);
        // The "default" section is whatever mode matugen ran with; detect it by
        // comparing its surface colour with the light/dark ones.
        let def = section("default");
        self.default_dark = match def.get("surface").and_then(|h| parse_hex(h)) {
            Some(c) => c != self.light.surface,
            None => true,
        };
        self.from_matugen = true;
    }

    /// Returns true when the file changed and the scheme was reloaded.
    pub fn poll(&mut self) -> bool {
        if self.last_check.elapsed() < Duration::from_millis(750) {
            return false;
        }
        self.last_check = Instant::now();
        let mtime = std::fs::metadata(&self.path)
            .and_then(|m| m.modified())
            .ok();
        if mtime != self.mtime {
            self.reload();
            return true;
        }
        false
    }

    pub fn scheme(&self, mode: Mode) -> Scheme {
        match mode {
            Mode::Light => self.light,
            Mode::Dark => self.dark,
            Mode::Auto if self.default_dark => self.dark,
            Mode::Auto => self.light,
        }
    }
}

/// Mix `over` onto `base` with opacity `a` (M3 state layers).
pub fn layer(base: Color32, over: Color32, a: f32) -> Color32 {
    let m = |b: u8, o: u8| (b as f32 + (o as f32 - b as f32) * a).round() as u8;
    Color32::from_rgb(
        m(base.r(), over.r()),
        m(base.g(), over.g()),
        m(base.b(), over.b()),
    )
}

pub fn medium() -> FontFamily {
    FontFamily::Name("medium".into())
}

/// Apply the scheme to egui's global style so stock widgets match M3 too.
pub fn apply(ctx: &egui::Context, s: &Scheme) {
    let mut v = if s.dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    v.dark_mode = s.dark;
    v.override_text_color = Some(s.on_surface);
    v.panel_fill = s.surface;
    v.window_fill = s.surface_container_high;
    v.window_stroke = Stroke::NONE;
    v.window_corner_radius = CornerRadius::same(28);
    v.menu_corner_radius = CornerRadius::same(12);
    v.extreme_bg_color = s.surface_container_highest;
    v.faint_bg_color = s.surface_container_low;
    v.code_bg_color = s.surface_container_highest;
    v.hyperlink_color = s.primary;
    v.warn_fg_color = s.tertiary;
    v.error_fg_color = s.error;
    v.selection.bg_fill = s.primary_container;
    v.selection.stroke = Stroke::new(1.0, s.on_primary_container);
    v.text_cursor.stroke = Stroke::new(2.0, s.primary);

    let w = &mut v.widgets;
    w.noninteractive.bg_fill = s.surface_container;
    w.noninteractive.weak_bg_fill = s.surface_container;
    w.noninteractive.bg_stroke = Stroke::new(1.0, s.outline_variant);
    w.noninteractive.fg_stroke = Stroke::new(1.0, s.on_surface_variant);
    for (st, a) in [
        (&mut w.inactive, 0.0),
        (&mut w.hovered, 0.08),
        (&mut w.active, 0.12),
        (&mut w.open, 0.12),
    ] {
        let fill = layer(s.surface_container_highest, s.on_surface, a);
        st.bg_fill = fill;
        st.weak_bg_fill = fill;
        st.bg_stroke = Stroke::new(
            1.0,
            if a > 0.0 {
                s.outline
            } else {
                s.outline_variant
            },
        );
        st.fg_stroke = Stroke::new(1.5, s.on_surface);
        st.corner_radius = CornerRadius::same(8);
        st.expansion = 0.0;
    }
    w.active.bg_stroke = Stroke::new(2.0, s.primary);

    ctx.set_visuals_of(
        if s.dark {
            egui::Theme::Dark
        } else {
            egui::Theme::Light
        },
        v.clone(),
    );
    ctx.set_theme(if s.dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    });

    ctx.all_styles_mut(|style| {
        style.visuals = v.clone();
        style.text_styles = [
            (
                TextStyle::Small,
                FontId::new(12.0, FontFamily::Proportional),
            ),
            (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
            (TextStyle::Button, FontId::new(14.0, medium())),
            (
                TextStyle::Heading,
                FontId::new(22.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Monospace,
                FontId::new(13.0, FontFamily::Monospace),
            ),
        ]
        .into();
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(16.0, 8.0);
        style.spacing.interact_size.y = 32.0;
        style.spacing.window_margin = Margin::same(24);
        style.spacing.scroll.floating = true;
    });
}

// ---------------------------------------------------------------------------
// Theme selection: matugen, EVE faction presets, or a custom seed colour.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Faction {
    Caldari,
    Amarr,
    Gallente,
    Minmatar,
}

impl Faction {
    pub const ALL: [Faction; 4] = [
        Faction::Caldari,
        Faction::Amarr,
        Faction::Gallente,
        Faction::Minmatar,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Faction::Caldari => "Caldari State",
            Faction::Amarr => "Amarr Empire",
            Faction::Gallente => "Gallente Federation",
            Faction::Minmatar => "Minmatar Republic",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Faction::Caldari => "Cold steel blue",
            Faction::Amarr => "Imperial gold",
            Faction::Gallente => "Federation green",
            Faction::Minmatar => "Rust and ember",
        }
    }

    /// Seed colour the whole M3 palette is generated from.
    pub fn seed(self) -> Color32 {
        match self {
            Faction::Caldari => rgb(0x3B7DB8),
            Faction::Amarr => rgb(0xC9A23A),
            Faction::Gallente => rgb(0x2F9C74),
            Faction::Minmatar => rgb(0xB5502E),
        }
    }
}

/// Palette generation styles exposed for custom themes (M3 "scheme variants").
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Style {
    #[default]
    Tonal,
    Vibrant,
    Expressive,
    Fidelity,
    Neutral,
    Monochrome,
}

impl Style {
    pub const ALL: [Style; 6] = [
        Style::Tonal,
        Style::Vibrant,
        Style::Expressive,
        Style::Fidelity,
        Style::Neutral,
        Style::Monochrome,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Style::Tonal => "Tonal spot",
            Style::Vibrant => "Vibrant",
            Style::Expressive => "Expressive",
            Style::Fidelity => "Fidelity",
            Style::Neutral => "Neutral",
            Style::Monochrome => "Monochrome",
        }
    }

    fn variant(self) -> material_colors::dynamic_color::variant::Variant {
        use material_colors::dynamic_color::variant::Variant as V;
        match self {
            Style::Tonal => V::TonalSpot,
            Style::Vibrant => V::Vibrant,
            Style::Expressive => V::Expressive,
            Style::Fidelity => V::Fidelity,
            Style::Neutral => V::Neutral,
            Style::Monochrome => V::Monochrome,
        }
    }
}

/// A user-made theme: one seed colour plus a palette style.
#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub struct Custom {
    /// "#rrggbb"
    #[serde(with = "hex_color")]
    pub seed: Color32,
    #[serde(default)]
    pub style: Style,
}

impl Default for Custom {
    fn default() -> Self {
        Custom {
            seed: rgb(0x7A5CC7),
            style: Style::Tonal,
        }
    }
}

mod hex_color {
    use egui::Color32;
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(c: &Color32, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&super::to_hex(*c))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Color32, D::Error> {
        let s = String::deserialize(d)?;
        super::parse_hex(&s).ok_or_else(|| serde::de::Error::custom("bad colour"))
    }
}

pub fn to_hex(c: Color32) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b())
}

pub fn parse_hex_pub(s: &str) -> Option<Color32> {
    parse_hex(s)
}

/// Which source the colours come from. Stored in the app config.
#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase", tag = "kind", content = "value")]
pub enum Choice {
    Matugen,
    Faction(Faction),
    Custom,
}

/// Generate light + dark M3 schemes from a seed colour (same engine matugen uses).
pub fn generate(seed: Color32, style: Style) -> (Scheme, Scheme) {
    let theme = material_colors::theme::ThemeBuilder::with_source(
        material_colors::color::Rgb::new(seed.r(), seed.g(), seed.b()),
    )
    .variant(style.variant())
    .build();
    (
        Scheme::from_mc(&theme.schemes.light, false),
        Scheme::from_mc(&theme.schemes.dark, true),
    )
}

/// Is matugen usable here? True when the binary is on PATH or its output for
/// this app already exists.
pub fn matugen_available(colors_path: &std::path::Path) -> bool {
    colors_path.exists()
        || std::env::var_os("PATH")
            .is_some_and(|paths| std::env::split_paths(&paths).any(|d| d.join("matugen").is_file()))
}

impl ThemeSource {
    /// Resolve the scheme for the current choice + light/dark mode.
    /// EVE is a dark game, so `Auto` means dark for generated themes.
    pub fn resolve(&self, choice: Choice, custom: &Custom, mode: Mode) -> Scheme {
        let pick = |(l, d): (Scheme, Scheme)| if mode == Mode::Light { l } else { d };
        match choice {
            Choice::Matugen if self.from_matugen => self.scheme(mode),
            Choice::Matugen => pick(generate(Faction::Caldari.seed(), Style::Fidelity)),
            Choice::Faction(f) => pick(generate(f.seed(), Style::Fidelity)),
            Choice::Custom => pick(generate(custom.seed, custom.style)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faction_and_custom_generate_distinct_schemes() {
        let (l, d) = generate(Faction::Amarr.seed(), Style::Fidelity);
        assert!(!l.dark && d.dark);
        assert_ne!(l.surface, d.surface);
        let (_, cal) = generate(Faction::Caldari.seed(), Style::Fidelity);
        assert_ne!(cal.primary, d.primary);
        // primary must contrast with on_primary
        let lum = |c: Color32| c.r() as i32 + c.g() as i32 + c.b() as i32;
        assert!((lum(d.primary) - lum(d.on_primary)).abs() > 200);
    }

    #[test]
    fn choice_roundtrips_through_json() {
        for c in [
            Choice::Matugen,
            Choice::Custom,
            Choice::Faction(Faction::Gallente),
        ] {
            let j = serde_json::to_string(&c).unwrap();
            assert_eq!(serde_json::from_str::<Choice>(&j).unwrap(), c);
        }
        let custom = Custom {
            seed: Color32::from_rgb(1, 2, 3),
            style: Style::Vibrant,
        };
        let j = serde_json::to_string(&custom).unwrap();
        assert!(j.contains("#010203"));
        assert_eq!(serde_json::from_str::<Custom>(&j).unwrap(), custom);
    }

    #[test]
    fn matugen_file_parses() {
        let dir = std::env::temp_dir().join(format!("esm-theme-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("colors.json");
        std::fs::write(&p, r##"{"default":{"surface":"#111111"},"light":{"surface":"#fafafa","primary":"#123456"},"dark":{"surface":"#111111","primary":"#abcdef"}}"##).unwrap();
        let src = ThemeSource::new(p);
        assert!(src.from_matugen);
        assert_eq!(
            src.scheme(Mode::Auto).primary,
            Color32::from_rgb(0xab, 0xcd, 0xef)
        );
        assert_eq!(
            src.scheme(Mode::Light).primary,
            Color32::from_rgb(0x12, 0x34, 0x56)
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
