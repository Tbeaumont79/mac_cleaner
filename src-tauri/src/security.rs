//! Bilan des protections intégrées à macOS. Lecture seule : Sweep n'est pas
//! un antivirus, il vérifie que les défenses du système sont actives.

use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Status {
    Ok,
    Warning,
    Unknown,
}

#[derive(Debug, Serialize)]
pub struct Check {
    pub id: &'static str,
    pub label: &'static str,
    pub status: Status,
    pub detail: String,
    /// Volet des Réglages Système où corriger le problème, s'il y en a un.
    pub settings_url: Option<&'static str>,
}

fn output(program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program).args(args).output().ok()?;
    let mut text = String::from_utf8_lossy(&out.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&out.stderr));
    Some(text)
}

fn from_output(text: Option<String>, on: &str, off: &str) -> Status {
    match text {
        Some(t) if t.contains(on) => Status::Ok,
        Some(t) if t.contains(off) => Status::Warning,
        _ => Status::Unknown,
    }
}

fn sip() -> Check {
    let status = from_output(output("csrutil", &["status"]), "status: enabled", "status: disabled");
    Check {
        id: "sip",
        label: "Protection de l'intégrité du système",
        detail: match status {
            Status::Ok => "Les fichiers système ne peuvent pas être modifiés.".into(),
            Status::Warning => "Désactivée : se réactive depuis le mode de récupération (csrutil enable).".into(),
            Status::Unknown => "État indisponible.".into(),
        },
        status,
        settings_url: None,
    }
}

fn gatekeeper() -> Check {
    let status = from_output(output("spctl", &["--status"]), "assessments enabled", "assessments disabled");
    Check {
        id: "gatekeeper",
        label: "Gatekeeper",
        detail: match status {
            Status::Ok => "Seules les apps vérifiées par Apple s'ouvrent sans avertissement.".into(),
            Status::Warning => "Désactivé : n'importe quelle app peut s'ouvrir sans contrôle.".into(),
            Status::Unknown => "État indisponible.".into(),
        },
        status,
        settings_url: Some("x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension"),
    }
}

fn filevault() -> Check {
    let status = from_output(output("fdesetup", &["status"]), "FileVault is On", "FileVault is Off");
    Check {
        id: "filevault",
        label: "FileVault",
        detail: match status {
            Status::Ok => "Le disque est chiffré.".into(),
            Status::Warning => "Le disque n'est pas chiffré : tes données sont lisibles si le Mac est volé.".into(),
            Status::Unknown => "État indisponible (chiffrement en cours ?).".into(),
        },
        status,
        settings_url: Some("x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension?FileVault"),
    }
}

fn firewall() -> Check {
    let status = from_output(
        output("/usr/libexec/ApplicationFirewall/socketfilterfw", &["--getglobalstate"]),
        "is enabled",
        "is disabled",
    );
    Check {
        id: "firewall",
        label: "Coupe-feu",
        detail: match status {
            Status::Ok => "Les connexions entrantes non autorisées sont bloquées.".into(),
            Status::Warning => "Désactivé : recommandé sur les réseaux Wi-Fi publics.".into(),
            Status::Unknown => "État indisponible.".into(),
        },
        status,
        settings_url: Some("x-apple.systempreferences:com.apple.Network-Settings.extension?Firewall"),
    }
}

const SOFTWARE_UPDATE_PLIST: &str = "/Library/Preferences/com.apple.SoftwareUpdate.plist";
const XPROTECT_PLIST: &str = "/Library/Apple/System/Library/CoreServices/XProtect.bundle/Contents/Info.plist";
const UPDATES_URL: &str = "x-apple.systempreferences:com.apple.Software-Update-Settings.extension";

fn software_update_prefs() -> Option<plist::Dictionary> {
    plist::Value::from_file(SOFTWARE_UPDATE_PLIST).ok()?.into_dictionary()
}

fn days_since(path: &Path) -> Option<u64> {
    let modified = std::fs::metadata(path).ok()?.modified().ok()?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;
    let then = modified.duration_since(UNIX_EPOCH).ok()?;
    Some(now.saturating_sub(then).as_secs() / 86_400)
}

fn security_updates(prefs: Option<&plist::Dictionary>) -> Check {
    // Clé absente : macOS applique la valeur par défaut, c'est-à-dire activé.
    let enabled = |key: &str| {
        prefs
            .and_then(|p| p.get(key))
            .map(|v| v.as_boolean().unwrap_or_else(|| v.as_signed_integer() == Some(1)))
            .unwrap_or(true)
    };
    let on = enabled("CriticalUpdateInstall") && enabled("ConfigDataInstall");
    let xprotect = plist::Value::from_file(XPROTECT_PLIST)
        .ok()
        .and_then(|v| {
            v.as_dictionary()?
                .get("CFBundleShortVersionString")?
                .as_string()
                .map(str::to_string)
        })
        .map(|version| match days_since(Path::new(XPROTECT_PLIST)) {
            Some(days) => format!(" XProtect {version}, mis à jour il y a {days} j."),
            None => format!(" XProtect {version}."),
        })
        .unwrap_or_default();
    Check {
        id: "security_updates",
        label: "Mises à jour de sécurité automatiques",
        status: if on { Status::Ok } else { Status::Warning },
        detail: if on {
            format!("Les correctifs et définitions anti-malware s'installent seuls.{xprotect}")
        } else {
            format!("Désactivées : les définitions anti-malware ne sont plus mises à jour.{xprotect}")
        },
        settings_url: Some(UPDATES_URL),
    }
}

fn pending_updates(prefs: Option<&plist::Dictionary>) -> Check {
    let names: Option<Vec<String>> = prefs.map(|p| {
        p.get("RecommendedUpdates")
            .and_then(|v| v.as_array())
            .map(|list| {
                list.iter()
                    .filter_map(|u| u.as_dictionary()?.get("Display Name")?.as_string())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    });
    let (status, detail) = match names {
        None => (Status::Unknown, "État indisponible.".to_string()),
        Some(n) if n.is_empty() => (Status::Ok, "Le système est à jour.".to_string()),
        Some(n) => (Status::Warning, format!("En attente : {}.", n.join(", "))),
    };
    Check {
        id: "pending_updates",
        label: "Mises à jour logicielles",
        status,
        detail,
        settings_url: Some(UPDATES_URL),
    }
}

pub fn checks() -> Vec<Check> {
    let prefs = software_update_prefs();
    vec![
        sip(),
        gatekeeper(),
        filevault(),
        firewall(),
        security_updates(prefs.as_ref()),
        pending_updates(prefs.as_ref()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_command_output() {
        assert_eq!(from_output(Some("Firewall is enabled. (State = 1)".into()), "is enabled", "is disabled"), Status::Ok);
        assert_eq!(from_output(Some("Firewall is disabled. (State = 0)".into()), "is enabled", "is disabled"), Status::Warning);
        assert_eq!(from_output(None, "a", "b"), Status::Unknown);
    }

    #[test]
    fn runs_all_checks() {
        let checks = checks();
        assert_eq!(checks.len(), 6);
        assert!(checks.iter().all(|c| !c.detail.is_empty()));
    }
}
