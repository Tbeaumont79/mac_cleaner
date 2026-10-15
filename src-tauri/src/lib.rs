mod apps;
mod cleaner;
mod disk;
mod guard;
mod launchitems;
mod largefiles;
mod maintenance;
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
async fn clean(mut category_ids: Vec<CategoryId>) -> Vec<cleaner::CleanResult> {
    // Vider la corbeille en premier : sinon elle emporterait définitivement ce
    // que les autres catégories viennent d'y déplacer.
    category_ids.sort_by_key(|id| !targets::find(*id).is_some_and(|c| c.permanent));
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

#[tauri::command]
async fn list_apps() -> Vec<apps::AppInfo> {
    apps::list()
}

#[tauri::command]
async fn uninstall_apps(paths: Vec<String>) -> Vec<apps::UninstallResult> {
    apps::uninstall(paths)
}

#[tauri::command]
async fn list_maintenance_tasks() -> Vec<maintenance::MaintenanceTask> {
    maintenance::list()
}

#[tauri::command]
async fn run_maintenance(ids: Vec<maintenance::TaskId>) -> Vec<maintenance::TaskResult> {
    maintenance::run(ids)
}

#[tauri::command]
async fn list_launch_items(with_signatures: bool) -> Vec<launchitems::LaunchItem> {
    launchitems::list(with_signatures)
}

#[tauri::command]
async fn set_launch_item_enabled(plist_path: String, enabled: bool) -> Result<(), String> {
    launchitems::set_enabled(&plist_path, enabled)
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
            disk_usage,
            list_apps,
            uninstall_apps,
            list_maintenance_tasks,
            run_maintenance,
            list_launch_items,
            set_launch_item_enabled
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
