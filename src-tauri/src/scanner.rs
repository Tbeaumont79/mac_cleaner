//! Scan **lecture seule** des catégories de la liste blanche.
//!
//! Ce module ne fait *jamais* d'écriture ni de suppression : il parcourt les
//! dossiers cibles et additionne les tailles. C'est volontaire — toute la
//! partie risquée (suppression) vit dans `cleaner.rs`.

use serde::Serialize;
use walkdir::WalkDir;

use crate::targets::{self, Category, CategoryId};

/// Résultat de scan pour une catégorie, prêt à être affiché.
#[derive(Debug, Serialize)]
pub struct CategoryScan {
    pub id: CategoryId,
    pub label: String,
    pub description: String,
    /// Taille récupérable, en octets.
    pub size_bytes: u64,
    /// Nombre de fichiers comptabilisés.
    pub file_count: u64,
    /// `true` si au moins un chemin de la catégorie existe sur le disque.
    pub exists: bool,
    /// `true` si le nettoyage est définitif (pas la corbeille).
    pub permanent: bool,
    /// `true` si macOS a refusé la lecture d'au moins un dossier (Accès complet au disque).
    pub access_denied: bool,
}

/// Scanne toutes les catégories et renvoie un rapport par catégorie.
pub fn scan_all() -> Vec<CategoryScan> {
    targets::categories().iter().map(scan_category).collect()
}

/// Calcule la taille et le nombre de fichiers d'une catégorie.
///
/// On suit volontairement *pas* les liens symboliques (`follow_links` à false
/// par défaut dans WalkDir) pour éviter de compter — ou plus tard de supprimer
/// — des fichiers hors des dossiers ciblés. Les erreurs d'accès (permissions)
/// sont ignorées : on compte ce qu'on peut lire.
fn scan_category(cat: &Category) -> CategoryScan {
    let mut size_bytes: u64 = 0;
    let mut file_count: u64 = 0;
    let mut exists = false;
    let mut access_denied = targets::containers_denied(cat);

    for path in targets::resolved_paths(cat) {
        if path.exists() {
            exists = true;
        }
        if let Err(e) = std::fs::read_dir(&path) {
            access_denied |= e.kind() == std::io::ErrorKind::PermissionDenied;
        }
        for entry in WalkDir::new(&path).follow_root_links(false).into_iter().filter_map(Result::ok) {
            if entry.file_type().is_file() {
                if let Ok(meta) = entry.metadata() {
                    size_bytes = size_bytes.saturating_add(meta.len());
                    file_count += 1;
                }
            }
        }
    }

    CategoryScan {
        id: cat.id,
        label: cat.label.to_string(),
        description: cat.description.to_string(),
        size_bytes,
        file_count,
        exists,
        permanent: cat.permanent,
        access_denied,
    }
}
