import { computed, ref } from "vue";
import {
  type CategoryId,
  type CategoryScan,
  type CleanResult,
  type DiskUsage,
  type LargeFile,
  type TrashResult,
  clean,
  diskUsage,
  findLargeFiles,
  formatBytes,
  scan,
  trashPaths,
} from "./api";

export type View = "scan" | "junk" | "files";

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
    const firstScan = !junkScanned.value;
    const previous = selectedCategories.value;
    categories.value = await scan();
    // La corbeille est définitive : jamais présélectionnée.
    selectedCategories.value = new Set(
      categories.value
        .filter((c) => c.size_bytes > 0 && (firstScan ? !c.permanent : previous.has(c.id)))
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
    const results = await trashPaths([...selectedFiles.value]);
    const ok = new Set(results.filter((r) => r.ok).map((r) => r.path));
    largeFiles.value = largeFiles.value.filter((f) => !ok.has(f.path));
    selectedFiles.value = new Set();
    const failed = results.filter((r) => !r.ok);
    if (failed.length) {
      filesError.value = `${failed.length} fichier(s) non déplacé(s) : ${failed[0].error ?? ""}`;
    }
    // La corbeille vient de grossir : sa taille affichée doit suivre.
    if (junkScanned.value) await scanJunk();
    return results;
  } catch (e) {
    filesError.value = String(e);
    return [];
  } finally {
    filesTrashing.value = false;
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
