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

fn from_output(text: Option<String>, on: &[&str], off: &[&str]) -> Status {
    match text {
        Some(t) if on.iter().any(|p| t.contains(p)) => Status::Ok,
        Some(t) if off.iter().any(|p| t.contains(p)) => Status::Warning,
        _ => Status::Unknown,
    }
}

fn sip() -> Check {
    let status = from_output(output("csrutil", &["status"]), &["status: enabled"], &["status: disabled"]);
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
    let status = from_output(output("spctl", &["--status"]), &["assessments enabled"], &["assessments disabled"]);
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
    let status = from_output(output("fdesetup", &["status"]), &["FileVault is On"], &["FileVault is Off"]);
    Check {
        id: "filevault",
        label: "FileVault",
        detail: match status {
            Status::Ok => "Le disque est chiffré.".into(),
            Status::Warning => "Le disque n'est pas chiffré : tes données sont lisibles si le Mac est volé.".into(),
            Status::Unknown => "État indisponible.".into(),
        },
        status,
        settings_url: Some("x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension?FileVault"),
    }
}

fn firewall() -> Check {
    let status = from_output(
        output("/usr/libexec/ApplicationFirewall/socketfilterfw", &["--getglobalstate"]),
        &["is enabled", "is blocking all"],
        &["is disabled"],
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
// Depuis macOS 15, les mises à jour d'XProtect arrivent dans /private/var/protected.
const XPROTECT_PLISTS: &[&str] = &[
    "/Library/Apple/System/Library/CoreServices/XProtect.bundle/Contents/Info.plist",
    "/private/var/protected/xprotect/XProtect.bundle/Contents/Info.plist",
];
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

/// Version la plus haute et mise à jour la plus récente parmi les copies d'XProtect.
fn xprotect_info() -> Option<(u64, Option<u64>)> {
    let copies: Vec<(u64, Option<u64>)> = XPROTECT_PLISTS
        .iter()
        .filter_map(|p| {
            let version = plist::Value::from_file(p)
                .ok()?
                .as_dictionary()?
                .get("CFBundleShortVersionString")?
                .as_string()?
                .parse()
                .ok()?;
            Some((version, days_since(Path::new(p))))
        })
        .collect();
    let version = copies.iter().map(|c| c.0).max()?;
    let days = copies.iter().filter_map(|c| c.1).min();
    Some((version, days))
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
    let xprotect = xprotect_info()
        .map(|(version, days)| match days {
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

fn parse_version(v: &str) -> Vec<u64> {
    v.split('.').map_while(|p| p.parse().ok()).collect()
}

/// Versions de macOS proposées, d'après les clés `MSU_UPDATE_<build>_patch_<version>_<type>`.
fn offered_os_versions(prefs: &plist::Dictionary) -> Vec<String> {
    prefs
        .get("FirstOfferDateDictionary")
        .and_then(|v| v.as_dictionary())
        .map(|d| {
            d.keys()
                .filter_map(|k| k.split("_patch_").nth(1)?.split('_').next().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn pending_updates(prefs: Option<&plist::Dictionary>, current_os: Option<&str>) -> Check {
    let check = |status, detail: String| Check {
        id: "pending_updates",
        label: "Mises à jour logicielles",
        status,
        detail,
        settings_url: Some(UPDATES_URL),
    };
    let (Some(prefs), Some(current)) = (prefs, current_os) else {
        return check(Status::Unknown, "État indisponible.".into());
    };
    let current = parse_version(current);
    let newer: Vec<Vec<u64>> = offered_os_versions(prefs)
        .iter()
        .map(|v| parse_version(v))
        .filter(|v| !v.is_empty() && *v > current)
        .collect();
    // Les correctifs de la version installée sont des mises à jour de sécurité ;
    // une nouvelle version majeure reste un choix.
    let patch = newer.iter().filter(|v| v.first() == current.first()).max();
    let major = newer.iter().filter(|v| v.first() > current.first()).max();
    let fmt = |v: &Vec<u64>| v.iter().map(u64::to_string).collect::<Vec<_>>().join(".");

    let mut pending: Vec<String> = patch.map(|v| format!("macOS {}", fmt(v))).into_iter().collect();
    if let Some(apps) = prefs.get("RecommendedUpdates").and_then(|v| v.as_array()) {
        pending.extend(
            apps.iter()
                .filter_map(|u| u.as_dictionary()?.get("Display Name")?.as_string())
                .map(str::to_string),
        );
    }
    let major_note = major.map(|v| format!(" macOS {} est aussi disponible.", fmt(v))).unwrap_or_default();
    if pending.is_empty() {
        check(Status::Ok, format!("Aucune mise à jour en attente.{major_note}"))
    } else {
        check(Status::Warning, format!("En attente : {}.{major_note}", pending.join(", ")))
    }
}

fn os_version() -> Option<String> {
    output("sw_vers", &["-productVersion"]).map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

pub fn checks() -> Vec<Check> {
    let prefs = software_update_prefs();
    vec![
        sip(),
        gatekeeper(),
        filevault(),
        firewall(),
        security_updates(prefs.as_ref()),
        pending_updates(prefs.as_ref(), os_version().as_deref()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_command_output() {
        let on: &[&str] = &["is enabled", "is blocking all"];
        let off: &[&str] = &["is disabled"];
        assert_eq!(from_output(Some("Firewall is enabled. (State = 1)".into()), on, off), Status::Ok);
        assert_eq!(
            from_output(Some("Firewall is blocking all non-essential incoming connections. (State = 2)".into()), on, off),
            Status::Ok
        );
        assert_eq!(from_output(Some("Firewall is disabled. (State = 0)".into()), on, off), Status::Warning);
        assert_eq!(from_output(None, on, off), Status::Unknown);
    }

    fn prefs(keys: &[&str], apps: &[&str]) -> plist::Dictionary {
        let mut offers = plist::Dictionary::new();
        for k in keys {
            offers.insert(k.to_string(), plist::Value::String("2026-09-14".into()));
        }
        let mut d = plist::Dictionary::new();
        d.insert("FirstOfferDateDictionary".into(), plist::Value::Dictionary(offers));
        let list = apps
            .iter()
            .map(|a| {
                let mut u = plist::Dictionary::new();
                u.insert("Display Name".into(), plist::Value::String(a.to_string()));
                plist::Value::Dictionary(u)
            })
            .collect();
        d.insert("RecommendedUpdates".into(), plist::Value::Array(list));
        d
    }

    #[test]
    fn detects_pending_os_patches() {
        let p = prefs(
            &["MSU_UPDATE_25G76_patch_26.6.1_minor", "MSU_UPDATE_25G241_patch_26.7.1_minor", "MSU_UPDATE_26A434_patch_27.0.1_major"],
            &[],
        );
        let c = pending_updates(Some(&p), Some("26.5.2"));
        assert_eq!(c.status, Status::Warning);
        assert_eq!(c.detail, "En attente : macOS 26.7.1. macOS 27.0.1 est aussi disponible.");

        let c = pending_updates(Some(&p), Some("26.7.1"));
        assert_eq!(c.status, Status::Ok);

        let c = pending_updates(Some(&prefs(&[], &["Safari"])), Some("26.7.1"));
        assert_eq!(c.detail, "En attente : Safari.");

        assert_eq!(pending_updates(None, Some("26.7.1")).status, Status::Unknown);
    }

    #[test]
    fn runs_all_checks() {
        let checks = checks();
        assert_eq!(checks.len(), 6);
        assert!(checks.iter().all(|c| !c.detail.is_empty()));
    }
}
