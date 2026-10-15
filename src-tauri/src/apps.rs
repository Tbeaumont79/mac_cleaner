//! Désinstallateur : liste les apps de /Applications et ~/Applications et
//! supprime définitivement une app avec ses fichiers associés dans ~/Library.
//!
//! Le frontend n'envoie que le chemin du bundle ; il est revalidé ici et les
//! fichiers associés sont toujours recalculés côté Rust à partir de l'identifiant
//! de bundle, jamais reçus du frontend.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::cleaner::{dir_size, remove_permanently, OWN_BUNDLE_ID};

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
    let bundle_id = get("CFBundleIdentifier").filter(|id| is_valid_bundle_id(id))?;
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

/// Forme inverse-DNS stricte : l'identifiant sert à construire des chemins, il
/// ne doit jamais pouvoir être vide, absolu ou contenir `..` ou `/`.
fn is_valid_bundle_id(id: &str) -> bool {
    let parts: Vec<&str> = id.split('.').collect();
    parts.len() >= 2
        && parts.iter().all(|p| {
            !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        })
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
/// (et par nom exact pour Application Support et Logs). Rien n'est renvoyé
/// pour ce qu'une autre copie installée de l'app partage encore.
fn leftovers_for(home: &Path, bundle: &Bundle, installed: &[Bundle]) -> Vec<PathBuf> {
    let others = || installed.iter().filter(|o| o.path != bundle.path);
    if others().any(|o| o.bundle_id == bundle.bundle_id) {
        return Vec::new();
    }
    let id = bundle.bundle_id.as_str();
    let lib = home.join("Library");
    let mut candidates = vec![
        lib.join("Application Support").join(id),
        lib.join("Application Scripts").join(id),
        lib.join("Caches").join(id),
        lib.join("Containers").join(id),
        lib.join("Cookies").join(format!("{id}.binarycookies")),
        lib.join("HTTPStorages").join(id),
        lib.join("HTTPStorages").join(format!("{id}.binarycookies")),
        lib.join("LaunchAgents").join(format!("{id}.plist")),
        lib.join("Logs").join(id),
        lib.join("Preferences").join(format!("{id}.plist")),
        lib.join("Saved Application State").join(format!("{id}.savedState")),
        lib.join("WebKit").join(id),
    ];
    let name = bundle.name.as_str();
    if is_safe_name(name) && !others().any(|o| o.name.eq_ignore_ascii_case(name)) {
        candidates.push(lib.join("Application Support").join(name));
        candidates.push(lib.join("Logs").join(name));
    }
    if let Ok(read) = std::fs::read_dir(lib.join("Group Containers")) {
        let suffix = format!(".{id}");
        for entry in read.filter_map(Result::ok) {
            // TEAMID.<id> ou group.<id> : un préfixe à points désignerait une autre app.
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.strip_suffix(&suffix).is_some_and(|prefix| !prefix.is_empty() && !prefix.contains('.')) {
                candidates.push(entry.path());
            }
        }
    }
    candidates.sort();
    candidates.dedup();
    candidates
        .into_iter()
        // Toujours exactement ~/Library/<dossier>/<élément>, jamais plus haut.
        .filter(|p| p.parent().and_then(Path::parent) == Some(lib.as_path()))
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
    // Un chemin en échec interrompt mdls pour tous les suivants : on refait
    // alors un appel par chemin.
    if !out.status.success() && paths.len() > 1 {
        return paths
            .iter()
            .flat_map(|p| last_used_days(std::slice::from_ref(p)))
            .collect();
    }
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

fn installed_bundles() -> Vec<Bundle> {
    app_dirs()
        .iter()
        .filter_map(|d| std::fs::read_dir(d).ok())
        .flat_map(|read| read.filter_map(Result::ok))
        .map(|e| e.path())
        .filter(|p| is_app_dir(p))
        .filter_map(|p| read_bundle(&p))
        .collect()
}

pub fn list() -> Vec<AppInfo> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };
    let installed = installed_bundles();
    let mut bundles: Vec<&Bundle> = installed.iter().filter(|b| is_removable(&b.bundle_id)).collect();
    bundles.sort_by_key(|b| b.name.to_lowercase());

    let paths: Vec<PathBuf> = bundles.iter().map(|b| b.path.clone()).collect();
    let days = last_used_days(&paths);
    let procs = running_executables();

    bundles
        .into_iter()
        .zip(days)
        .map(|(b, last_used_days)| {
            let leftovers: Vec<Leftover> = leftovers_for(&home, b, &installed)
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
                name: b.name.clone(),
                bundle_id: b.bundle_id.clone(),
                version: b.version.clone(),
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

/// `rm -rf` en root sur un chemin déjà validé par `validate` (bundle .app direct
/// de /Applications ou ~/Applications). `rm` ne suit pas les liens symboliques.
fn remove_as_admin(path: &Path) -> Result<(), String> {
    let path = path.to_str().ok_or_else(|| "chemin invalide".to_string())?;
    let literal = path.replace('\\', "\\\\").replace('"', "\\\"");
    let script = format!(
        "do shell script \"/bin/rm -rf \" & quoted form of \"{literal}\" with administrator privileges"
    );
    let out = Command::new("osascript")
        .args(["-e", &script])
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    Err(if stderr.contains("-128") {
        "mot de passe administrateur refusé ou annulé".to_string()
    } else {
        stderr.trim().to_string()
    })
}

/// Renomme d'abord le bundle (atomique) pour qu'un échec en cours de suppression
/// ne laisse jamais une app à moitié effacée à son emplacement. Tout refus de
/// droits (bundle root, fichiers root ajoutés par un updater, protection macOS)
/// bascule sur la suppression administrateur.
fn remove_app(path: &Path) -> Result<(), String> {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    // Nom court et indépendant du nom de l'app : jamais trop long, jamais en collision.
    let staged = path.with_file_name(format!(".sweep-delete-{}-{nanos}", std::process::id()));
    let removed = match std::fs::rename(path, &staged) {
        Ok(()) => match remove_permanently(&staged).or_else(|_| remove_as_admin(&staged)) {
            Ok(()) => Ok(()),
            // En cas d'échec, l'app ne doit pas rester cachée sous son nom temporaire.
            Err(e) => Err(match std::fs::rename(&staged, path) {
                Ok(()) => format!("{e}. L'app a pu être partiellement supprimée : relance la désinstallation."),
                Err(_) => format!("{e}. Restes de l'app dans {}", staged.display()),
            }),
        },
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => remove_as_admin(path),
        Err(e) => Err(e.to_string()),
    };
    let leftover = [path, staged.as_path()].into_iter().find(|p| std::fs::symlink_metadata(p).is_ok());
    match (removed, leftover) {
        (Ok(()), None) => Ok(()),
        (Err(e), _) => Err(e),
        (Ok(()), Some(p)) => Err(format!("{} n'a pas pu être entièrement supprimé", p.display())),
    }
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
            // Les fichiers associés se calculent avant que l'app ne quitte son dossier.
            let installed = installed_bundles();
            let leftovers = home
                .as_deref()
                .map(|h| leftovers_for(h, &bundle, &installed))
                .unwrap_or_default();
            if let Err(e) = remove_app(&bundle.path) {
                return UninstallResult { path: p, ok: false, freed_bytes: 0, errors: vec![e] };
            }
            let mut freed_bytes = app_bytes;
            let mut errors = Vec::new();
            for leftover in leftovers {
                let size = dir_size(&leftover);
                match remove_permanently(&leftover) {
                    Ok(()) => freed_bytes += size,
                    Err(e) => errors.push(format!("{} : {e}", leftover.display())),
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
    fn removes_a_user_writable_app_permanently() {
        let dir = dirs::home_dir().unwrap().join(".mac_cleaner_test_app/Fake.app/Contents");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("Info.plist"), b"x").unwrap();
        let app = dir.parent().unwrap().to_path_buf();
        let res = remove_app(&app);
        let gone = !app.exists();
        let _ = std::fs::remove_dir_all(app.parent().unwrap());
        assert!(res.is_ok(), "{res:?}");
        assert!(gone);
    }

    #[test]
    fn bundle_ids_must_be_reverse_dns() {
        assert!(is_valid_bundle_id("com.tinyspeck.slackmacgap"));
        assert!(is_valid_bundle_id("dev.kdrag0n.MacVirt"));
        for bad in ["", "..", "Office", "../../Documents", "/Users/x", "com..x", "com.x/y", "com.x."] {
            assert!(!is_valid_bundle_id(bad), "{bad}");
        }
    }

    fn bundle(path: &str, id: &str, name: &str) -> Bundle {
        Bundle { path: PathBuf::from(path), name: name.into(), bundle_id: id.into(), version: None }
    }

    #[test]
    fn shared_bundle_ids_keep_their_leftovers() {
        let home = dirs::home_dir().unwrap();
        let a = bundle("/Applications/Foo.app", "com.google.Chrome", "Foo");
        let b = bundle("/Applications/Foo 2.app", "com.google.Chrome", "Foo");
        assert!(leftovers_for(&home, &a, &[bundle("/Applications/Foo.app", "com.google.Chrome", "Foo"), b]).is_empty());
    }

    #[test]
    fn leftovers_stay_one_level_under_library() {
        let home = dirs::home_dir().unwrap();
        let lib = home.join("Library");
        for app in installed_bundles() {
            for p in leftovers_for(&home, &app, &installed_bundles()) {
                assert_eq!(p.parent().and_then(Path::parent), Some(lib.as_path()), "{}", p.display());
            }
        }
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
