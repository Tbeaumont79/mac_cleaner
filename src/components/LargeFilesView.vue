<script setup lang="ts">
import { computed, ref } from "vue";
import { formatBytes } from "../lib/api";
import {
  filesError,
  filesScanned,
  filesScanning,
  filesTrashing,
  largeFiles,
  minMb,
  refreshDisk,
  scanLargeFiles,
  selectedFiles,
  selectedFilesBytes,
  toggleFile,
  trashSelectedFiles,
} from "../lib/store";
import ConfirmModal from "./ConfirmModal.vue";

const THRESHOLDS = [100, 500, 1000, 5000];

const KINDS: [string, string[]][] = [
  ["Image disque", ["iso", "dmg", "img", "sparseimage", "sparsebundle"]],
  ["Disque virtuel", ["vmdk", "vdi", "qcow2", "vhd", "vhdx", "utm"]],
  ["Vidéo", ["mov", "mp4", "m4v", "mkv", "avi", "webm"]],
  ["Audio", ["mp3", "wav", "aac", "flac", "m4a", "aiff"]],
  ["Archive", ["zip", "tar", "gz", "tgz", "rar", "7z", "xz", "bz2"]],
  ["Installeur", ["pkg", "mpkg", "ipsw"]],
  ["Image", ["jpg", "jpeg", "png", "heic", "tif", "tiff", "raw", "psd"]],
  ["PDF", ["pdf"]],
];

function fileName(path: string): string {
  return path.slice(path.lastIndexOf("/") + 1);
}

function parentDir(path: string): string {
  return path.slice(0, path.lastIndexOf("/")).replace(/^\/Users\/[^/]+/, "~");
}

function kind(path: string): string {
  const name = fileName(path);
  const dot = name.lastIndexOf(".");
  if (dot <= 0) return "Fichier";
  const ext = name.slice(dot + 1).toLowerCase();
  return KINDS.find(([, exts]) => exts.includes(ext))?.[0] ?? ext.toUpperCase();
}

function ageLabel(days: number | null): string {
  if (days === null) return "Inconnu";
  if (days < 1) return "aujourd'hui";
  if (days < 2) return "hier";
  if (days < 30) return `il y a ${days} j`;
  if (days < 365) return `il y a ${Math.round(days / 30)} mois`;
  const years = Math.round(days / 365);
  return `il y a ${years} an${years > 1 ? "s" : ""}`;
}

const confirmOpen = ref(false);
const toast = ref<string | null>(null);
const busy = computed(() => filesScanning.value || filesTrashing.value);

const footer = computed(() => {
  if (filesScanning.value) return "Recherche en cours…";
  if (filesTrashing.value) return "Déplacement vers la corbeille…";
  if (toast.value) return toast.value;
  const n = selectedFiles.value.size;
  return n
    ? `${n} fichier${n > 1 ? "s" : ""} sélectionné${n > 1 ? "s" : ""} · ${formatBytes(selectedFilesBytes.value)}`
    : "Sélectionne les fichiers à mettre à la corbeille";
});

function toggle(path: string) {
  toast.value = null;
  toggleFile(path);
}

function search() {
  toast.value = null;
  scanLargeFiles();
}

async function confirmTrash() {
  confirmOpen.value = false;
  const sizes = new Map(largeFiles.value.map((f) => [f.path, f.size_bytes]));
  const results = await trashSelectedFiles();
  await refreshDisk();
  const done = results.filter((r) => r.ok);
  const ok = done.length;
  const bytes = done.reduce((sum, r) => sum + (sizes.get(r.path) ?? 0), 0);
  if (ok) toast.value = `${ok} fichier${ok > 1 ? "s" : ""} déplacé${ok > 1 ? "s" : ""} vers la corbeille (${formatBytes(bytes)})`;
}
</script>

<template>
  <div class="view">
    <header class="view-header">
      <div>
        <h1>Fichiers volumineux</h1>
        <p>Les plus gros fichiers de ton dossier personnel, triés par taille.</p>
      </div>
      <div class="controls">
        <select v-model.number="minMb" :disabled="busy" aria-label="Taille minimum">
          <option v-for="mb in THRESHOLDS" :key="mb" :value="mb">Plus de {{ formatBytes(mb * 1_000_000) }}</option>
        </select>
        <button type="button" class="btn-ghost" :disabled="busy" @click="search">
          {{ filesScanned ? "Rechercher à nouveau" : "Rechercher" }}
        </button>
      </div>
    </header>

    <div class="view-body">
      <p v-if="filesError" class="error">{{ filesError }}</p>

      <p v-if="filesScanning" class="message">Parcours de ton dossier personnel…</p>
      <p v-else-if="!filesScanned" class="message">
        Lance une recherche pour lister les fichiers de plus de {{ formatBytes(minMb * 1_000_000) }}.
      </p>
      <p v-else-if="largeFiles.length === 0" class="message">
        Aucun fichier au-delà de {{ formatBytes(minMb * 1_000_000) }}.
      </p>

      <template v-else>
        <div class="grid head">
          <span></span><span>Nom</span><span>Type</span><span>Modifié</span><span class="r">Taille</span>
        </div>
        <div class="rows">
          <label v-for="f in largeFiles" :key="f.path" class="grid row" :title="f.path">
            <input
              type="checkbox"
              :checked="selectedFiles.has(f.path)"
              :disabled="filesTrashing"
              @change="toggle(f.path)"
            />
            <span class="name-cell">
              <span class="name">{{ fileName(f.path) }}</span>
              <span class="path">{{ parentDir(f.path) }}</span>
            </span>
            <span class="muted">{{ kind(f.path) }}</span>
            <span class="muted">{{ ageLabel(f.age_days) }}</span>
            <span class="size r">{{ formatBytes(f.size_bytes) }}</span>
          </label>
        </div>
      </template>
    </div>

    <footer class="view-footer">
      <span class="info">{{ footer }}</span>
      <button
        type="button"
        class="btn-accent"
        :disabled="busy || selectedFiles.size === 0"
        @click="confirmOpen = true"
      >
        Mettre à la corbeille
      </button>
    </footer>

    <ConfirmModal
      v-if="confirmOpen"
      title="Mettre à la corbeille"
      confirm-label="Déplacer"
      @confirm="confirmTrash"
      @cancel="confirmOpen = false"
    >
      <p>
        {{ selectedFiles.size }} fichier(s), <strong>{{ formatBytes(selectedFilesBytes) }}</strong>.
      </p>
      <p>Récupérable depuis la corbeille. L'espace sera libéré une fois la corbeille vidée.</p>
    </ConfirmModal>
  </div>
</template>

<style scoped>
.controls {
  display: flex;
  gap: 10px;
  align-items: center;
}
select {
  min-height: 44px;
  padding: 0 14px;
  border-radius: 10px;
  border: 1px solid rgba(255, 255, 255, 0.22);
  background: var(--surface-strong);
  font-size: 14px;
  font-weight: 600;
}
.grid {
  display: grid;
  grid-template-columns: 34px minmax(0, 1fr) 120px 140px 90px;
  gap: 12px;
  align-items: center;
}
.head {
  padding: 0 18px 10px;
  font-size: 12px;
  font-weight: 600;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: var(--label);
}
.rows {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.row {
  font-size: 14px;
}
.name-cell {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}
.name,
.path {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.name {
  font-weight: 600;
}
.path {
  font-size: 12px;
  color: var(--label);
}
.r {
  text-align: right;
}
</style>
