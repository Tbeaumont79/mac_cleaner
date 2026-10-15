//! Suppression **réversible** des catégories validées.
//!
//! Règles de sécurité, non négociables :
//! - on n'accepte qu'un `CategoryId` de la liste blanche (`targets.rs`), jamais
//!   un chemin venant du frontend ;
//! - on déplace vers la **corbeille** ; la suppression définitive est réservée
//!   aux catégories marquées `permanent` (vider la corbeille) ;
//! - on vide le *contenu* des dossiers cibles mais on **conserve les dossiers
//!   eux-mêmes** (ex. `~/Library/Caches` reste, les apps le recréent) ;
//! - on re-valide que chaque chemin reste sous `$HOME` avant d'agir.

use std::path::Path;

use serde::Serialize;

use crate::guard;
use crate::targets::{self, CategoryId};

pub(crate) const OWN_BUNDLE_ID: &str = "com.thibaultbeaumont.mac-cleaner";

/// Bilan d'un nettoyage de catégorie.
#[derive(Debug, Serialize)]
pub struct CleanResult {
    pub id: CategoryId,
    /// Octets estimés envoyés à la corbeille.
    pub freed_bytes: u64,
    /// Nombre d'éléments (fichiers/dossiers de premier niveau) déplacés.
    pub items_trashed: u64,
    /// Erreurs non bloquantes rencontrées (ex. fichier verrouillé).
    pub errors: Vec<String>,
}

/// Nettoie une catégorie identifiée par son `CategoryId`.
///
/// Renvoie `Err` seulement si la catégorie est inconnue (cas qui ne devrait pas
/// arriver depuis l'UI). Les erreurs partielles sont remontées dans
/// `CleanResult.errors` sans interrompre le reste.
pub fn clean_category(id: CategoryId) -> Result<CleanResult, String> {
    let cat = targets::find(id).ok_or_else(|| "Catégorie inconnue".to_string())?;

    let mut freed_bytes: u64 = 0;
    let mut items_trashed: u64 = 0;
    let mut errors: Vec<String> = Vec::new();

    for dir in targets::resolved_paths(cat) {
        // Le dossier cible peut ne pas exister : rien à faire.
        let read = match std::fs::read_dir(&dir) {
            Ok(r) => r,
            Err(_) => continue,
        };
        // Revérifie juste avant d'agir que le dossier n'a pas été remplacé par un lien.
        if !std::fs::symlink_metadata(&dir).is_ok_and(|m| m.is_dir()) {
            continue;
        }

        for entry in read.filter_map(Result::ok) {
            // Vider son propre cache casserait la webview en cours d'exécution.
            if entry.file_name() == OWN_BUNDLE_ID {
                continue;
            }
            let path = entry.path();

            // Estimation de la taille avant déplacement (best-effort).
            let entry_size = dir_size(&path);

            // Réversible (corbeille) par défaut ; définitif seulement pour les
            // catégories explicitement marquées (ex. vider la corbeille).
            let outcome = if cat.permanent {
                remove_permanently(&path)
            } else {
                move_to_trash(&path)
            };

            match outcome {
                Ok(()) => {
                    freed_bytes = freed_bytes.saturating_add(entry_size);
                    items_trashed += 1;
                }
                Err(e) => {
                    errors.push(format!("{} : {}", path.display(), e));
                }
            }
        }
    }

    Ok(CleanResult {
        id,
        freed_bytes,
        items_trashed,
        errors,
    })
}

/// Résultat de la mise à la corbeille d'un chemin précis.
#[derive(Debug, Serialize)]
pub struct TrashResult {
    pub path: String,
    pub ok: bool,
    pub error: Option<String>,
}

/// Déplace une liste de chemins précis vers la corbeille (réversible).
///
/// Utilisé par la fonctionnalité « gros fichiers », où l'utilisateur désigne
/// des fichiers individuels. Chaque chemin est validé indépendamment.
pub fn trash_paths(paths: Vec<String>) -> Vec<TrashResult> {
    paths
        .into_iter()
        .map(|p| match trash_one_path(&p) {
            Ok(()) => TrashResult {
                path: p,
                ok: true,
                error: None,
            },
            Err(e) => TrashResult {
                path: p,
                ok: false,
                error: Some(e),
            },
        })
        .collect()
}

/// Met **un** chemin à la corbeille après validation stricte : il doit être
/// sous `~` (sans être `~` lui-même) et hors des zones protégées de `guard`.
fn trash_one_path(path: &str) -> Result<(), String> {
    let home = dirs::home_dir().ok_or_else(|| "dossier personnel introuvable".to_string())?;
    let home = std::fs::canonicalize(&home).unwrap_or(home);
    let path = Path::new(path);
    let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
        return Err("chemin invalide".to_string());
    };
    // On résout le parent mais pas l'entrée : un lien part à la corbeille, pas sa cible.
    let target = std::fs::canonicalize(parent)
        .map_err(|e| e.to_string())?
        .join(name);
    let meta = std::fs::symlink_metadata(&target).map_err(|e| e.to_string())?;
    if meta.is_dir() {
        return Err("seuls les fichiers peuvent être mis à la corbeille ici".to_string());
    }
    if target == home || !target.starts_with(&home) {
        return Err("chemin hors du dossier personnel : refusé".to_string());
    }
    if guard::within_protected(&home, &target) {
        return Err("zone protégée (données d'apps ou bibliothèque) : refusé".to_string());
    }
    move_to_trash(&target)
}

// Finder (méthode par défaut du crate) exige l'autorisation d'automatisation,
// lance un AppleScript par élément et refuse les liens symboliques.
pub(crate) fn move_to_trash(path: &Path) -> Result<(), String> {
    let mut ctx = trash::TrashContext::default();
    #[cfg(target_os = "macos")]
    {
        use trash::macos::{DeleteMethod, TrashContextExtMacos};
        ctx.set_delete_method(DeleteMethod::NsFileManager);
    }
    ctx.delete(path).map_err(|e| e.to_string())
}

/// Supprime **définitivement** un fichier ou un dossier.
///
/// On lit les métadonnées via `symlink_metadata` pour **ne pas suivre les liens
/// symboliques** : si l'entrée est un lien, on supprime le lien lui-même, jamais
/// sa cible (qui pourrait pointer hors de la zone autorisée).
pub(crate) fn remove_permanently(path: &std::path::Path) -> Result<(), String> {
    let meta = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    let res = if meta.file_type().is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    };
    res.map_err(|e| e.to_string())
}

/// Taille d'un fichier ou d'un dossier (récursif), best-effort.
pub(crate) fn dir_size(path: &std::path::Path) -> u64 {
    use walkdir::WalkDir;
    let mut total = 0u64;
    for entry in WalkDir::new(path).follow_root_links(false).into_iter().filter_map(Result::ok) {
        if entry.file_type().is_file() {
            if let Ok(meta) = entry.metadata() {
                total = total.saturating_add(meta.len());
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::trash_one_path;

    #[test]
    fn refuses_home_outside_and_protected() {
        let home = dirs::home_dir().unwrap();
        assert!(trash_one_path(home.to_str().unwrap()).is_err());
        assert!(trash_one_path("/etc/hosts").is_err());
        assert!(trash_one_path(home.join("Library").to_str().unwrap()).is_err());
        assert!(trash_one_path(home.join("..").to_str().unwrap()).is_err());
    }

    #[test]
    fn trashes_symlink_not_its_target() {
        let dir = dirs::home_dir().unwrap().join(".mac_cleaner_test_symlink");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let target = dir.join("target.txt");
        let link = dir.join("link.txt");
        std::fs::write(&target, b"keep").unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();

        let res = trash_one_path(link.to_str().unwrap());
        let link_gone = std::fs::symlink_metadata(&link).is_err();
        let target_kept = target.exists();
        std::fs::remove_dir_all(&dir).unwrap();

        assert!(res.is_ok(), "{res:?}");
        assert!(link_gone);
        assert!(target_kept);
    }

    #[test]
    fn trash_works_here() {
        let mut path = std::env::temp_dir();
        path.push("mac_cleaner_trash_probe.txt");
        std::fs::write(&path, b"probe").expect("write temp file");
        assert!(path.exists());

        let res = super::move_to_trash(&path);
        assert!(res.is_ok(), "move_to_trash a échoué : {res:?}");
        assert!(!path.exists(), "le fichier est toujours là après trash");
    }
}
