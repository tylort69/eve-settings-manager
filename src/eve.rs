//! EVE settings folder discovery and file operations (Linux: Proton / Wine prefixes).

use anyhow::{Context, Result, bail};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::SystemTime;

/// (folder-name fragment, display name)
pub const SERVERS: &[(&str, &str)] = &[
    ("tranquil", "Tranquility"),
    ("serenity", "Serenity"),
    ("infinity", "Infinity"),
    ("singulari", "Singularity"),
    ("duality", "Duality"),
    ("thunderdome", "Thunderdome"),
    ("buckshot", "Thunderdome"),
    ("tornado", "Tornado"),
];

const LANGS: &[(&str, &str)] = &[
    ("en", "English"),
    ("de", "German"),
    ("fr", "French"),
    ("ru", "Russian"),
    ("ja", "Japanese"),
    ("ko", "Korean"),
    ("zh", "Chinese"),
    ("es", "Spanish"),
    ("it", "Italian"),
    ("pt", "Portuguese"),
];

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Esi {
    Tq,
    Serenity,
    Infinity,
    Other,
}

impl Esi {
    pub fn infer(folder: &str) -> Esi {
        let t = folder.to_lowercase();
        if t.contains("serenity") {
            Esi::Serenity
        } else if t.contains("infinity") {
            Esi::Infinity
        } else if ["singulari", "duality", "thunderdome", "buckshot", "tornado"]
            .iter()
            .any(|k| t.contains(k))
        {
            Esi::Other
        } else {
            Esi::Tq
        }
    }

    pub fn status_url(self) -> Option<&'static str> {
        match self {
            Esi::Tq => Some("https://esi.evetech.net/latest/status/?datasource=tranquility"),
            Esi::Serenity => {
                Some("https://ali-esi.evepc.163.com/latest/status/?datasource=serenity")
            }
            Esi::Infinity => {
                Some("https://ali-esi.evepc.163.com/latest/status/?datasource=infinity")
            }
            Esi::Other => None,
        }
    }

    pub fn names_url(self) -> Option<&'static str> {
        match self {
            Esi::Tq => {
                Some("https://esi.evetech.net/latest/universe/names/?datasource=tranquility")
            }
            Esi::Serenity => {
                Some("https://ali-esi.evepc.163.com/latest/universe/names/?datasource=serenity")
            }
            Esi::Infinity => {
                Some("https://ali-esi.evepc.163.com/latest/universe/names/?datasource=infinity")
            }
            Esi::Other => None,
        }
    }
}

pub fn display_name(folder: &str) -> String {
    let t = folder.to_lowercase();
    let mut name = folder.to_string();
    for (k, v) in SERVERS {
        if t.contains(k) {
            name = v.to_string();
            break;
        }
    }
    static LANG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[_-]([a-z]{2})$").unwrap());
    if let Some(c) = LANG.captures(&t)
        && let Some((_, l)) = LANGS.iter().find(|(k, _)| *k == &c[1])
    {
        return format!("{name} ({l})");
    }
    name
}

fn is_server_dir(name: &str) -> bool {
    let t = name.to_lowercase();
    SERVERS.iter().any(|(k, _)| t.contains(k))
}

// ---------------------------------------------------------------------------
// Discovery
// ---------------------------------------------------------------------------

/// Glob patterns (relative to $HOME) where Linux EVE installs keep `CCP/EVE`.
const PATTERNS: &[&str] = &[
    // Steam / Proton (EVE app id 8500)
    ".steam/steam/steamapps/compatdata/8500/pfx/drive_c/users/*/AppData/Local/CCP/EVE",
    ".local/share/Steam/steamapps/compatdata/8500/pfx/drive_c/users/*/AppData/Local/CCP/EVE",
    ".var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/compatdata/8500/pfx/drive_c/users/*/AppData/Local/CCP/EVE",
    // Lutris / umu / Heroic style prefixes
    "Games/*/drive_c/users/*/AppData/Local/CCP/EVE",
    "Games/*/pfx/drive_c/users/*/AppData/Local/CCP/EVE",
    "Games/Heroic/Prefixes/*/*/drive_c/users/*/AppData/Local/CCP/EVE",
    // Bottles
    ".local/share/bottles/bottles/*/drive_c/users/*/AppData/Local/CCP/EVE",
    ".var/app/com.usebottles.bottles/data/bottles/bottles/*/drive_c/users/*/AppData/Local/CCP/EVE",
    // Plain Wine
    ".wine/drive_c/users/*/AppData/Local/CCP/EVE",
    ".wine/drive_c/users/*/Local Settings/Application Data/CCP/EVE",
];

/// Shown in the Info dialog.
pub const PATTERN_HINTS: &[&str] = &[
    "~/.steam/steam/steamapps/compatdata/8500/pfx/…/AppData/Local/CCP/EVE",
    "~/Games/<prefix>/drive_c/users/<user>/AppData/Local/CCP/EVE",
    "~/.local/share/bottles/bottles/<bottle>/drive_c/…/CCP/EVE",
    "~/.wine/drive_c/users/<user>/AppData/Local/CCP/EVE",
];

/// Extra Steam library folders listed in libraryfolders.vdf.
fn steam_libraries(home: &Path) -> Vec<PathBuf> {
    static PATH: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#""path"\s+"([^"]+)""#).unwrap());
    let mut out = Vec::new();
    for vdf in [
        ".local/share/Steam/steamapps/libraryfolders.vdf",
        ".steam/steam/steamapps/libraryfolders.vdf",
        ".var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/libraryfolders.vdf",
    ] {
        if let Ok(text) = fs::read_to_string(home.join(vdf)) {
            out.extend(PATH.captures_iter(&text).map(|c| PathBuf::from(&c[1])));
        }
    }
    out
}

/// All `CCP/EVE` folders found on this machine, most recently used first.
pub fn find_roots() -> Vec<PathBuf> {
    let Some(home) = dirs::home_dir() else {
        return vec![];
    };
    let mut found: Vec<PathBuf> = Vec::new();
    let mut push = |p: PathBuf| {
        if p.is_dir() {
            let c = fs::canonicalize(&p).unwrap_or(p);
            if !found.contains(&c) {
                found.push(c);
            }
        }
    };
    for pat in PATTERNS {
        let full = home.join(pat);
        if let Ok(paths) = glob::glob(&full.to_string_lossy()) {
            paths.flatten().for_each(&mut push);
        }
    }
    for lib in steam_libraries(&home) {
        let pat = lib.join("steamapps/compatdata/8500/pfx/drive_c/users/*/AppData/Local/CCP/EVE");
        if let Ok(paths) = glob::glob(&pat.to_string_lossy()) {
            paths.flatten().for_each(&mut push);
        }
    }
    // Prefer folders that actually contain server data, newest first.
    let score = |p: &PathBuf| {
        let servers = detect_servers(p).len();
        let mtime = fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        (servers > 0, mtime)
    };
    found.sort_by_key(|p| std::cmp::Reverse(score(p)));
    found
}

/// If the user picked a server folder directly, split it into (root, server).
pub fn resolve_picked(p: &Path) -> (PathBuf, Option<PathBuf>) {
    let name = p
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    if is_server_dir(&name) {
        (p.parent().unwrap_or(p).to_path_buf(), Some(p.to_path_buf()))
    } else if name.starts_with("settings_") {
        let server = p.parent().unwrap_or(p);
        (
            server.parent().unwrap_or(server).to_path_buf(),
            Some(server.to_path_buf()),
        )
    } else {
        (p.to_path_buf(), None)
    }
}

#[derive(Clone, Debug)]
pub struct Server {
    pub name: String,
    pub path: PathBuf,
    pub display: String,
    pub esi: Esi,
}

pub fn detect_servers(root: &Path) -> Vec<Server> {
    let Ok(rd) = fs::read_dir(root) else {
        return vec![];
    };
    let mut v: Vec<Server> = rd
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            is_server_dir(&name).then(|| Server {
                display: display_name(&name),
                esi: Esi::infer(&name),
                path: e.path(),
                name,
            })
        })
        .collect();
    v.sort_by(|a, b| a.name.cmp(&b.name));
    v
}

// ---------------------------------------------------------------------------
// Profiles (settings_<name> folders)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct Profile {
    pub name: String,
    pub path: PathBuf,
}

fn profile_dir(server: &Path, name: &str) -> PathBuf {
    server.join(format!("settings_{name}"))
}

pub fn validate_name(name: &str) -> Result<()> {
    let n = name.trim();
    if n.is_empty() {
        bail!("Name cannot be empty");
    }
    if n.contains(['/', '\\', '\0']) || n == "." || n == ".." {
        bail!("Name contains invalid characters");
    }
    Ok(())
}

pub fn list_profiles(server: &Path) -> Vec<Profile> {
    let Ok(rd) = fs::read_dir(server) else {
        return vec![];
    };
    let mut v: Vec<Profile> = rd
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            n.strip_prefix("settings_").map(|s| Profile {
                name: s.to_string(),
                path: e.path(),
            })
        })
        .collect();
    v.sort_by(|a, b| a.name.cmp(&b.name));
    v
}

pub fn create_profile(server: &Path, name: &str) -> Result<()> {
    validate_name(name)?;
    let p = profile_dir(server, name.trim());
    if p.exists() {
        bail!("A profile with this name already exists");
    }
    fs::create_dir(&p).with_context(|| format!("creating {}", p.display()))
}

pub fn rename_profile(server: &Path, old: &str, new: &str) -> Result<()> {
    validate_name(new)?;
    let to = profile_dir(server, new.trim());
    if to.exists() {
        bail!("A profile with this name already exists");
    }
    fs::rename(profile_dir(server, old), to).context("renaming profile")
}

pub fn duplicate_profile(server: &Path, src: &str, new: &str) -> Result<()> {
    validate_name(new)?;
    let to = profile_dir(server, new.trim());
    if to.exists() {
        bail!("A profile with this name already exists");
    }
    copy_dir(&profile_dir(server, src), &to)
}

pub fn delete_profile(server: &Path, name: &str) -> Result<()> {
    fs::remove_dir_all(profile_dir(server, name)).context("deleting profile")
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to)?;
    for e in fs::read_dir(from)?.flatten() {
        let t = to.join(e.file_name());
        if e.file_type()?.is_dir() {
            copy_dir(&e.path(), &t)?;
        } else {
            fs::copy(e.path(), t)?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Settings files
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FileKind {
    Char,
    User,
}

#[derive(Clone, Debug)]
pub struct SettingFile {
    pub kind: FileKind,
    pub id: String,
    pub filename: String,
    pub path: PathBuf,
    pub modified: SystemTime,
}

pub fn list_settings(profile: &Path) -> Vec<SettingFile> {
    static CHAR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^core_char_(\d+)\.dat$").unwrap());
    static USER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^core_user_(\d+)\.dat$").unwrap());
    let Ok(rd) = fs::read_dir(profile) else {
        return vec![];
    };
    let mut v: Vec<SettingFile> = rd
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let (kind, id) = if let Some(c) = CHAR.captures(&name) {
                (FileKind::Char, c[1].to_string())
            } else {
                let c = USER.captures(&name)?;
                (FileKind::User, c[1].to_string())
            };
            let modified = e
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            Some(SettingFile {
                kind,
                id,
                filename: name,
                path: e.path(),
                modified,
            })
        })
        .collect();
    v.sort_by_key(|f| std::cmp::Reverse(f.modified));
    v
}

pub fn copy_settings(src: &Path, targets: &[PathBuf]) -> Result<usize> {
    for t in targets {
        fs::copy(src, t).with_context(|| format!("writing {}", t.display()))?;
    }
    Ok(targets.len())
}

// ---------------------------------------------------------------------------
// Backups (stored in ~/.local/share/eve-settings-manager/backups)
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Clone, Default)]
struct FileMeta {
    /// Original filename to restore to, e.g. core_char_123.dat
    target: String,
    display: String,
}

#[derive(Clone, Debug)]
pub enum BackupKind {
    /// A whole profile snapshot (folder of .dat files).
    Folder,
    /// A single settings file; restores to `target`.
    File { target: String },
}

#[derive(Clone, Debug)]
pub struct Backup {
    pub kind: BackupKind,
    pub name: String,
    pub display: String,
    pub path: PathBuf,
    pub created: SystemTime,
    pub file_count: usize,
}

pub fn backups_dir() -> PathBuf {
    crate::store::data_dir().join("backups")
}

fn meta_path() -> PathBuf {
    backups_dir().join("meta.json")
}

fn read_meta() -> HashMap<String, FileMeta> {
    fs::read_to_string(meta_path())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn write_meta(m: &HashMap<String, FileMeta>) -> Result<()> {
    fs::create_dir_all(backups_dir())?;
    fs::write(meta_path(), serde_json::to_string_pretty(m)?)?;
    Ok(())
}

pub fn sanitize(name: &str) -> String {
    name.trim()
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | '\0' | ':') {
                '_'
            } else {
                c
            }
        })
        .collect()
}

pub fn folder_backup_exists(name: &str) -> bool {
    backups_dir().join(sanitize(name)).exists()
}

/// Snapshot every .dat file of a profile.
pub fn backup_profile(profile: &Path, name: &str) -> Result<usize> {
    validate_name(name)?;
    let dir = backups_dir().join(sanitize(name));
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    fs::create_dir_all(&dir)?;
    let mut n = 0;
    for e in fs::read_dir(profile)?.flatten() {
        if e.file_type()?.is_file() && e.file_name().to_string_lossy().ends_with(".dat") {
            fs::copy(e.path(), dir.join(e.file_name()))?;
            n += 1;
        }
    }
    Ok(n)
}

/// Back up a single settings file.
pub fn backup_file(file: &SettingFile, display: &str) -> Result<()> {
    let dir = backups_dir();
    fs::create_dir_all(&dir)?;
    let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let stem = format!("{}__{stamp}", file.filename.trim_end_matches(".dat"));
    fs::copy(&file.path, dir.join(format!("{stem}.dat")))?;
    let mut m = read_meta();
    m.insert(
        stem,
        FileMeta {
            target: file.filename.clone(),
            display: display.to_string(),
        },
    );
    write_meta(&m)
}

pub fn list_backups() -> Vec<Backup> {
    let dir = backups_dir();
    let Ok(rd) = fs::read_dir(&dir) else {
        return vec![];
    };
    let meta = read_meta();
    let mut v: Vec<Backup> = rd
        .flatten()
        .filter_map(|e| {
            let md = e.metadata().ok()?;
            let created = md
                .created()
                .or_else(|_| md.modified())
                .unwrap_or(SystemTime::UNIX_EPOCH);
            let name = e.file_name().to_string_lossy().to_string();
            if md.is_dir() {
                let count = fs::read_dir(e.path())
                    .map(|r| {
                        r.flatten()
                            .filter(|f| f.file_name().to_string_lossy().ends_with(".dat"))
                            .count()
                    })
                    .unwrap_or(0);
                Some(Backup {
                    kind: BackupKind::Folder,
                    display: name.clone(),
                    name,
                    path: e.path(),
                    created,
                    file_count: count,
                })
            } else if let Some(stem) = name.strip_suffix(".dat") {
                let fm = meta.get(stem).cloned().unwrap_or_else(|| FileMeta {
                    // Legacy/unknown: restore to "<stem>.dat" minus any timestamp suffix.
                    target: format!("{}.dat", stem.split("__").next().unwrap_or(stem)),
                    display: stem.to_string(),
                });
                Some(Backup {
                    kind: BackupKind::File { target: fm.target },
                    display: if fm.display.is_empty() {
                        stem.to_string()
                    } else {
                        fm.display
                    },
                    name: stem.to_string(),
                    path: e.path(),
                    created,
                    file_count: 1,
                })
            } else {
                None
            }
        })
        .collect();
    v.sort_by_key(|b| std::cmp::Reverse(b.created));
    v
}

pub fn restore_backup(b: &Backup, profile: &Path) -> Result<usize> {
    match &b.kind {
        BackupKind::Folder => {
            let mut n = 0;
            for e in fs::read_dir(&b.path)?.flatten() {
                if e.file_name().to_string_lossy().ends_with(".dat") {
                    fs::copy(e.path(), profile.join(e.file_name()))?;
                    n += 1;
                }
            }
            Ok(n)
        }
        BackupKind::File { target } => {
            fs::copy(&b.path, profile.join(target))?;
            Ok(1)
        }
    }
}

pub fn delete_backup(b: &Backup) -> Result<()> {
    match b.kind {
        BackupKind::Folder => fs::remove_dir_all(&b.path)?,
        BackupKind::File { .. } => {
            fs::remove_file(&b.path)?;
            let mut m = read_meta();
            if m.remove(&b.name).is_some() {
                write_meta(&m)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    //! Run with XDG_DATA_HOME pointed at a scratch dir so backups stay isolated.
    use super::*;

    fn fake_server(base: &Path) -> PathBuf {
        let srv = base.join("c_ccp_eve_tq_tranquility");
        let def = srv.join("settings_Default");
        fs::create_dir_all(&def).unwrap();
        fs::write(def.join("core_char_111.dat"), b"char-111").unwrap();
        fs::write(def.join("core_char_222.dat"), b"char-222").unwrap();
        fs::write(def.join("core_user_9.dat"), b"user-9").unwrap();
        fs::write(def.join("core_char__.dat"), b"generic").unwrap();
        srv
    }

    #[test]
    fn end_to_end() {
        let base = std::env::temp_dir().join(format!("esm-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        let srv = fake_server(&base);

        // discovery
        let servers = detect_servers(&base);
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].display, "Tranquility");
        assert_eq!(servers[0].esi, Esi::Tq);
        assert_eq!(resolve_picked(&srv).0, base);
        assert_eq!(
            display_name("c_eve_sharedcache_serenity_zh"),
            "Serenity (Chinese)"
        );

        // settings files (generic core_char__.dat ignored)
        let def = srv.join("settings_Default");
        let files = list_settings(&def);
        assert_eq!(files.iter().filter(|f| f.kind == FileKind::Char).count(), 2);
        assert_eq!(files.iter().filter(|f| f.kind == FileKind::User).count(), 1);

        // profiles
        create_profile(&srv, "PvP").unwrap();
        assert!(create_profile(&srv, "PvP").is_err());
        assert!(create_profile(&srv, "a/b").is_err());
        duplicate_profile(&srv, "Default", "Mining").unwrap();
        assert_eq!(list_settings(&srv.join("settings_Mining")).len(), 3);
        rename_profile(&srv, "Mining", "Mining 2").unwrap();
        let names: Vec<_> = list_profiles(&srv).into_iter().map(|p| p.name).collect();
        assert_eq!(names, ["Default", "Mining 2", "PvP"]);
        delete_profile(&srv, "PvP").unwrap();
        assert_eq!(list_profiles(&srv).len(), 2);

        // sync char 111 -> 222
        copy_settings(
            &def.join("core_char_111.dat"),
            &[def.join("core_char_222.dat")],
        )
        .unwrap();
        assert_eq!(
            fs::read(def.join("core_char_222.dat")).unwrap(),
            b"char-111"
        );

        // folder backup + restore
        let n = backup_profile(&def, "snap").unwrap();
        assert_eq!(n, 4);
        assert!(folder_backup_exists("snap"));
        fs::write(def.join("core_user_9.dat"), b"changed").unwrap();
        let b = list_backups()
            .into_iter()
            .find(|b| b.name == "snap")
            .unwrap();
        restore_backup(&b, &def).unwrap();
        assert_eq!(fs::read(def.join("core_user_9.dat")).unwrap(), b"user-9");

        // single-file backup restores to its original filename
        let f = list_settings(&def)
            .into_iter()
            .find(|f| f.id == "111")
            .unwrap();
        backup_file(&f, "Pilot A").unwrap();
        fs::write(def.join("core_char_111.dat"), b"broken").unwrap();
        let fb = list_backups()
            .into_iter()
            .find(|b| b.display == "Pilot A")
            .unwrap();
        assert!(matches!(&fb.kind, BackupKind::File { target } if target == "core_char_111.dat"));
        restore_backup(&fb, &def).unwrap();
        assert_eq!(
            fs::read(def.join("core_char_111.dat")).unwrap(),
            b"char-111"
        );
        delete_backup(&fb).unwrap();
        delete_backup(&b).unwrap();
        assert!(list_backups().is_empty());

        fs::remove_dir_all(&base).unwrap();
    }
}
