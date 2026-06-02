<script setup lang="ts">
import { computed, ref } from "vue";
import {
  type LargeFile,
  findLargeFiles,
  formatBytes,
  trashPaths,
} from "../lib/api";

const THRESHOLDS = [
  { mb: 100, label: "100 Mo" },
  { mb: 500, label: "500 Mo" },
  { mb: 1024, label: "1 Go" },
  { mb: 5120, label: "5 Go" },
];

const minMb = ref(100);
const files = ref<LargeFile[]>([]);
const selected = ref<Set<string>>(new Set());
const scanning = ref(false);
const trashing = ref(false);
const confirmOpen = ref(false);
const scanned = ref(false);
const error = ref<string | null>(null);
const lastTrashedCount = ref<number | null>(null);

const selectedBytes = computed(() =>
  files.value
    .filter((f) => selected.value.has(f.path))
    .reduce((sum, f) => sum + f.size_bytes, 0),
);

function ageLabel(days: number | null): string {
  if (days === null) return "—";
  if (days < 1) return "aujourd'hui";
  if (days < 30) return `il y a ${days} j`;
  if (days < 365) return `il y a ${Math.round(days / 30)} mois`;
  return `il y a ${(days / 365).toFixed(1)} an(s)`;
}

/** Raccourcit un chemin long en gardant début et fin lisibles. */
function shortPath(path: string): string {
  return path.replace(/^\/Users\/[^/]+/, "~");
}

async function runScan() {
  scanning.value = true;
  error.value = null;
  lastTrashedCount.value = null;
  selected.value = new Set();
  try {
    files.value = await findLargeFiles(minMb.value);
    scanned.value = true;
  } catch (e) {
    error.value = String(e);
  } finally {
    scanning.value = false;
  }
}

function toggle(path: string) {
  const next = new Set(selected.value);
  next.has(path) ? next.delete(path) : next.add(path);
  selected.value = next;
}

async function confirmTrash() {
  confirmOpen.value = false;
  trashing.value = true;
  error.value = null;
  try {
    const paths = [...selected.value];
    const results = await trashPaths(paths);
    const okPaths = new Set(results.filter((r) => r.ok).map((r) => r.path));
    // Retire de la liste ceux qui sont partis à la corbeille.
    files.value = files.value.filter((f) => !okPaths.has(f.path));
    selected.value = new Set();
    lastTrashedCount.value = okPaths.size;
    const failed = results.filter((r) => !r.ok);
    if (failed.length) {
      error.value = `${failed.length} fichier(s) non supprimé(s) : ${failed[0].error ?? ""}`;
    }
  } catch (e) {
    error.value = String(e);
  } finally {
    trashing.value = false;
  }
}
</script>

<template>
  <section class="large">
    <div class="controls">
      <label>Taille minimum :</label>
      <select v-model.number="minMb" :disabled="scanning || trashing">
        <option v-for="t in THRESHOLDS" :key="t.mb" :value="t.mb">
          {{ t.label }}
        </option>
      </select>
      <button class="primary" :disabled="scanning || trashing" @click="runScan">
        {{ scanning ? "Recherche…" : "Rechercher" }}
      </button>
    </div>

    <p class="hint">
      Parcourt ton dossier personnel. Les fichiers cochés partent à la corbeille
      (réversible).
    </p>

    <p v-if="error" class="error">{{ error }}</p>
    <p v-if="lastTrashedCount !== null" class="done">
      ✅ {{ lastTrashedCount }} fichier(s) déplacé(s) vers la corbeille.
    </p>

    <p v-if="scanned && !scanning && files.length === 0" class="empty-msg">
      Aucun fichier au-dessus de ce seuil. 🎉
    </p>

    <div class="list">
      <label
        v-for="f in files"
        :key="f.path"
        class="row"
        :class="{ selected: selected.has(f.path) }"
      >
        <input
          type="checkbox"
          :checked="selected.has(f.path)"
          :disabled="trashing"
          @change="toggle(f.path)"
        />
        <div class="meta">
          <div class="path" :title="f.path">{{ shortPath(f.path) }}</div>
          <div class="age">{{ ageLabel(f.age_days) }}</div>
        </div>
        <div class="size">{{ formatBytes(f.size_bytes) }}</div>
      </label>
    </div>

    <footer v-if="selected.size > 0">
      <span class="footer-info">
        {{ selected.size }} fichier(s) — {{ formatBytes(selectedBytes) }}
      </span>
      <button class="primary" :disabled="trashing" @click="confirmOpen = true">
        {{ trashing ? "Déplacement…" : "Déplacer vers la corbeille" }}
      </button>
    </footer>

    <div v-if="confirmOpen" class="modal-overlay" @click.self="confirmOpen = false">
      <div class="modal">
        <h2>Déplacer vers la corbeille</h2>
        <p>
          {{ selected.size }} fichier(s),
          <strong>{{ formatBytes(selectedBytes) }}</strong>.
          Récupérable depuis la corbeille.
        </p>
        <div class="modal-actions">
          <button class="secondary" @click="confirmOpen = false">Annuler</button>
          <button class="primary" @click="confirmTrash">Déplacer</button>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.large {
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
}
.controls {
  display: flex;
  align-items: center;
  gap: 0.6rem;
}
.controls label {
  color: var(--muted);
  font-size: 0.9rem;
}
select {
  padding: 0.45rem 0.6rem;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--card-bg);
  color: inherit;
}
.hint {
  font-size: 0.8rem;
  color: var(--muted);
  margin: 0;
}
.error {
  color: #d33;
  font-size: 0.85rem;
}
.done {
  color: var(--accent);
  font-weight: 600;
}
.empty-msg {
  color: var(--muted);
  text-align: center;
  padding: 2rem 0;
}
.list {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}
.row {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.6rem 0.8rem;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--card-bg);
  cursor: pointer;
}
.row.selected {
  border-color: var(--accent);
  background: var(--card-selected);
}
input[type="checkbox"] {
  width: 17px;
  height: 17px;
  accent-color: var(--accent);
}
.meta {
  flex: 1;
  min-width: 0;
}
.path {
  font-size: 0.85rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  direction: rtl;
  text-align: left;
}
.age {
  font-size: 0.72rem;
  color: var(--muted);
}
.size {
  font-weight: 600;
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}
footer {
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 1rem;
  padding: 0.85rem 1.25rem;
  background: var(--footer-bg);
  border-top: 1px solid var(--border);
  backdrop-filter: blur(8px);
}
.footer-info {
  color: var(--muted);
  font-size: 0.85rem;
}
button {
  border-radius: 9px;
  padding: 0.55rem 1.05rem;
  font-weight: 600;
  font-size: 0.9rem;
  border: 1px solid var(--border);
  cursor: pointer;
}
button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.primary {
  background: var(--accent);
  color: #fff;
  border-color: var(--accent);
}
.secondary {
  background: var(--card-bg);
  color: inherit;
}
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.45);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10;
}
.modal {
  background: var(--card-bg);
  border: 1px solid var(--border);
  border-radius: 16px;
  padding: 1.5rem;
  max-width: 420px;
  width: calc(100% - 3rem);
}
.modal h2 {
  margin: 0 0 0.75rem;
  font-size: 1.2rem;
}
.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.6rem;
  margin-top: 1.25rem;
}
</style>
