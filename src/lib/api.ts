// Wrappers typés autour des commandes Tauri (backend Rust).
// Les identifiants de catégorie doivent correspondre exactement à l'enum
// `CategoryId` de src-tauri/src/targets.rs (serde sérialise par nom de variante).

import { invoke } from "@tauri-apps/api/core";

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

/** Formate un nombre d'octets en chaîne lisible (Ko, Mo, Go…). */
export function formatBytes(bytes: number): string {
  if (bytes <= 0) return "0 o";
  const units = ["o", "Ko", "Mo", "Go", "To"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  const value = bytes / Math.pow(1024, i);
  return `${value.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}
