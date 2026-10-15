//! Tâches de maintenance : une liste blanche de commandes système fixes.
//! Le frontend n'envoie que des `TaskId`, jamais de commande.

use std::process::Command;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskId {
    FlushDns,
    ReindexSpotlight,
    PurgeMemory,
    RebuildLaunchServices,
    ResetQuickLook,
    ClearFontCaches,
}

#[derive(Debug, Serialize)]
pub struct MaintenanceTask {
    pub id: TaskId,
    pub label: &'static str,
    pub description: &'static str,
    pub admin: bool,
}

#[derive(Debug, Serialize)]
pub struct TaskResult {
    pub id: TaskId,
    pub ok: bool,
    pub error: Option<String>,
}

struct Spec {
    task: MaintenanceTask,
    command: String,
}

const LSREGISTER: &str = "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister";

fn specs() -> Vec<Spec> {
    vec![
        Spec {
            task: MaintenanceTask {
                id: TaskId::FlushDns,
                label: "Vider le cache DNS",
                description: "Corrige les sites qui ne se chargent plus après un changement de réseau.",
                admin: true,
            },
            command: "dscacheutil -flushcache && killall -HUP mDNSResponder".to_string(),
        },
        Spec {
            task: MaintenanceTask {
                id: TaskId::ReindexSpotlight,
                label: "Réindexer Spotlight",
                description: "Reconstruit l'index de recherche du disque. Spotlight sera lent pendant quelques heures.",
                admin: true,
            },
            command: "mdutil -E /".to_string(),
        },
        Spec {
            task: MaintenanceTask {
                id: TaskId::PurgeMemory,
                label: "Libérer la mémoire inactive",
                description: "Vide les caches disque gardés en RAM. Effet souvent temporaire.",
                admin: true,
            },
            command: "purge".to_string(),
        },
        Spec {
            task: MaintenanceTask {
                id: TaskId::RebuildLaunchServices,
                label: "Reconstruire les services",
                description: "Répare le menu « Ouvrir avec » en double et les associations de fichiers.",
                admin: false,
            },
            command: format!("{LSREGISTER} -r -domain local -domain system -domain user"),
        },
        Spec {
            task: MaintenanceTask {
                id: TaskId::ResetQuickLook,
                label: "Réinitialiser Quick Look",
                description: "Corrige les aperçus et vignettes qui ne s'affichent plus.",
                admin: false,
            },
            command: "qlmanage -r cache && qlmanage -r".to_string(),
        },
        Spec {
            task: MaintenanceTask {
                id: TaskId::ClearFontCaches,
                label: "Vider les caches de polices",
                description: "Corrige les polices corrompues ou mal affichées. Prend effet à la prochaine session.",
                admin: false,
            },
            command: "atsutil databases -removeUser".to_string(),
        },
    ]
}

pub fn list() -> Vec<MaintenanceTask> {
    specs().into_iter().map(|s| s.task).collect()
}

fn run_shell(cmd: &str) -> Result<(), String> {
    let out = Command::new("/bin/sh")
        .args(["-c", cmd])
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// Toutes les tâches admin passent par une seule demande de mot de passe.
/// Chaque commande signale son échec par un marqueur sur stdout.
fn run_admin(specs: &[&Spec]) -> Vec<TaskResult> {
    let script: String = specs
        .iter()
        .map(|s| format!("({}) >/dev/null 2>&1 || echo FAILED:{:?}", s.command, s.task.id))
        .collect::<Vec<_>>()
        .join("; ");
    let apple_script = format!(
        "do shell script \"{}\" with administrator privileges",
        script.replace('\\', "\\\\").replace('"', "\\\"")
    );
    let out = Command::new("osascript").args(["-e", &apple_script]).output();
    match out {
        Ok(o) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            specs
                .iter()
                .map(|s| {
                    let failed = stdout.contains(&format!("FAILED:{:?}", s.task.id));
                    TaskResult {
                        id: s.task.id,
                        ok: !failed,
                        error: failed.then(|| "la commande a échoué".to_string()),
                    }
                })
                .collect()
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            let error = if stderr.contains("-128") {
                "mot de passe administrateur refusé ou annulé".to_string()
            } else {
                stderr.trim().to_string()
            };
            specs
                .iter()
                .map(|s| TaskResult { id: s.task.id, ok: false, error: Some(error.clone()) })
                .collect()
        }
        Err(e) => specs
            .iter()
            .map(|s| TaskResult { id: s.task.id, ok: false, error: Some(e.to_string()) })
            .collect(),
    }
}

pub fn run(ids: Vec<TaskId>) -> Vec<TaskResult> {
    let all = specs();
    let selected: Vec<&Spec> = all.iter().filter(|s| ids.contains(&s.task.id)).collect();
    let admin: Vec<&Spec> = selected.iter().copied().filter(|s| s.task.admin).collect();

    let mut results: Vec<TaskResult> = selected
        .iter()
        .filter(|s| !s.task.admin)
        .map(|s| match run_shell(&s.command) {
            Ok(()) => TaskResult { id: s.task.id, ok: true, error: None },
            Err(e) => TaskResult { id: s.task.id, ok: false, error: Some(e) },
        })
        .collect();
    if !admin.is_empty() {
        results.extend(run_admin(&admin));
    }
    results
}
