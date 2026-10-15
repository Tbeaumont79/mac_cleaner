//! Éléments lancés automatiquement (LaunchAgents et LaunchDaemons hors Apple).
//!
//! Seuls les agents de ~/Library/LaunchAgents peuvent être activés ou
//! désactivés, via `launchctl disable`, ce qui est réversible et ne touche
//! pas au fichier plist.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Scope {
    User,
    SystemAgent,
    SystemDaemon,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Signature {
    /// Signé par un développeur identifié ou par Apple.
    Trusted { authority: String },
    /// Signature locale (ex. paquets Homebrew), sans identité vérifiable.
    AdHoc,
    Unsigned,
    Invalid,
    /// Interpréteur (sh, python…) : la signature ne dit rien du script lancé.
    Script,
    ProgramMissing,
}

#[derive(Debug, Serialize)]
pub struct LaunchItem {
    pub label: String,
    pub plist_path: String,
    pub program: Option<String>,
    pub scope: Scope,
    pub enabled: bool,
    /// Peut être activé ou désactivé par l'utilisateur (agent personnel, plist réel).
    pub editable: bool,
    /// Absente quand la liste est demandée sans analyse des signatures.
    pub signature: Option<Signature>,
}

fn uid() -> u32 {
    unsafe { libc::getuid() }
}

fn user_agents_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join("Library/LaunchAgents"))
}

fn locations() -> Vec<(PathBuf, Scope)> {
    let mut out = Vec::new();
    if let Some(dir) = user_agents_dir() {
        out.push((dir, Scope::User));
    }
    out.push((PathBuf::from("/Library/LaunchAgents"), Scope::SystemAgent));
    out.push((PathBuf::from("/Library/LaunchDaemons"), Scope::SystemDaemon));
    out
}

/// Les labels deviennent des arguments de `launchctl` : on n'accepte que des
/// caractères sûrs.
fn is_safe_label(label: &str) -> bool {
    !label.is_empty()
        && label
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '@'))
}

struct Parsed {
    label: String,
    program: Option<String>,
    disabled_in_plist: bool,
}

fn parse_plist(path: &Path) -> Option<Parsed> {
    let value = plist::Value::from_file(path).ok()?;
    let dict = value.as_dictionary()?;
    let label = dict.get("Label")?.as_string()?.to_string();
    let program = dict
        .get("Program")
        .and_then(|v| v.as_string())
        .map(str::to_string)
        .or_else(|| {
            dict.get("ProgramArguments")
                .and_then(|v| v.as_array())
                .and_then(|a| a.first())
                .and_then(|v| v.as_string())
                .map(str::to_string)
        });
    let disabled_in_plist = dict.get("Disabled").and_then(|v| v.as_boolean()).unwrap_or(false);
    Some(Parsed { label, program, disabled_in_plist })
}

/// Surcharges `launchctl disable/enable` d'un domaine (`gui/<uid>` ou `system`).
fn overrides(domain: &str) -> HashMap<String, bool> {
    let Ok(out) = Command::new("launchctl").args(["print-disabled", domain]).output() else {
        return HashMap::new();
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let (label, state) = line.trim().split_once("=>")?;
            let label = label.trim().trim_matches('"').to_string();
            let disabled = matches!(state.trim(), "disabled" | "true");
            Some((label, disabled))
        })
        .collect()
}

const INTERPRETERS: &[&str] = &[
    "sh", "bash", "zsh", "env", "python", "python3", "perl", "ruby", "node", "osascript",
];

// launchd cherche les noms nus dans un PATH minimal.
const LAUNCHD_PATH: &[&str] = &["/usr/bin", "/bin", "/usr/sbin", "/sbin", "/usr/local/bin", "/opt/homebrew/bin"];

fn resolve_program(program: &str) -> Option<PathBuf> {
    if program.contains('/') {
        let path = PathBuf::from(program);
        return path.is_absolute().then_some(path).filter(|p| p.exists());
    }
    LAUNCHD_PATH
        .iter()
        .map(|dir| Path::new(dir).join(program))
        .find(|p| p.exists())
}

fn is_script(path: &Path) -> bool {
    use std::io::Read;
    let mut head = [0u8; 2];
    std::fs::File::open(path)
        .and_then(|mut f| f.read_exact(&mut head))
        .is_ok_and(|()| &head == b"#!")
}

pub fn signature(program: Option<&str>) -> Signature {
    let Some(program) = program else {
        return Signature::ProgramMissing;
    };
    let name = Path::new(program).file_name().and_then(|n| n.to_str()).unwrap_or_default();
    if INTERPRETERS.contains(&name) {
        return Signature::Script;
    }
    let Some(path) = resolve_program(program) else {
        return Signature::ProgramMissing;
    };
    if is_script(&path) {
        return Signature::Script;
    }
    let path = path.as_path();
    let Ok(info) = Command::new("codesign").args(["-dv", "--verbose=2"]).arg(path).output() else {
        return Signature::Unsigned;
    };
    let details = String::from_utf8_lossy(&info.stderr);
    if details.contains("not signed at all") {
        return Signature::Unsigned;
    }
    let valid = Command::new("codesign")
        .args(["--verify", "--strict"])
        .arg(path)
        .status()
        .is_ok_and(|s| s.success());
    if !valid {
        return Signature::Invalid;
    }
    match details.lines().find_map(|l| l.strip_prefix("Authority=")) {
        Some(authority) => Signature::Trusted { authority: authority.to_string() },
        None => Signature::AdHoc,
    }
}

pub fn list(with_signatures: bool) -> Vec<LaunchItem> {
    let gui = overrides(&format!("gui/{}", uid()));
    let system = overrides("system");
    let mut items = Vec::new();
    for (dir, scope) in locations() {
        let Ok(read) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in read.filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("plist") {
                continue;
            }
            let Some(parsed) = parse_plist(&path) else {
                continue;
            };
            if parsed.label.starts_with("com.apple.") {
                continue;
            }
            let domain = if scope == Scope::SystemDaemon { &system } else { &gui };
            let disabled = domain.get(&parsed.label).copied().unwrap_or(parsed.disabled_in_plist);
            let signature = with_signatures.then(|| signature(parsed.program.as_deref()));
            let editable = scope == Scope::User
                && is_safe_label(&parsed.label)
                && std::fs::symlink_metadata(&path).is_ok_and(|m| m.is_file());
            items.push(LaunchItem {
                label: parsed.label,
                plist_path: path.to_string_lossy().into_owned(),
                program: parsed.program,
                scope,
                enabled: !disabled,
                editable,
                signature,
            });
        }
    }
    items.sort_by_key(|i| i.label.to_lowercase());
    items
}

pub fn set_enabled(plist_path: &str, enabled: bool) -> Result<(), String> {
    let dir = user_agents_dir().ok_or_else(|| "dossier personnel introuvable".to_string())?;
    let dir = std::fs::canonicalize(&dir).map_err(|e| e.to_string())?;
    let path = std::fs::canonicalize(plist_path).map_err(|e| e.to_string())?;
    if path.parent() != Some(dir.as_path()) {
        return Err("seuls les agents de ~/Library/LaunchAgents sont modifiables".to_string());
    }
    let parsed = parse_plist(&path).ok_or_else(|| "plist illisible".to_string())?;
    if !is_safe_label(&parsed.label) || parsed.label.starts_with("com.apple.") {
        return Err("agent non modifiable".to_string());
    }
    let domain = format!("gui/{}", uid());
    let target = format!("{domain}/{}", parsed.label);
    let action = if enabled { "enable" } else { "disable" };
    let status = Command::new("launchctl")
        .args([action, &target])
        .output()
        .map_err(|e| e.to_string())?;
    if !status.status.success() {
        return Err(String::from_utf8_lossy(&status.stderr).trim().to_string());
    }
    // Charger ou arrêter tout de suite ; un échec ici (déjà chargé, déjà
    // arrêté) n'empêche pas la surcharge de s'appliquer au prochain démarrage.
    let _ = if enabled {
        Command::new("launchctl").args(["bootstrap", &domain]).arg(&path).output()
    } else {
        Command::new("launchctl").args(["bootout", &domain]).arg(&path).output()
    };
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_are_restricted() {
        assert!(is_safe_label("homebrew.mxcl.postgresql@16"));
        assert!(!is_safe_label("evil; rm -rf ~"));
        assert!(!is_safe_label(""));
    }

    #[test]
    fn interpreters_are_flagged_as_scripts() {
        assert_eq!(signature(Some("/bin/sh")), Signature::Script);
        let script = std::env::temp_dir().join("mac_cleaner_shebang_test.sh");
        std::fs::write(&script, "#!/bin/sh\necho hi\n").unwrap();
        let result = signature(script.to_str());
        std::fs::remove_file(&script).unwrap();
        assert_eq!(result, Signature::Script);
        assert!(matches!(signature(Some("true")), Signature::Trusted { .. }));
        assert_eq!(signature(Some("/nonexistent/tool")), Signature::ProgramMissing);
        assert_eq!(signature(None), Signature::ProgramMissing);
    }

    #[test]
    fn apple_binaries_are_trusted() {
        assert!(matches!(signature(Some("/usr/bin/true")), Signature::Trusted { .. }));
    }

    #[test]
    fn refuses_plists_outside_user_agents() {
        assert!(set_enabled("/Library/LaunchDaemons/x.plist", false).is_err());
        assert!(set_enabled("/etc/hosts", true).is_err());
    }
}
