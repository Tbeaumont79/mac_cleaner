import { computed, ref } from "vue";
import {
  type AppInfo,
  type CategoryId,
  type LaunchItem,
  type MaintenanceTask,
  type TaskId,
  type TaskResult,
  type CategoryScan,
  type CleanResult,
  type DiskUsage,
  type LargeFile,
  type TrashResult,
  type UninstallResult,
  clean,
  diskUsage,
  findLargeFiles,
  listApps,
  listLaunchItems,
  listMaintenanceTasks,
  runMaintenance,
  setLaunchItemEnabled,
  formatBytes,
  scan,
  trashPaths,
  uninstallApps,
} from "./api";

export type View = "scan" | "junk" | "speed" | "files" | "apps";

export const view = ref<View>("scan");

export const disk = ref<DiskUsage | null>(null);

export async function refreshDisk() {
  try {
    disk.value = await diskUsage();
  } catch {
    disk.value = null;
  }
}

export const categories = ref<CategoryScan[]>([]);
export const selectedCategories = ref<Set<CategoryId>>(new Set());
// Choix explicites de l'utilisateur, pour qu'une catégorie vidée revienne
// cochée quand elle se remplit à nouveau.
const categoryChoices = new Map<CategoryId, boolean>();
export const junkScanning = ref(false);
export const junkCleaning = ref(false);
export const junkScanned = ref(false);
export const junkError = ref<string | null>(null);

export const junkTotal = computed(() =>
  categories.value.reduce((sum, c) => sum + c.size_bytes, 0),
);

export const selectedJunk = computed(() =>
  categories.value.filter((c) => selectedCategories.value.has(c.id)),
);

export const selectedJunkBytes = computed(() =>
  selectedJunk.value.reduce((sum, c) => sum + c.size_bytes, 0),
);

export const trashCategory = computed(() =>
  categories.value.find((c) => c.id === "Trash"),
);

let junkScan: Promise<void> | null = null;

export function scanJunk(): Promise<void> {
  junkScan ??= doScanJunk().finally(() => (junkScan = null));
  return junkScan;
}

async function doScanJunk() {
  junkScanning.value = true;
  junkError.value = null;
  try {
    categories.value = await scan();
    // La corbeille est définitive : jamais présélectionnée.
    selectedCategories.value = new Set(
      categories.value
        .filter((c) => c.size_bytes > 0 && (categoryChoices.get(c.id) ?? !c.permanent))
        .map((c) => c.id),
    );
    junkScanned.value = true;
  } catch (e) {
    junkError.value = String(e);
  } finally {
    junkScanning.value = false;
  }
}

export function toggleCategory(id: CategoryId) {
  const next = new Set(selectedCategories.value);
  next.has(id) ? next.delete(id) : next.add(id);
  // Une suppression définitive ne doit jamais être présélectionnée au scan suivant.
  const permanent = categories.value.some((c) => c.id === id && c.permanent);
  if (permanent) categoryChoices.delete(id);
  else categoryChoices.set(id, next.has(id));
  selectedCategories.value = next;
}

export interface CleanSummary {
  trashedBytes: number;
  deletedBytes: number;
  errors: string[];
}

export async function cleanSelected(): Promise<CleanSummary | null> {
  const targets = selectedJunk.value;
  if (targets.length === 0 || junkCleaning.value || junkScanning.value) return null;
  const permanent = new Set(targets.filter((c) => c.permanent).map((c) => c.id));
  junkCleaning.value = true;
  junkError.value = null;
  let results: CleanResult[] = [];
  try {
    results = await clean(targets.map((c) => c.id));
  } catch (e) {
    junkError.value = String(e);
    return null;
  } finally {
    junkCleaning.value = false;
  }
  const summary: CleanSummary = { trashedBytes: 0, deletedBytes: 0, errors: [] };
  for (const r of results) {
    if (permanent.has(r.id)) summary.deletedBytes += r.freed_bytes;
    else summary.trashedBytes += r.freed_bytes;
    summary.errors.push(...r.errors);
  }
  await Promise.all([scanJunk(), refreshDisk()]);
  return summary;
}

export const minMb = ref(500);
export const largeFiles = ref<LargeFile[]>([]);
export const selectedFiles = ref<Set<string>>(new Set());
export const filesScanning = ref(false);
export const filesTrashing = ref(false);
export const filesScanned = ref(false);
export const scannedMinMb = ref(minMb.value);
export const filesError = ref<string | null>(null);

export const largeFilesTotal = computed(() =>
  largeFiles.value.reduce((sum, f) => sum + f.size_bytes, 0),
);

export const selectedFilesBytes = computed(() =>
  largeFiles.value
    .filter((f) => selectedFiles.value.has(f.path))
    .reduce((sum, f) => sum + f.size_bytes, 0),
);

let filesScan: Promise<void> | null = null;

export function scanLargeFiles(): Promise<void> {
  filesScan ??= doScanLargeFiles().finally(() => (filesScan = null));
  return filesScan;
}

async function doScanLargeFiles() {
  filesScanning.value = true;
  filesError.value = null;
  selectedFiles.value = new Set();
  const threshold = minMb.value;
  try {
    largeFiles.value = await findLargeFiles(threshold);
    scannedMinMb.value = threshold;
    filesScanned.value = true;
  } catch (e) {
    filesError.value = String(e);
  } finally {
    filesScanning.value = false;
  }
}

export function toggleFile(path: string) {
  const next = new Set(selectedFiles.value);
  next.has(path) ? next.delete(path) : next.add(path);
  selectedFiles.value = next;
}

export async function trashSelectedFiles(): Promise<TrashResult[]> {
  if (filesTrashing.value || selectedFiles.value.size === 0) return [];
  filesTrashing.value = true;
  filesError.value = null;
  try {
    return await doTrashSelectedFiles();
  } catch (e) {
    filesError.value = String(e);
    return [];
  } finally {
    filesTrashing.value = false;
    // La corbeille vient de grossir : sa taille affichée doit suivre.
    if (junkScanned.value) void scanJunk();
  }
}

async function doTrashSelectedFiles(): Promise<TrashResult[]> {
  {
    const results = await trashPaths([...selectedFiles.value]);
    const ok = new Set(results.filter((r) => r.ok).map((r) => r.path));
    largeFiles.value = largeFiles.value.filter((f) => !ok.has(f.path));
    selectedFiles.value = new Set();
    const failed = results.filter((r) => !r.ok);
    if (failed.length) {
      filesError.value = `${failed.length} fichier(s) non déplacé(s) : ${failed[0].error ?? ""}`;
    }
    return results;
  }
}

export type SmartPhase = "idle" | "scanning" | "results" | "cleaning" | "cleaned";

export const smartPhase = ref<SmartPhase>("idle");
export const smartStep = ref<"junk" | "files">("junk");
export const smartSummary = ref<CleanSummary | null>(null);

export async function runSmartScan() {
  if (smartPhase.value === "scanning" || smartPhase.value === "cleaning") return;
  smartPhase.value = "scanning";
  smartStep.value = "junk";
  await scanJunk();
  smartStep.value = "files";
  await scanLargeFiles();
  smartPhase.value = "results";
}

export async function runSmartClean() {
  if (smartPhase.value !== "results") return;
  smartPhase.value = "cleaning";
  smartSummary.value = await cleanSelected();
  smartPhase.value = smartSummary.value ? "cleaned" : "results";
}

export function describeClean(s: CleanSummary): string {
  const parts: string[] = [];
  if (s.trashedBytes > 0) parts.push(`${formatBytes(s.trashedBytes)} déplacés vers la corbeille`);
  if (s.deletedBytes > 0) parts.push(`${formatBytes(s.deletedBytes)} supprimés définitivement`);
  if (s.errors.length) parts.push(`${s.errors.length} élément(s) ignoré(s)`);
  return parts.length ? parts.join(" · ") : "Rien n'a été déplacé";
}

export const apps = ref<AppInfo[]>([]);
export const selectedApps = ref<Set<string>>(new Set());
export const appsLoading = ref(false);
export const appsLoaded = ref(false);
export const appsUninstalling = ref(false);
export const appsError = ref<string | null>(null);

export const appBytes = (a: AppInfo) => a.app_bytes + a.leftover_bytes;

export const selectedAppsBytes = computed(() =>
  apps.value.filter((a) => selectedApps.value.has(a.path)).reduce((sum, a) => sum + appBytes(a), 0),
);

let appsLoad: Promise<void> | null = null;

export function loadApps(): Promise<void> {
  appsLoad ??= doLoadApps().finally(() => (appsLoad = null));
  return appsLoad;
}

async function doLoadApps() {
  appsLoading.value = true;
  appsError.value = null;
  try {
    apps.value = (await listApps()).sort((a, b) => appBytes(b) - appBytes(a));
    const present = new Set(apps.value.filter((a) => !a.running).map((a) => a.path));
    selectedApps.value = new Set([...selectedApps.value].filter((p) => present.has(p)));
    appsLoaded.value = true;
  } catch (e) {
    appsError.value = String(e);
  } finally {
    appsLoading.value = false;
  }
}

export function toggleApp(path: string) {
  appsError.value = null;
  const next = new Set(selectedApps.value);
  next.has(path) ? next.delete(path) : next.add(path);
  selectedApps.value = next;
}

export async function uninstallSelectedApps(): Promise<UninstallResult[]> {
  if (appsUninstalling.value || appsLoading.value || selectedApps.value.size === 0) return [];
  appsUninstalling.value = true;
  appsError.value = null;
  try {
    const results = await uninstallApps([...selectedApps.value]);
    const removed = new Set(results.filter((r) => r.ok).map((r) => r.path));
    apps.value = apps.value.filter((a) => !removed.has(a.path));
    selectedApps.value = new Set([...selectedApps.value].filter((p) => !removed.has(p)));
    const failed = results.filter((r) => !r.ok);
    if (failed.length) appsError.value = failed.map((r) => r.errors[0] ?? r.path).join(" · ");
    return results;
  } catch (e) {
    appsError.value = String(e);
    return [];
  } finally {
    appsUninstalling.value = false;
    void refreshDisk();
    if (junkScanned.value) void scanJunk();
  }
}

export const tasks = ref<MaintenanceTask[]>([]);
export const selectedTasks = ref<Set<TaskId>>(new Set());
export const taskResults = ref<Map<TaskId, TaskResult>>(new Map());
export const tasksRunning = ref(false);
export const launchItems = ref<LaunchItem[]>([]);
export const launchItemsLoaded = ref(false);
export const launchItemBusy = ref<string | null>(null);
export const speedError = ref<string | null>(null);

// Présélection : rapide et sans mot de passe administrateur.
const DEFAULT_TASKS: TaskId[] = ["ResetQuickLook"];

export async function loadSpeed() {
  speedError.value = null;
  try {
    const [list, items] = await Promise.all([listMaintenanceTasks(), listLaunchItems(false)]);
    if (tasks.value.length === 0) {
      selectedTasks.value = new Set(DEFAULT_TASKS.filter((id) => list.some((t) => t.id === id)));
    }
    tasks.value = list;
    launchItems.value = items;
    launchItemsLoaded.value = true;
  } catch (e) {
    speedError.value = String(e);
  }
}

export function toggleTask(id: TaskId) {
  const next = new Set(selectedTasks.value);
  next.has(id) ? next.delete(id) : next.add(id);
  selectedTasks.value = next;
}

export async function runSelectedTasks(): Promise<TaskResult[]> {
  if (tasksRunning.value || selectedTasks.value.size === 0) return [];
  tasksRunning.value = true;
  speedError.value = null;
  try {
    const results = await runMaintenance([...selectedTasks.value]);
    const next = new Map(taskResults.value);
    for (const r of results) next.set(r.id, r);
    taskResults.value = next;
    selectedTasks.value = new Set(results.filter((r) => !r.ok).map((r) => r.id));
    return results;
  } catch (e) {
    speedError.value = String(e);
    return [];
  } finally {
    tasksRunning.value = false;
  }
}

export async function setLaunchItem(item: LaunchItem, enabled: boolean): Promise<boolean> {
  if (launchItemBusy.value) return false;
  launchItemBusy.value = item.plist_path;
  speedError.value = null;
  try {
    await setLaunchItemEnabled(item.plist_path, enabled);
    launchItems.value = launchItems.value.map((i) =>
      i.plist_path === item.plist_path ? { ...i, enabled } : i,
    );
    return true;
  } catch (e) {
    speedError.value = `${item.label} : ${e}`;
    return false;
  } finally {
    launchItemBusy.value = null;
  }
}
