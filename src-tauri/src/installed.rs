//! The applications of this computer, to choose which ones Clipper ignores.
//!
//! Gathered in a fraction of a second, without installing or starting
//! anything, from what Windows already knows:
//! - the Start menu shortcuts (their name is the one people know),
//! - the applications registered in "App Paths",
//! - the uninstall entries (name, icon, install folder),
//! - the running processes,
//! - the applications clips came from.
//!
//! Each executable appears once. Those that usually handle secrets
//! (password managers, authenticators, wallets…) are recognised by name and
//! suggested first.

use crate::processes;
use crate::registry;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use windows_sys::Win32::System::Registry::{HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

#[derive(Debug, Clone, Serialize)]
pub struct InstalledApp {
    /// Executable file name, lowercase: what the ignore list holds.
    pub exe: String,
    pub name: String,
    pub path: Option<String>,
    /// Why it is suggested (e.g. "Gestionnaire de mots de passe").
    pub category: Option<&'static str>,
    /// Higher is more relevant to ignore; 0 when not suggested.
    pub score: u32,
    /// Clips in the history that came from it.
    pub copies: i64,
    /// Last time one of its clips was used (RFC 3339).
    pub last_used: Option<String>,
    pub running: bool,
}

/// Names from the most to the least reliable source.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Origin {
    Guessed,
    FileDescription,
    Uninstall,
    Shortcut,
}

struct Found {
    name: String,
    origin: Origin,
    path: Option<PathBuf>,
}

#[derive(Default)]
struct Catalog(HashMap<String, Found>);

impl Catalog {
    fn add(&mut self, path: Option<PathBuf>, name: Option<String>, origin: Origin) {
        let Some(exe) = path.as_deref().and_then(exe_name) else {
            return;
        };
        if is_noise(&exe) || path.as_deref().is_some_and(in_installer_cache) {
            return;
        }
        let name = name
            .map(|n| tidy_name(&n))
            .filter(|n| !n.is_empty() && n.chars().count() <= 60);
        match self.0.get_mut(&exe) {
            Some(found) => {
                if let Some(name) = name {
                    // Among shortcuts to the same program, the shortest name
                    // is the program's ("VLC" rather than "VLC - reset…").
                    if origin > found.origin
                        || (origin == found.origin && name.len() < found.name.len())
                    {
                        found.name = name;
                        found.origin = origin;
                    }
                }
                if found.path.is_none() {
                    found.path = path;
                }
            }
            None => {
                let (name, origin) = match name {
                    Some(n) => (n, origin),
                    None => (guess_name(&exe), Origin::Guessed),
                };
                self.0.insert(exe, Found { name, origin, path });
            }
        }
    }
}

/// Every application found, sorted by name. `usage`: executable name,
/// number of clips and last use, from the history.
pub fn list(usage: &[(String, i64, String)]) -> Vec<InstalledApp> {
    let mut catalog = Catalog::default();
    let running = processes::snapshot();

    for (path, name) in start_menu_shortcuts() {
        catalog.add(Some(path), Some(name), Origin::Shortcut);
    }
    for (path, name) in uninstall_entries() {
        catalog.add(Some(path), name, Origin::Uninstall);
    }
    for path in app_paths() {
        catalog.add(Some(path), None, Origin::Guessed);
    }
    // Running programs with a window of their own, not background services.
    let windows = windows_dir();
    let visible = processes::with_windows();
    let mut running_names = std::collections::HashSet::new();
    for p in &running {
        running_names.insert(p.exe.clone());
        if !visible.contains(&p.pid) || catalog.0.contains_key(&p.exe) || is_noise(&p.exe) {
            continue;
        }
        if let Some(path) = processes::image_path(p.pid).map(PathBuf::from) {
            if !path.starts_with(&windows) {
                catalog.add(Some(path), None, Origin::Guessed);
            }
        }
    }
    for (exe, _, _) in usage {
        if !catalog.0.contains_key(exe) {
            catalog.add(Some(PathBuf::from(exe)), None, Origin::Guessed);
        }
    }

    // Better names for executables only known by their file name.
    for found in catalog.0.values_mut() {
        if found.origin < Origin::FileDescription {
            if let Some(desc) = found.path.as_deref().and_then(file_description) {
                found.name = desc;
                found.origin = Origin::FileDescription;
            }
        }
    }

    let usage: HashMap<&str, (i64, &str)> = usage
        .iter()
        .map(|(exe, n, last)| (exe.as_str(), (*n, last.as_str())))
        .collect();
    let mut apps: Vec<InstalledApp> = catalog
        .0
        .into_iter()
        .map(|(exe, found)| {
            let (copies, last_used) = usage
                .get(exe.as_str())
                .map_or((0, None), |(n, last)| (*n, Some(last.to_string())));
            let running = running_names.contains(&exe);
            let category = categorize(&exe, &found.name);
            let score = category.map_or(0, |(_, weight)| {
                weight + if copies > 0 { 15 } else { 0 } + if running { 10 } else { 0 }
            });
            InstalledApp {
                path: found
                    .path
                    .filter(|p| p.is_absolute())
                    .map(|p| p.to_string_lossy().into_owned()),
                name: found.name,
                category: category.map(|(label, _)| label),
                score,
                copies,
                last_used,
                running,
                exe,
            }
        })
        .collect();
    apps.sort_by_cached_key(|a| (fold(&a.name), a.exe.clone()));
    apps
}

/// An executable chosen by hand.
pub fn describe(path: &Path) -> Option<InstalledApp> {
    let exe = exe_name(path)?;
    let name = file_description(path).unwrap_or_else(|| guess_name(&exe));
    let category = categorize(&exe, &name);
    Some(InstalledApp {
        path: Some(path.to_string_lossy().into_owned()),
        category: category.map(|c| c.0),
        score: category.map_or(0, |c| c.1),
        copies: 0,
        last_used: None,
        running: false,
        name,
        exe,
    })
}

// ---------------------------------------------------------------- sources

fn start_menu_shortcuts() -> Vec<(PathBuf, String)> {
    let mut out = Vec::new();
    for var in ["APPDATA", "ProgramData"] {
        if let Some(base) = std::env::var_os(var) {
            let dir = PathBuf::from(base).join(r"Microsoft\Windows\Start Menu\Programs");
            walk_shortcuts(&dir, 0, &mut out);
        }
    }
    out
}

fn walk_shortcuts(dir: &Path, depth: u32, out: &mut Vec<(PathBuf, String)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            if depth < 3 {
                walk_shortcuts(&path, depth + 1, out);
            }
            continue;
        }
        if !path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("lnk"))
        {
            continue;
        }
        let Some(name) = path.file_stem().map(|s| s.to_string_lossy().into_owned()) else {
            continue;
        };
        let lower = name.to_lowercase();
        if SHORTCUT_NOISE.iter().any(|w| lower.contains(w)) {
            continue;
        }
        if let Some(target) = std::fs::read(&path).ok().and_then(|d| shortcut_target(&d)) {
            if target.is_file() {
                out.push((target, name));
            }
        }
    }
}

const SHORTCUT_NOISE: &[&str] = &[
    "uninstall",
    "désinstall",
    "desinstall",
    "readme",
    "lisez",
    "help",
    "aide",
    "documentation",
    "manual",
    "website",
    "site web",
    "release notes",
    "changelog",
    "license",
    "licence",
];

const UNINSTALL: &[(HKEY, &str)] = &[
    (
        HKEY_CURRENT_USER,
        r"Software\Microsoft\Windows\CurrentVersion\Uninstall",
    ),
    (
        HKEY_LOCAL_MACHINE,
        r"Software\Microsoft\Windows\CurrentVersion\Uninstall",
    ),
    (
        HKEY_LOCAL_MACHINE,
        r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
    ),
];

/// Executable and display name of each installed program. The executable is
/// the one of its icon, else the one of its install folder whose name looks
/// most like the program's.
fn uninstall_entries() -> Vec<(PathBuf, Option<String>)> {
    let mut out = Vec::new();
    for &(hive, root) in UNINSTALL {
        for sub in registry::subkeys(hive, root) {
            let key = format!(r"{root}\{sub}");
            if registry::read_dword(hive, &key, "SystemComponent") == Some(1)
                || registry::read_string(hive, &key, "ParentKeyName").is_some()
            {
                continue;
            }
            let Some(name) = registry::read_string(hive, &key, "DisplayName") else {
                continue;
            };
            let lower = name.to_lowercase();
            if PROGRAM_NOISE.iter().any(|w| lower.contains(w))
                || SHORTCUT_NOISE.iter().any(|w| lower.contains(w))
            {
                continue;
            }
            let from_icon = registry::read_string(hive, &key, "DisplayIcon")
                .map(|icon| clean_path(&icon))
                .filter(|p| is_exe(p) && p.is_file() && !exe_name(p).is_some_and(|e| is_noise(&e)));
            let exe = from_icon.or_else(|| {
                registry::read_string(hive, &key, "InstallLocation")
                    .map(|dir| clean_path(&dir))
                    .and_then(|dir| best_exe_in(&dir, &name))
            });
            if let Some(exe) = exe {
                out.push((exe, Some(name)));
            }
        }
    }
    out
}

/// Runtimes, drivers and updates rather than applications.
const PROGRAM_NOISE: &[&str] = &[
    "redistributable",
    "runtime",
    ".net",
    "sdk",
    "driver",
    "pilote",
    "update for",
    "mise à jour",
    "hotfix",
    "language pack",
    "module linguistique",
    "webview2",
    "visual c++",
    "vc++",
    "directx",
];

fn app_paths() -> Vec<PathBuf> {
    let root = r"Software\Microsoft\Windows\CurrentVersion\App Paths";
    let mut out = Vec::new();
    for hive in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        for sub in registry::subkeys(hive, root) {
            if let Some(path) = registry::read_string(hive, &format!(r"{root}\{sub}"), "")
                .map(|p| clean_path(&p))
                .filter(|p| is_exe(p) && p.is_file())
            {
                out.push(path);
            }
        }
    }
    out
}

/// The executable of `dir` (not in subfolders) that best matches `name`.
fn best_exe_in(dir: &Path, name: &str) -> Option<PathBuf> {
    let wanted = fold(name);
    let first_word = fold(name.split_whitespace().next().unwrap_or(""));
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| is_exe(p))
        .filter_map(|p| {
            let exe = exe_name(&p)?;
            if is_noise(&exe) {
                return None;
            }
            let stem = fold(exe.trim_end_matches(".exe"));
            let score = if stem.is_empty() {
                0
            } else if wanted == stem {
                4
            } else if wanted.starts_with(&stem) || stem.starts_with(&wanted) {
                3
            } else if !first_word.is_empty() && stem.contains(&first_word) {
                2
            } else if wanted.contains(&stem) {
                1
            } else {
                0
            };
            (score > 0).then_some((score, p))
        })
        .max_by_key(|(score, _)| *score)
        .map(|(_, p)| p)
}

// ---------------------------------------------------------------- shortcuts

/// Target of a Windows shortcut (`.lnk`, [MS-SHLLINK]): its local path,
/// else its path with environment variables (`%ProgramFiles%\…`).
pub fn shortcut_target(data: &[u8]) -> Option<PathBuf> {
    const HAS_ID_LIST: u32 = 0x1;
    const HAS_LINK_INFO: u32 = 0x2;
    const HAS_NAME: u32 = 0x4;
    const HAS_RELATIVE_PATH: u32 = 0x8;
    const HAS_WORKING_DIR: u32 = 0x10;
    const HAS_ARGUMENTS: u32 = 0x20;
    const HAS_ICON_LOCATION: u32 = 0x40;
    const IS_UNICODE: u32 = 0x80;
    const ENVIRONMENT_BLOCK: u32 = 0xA000_0001;

    let u16_at = |at: usize| Some(u16::from_le_bytes(data.get(at..at + 2)?.try_into().ok()?));
    let u32_at = |at: usize| Some(u32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?));

    if u32_at(0)? != 0x4C {
        return None;
    }
    let flags = u32_at(0x14)?;
    let mut at = 0x4C;
    if flags & HAS_ID_LIST != 0 {
        at += 2 + u16_at(at)? as usize;
    }
    let mut local = None;
    if flags & HAS_LINK_INFO != 0 {
        let info = at;
        let size = u32_at(info)? as usize;
        let header = u32_at(info + 4)? as usize;
        let has_local = u32_at(info + 8)? & 1 != 0;
        if has_local {
            local = if header >= 0x24 {
                let offset = u32_at(info + 28)? as usize;
                utf16_z(data.get(info + offset..)?)
            } else {
                None
            }
            .or_else(|| {
                let offset = u32_at(info + 16)? as usize;
                ascii_z(data.get(info + offset..)?)
            });
        }
        at = info + size;
    }
    if let Some(path) = local.filter(|p| !p.is_empty()) {
        return Some(PathBuf::from(path));
    }
    // Skip the strings to reach the extra data.
    for flag in [
        HAS_NAME,
        HAS_RELATIVE_PATH,
        HAS_WORKING_DIR,
        HAS_ARGUMENTS,
        HAS_ICON_LOCATION,
    ] {
        if flags & flag != 0 {
            let chars = u16_at(at)? as usize;
            at += 2 + chars * if flags & IS_UNICODE != 0 { 2 } else { 1 };
        }
    }
    while let Some(size) = u32_at(at).map(|s| s as usize) {
        if size < 8 {
            break;
        }
        if u32_at(at + 4)? == ENVIRONMENT_BLOCK {
            // TargetAnsi (260 bytes), then TargetUnicode.
            let target = utf16_z(data.get(at + 8 + 260..at + size)?)?;
            return Some(clean_path(&expand_env(&target)));
        }
        at += size;
    }
    None
}

fn utf16_z(bytes: &[u8]) -> Option<String> {
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .take_while(|&u| u != 0)
        .collect();
    Some(String::from_utf16_lossy(&units))
}

/// A path in the system code page: only kept when plain ASCII, as other
/// characters cannot be decoded reliably.
fn ascii_z(bytes: &[u8]) -> Option<String> {
    let end = bytes.iter().position(|&b| b == 0)?;
    let s = &bytes[..end];
    s.is_ascii()
        .then(|| String::from_utf8_lossy(s).into_owned())
}

fn expand_env(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) => {
                let var = &after[..end];
                match std::env::var(var) {
                    Ok(v) if !var.is_empty() => out.push_str(&v),
                    _ => {
                        out.push('%');
                        out.push_str(var);
                        out.push('%');
                    }
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

// ---------------------------------------------------------------- helpers

/// `"C:\x\app.exe",0` or `C:\x\app.exe -arg` → `C:\x\app.exe`.
fn clean_path(raw: &str) -> PathBuf {
    let raw = raw.trim();
    let raw = if let Some(quoted) = raw.strip_prefix('"') {
        quoted.split('"').next().unwrap_or("")
    } else if let Some(end) = raw.to_lowercase().find(".exe") {
        &raw[..end + 4]
    } else {
        raw.split(',').next().unwrap_or(raw)
    };
    PathBuf::from(raw.trim())
}

fn is_exe(p: &Path) -> bool {
    p.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe"))
}

fn exe_name(p: &Path) -> Option<String> {
    let name = p.file_name()?.to_string_lossy().to_lowercase();
    name.ends_with(".exe").then_some(name)
}

/// Installers, updaters and helpers that are not the application itself.
fn is_noise(exe: &str) -> bool {
    let stem = exe.trim_end_matches(".exe");
    const WORDS: &[&str] = &[
        "unins",
        "uninstall",
        "setup",
        "install",
        "update",
        "crashpad",
        "crashreport",
        "crash_handler",
        "errorreport",
        "helper",
        "elevate",
        "notification_helper",
        "maintenanceservice",
        "clicktorun",
        "werfault",
    ];
    WORDS.iter().any(|w| stem.contains(w))
        || matches!(
            stem,
            "clipper"
                | "msedgewebview2"
                | "chrome_proxy"
                | "msedge_proxy"
                | "svchost"
                | "conhost"
                | "dllhost"
                | "runtimebroker"
                | "rundll32"
                | "taskhostw"
                | "sihost"
                | "ctfmon"
        )
}

/// Copies of installers kept by Windows or by setup programs.
fn in_installer_cache(path: &Path) -> bool {
    let lower = path.to_string_lossy().to_lowercase();
    [
        r"\package cache\",
        r"\windows\installer\",
        r"\downloaded installations\",
    ]
    .iter()
    .any(|dir| lower.contains(dir))
}

/// Without the version and architecture that installers add to names:
/// "Python 3.14.7 (64-bit)" → "Python".
fn tidy_name(name: &str) -> String {
    let mut words: Vec<&str> = name.split_whitespace().collect();
    while words.len() > 1 {
        let last = words[words.len() - 1]
            .trim_matches(|c| c == '(' || c == ')')
            .to_lowercase();
        let version = last.trim_start_matches('v');
        let is_version = version.contains('.')
            && version.starts_with(|c: char| c.is_ascii_digit())
            && version.chars().all(|c| c.is_ascii_digit() || c == '.');
        let is_arch = matches!(
            last.as_str(),
            "x64" | "x86" | "64-bit" | "32-bit" | "64-bits" | "32-bits" | "amd64" | "arm64" | "-"
        );
        if !(is_version || is_arch) {
            break;
        }
        words.pop();
    }
    let name = words.join(" ");
    // "IDLE (Python 3.14 64-bit)" lost its closing parenthesis above.
    match name.rfind('(') {
        Some(open) if !name[open..].contains(')') && open > 0 => {
            name[..open].trim_end().to_string()
        }
        _ => name,
    }
}

fn windows_dir() -> PathBuf {
    std::env::var_os("WINDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
}

/// `keepassxc.exe` → `Keepassxc`.
fn guess_name(exe: &str) -> String {
    let stem = exe.trim_end_matches(".exe");
    let mut chars = stem.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// Lowercase letters and digits only, accents removed: for comparisons.
pub fn fold(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'à' | 'â' | 'ä' | 'á' | 'ã' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' => 'i',
            'ô' | 'ö' | 'ó' | 'õ' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ç' => 'c',
            c => c,
        })
        .filter(char::is_ascii_alphanumeric)
        .collect()
}

/// The "File description" of an executable's version information.
fn file_description(path: &Path) -> Option<String> {
    use windows_sys::Win32::Storage::FileSystem::{
        GetFileVersionInfoExW, GetFileVersionInfoSizeExW, VerQueryValueW, FILE_VER_GET_NEUTRAL,
    };
    let wide: Vec<u16> = path
        .as_os_str()
        .to_string_lossy()
        .encode_utf16()
        .chain(Some(0))
        .collect();
    unsafe {
        // Neutral: the executable's own resources, without looking for
        // language files (several times faster).
        let size =
            GetFileVersionInfoSizeExW(FILE_VER_GET_NEUTRAL, wide.as_ptr(), std::ptr::null_mut());
        if size == 0 {
            return None;
        }
        let mut data = vec![0u8; size as usize];
        if GetFileVersionInfoExW(
            FILE_VER_GET_NEUTRAL,
            wide.as_ptr(),
            0,
            size,
            data.as_mut_ptr().cast(),
        ) == 0
        {
            return None;
        }
        let query = |q: &str| -> Option<(*const u8, u32)> {
            let q: Vec<u16> = q.encode_utf16().chain(Some(0)).collect();
            let mut ptr: *mut core::ffi::c_void = std::ptr::null_mut();
            let mut len = 0u32;
            (VerQueryValueW(data.as_ptr().cast(), q.as_ptr(), &mut ptr, &mut len) != 0
                && !ptr.is_null()
                && len > 0)
                .then_some((ptr as *const u8, len))
        };
        let (ptr, len) = query(r"\VarFileInfo\Translation")?;
        if len < 4 {
            return None;
        }
        let lang = u16::from_le_bytes([*ptr, *ptr.add(1)]);
        let page = u16::from_le_bytes([*ptr.add(2), *ptr.add(3)]);
        let (text, chars) = query(&format!(
            r"\StringFileInfo\{lang:04x}{page:04x}\FileDescription"
        ))?;
        let units = std::slice::from_raw_parts(text as *const u16, chars as usize);
        let desc = String::from_utf16_lossy(units)
            .trim_end_matches('\0')
            .trim()
            .to_string();
        (!desc.is_empty() && desc.chars().count() <= 60).then_some(desc)
    }
}

// ---------------------------------------------------------------- relevance

/// Kinds of applications whose copies are usually secrets, with their
/// weight. A name matches when one of the words is in it (letters and
/// digits only), or equals it for the short, ambiguous ones.
const CATEGORIES: &[(&str, u32, &[&str], &[&str])] = &[
    (
        "Gestionnaire de mots de passe",
        100,
        &[
            "keepass",
            "1password",
            "bitwarden",
            "dashlane",
            "lastpass",
            "nordpass",
            "protonpass",
            "enpass",
            "roboform",
            "keepersecurity",
            "passwordsafe",
            "pwsafe",
            "stickypassword",
            "zohovault",
            "passbolt",
            "buttercup",
            "keeweb",
            "passwordmanager",
            "gestionnairedemotsdepasse",
        ],
        &["keeper", "psono", "padloc"],
    ),
    (
        "Authentification à deux facteurs",
        95,
        &[
            "authy",
            "winauth",
            "authenticator",
            "yubico",
            "enteauth",
            "2fast",
        ],
        &[],
    ),
    (
        "Portefeuille de cryptomonnaies",
        90,
        &[
            "ledgerlive",
            "exodus",
            "electrum",
            "trezor",
            "atomicwallet",
            "wasabiwallet",
            "sparrow",
            "bitcoinqt",
            "bitcoincore",
            "monerowallet",
            "guarda",
            "coinomi",
            "metamask",
            "wallet",
        ],
        &[],
    ),
    (
        "Chiffrement et clés",
        80,
        &[
            "veracrypt",
            "truecrypt",
            "kleopatra",
            "gpg4win",
            "cryptomator",
            "axcrypt",
            "boxcryptor",
            "puttygen",
            "pageant",
            "keybase",
        ],
        &["gpg"],
    ),
    (
        "Banque et finances",
        75,
        &[
            "banque",
            "bank",
            "boursorama",
            "revolut",
            "paypal",
            "gnucash",
            "quicken",
            "moneydance",
            "homebank",
            "grisbi",
            "skrooge",
        ],
        &[],
    ),
    (
        "Accès à distance",
        50,
        &["anydesk", "teamviewer", "rustdesk", "remotedesktop"],
        &["mstsc"],
    ),
    (
        "Messagerie privée",
        40,
        &["threema", "simplex"],
        &["signal", "session", "element", "wire"],
    ),
];

fn categorize(exe: &str, name: &str) -> Option<(&'static str, u32)> {
    let stem = fold(exe.trim_end_matches(".exe"));
    let name = fold(name);
    CATEGORIES
        .iter()
        .find(|(_, _, words, exact)| {
            words.iter().any(|w| stem.contains(w) || name.contains(w))
                || exact.iter().any(|w| stem == *w || name == *w)
        })
        .map(|(label, weight, _, _)| (*label, *weight))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_sensitive_applications() {
        assert_eq!(
            categorize("keepassxc.exe", "KeePassXC").map(|c| c.0),
            Some("Gestionnaire de mots de passe")
        );
        assert_eq!(
            categorize("1password.exe", "1Password").map(|c| c.0),
            Some("Gestionnaire de mots de passe")
        );
        assert_eq!(
            categorize("ledger live.exe", "Ledger Live").map(|c| c.0),
            Some("Portefeuille de cryptomonnaies")
        );
        assert_eq!(
            categorize("signal.exe", "Signal").map(|c| c.0),
            Some("Messagerie privée")
        );
        // Short words only match whole names.
        assert_eq!(categorize("wireshark.exe", "Wireshark"), None);
        assert_eq!(categorize("chrome.exe", "Google Chrome"), None);
    }

    #[test]
    fn cleans_registry_paths() {
        assert_eq!(
            clean_path(r#""C:\Program Files\App\app.exe",0"#),
            PathBuf::from(r"C:\Program Files\App\app.exe")
        );
        assert_eq!(
            clean_path(r"C:\Program Files\App\App.EXE,1"),
            PathBuf::from(r"C:\Program Files\App\App.EXE")
        );
        assert_eq!(
            clean_path(r"C:\Tools\tool.exe --flag"),
            PathBuf::from(r"C:\Tools\tool.exe")
        );
    }

    #[test]
    fn expands_environment_variables() {
        let windir = std::env::var("WINDIR").unwrap();
        assert_eq!(
            expand_env(r"%WINDIR%\notepad.exe"),
            format!(r"{windir}\notepad.exe")
        );
        assert_eq!(expand_env("100% sûr"), "100% sûr");
        assert_eq!(expand_env("%NO_SUCH_VAR_X%"), "%NO_SUCH_VAR_X%");
    }

    #[test]
    fn tidies_names() {
        assert_eq!(tidy_name("Python 3.14.7 (64-bit)"), "Python");
        assert_eq!(tidy_name("WebStorm 2026.1.1"), "WebStorm");
        assert_eq!(tidy_name("7-Zip 24.08 (x64)"), "7-Zip");
        assert_eq!(tidy_name("MSI Kombustor 4"), "MSI Kombustor 4");
        assert_eq!(tidy_name("Office 2016"), "Office 2016");
        assert_eq!(tidy_name("1.2.3"), "1.2.3");
        assert_eq!(tidy_name("IDLE (Python 3.14 64-bit)"), "IDLE");
    }

    #[test]
    fn filters_installers_and_helpers() {
        assert!(is_noise("unins000.exe"));
        assert!(is_noise("squirrel_update.exe"));
        assert!(is_noise("msedgewebview2.exe"));
        assert!(!is_noise("keepassxc.exe"));
    }

    #[test]
    fn reads_start_menu_shortcuts() {
        let found = start_menu_shortcuts();
        assert!(!found.is_empty(), "no shortcut resolved");
        assert!(found.iter().all(|(p, _)| p.is_file()));
    }

    #[test]
    fn lists_installed_applications_quickly() {
        let started = std::time::Instant::now();
        let apps = list(&[("keepassxc.exe".into(), 3, "2026-01-01T00:00:00Z".into())]);
        let elapsed = started.elapsed();
        eprintln!("{} applications in {elapsed:?}", apps.len());
        assert!(apps.len() > 5);
        let keepass = apps.iter().find(|a| a.exe == "keepassxc.exe").unwrap();
        assert_eq!(keepass.copies, 3);
        assert!(keepass.score > 0);
        let mut exes: Vec<&str> = apps.iter().map(|a| a.exe.as_str()).collect();
        exes.sort();
        exes.dedup();
        assert_eq!(exes.len(), apps.len(), "duplicates");
    }
}
