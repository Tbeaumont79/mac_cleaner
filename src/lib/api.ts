// Wrappers typés autour des commandes Tauri (backend Rust).
// Les identifiants de catégorie doivent correspondre exactement à l'enum
// `CategoryId` de src-tauri/src/targets.rs (serde sérialise par nom de variante).

import { invoke } from "@tauri-apps/api/core";
import { openUrl, revealItemInDir } from "@tauri-apps/plugin-opener";

export type CategoryId =
  | "UserCaches"
  | "ContainerCaches"
  | "UserLogs"
  | "SavedAppState"
  | "XcodeDerivedData"
  | "Trash";

export interface CategoryScan {
  id: CategoryId;
  label: string;
  description: string;
  size_bytes: number;
  file_count: number;
  exists: boolean;
  /** true = nettoyage définitif (pas la corbeille). */
  permanent: boolean;
  access_denied: boolean;
}

export interface CleanResult {
  id: CategoryId;
  freed_bytes: number;
  items_trashed: number;
  errors: string[];
}

export interface LargeFile {
  path: string;
  size_bytes: number;
  age_days: number | null;
}

export interface DiskUsage {
  total_bytes: number;
  available_bytes: number;
}

export interface Leftover {
  path: string;
  size_bytes: number;
}

export interface AppInfo {
  path: string;
  name: string;
  bundle_id: string;
  version: string | null;
  app_bytes: number;
  leftovers: Leftover[];
  leftover_bytes: number;
  last_used_days: number | null;
  running: boolean;
}

export interface UninstallResult {
  path: string;
  ok: boolean;
  freed_bytes: number;
  errors: string[];
}

export type TaskId =
  | "FlushDns"
  | "ReindexSpotlight"
  | "PurgeMemory"
  | "RebuildLaunchServices"
  | "ResetQuickLook"
  | "ClearFontCaches";

export interface MaintenanceTask {
  id: TaskId;
  label: string;
  description: string;
  admin: boolean;
}

export interface TaskResult {
  id: TaskId;
  ok: boolean;
  error: string | null;
}

export type Signature =
  | { Trusted: { authority: string } }
  | "AdHoc"
  | "Unsigned"
  | "Invalid"
  | "Script"
  | "ProgramMissing";

export interface LaunchItem {
  label: string;
  plist_path: string;
  program: string | null;
  scope: "User" | "SystemAgent" | "SystemDaemon";
  enabled: boolean;
  editable: boolean;
  signature: Signature | null;
}

export interface SecurityCheck {
  id: string;
  label: string;
  status: "Ok" | "Warning" | "Unknown";
  detail: string;
  settings_url: string | null;
}

export interface TrashResult {
  path: string;
  ok: boolean;
  error: string | null;
}

/** Scan lecture seule de toutes les catégories. */
export function scan(): Promise<CategoryScan[]> {
  return invoke<CategoryScan[]>("scan");
}

/** Cherche les gros fichiers de ~ au-dessus du seuil (en Mo). */
export function findLargeFiles(minMb: number): Promise<LargeFile[]> {
  return invoke<LargeFile[]>("find_large_files", { minMb });
}

/** Déplace des fichiers précis vers la corbeille. */
export function trashPaths(paths: string[]): Promise<TrashResult[]> {
  return invoke<TrashResult[]>("trash_paths", { paths });
}

/** Nettoie les catégories sélectionnées (déplacement vers la corbeille). */
export function clean(categoryIds: CategoryId[]): Promise<CleanResult[]> {
  return invoke<CleanResult[]>("clean", { categoryIds });
}

export function listApps(): Promise<AppInfo[]> {
  return invoke<AppInfo[]>("list_apps");
}

export function uninstallApps(paths: string[]): Promise<UninstallResult[]> {
  return invoke<UninstallResult[]>("uninstall_apps", { paths });
}

export function listMaintenanceTasks(): Promise<MaintenanceTask[]> {
  return invoke<MaintenanceTask[]>("list_maintenance_tasks");
}

export function runMaintenance(ids: TaskId[]): Promise<TaskResult[]> {
  return invoke<TaskResult[]>("run_maintenance", { ids });
}

export function listLaunchItems(withSignatures: boolean): Promise<LaunchItem[]> {
  return invoke<LaunchItem[]>("list_launch_items", { withSignatures });
}

export function setLaunchItemEnabled(plistPath: string, enabled: boolean): Promise<void> {
  return invoke<void>("set_launch_item_enabled", { plistPath, enabled });
}

export function securityChecks(): Promise<SecurityCheck[]> {
  return invoke<SecurityCheck[]>("security_checks");
}

export function openSettings(url: string): Promise<void> {
  return openUrl(url);
}

export function revealInFinder(path: string): Promise<void> {
  return revealItemInDir(path);
}

export function diskUsage(): Promise<DiskUsage | null> {
  return invoke<DiskUsage | null>("disk_usage");
}

// Base 1000 comme le Finder, pour que les tailles concordent avec macOS.
export function formatBytes(bytes: number): string {
  if (bytes <= 0) return "0 o";
  const units = ["o", "Ko", "Mo", "Go", "To"];
  const i = Math.min(units.length - 1, Math.floor(Math.log(bytes) / Math.log(1000)));
  const value = bytes / Math.pow(1000, i);
  return `${value.toLocaleString("fr-FR", { maximumFractionDigits: i < 2 ? 0 : 1 })} ${units[i]}`;
}

export function openFullDiskAccessSettings(): Promise<void> {
  return openUrl("x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles");
}
