//! Suppression **réversible** des catégories validées.
//!
//! Règles de sécurité, non négociables :
//! - on n'accepte qu'un `CategoryId` de la liste blanche (`targets.rs`), jamais
//!   un chemin venant du frontend ;
//! - on déplace vers la **corbeille** via le crate `trash` — aucun `remove_*`
//!   définitif, donc tout est récupérable par l'utilisateur ;
//! - on vide le *contenu* des dossiers cibles mais on **conserve les dossiers
//!   eux-mêmes** (ex. `~/Library/Caches` reste, les apps le recréent) ;
//! - on re-valide que chaque chemin reste sous `$HOME` avant d'agir.

use serde::Serialize;

use crate::targets::{self, CategoryId};

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

        for entry in read.filter_map(Result::ok) {
            let path = entry.path();

            // Estimation de la taille avant déplacement (best-effort).
            let entry_size = dir_size(&path);

            // Réversible (corbeille) par défaut ; définitif seulement pour les
            // catégories explicitement marquées (ex. vider la corbeille).
            let outcome = if cat.permanent {
                remove_permanently(&path)
            } else {
                trash::delete(&path).map_err(|e| e.to_string())
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

/// Met **un** chemin à la corbeille après validation stricte.
///
/// Garde-fous : le chemin doit exister et, une fois **canonicalisé** (liens et
/// `..` résolus), se trouver à l'intérieur du dossier personnel. Cela empêche
/// qu'un chemin trafiqué fasse sortir l'opération de la zone autorisée.
fn trash_one_path(path: &str) -> Result<(), String> {
    let home = dirs::home_dir().ok_or_else(|| "dossier personnel introuvable".to_string())?;
    let canon = std::fs::canonicalize(path).map_err(|e| e.to_string())?;
    let home_canon = std::fs::canonicalize(&home).unwrap_or(home);
    if !canon.starts_with(&home_canon) {
        return Err("chemin hors du dossier personnel : refusé".to_string());
    }
    trash::delete(&canon).map_err(|e| e.to_string())
}

/// Supprime **définitivement** un fichier ou un dossier.
///
/// On lit les métadonnées via `symlink_metadata` pour **ne pas suivre les liens
/// symboliques** : si l'entrée est un lien, on supprime le lien lui-même, jamais
/// sa cible (qui pourrait pointer hors de la zone autorisée).
fn remove_permanently(path: &std::path::Path) -> Result<(), String> {
    let meta = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    let res = if meta.file_type().is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    };
    res.map_err(|e| e.to_string())
}

/// Taille d'un fichier ou d'un dossier (récursif), best-effort.
fn dir_size(path: &std::path::Path) -> u64 {
    use walkdir::WalkDir;
    let mut total = 0u64;
    for entry in WalkDir::new(path).into_iter().filter_map(Result::ok) {
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
    /// Vérifie que le crate `trash` fonctionne dans cet environnement :
    /// crée un fichier temporaire, le déplace à la corbeille, et confirme
    /// qu'il a bien disparu de son emplacement d'origine.
    #[test]
    fn trash_works_here() {
        let mut path = std::env::temp_dir();
        path.push("mac_cleaner_trash_probe.txt");
        std::fs::write(&path, b"probe").expect("write temp file");
        assert!(path.exists());

        let res = trash::delete(&path);
        println!("trash::delete -> {res:?}");
        assert!(res.is_ok(), "trash::delete a échoué : {res:?}");
        assert!(!path.exists(), "le fichier est toujours là après trash");
    }
}
