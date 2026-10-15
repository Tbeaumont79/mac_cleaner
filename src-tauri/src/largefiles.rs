//! Recherche **lecture seule** des gros fichiers dans le dossier personnel.
//!
//! Parcourt récursivement `~`, retient les fichiers dont la taille dépasse un
//! seuil, et renvoie les plus gros (triés par taille décroissante). On ne suit
//! pas les liens symboliques et on saute les zones protégées (`guard`).
//!
//! Aucune écriture : la suppression éventuelle passe par `cleaner::trash_paths`.

use std::time::SystemTime;

use serde::Serialize;
use walkdir::WalkDir;

use crate::guard;

/// Nombre maximum de fichiers renvoyés (les plus gros).
const MAX_RESULTS: usize = 200;

/// Un gros fichier trouvé sur le disque.
#[derive(Debug, Serialize)]
pub struct LargeFile {
    pub path: String,
    pub size_bytes: u64,
    /// Âge depuis la dernière modification, en jours (None si indisponible).
    pub age_days: Option<u64>,
}

/// Trouve les fichiers de `~` dont la taille est >= `min_bytes`.
///
/// Renvoie au plus `MAX_RESULTS` fichiers, triés du plus gros au plus petit.
pub fn find(min_bytes: u64) -> Vec<LargeFile> {
    let Some(home) = dirs::home_dir() else {
        return Vec::new();
    };
    let now = SystemTime::now();

    let mut found: Vec<LargeFile> = Vec::new();

    let walker = WalkDir::new(&home)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !(e.file_type().is_dir() && guard::is_protected_dir(&home, e.path())));

    for entry in walker.filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let Ok(meta) = entry.metadata() else {
            continue;
        };
        let size = meta.len();
        if size < min_bytes {
            continue;
        }

        let age_days = meta
            .modified()
            .ok()
            .and_then(|m| now.duration_since(m).ok())
            .map(|d| d.as_secs() / 86_400);

        found.push(LargeFile {
            path: entry.path().to_string_lossy().into_owned(),
            size_bytes: size,
            age_days,
        });
    }

    found.sort_by_key(|f| std::cmp::Reverse(f.size_bytes));
    found.truncate(MAX_RESULTS);
    found
}

/// Convertit un seuil en mégaoctets vers des octets.
pub fn mb_to_bytes(mb: u64) -> u64 {
    mb.saturating_mul(1_000_000)
}
