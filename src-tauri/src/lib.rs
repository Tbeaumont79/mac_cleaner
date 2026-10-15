mod cleaner;
mod disk;
mod guard;
mod largefiles;
mod scanner;
mod targets;

use targets::CategoryId;

/// Scanne toutes les catégories (lecture seule) et renvoie les tailles
/// récupérables. Appelé au lancement de l'app.
#[tauri::command]
async fn scan() -> Vec<scanner::CategoryScan> {
    scanner::scan_all()
}

/// Nettoie les catégories sélectionnées (déplacement vers la corbeille).
/// Renvoie un bilan par catégorie. Une catégorie inconnue est ignorée.
#[tauri::command]
async fn clean(category_ids: Vec<CategoryId>) -> Vec<cleaner::CleanResult> {
    category_ids
        .into_iter()
        .filter_map(|id| cleaner::clean_category(id).ok())
        .collect()
}

/// Recherche les gros fichiers de `~` au-dessus du seuil donné (en Mo).
#[tauri::command]
async fn find_large_files(min_mb: u64) -> Vec<largefiles::LargeFile> {
    largefiles::find(largefiles::mb_to_bytes(min_mb))
}

/// Déplace des fichiers précis vers la corbeille (chemins validés, réversible).
#[tauri::command]
async fn trash_paths(paths: Vec<String>) -> Vec<cleaner::TrashResult> {
    cleaner::trash_paths(paths)
}

#[tauri::command]
async fn disk_usage() -> Option<disk::DiskUsage> {
    disk::usage()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            scan,
            clean,
            find_large_files,
            trash_paths,
            disk_usage
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
