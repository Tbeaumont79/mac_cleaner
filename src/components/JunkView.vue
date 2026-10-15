<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { type CategoryId, formatBytes, openFullDiskAccessSettings } from "../lib/api";
import {
  type CleanSummary,
  categories,
  cleanSelected,
  describeClean,
  junkCleaning,
  junkError,
  junkScanned,
  junkScanning,
  junkTotal,
  scanJunk,
  selectedCategories,
  selectedJunkBytes,
  toggleCategory,
} from "../lib/store";
import CleanConfirm from "./CleanConfirm.vue";

const confirmOpen = ref(false);
const summary = ref<CleanSummary | null>(null);
const denied = computed(() => categories.value.filter((c) => c.access_denied));
const busy = computed(() => junkScanning.value || junkCleaning.value);

const footer = computed(() => {
  if (junkCleaning.value) return "Nettoyage en cours…";
  if (junkScanning.value) return "Analyse en cours…";
  if (summary.value) return describeClean(summary.value);
  return selectedJunkBytes.value
    ? `${formatBytes(selectedJunkBytes.value)} sélectionnés sur ${formatBytes(junkTotal.value)}`
    : "Aucun élément sélectionné";
});

function toggle(id: CategoryId) {
  summary.value = null;
  toggleCategory(id);
}

async function confirmClean() {
  confirmOpen.value = false;
  summary.value = await cleanSelected();
}

onMounted(() => {
  if (!junkScanned.value && !junkScanning.value) scanJunk();
});
</script>

<template>
  <div class="view">
    <header class="view-header">
      <div>
        <h1>Déchets système</h1>
        <p>Caches, journaux et fichiers temporaires que macOS n'a plus besoin de garder.</p>
      </div>
      <span class="total">{{ junkScanned ? formatBytes(junkTotal) : "…" }}</span>
    </header>

    <div class="view-body list">
      <p v-if="junkError" class="error">{{ junkError }}</p>

      <div v-if="denied.length" class="notice">
        <div>
          <strong>Accès limité par macOS</strong>
          <span>
            {{ denied.map((c) => c.label).join(", ") }} ne peu{{ denied.length > 1 ? "vent" : "t" }} pas
            être analysé{{ denied.length > 1 ? "s" : "" }} entièrement. Autorise Sweep dans
            « Accès complet au disque », puis relance l'app.
          </span>
        </div>
        <button type="button" class="btn-ghost" @click="openFullDiskAccessSettings">Ouvrir les réglages</button>
      </div>

      <label
        v-for="c in categories"
        :key="c.id"
        class="row"
        :class="{ disabled: c.size_bytes === 0 }"
      >
        <input
          type="checkbox"
          :checked="selectedCategories.has(c.id)"
          :disabled="busy || c.size_bytes === 0"
          @change="toggle(c.id)"
        />
        <div class="text">
          <span class="name">
            {{ c.label }}
            <span v-if="c.permanent" class="badge">Définitif</span>
            <span v-if="c.access_denied" class="badge warn">Accès refusé</span>
          </span>
          <span class="desc">{{ c.description }}</span>
        </div>
        <div class="right">
          <span class="size">{{ formatBytes(c.size_bytes) }}</span>
          <span v-if="c.file_count" class="count">{{ c.file_count.toLocaleString("fr-FR") }} fichiers</span>
        </div>
      </label>

      <p v-if="junkScanned && categories.every((c) => c.size_bytes === 0)" class="message">
        Rien à nettoyer pour le moment.
      </p>
      <p v-else-if="!junkScanned && junkScanning" class="message">Analyse en cours…</p>

      <details v-if="summary?.errors.length" class="errors">
        <summary>{{ summary.errors.length }} élément(s) non déplacé(s)</summary>
        <ul>
          <li v-for="(msg, i) in summary.errors.slice(0, 30)" :key="i">{{ msg }}</li>
        </ul>
      </details>
    </div>

    <footer class="view-footer">
      <span class="info">{{ footer }}</span>
      <div class="actions">
        <button type="button" class="btn-ghost" :disabled="busy" @click="summary = null; scanJunk()">
          Analyser à nouveau
        </button>
        <button
          type="button"
          class="btn-accent"
          :disabled="busy || selectedCategories.size === 0"
          @click="confirmOpen = true"
        >
          Nettoyer {{ selectedJunkBytes ? formatBytes(selectedJunkBytes) : "" }}
        </button>
      </div>
    </footer>

    <CleanConfirm v-if="confirmOpen" @confirm="confirmClean" @cancel="confirmOpen = false" />
  </div>
</template>

<style scoped>
.total {
  font-size: 32px;
  font-weight: 700;
  color: var(--accent);
  white-space: nowrap;
}
.list {
  display: flex;
  flex-direction: column;
  gap: 8px;
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
  background: var(--danger);
  vertical-align: 1px;
}
.badge.warn {
  background: var(--warning);
}
.notice {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 14px 18px;
  border-radius: 14px;
  background: rgba(255, 194, 92, 0.12);
  border: 1px solid rgba(255, 194, 92, 0.35);
  font-size: 13px;
  color: var(--muted);
}
.notice > div {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.notice strong {
  color: var(--warning);
  font-size: 14px;
}
.right {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 2px;
}
.count {
  font-size: 12px;
  color: var(--muted);
}
.errors {
  font-size: 13px;
  color: var(--muted);
  background: var(--surface);
  border: 1px solid var(--surface-border);
  border-radius: 14px;
  padding: 12px 18px;
}
.errors summary {
  cursor: pointer;
  font-weight: 600;
}
.errors ul {
  margin: 8px 0 0;
  padding-left: 18px;
  max-height: 160px;
  overflow: auto;
  user-select: text;
}
</style>
