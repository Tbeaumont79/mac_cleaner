<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import {
  type CategoryId,
  type CategoryScan,
  type CleanResult,
  clean,
  formatBytes,
  scan,
} from "./lib/api";
import CategoryCard from "./components/CategoryCard.vue";
import LargeFiles from "./components/LargeFiles.vue";

type Tab = "clean" | "large";
const activeTab = ref<Tab>("clean");

const scans = ref<CategoryScan[]>([]);
const selected = ref<Set<CategoryId>>(new Set());
const scanning = ref(false);
const cleaning = ref(false);
const lastResult = ref<CleanResult[] | null>(null);
const error = ref<string | null>(null);
const confirmOpen = ref(false);

const permanentSelected = computed(() =>
  scans.value.filter((s) => selected.value.has(s.id) && s.permanent),
);

const cleanErrors = computed(() =>
  (lastResult.value ?? []).flatMap((r) => r.errors),
);

const totalSelectedBytes = computed(() =>
  scans.value
    .filter((s) => selected.value.has(s.id))
    .reduce((sum, s) => sum + s.size_bytes, 0),
);

const totalFreed = computed(() =>
  (lastResult.value ?? []).reduce((sum, r) => sum + r.freed_bytes, 0),
);

async function runScan() {
  scanning.value = true;
  error.value = null;
  lastResult.value = null;
  try {
    scans.value = await scan();
    // Présélectionne ce qui contient quelque chose, SAUF les catégories à
    // suppression définitive (corbeille) : elles restent à cocher manuellement.
    selected.value = new Set(
      scans.value
        .filter((s) => s.size_bytes > 0 && !s.permanent)
        .map((s) => s.id),
    );
  } catch (e) {
    error.value = String(e);
  } finally {
    scanning.value = false;
  }
}

function toggle(id: CategoryId) {
  const next = new Set(selected.value);
  next.has(id) ? next.delete(id) : next.add(id);
  selected.value = next;
}

// Ouvre la modale de confirmation in-app (on n'utilise PAS window.confirm,
// non fiable dans la webview Tauri).
function requestClean() {
  if (selected.value.size === 0) return;
  confirmOpen.value = true;
}

// Exécute réellement le nettoyage après confirmation dans la modale.
async function confirmClean() {
  confirmOpen.value = false;
  cleaning.value = true;
  error.value = null;
  lastResult.value = null;
  try {
    const result = await clean([...selected.value]);
    await runScan(); // re-scan pour refléter l'espace libéré (remet lastResult à null)
    lastResult.value = result; // ré-affiche le bilan après le re-scan
  } catch (e) {
    error.value = String(e);
  } finally {
    cleaning.value = false;
  }
}

onMounted(runScan);
</script>

<template>
  <main class="app">
    <header>
      <h1>🧹 Mac Cleaner</h1>
      <p class="subtitle">
        Nettoyage sûr des caches et logs — tout passe par la corbeille.
      </p>
    </header>

    <nav class="tabs">
      <button :class="{ active: activeTab === 'clean' }" @click="activeTab = 'clean'">
        Nettoyage
      </button>
      <button :class="{ active: activeTab === 'large' }" @click="activeTab = 'large'">
        Gros fichiers
      </button>
    </nav>

    <LargeFiles v-if="activeTab === 'large'" />

    <template v-else>
    <section class="summary" v-if="!scanning">
      <div class="summary-num">{{ formatBytes(totalSelectedBytes) }}</div>
      <div class="summary-label">sélectionnés pour nettoyage</div>
    </section>
    <section class="summary" v-else>
      <div class="summary-num">…</div>
      <div class="summary-label">analyse en cours</div>
    </section>

    <p v-if="error" class="error">{{ error }}</p>

    <p v-if="lastResult" class="done">
      ✅ {{ formatBytes(totalFreed) }} libérés.
    </p>

    <details v-if="cleanErrors.length" class="errors-box">
      <summary>{{ cleanErrors.length }} élément(s) non supprimé(s)</summary>
      <ul>
        <li v-for="(msg, i) in cleanErrors.slice(0, 30)" :key="i">{{ msg }}</li>
      </ul>
      <p v-if="cleanErrors.length > 30">… et {{ cleanErrors.length - 30 }} de plus.</p>
    </details>

    <div class="list">
      <CategoryCard
        v-for="s in scans"
        :key="s.id"
        :scan="s"
        :selected="selected.has(s.id)"
        :disabled="cleaning || scanning"
        @toggle="toggle(s.id)"
      />
    </div>

    <footer>
      <button class="secondary" :disabled="scanning || cleaning" @click="runScan">
        {{ scanning ? "Analyse…" : "Re-analyser" }}
      </button>
      <button
        class="primary"
        :disabled="cleaning || scanning || selected.size === 0"
        @click="requestClean"
      >
        {{ cleaning ? "Nettoyage…" : "Nettoyer la sélection" }}
      </button>
    </footer>

    <!-- Modale de confirmation in-app (remplace window.confirm) -->
    <div v-if="confirmOpen" class="modal-overlay" @click.self="confirmOpen = false">
      <div class="modal">
        <h2 v-if="permanentSelected.length">⚠️ Suppression définitive</h2>
        <h2 v-else>Confirmer le nettoyage</h2>

        <p>
          Total à nettoyer :
          <strong>{{ formatBytes(totalSelectedBytes) }}</strong>.
        </p>

        <p v-if="permanentSelected.length" class="warn">
          {{ permanentSelected.map((s) => s.label).join(", ") }} sera supprimé
          <strong>pour de bon</strong>, sans passer par la corbeille. Action
          irréversible. Le reste part normalement à la corbeille.
        </p>
        <p v-else>
          Tout part à la corbeille : tu pourras le récupérer si besoin.
        </p>

        <div class="modal-actions">
          <button class="secondary" @click="confirmOpen = false">Annuler</button>
          <button
            :class="permanentSelected.length ? 'danger' : 'primary'"
            @click="confirmClean"
          >
            {{ permanentSelected.length ? "Supprimer définitivement" : "Nettoyer" }}
          </button>
        </div>
      </div>
    </div>
    </template>
  </main>
</template>

<style scoped>
.app {
  max-width: 680px;
  margin: 0 auto;
  padding: 1.5rem 1.25rem 6rem;
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}
header h1 {
  margin: 0;
  font-size: 1.6rem;
}
.subtitle {
  margin: 0.25rem 0 0;
  color: var(--muted);
  font-size: 0.9rem;
}
.tabs {
  display: flex;
  gap: 0.4rem;
  border-bottom: 1px solid var(--border);
}
.tabs button {
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  padding: 0.5rem 0.9rem;
  font-size: 0.95rem;
  font-weight: 600;
  color: var(--muted);
  cursor: pointer;
}
.tabs button.active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}
.summary {
  text-align: center;
  padding: 1rem;
  border-radius: 14px;
  background: var(--card-bg);
  border: 1px solid var(--border);
}
.summary-num {
  font-size: 2.2rem;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
  color: var(--accent);
}
.summary-label {
  color: var(--muted);
  font-size: 0.85rem;
}
.list {
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
}
.error {
  color: #d33;
  font-size: 0.85rem;
}
.done {
  color: var(--accent);
  font-weight: 600;
}
.errors-box {
  font-size: 0.8rem;
  color: var(--muted);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 0.5rem 0.75rem;
  background: var(--card-bg);
}
.errors-box summary {
  cursor: pointer;
  font-weight: 600;
}
.errors-box ul {
  margin: 0.5rem 0 0;
  padding-left: 1.1rem;
  max-height: 160px;
  overflow: auto;
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
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.3);
}
.modal h2 {
  margin: 0 0 0.75rem;
  font-size: 1.2rem;
}
.modal .warn {
  color: #d33;
}
.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 0.6rem;
  margin-top: 1.25rem;
}
.danger {
  background: #d33;
  color: #fff;
  border-color: #d33;
}
footer {
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
  padding: 0.85rem 1.25rem;
  background: var(--footer-bg);
  border-top: 1px solid var(--border);
  backdrop-filter: blur(8px);
}
button {
  border-radius: 9px;
  padding: 0.6rem 1.1rem;
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
</style>

<style>
:root {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Inter, sans-serif;
  font-size: 15px;
  --accent: #2f7d4f;
  --border: #e2e2e2;
  --card-bg: #ffffff;
  --card-selected: #eef7f0;
  --footer-bg: rgba(246, 246, 246, 0.85);
  --muted: #777;
  color: #1a1a1a;
  background-color: #f6f6f6;
  -webkit-font-smoothing: antialiased;
}
body {
  margin: 0;
}
@media (prefers-color-scheme: dark) {
  :root {
    --accent: #4caf75;
    --border: #3a3a3a;
    --card-bg: #1f1f1f;
    --card-selected: #1c2a22;
    --footer-bg: rgba(30, 30, 30, 0.85);
    --muted: #999;
    color: #f0f0f0;
    background-color: #181818;
  }
}
</style>
