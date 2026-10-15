<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { formatBytes } from "../lib/api";
import { ageLabel } from "../lib/format";
import {
  appBytes,
  apps,
  appsError,
  appsLoaded,
  appsLoading,
  appsUninstalling,
  loadApps,
  selectedApps,
  selectedAppsBytes,
  toggleApp,
  uninstallSelectedApps,
} from "../lib/store";
import ConfirmModal from "./ConfirmModal.vue";

const COLORS = ["#FF8AD8", "#7DB2FF", "#FFC25C", "#3FE0B8", "#C59BFF", "#5CE1FF"];

function tileColor(name: string): string {
  let h = 0;
  for (const c of name) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  return COLORS[h % COLORS.length];
}

function shortPath(path: string): string {
  return path.replace(/^\/Users\/[^/]+/, "~");
}

const query = ref("");
const expanded = ref<string | null>(null);
const confirmOpen = ref(false);
const toast = ref<string | null>(null);
const busy = computed(() => appsLoading.value || appsUninstalling.value);

const visible = computed(() => {
  const q = query.value.trim().toLowerCase();
  return q ? apps.value.filter((a) => a.name.toLowerCase().includes(q)) : apps.value;
});

const selectedNames = computed(() =>
  apps.value.filter((a) => selectedApps.value.has(a.path)).map((a) => a.name),
);

const footer = computed(() => {
  if (appsLoading.value) return "Analyse des applications…";
  if (appsUninstalling.value) return "Désinstallation en cours…";
  if (toast.value) return toast.value;
  const n = selectedApps.value.size;
  return n
    ? `${n} app${n > 1 ? "s" : ""} sélectionnée${n > 1 ? "s" : ""} · ${formatBytes(selectedAppsBytes.value)}`
    : "Sélectionne les apps à désinstaller";
});

function toggle(path: string) {
  toast.value = null;
  toggleApp(path);
}

async function confirmUninstall() {
  confirmOpen.value = false;
  toast.value = null;
  const results = await uninstallSelectedApps();
  const done = results.filter((r) => r.ok);
  if (!done.length) return;
  const bytes = done.reduce((sum, r) => sum + r.freed_bytes, 0);
  const leftoversFailed = done.reduce((sum, r) => sum + r.errors.length, 0);
  toast.value =
    `${done.length} app${done.length > 1 ? "s" : ""} désinstallée${done.length > 1 ? "s" : ""} (${formatBytes(bytes)} libérés)` +
    (leftoversFailed ? ` · ${leftoversFailed} fichier(s) associé(s) ignoré(s)` : "");
}

onMounted(() => {
  if (!appsLoaded.value) loadApps();
});
</script>

<template>
  <div class="view">
    <header class="view-header">
      <div>
        <h1>Désinstallateur</h1>
        <p>Supprime une app avec tous ses fichiers associés, sans rien laisser traîner.</p>
      </div>
      <input
        v-model="query"
        class="search"
        type="search"
        placeholder="Rechercher une app"
        aria-label="Rechercher une app"
      />
    </header>

    <div class="view-body list">
      <p v-if="appsError" class="error">{{ appsError }}</p>
      <p v-if="appsLoading && !appsLoaded" class="message">Analyse des applications…</p>
      <p v-else-if="appsLoaded && visible.length === 0" class="message">Aucune app trouvée.</p>

      <div v-for="a in visible" :key="a.path" class="app">
        <label class="row" :class="{ disabled: a.running }">
          <input
            type="checkbox"
            :checked="selectedApps.has(a.path)"
            :disabled="busy || a.running"
            @change="toggle(a.path)"
          />
          <div class="tile" :style="{ background: tileColor(a.name) }">{{ a.name.charAt(0) }}</div>
          <div class="text">
            <span class="name">
              {{ a.name }}
              <span v-if="a.running" class="badge">Ouverte</span>
            </span>
            <span class="desc">
              Dernière ouverture : {{ ageLabel(a.last_used_days) }}
              <template v-if="a.version"> · v{{ a.version }}</template>
            </span>
          </div>
          <div class="right">
            <span class="size">{{ formatBytes(appBytes(a)) }}</span>
            <button
              v-if="a.leftovers.length"
              type="button"
              class="link"
              @click.prevent="expanded = expanded === a.path ? null : a.path"
            >
              {{ a.leftovers.length }} fichier{{ a.leftovers.length > 1 ? "s" : "" }} associé{{
                a.leftovers.length > 1 ? "s" : ""
              }}
              · {{ formatBytes(a.leftover_bytes) }}
            </button>
          </div>
        </label>
        <ul v-if="expanded === a.path" class="leftovers">
          <li v-for="l in a.leftovers" :key="l.path">
            <span class="lpath" :title="l.path">{{ shortPath(l.path) }}</span>
            <span class="muted">{{ formatBytes(l.size_bytes) }}</span>
          </li>
        </ul>
      </div>
    </div>

    <footer class="view-footer">
      <span class="info">{{ footer }}</span>
      <div class="actions">
        <button type="button" class="btn-ghost" :disabled="busy" @click="toast = null; loadApps()">
          Actualiser
        </button>
        <button
          type="button"
          class="btn-accent"
          :disabled="busy || selectedApps.size === 0"
          @click="confirmOpen = true"
        >
          Désinstaller
        </button>
      </div>
    </footer>

    <ConfirmModal
      v-if="confirmOpen"
      title="Désinstaller définitivement"
      confirm-label="Désinstaller"
      danger
      @confirm="confirmUninstall"
      @cancel="confirmOpen = false"
    >
      <p>
        <strong>{{ selectedNames.join(", ") }}</strong>
        ({{ formatBytes(selectedAppsBytes) }}).
      </p>
      <p>
        Les apps et leurs fichiers associés sont <strong>supprimés définitivement</strong>, sans
        passer par la corbeille. macOS peut demander ton mot de passe pour les apps installées par
        un administrateur ou l'App Store.
      </p>
    </ConfirmModal>
  </div>
</template>

<style scoped>
.search {
  min-height: 40px;
  width: 220px;
  padding: 0 14px;
  border-radius: 10px;
  border: 1px solid rgba(255, 255, 255, 0.22);
  background: var(--surface-strong);
  color: var(--text);
  font-size: 14px;
  outline: none;
}
.search::placeholder {
  color: var(--label);
}
.search:focus {
  border-color: var(--accent);
}
.list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.app .row {
  padding: 12px 18px;
}
.tile {
  width: 40px;
  height: 40px;
  border-radius: 10px;
  color: var(--accent-ink);
  font-weight: 800;
  font-size: 18px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.text {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}
.name {
  font-weight: 600;
  font-size: 15px;
}
.desc {
  font-size: 13px;
  color: var(--muted);
}
.badge {
  display: inline-block;
  margin-left: 6px;
  padding: 1px 7px;
  font-size: 11px;
  font-weight: 700;
  border-radius: 6px;
  color: var(--accent-ink);
  background: var(--warning);
  vertical-align: 1px;
}
.right {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 2px;
}
.link {
  border: 0;
  padding: 0;
  background: none;
  font-size: 12px;
  color: var(--accent);
}
.leftovers {
  list-style: none;
  margin: 4px 0 0;
  padding: 10px 18px 10px 92px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 12px;
  background: var(--shade);
  border-radius: 12px;
  user-select: text;
}
.leftovers li {
  display: flex;
  justify-content: space-between;
  gap: 16px;
}
.lpath {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-soft);
}
</style>
