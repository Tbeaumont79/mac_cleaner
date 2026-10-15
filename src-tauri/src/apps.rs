//! Désinstallateur : liste les apps de /Applications et ~/Applications et
//! met à la corbeille une app avec ses fichiers associés dans ~/Library.
//!
//! Le frontend n'envoie que le chemin du bundle ; il est revalidé ici et les
//! fichiers associés sont toujours recalculés côté Rust à partir de l'identifiant
//! de bundle, jamais reçus du frontend.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::cleaner::{dir_size, move_to_trash, move_to_trash_as_admin, OWN_BUNDLE_ID};

#[derive(Debug, Serialize)]
pub struct Leftover {
    pub path: String,
    pub size_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct AppInfo {
    pub path: String,
    pub name: String,
    pub bundle_id: String,
    pub version: Option<String>,
    pub app_bytes: u64,
    pub leftovers: Vec<Leftover>,
    pub leftover_bytes: u64,
    pub last_used_days: Option<u64>,
    pub running: bool,
}

#[derive(Debug, Serialize)]
pub struct UninstallResult {
    pub path: String,
    pub ok: bool,
    pub freed_bytes: u64,
    pub errors: Vec<String>,
}

struct Bundle {
    path: PathBuf,
    name: String,
    bundle_id: String,
    version: Option<String>,
}

fn app_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![PathBuf::from("/Applications")];
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join("Applications"));
    }
    dirs
}

fn read_bundle(path: &Path) -> Option<Bundle> {
    let info = plist::Value::from_file(path.join("Contents/Info.plist")).ok()?;
    let dict = info.as_dictionary()?;
    let get = |k: &str| dict.get(k).and_then(|v| v.as_string()).map(str::to_string);
    let bundle_id = get("CFBundleIdentifier")?;
    let stem = path.file_stem()?.to_string_lossy().into_owned();
    Some(Bundle {
        path: path.to_path_buf(),
        name: get("CFBundleDisplayName")
            .or_else(|| get("CFBundleName"))
            .map(|n| clean_name(&n))
            .filter(|n| !n.is_empty())
            .unwrap_or(stem),
        bundle_id,
        version: get("CFBundleShortVersionString"),
    })
}

// Certains bundles préfixent leur nom de marques de direction (ex. WhatsApp).
fn clean_name(name: &str) -> String {
    name.chars()
        .filter(|c| !matches!(c, '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{FEFF}'))
        .collect::<String>()
        .trim()
        .to_string()
}

fn is_removable(bundle_id: &str) -> bool {
    !bundle_id.starts_with("com.apple.") && bundle_id != OWN_BUNDLE_ID
}

fn is_app_dir(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("app"))
        && std::fs::symlink_metadata(path).is_ok_and(|m| m.is_dir())
}

/// Fichiers associés à une app dans ~/Library, par identifiant de bundle exact
/// (et par nom exact pour Application Support et Logs).
fn leftovers_for(home: &Path, bundle_id: &str, name: &str) -> Vec<PathBuf> {
    let lib = home.join("Library");
    let mut candidates = vec![
        lib.join("Application Support").join(bundle_id),
        lib.join("Application Scripts").join(bundle_id),
        lib.join("Caches").join(bundle_id),
        lib.join("Containers").join(bundle_id),
        lib.join("Cookies").join(format!("{bundle_id}.binarycookies")),
        lib.join("HTTPStorages").join(bundle_id),
        lib.join("HTTPStorages").join(format!("{bundle_id}.binarycookies")),
        lib.join("LaunchAgents").join(format!("{bundle_id}.plist")),
        lib.join("Logs").join(bundle_id),
        lib.join("Preferences").join(format!("{bundle_id}.plist")),
        lib.join("Saved Application State").join(format!("{bundle_id}.savedState")),
        lib.join("WebKit").join(bundle_id),
    ];
    if is_safe_name(name) {
        candidates.push(lib.join("Application Support").join(name));
        candidates.push(lib.join("Logs").join(name));
    }
    if let Ok(read) = std::fs::read_dir(lib.join("Group Containers")) {
        let suffix = format!(".{bundle_id}");
        for entry in read.filter_map(Result::ok) {
            if entry.file_name().to_string_lossy().ends_with(&suffix) {
                candidates.push(entry.path());
            }
        }
    }
    candidates.sort();
    candidates.dedup();
    candidates
        .into_iter()
        .filter(|p| std::fs::symlink_metadata(p).is_ok())
        .collect()
}

// Un nom trop court ou générique pourrait désigner le dossier d'une autre app.
fn is_safe_name(name: &str) -> bool {
    const GENERIC: &[&str] = &["google", "microsoft", "adobe", "apple", "mozilla", "jetbrains"];
    name.chars().count() >= 3
        && !name.contains('/')
        && !name.starts_with('.')
        && !GENERIC.contains(&name.to_lowercase().as_str())
}

fn running_executables() -> Vec<String> {
    Command::new("ps")
        .args(["-axo", "comm="])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(|l| l.trim().to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn is_running(app: &Path, procs: &[String]) -> bool {
    let prefix = format!("{}/Contents/", app.display());
    procs.iter().any(|p| p.starts_with(&prefix))
}

/// Dates de dernière ouverture selon Spotlight, en un seul appel à `mdls`.
fn last_used_days(paths: &[PathBuf]) -> Vec<Option<u64>> {
    if paths.is_empty() {
        return Vec::new();
    }
    let out = Command::new("mdls")
        .args(["-name", "kMDItemLastUsedDate", "-raw"])
        .args(paths)
        .output();
    let Ok(out) = out else {
        return vec![None; paths.len()];
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let values: Vec<&str> = text.split('\0').collect();
    let today = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() / 86_400)
        .unwrap_or(0);
    (0..paths.len())
        .map(|i| {
            values
                .get(i)
                .and_then(|v| parse_day(v))
                .map(|day| today.saturating_sub(day))
        })
        .collect()
}

/// Jour (depuis l'epoch) d'une date `YYYY-MM-DD hh:mm:ss +0000`.
fn parse_day(s: &str) -> Option<u64> {
    let date = s.trim().split(' ').next()?;
    let mut it = date.split('-').map(|p| p.parse::<i64>().ok());
    let (y, m, d) = (it.next()??, it.next()??, it.next()??);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    // Algorithme « days from civil » de Howard Hinnant.
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    u64::try_from(era * 146_097 + doe - 719_468).ok()
}

pub fn list() -> Vec<AppInfo> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };
    let mut bundles: Vec<Bundle> = app_dirs()
        .iter()
        .filter_map(|d| std::fs::read_dir(d).ok())
        .flat_map(|read| read.filter_map(Result::ok))
        .map(|e| e.path())
        .filter(|p| is_app_dir(p))
        .filter_map(|p| read_bundle(&p))
        .filter(|b| is_removable(&b.bundle_id))
        .collect();
    bundles.sort_by_key(|b| b.name.to_lowercase());

    let paths: Vec<PathBuf> = bundles.iter().map(|b| b.path.clone()).collect();
    let days = last_used_days(&paths);
    let procs = running_executables();

    bundles
        .into_iter()
        .zip(days)
        .map(|(b, last_used_days)| {
            let leftovers: Vec<Leftover> = leftovers_for(&home, &b.bundle_id, &b.name)
                .into_iter()
                .map(|p| Leftover {
                    size_bytes: dir_size(&p),
                    path: p.to_string_lossy().into_owned(),
                })
                .collect();
            AppInfo {
                path: b.path.to_string_lossy().into_owned(),
                app_bytes: dir_size(&b.path),
                leftover_bytes: leftovers.iter().map(|l| l.size_bytes).sum(),
                leftovers,
                running: is_running(&b.path, &procs),
                last_used_days,
                name: b.name,
                bundle_id: b.bundle_id,
                version: b.version,
            }
        })
        .collect()
}

fn validate(path: &str) -> Result<Bundle, String> {
    let canon = std::fs::canonicalize(path).map_err(|e| e.to_string())?;
    let parent_ok = canon.parent().is_some_and(|parent| {
        app_dirs()
            .iter()
            .filter_map(|d| std::fs::canonicalize(d).ok())
            .any(|d| d == parent)
    });
    if !parent_ok || !is_app_dir(&canon) {
        return Err("ce n'est pas une app de /Applications ou ~/Applications".to_string());
    }
    let bundle = read_bundle(&canon).ok_or_else(|| "Info.plist illisible".to_string())?;
    if !is_removable(&bundle.bundle_id) {
        return Err("app système ou Sweep lui-même : refusé".to_string());
    }
    if is_running(&canon, &running_executables()) {
        return Err(format!("{} est ouverte : quitte-la d'abord", bundle.name));
    }
    Ok(bundle)
}

pub fn uninstall(paths: Vec<String>) -> Vec<UninstallResult> {
    let home = dirs::home_dir();
    paths
        .into_iter()
        .map(|p| {
            let bundle = match validate(&p) {
                Ok(b) => b,
                Err(e) => {
                    return UninstallResult { path: p, ok: false, freed_bytes: 0, errors: vec![e] };
                }
            };
            let app_bytes = dir_size(&bundle.path);
            if let Err(e) = move_to_trash(&bundle.path).or_else(|_| move_to_trash_as_admin(&bundle.path)) {
                return UninstallResult { path: p, ok: false, freed_bytes: 0, errors: vec![e] };
            }
            let mut freed_bytes = app_bytes;
            let mut errors = Vec::new();
            if let Some(home) = &home {
                for leftover in leftovers_for(home, &bundle.bundle_id, &bundle.name) {
                    let size = dir_size(&leftover);
                    match move_to_trash(&leftover) {
                        Ok(()) => freed_bytes += size,
                        Err(e) => errors.push(format!("{} : {e}", leftover.display())),
                    }
                }
            }
            UninstallResult { path: p, ok: true, freed_bytes, errors }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mdls_dates() {
        assert_eq!(parse_day("1970-01-01 00:00:00 +0000"), Some(0));
        assert_eq!(parse_day("2000-03-01 10:00:00 +0000"), Some(11_017));
        assert_eq!(parse_day("(null)"), None);
    }

    #[test]
    fn refuses_paths_outside_app_dirs() {
        assert!(validate("/etc/hosts").is_err());
        assert!(validate("/System/Applications/Calculator.app").is_err());
        assert!(validate("/Applications/Safari.app").is_err());
    }

    #[test]
    fn generic_names_are_not_matched() {
        assert!(!is_safe_name("Google"));
        assert!(!is_safe_name("X"));
        assert!(!is_safe_name("../Foo"));
        assert!(is_safe_name("Visual Studio Code"));
    }

    #[test]
    fn lists_apps_without_apple_ones() {
        let apps = list();
        assert!(apps.iter().all(|a| !a.bundle_id.starts_with("com.apple.")));
    }
}
