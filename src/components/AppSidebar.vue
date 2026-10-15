<script setup lang="ts">
import { computed } from "vue";
import { formatBytes } from "../lib/api";
import type { IconName } from "../lib/icons";
import {
  type View,
  disk,
  junkScanned,
  junkTotal,
  view,
} from "../lib/store";
import IconTile from "./IconTile.vue";

interface NavItem {
  id: View | null;
  icon: IconName;
  label: string;
  badge: string;
}

const modules = computed<NavItem[]>(() => [
  {
    id: "junk",
    icon: "junk",
    label: "Nettoyage",
    badge: junkScanned.value && junkTotal.value ? formatBytes(junkTotal.value) : "",
  },
  { id: null, icon: "prot", label: "Protection", badge: "Bientôt" },
  { id: null, icon: "speed", label: "Performance", badge: "Bientôt" },
  { id: "apps", icon: "apps", label: "Applications", badge: "" },
  {
    id: "files",
    icon: "files",
    label: "Fichiers volumineux",
    badge: "",
  },
]);

const diskStats = computed(() => {
  if (!disk.value || disk.value.total_bytes === 0) return null;
  const { total_bytes, available_bytes } = disk.value;
  return {
    pct: Math.round(((total_bytes - available_bytes) / total_bytes) * 100),
    free: formatBytes(available_bytes),
    total: formatBytes(total_bytes),
  };
});
</script>

<template>
  <nav class="sidebar" aria-label="Modules">
    <div class="traffic-space" data-tauri-drag-region></div>

    <button
      type="button"
      class="nav-item"
      :class="{ active: view === 'scan' }"
      @click="view = 'scan'"
    >
      <IconTile name="scan" />
      <span class="label">Analyse intelligente</span>
    </button>

    <div class="section">Modules</div>

    <button
      v-for="item in modules"
      :key="item.label"
      type="button"
      class="nav-item"
      :class="{ active: item.id !== null && view === item.id, soon: item.id === null }"
      :disabled="item.id === null"
      @click="item.id && (view = item.id)"
    >
      <IconTile :name="item.icon" />
      <span class="label">{{ item.label }}</span>
      <span class="badge">{{ item.badge }}</span>
    </button>

    <div v-if="diskStats" class="disk">
      <div class="disk-head">
        <span class="disk-name">Disque de démarrage</span>
        <span class="muted">{{ diskStats.pct }} %</span>
      </div>
      <div class="bar"><div class="bar-fill" :style="{ width: `${diskStats.pct}%` }"></div></div>
      <div class="disk-free">{{ diskStats.free }} disponibles sur {{ diskStats.total }}</div>
    </div>
  </nav>
</template>

<style scoped>
.sidebar {
  width: 236px;
  flex-shrink: 0;
  background: rgba(20, 4, 48, 0.28);
  display: flex;
  flex-direction: column;
  padding: 0 12px 16px;
  box-sizing: border-box;
  gap: 4px;
  border-right: 1px solid rgba(255, 255, 255, 0.08);
}
.traffic-space {
  height: 52px;
  flex-shrink: 0;
}
.nav-item {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
  min-height: 44px;
  padding: 8px 10px;
  border: 0;
  border-radius: 10px;
  font-size: 14px;
  font-weight: 500;
  background: transparent;
  color: var(--text-soft);
  text-align: left;
}
.nav-item:hover:not(:disabled) {
  background: rgba(255, 255, 255, 0.08);
}
.nav-item.active {
  font-weight: 600;
  background: rgba(255, 255, 255, 0.16);
  color: var(--text);
}
.nav-item.soon:disabled {
  opacity: 0.45;
  cursor: default;
}
.label {
  flex: 1;
  white-space: nowrap;
}
.badge {
  font-size: 12px;
  color: var(--muted);
}
.section {
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--label);
  padding: 18px 12px 8px;
}
.disk {
  margin-top: auto;
  background: rgba(255, 255, 255, 0.08);
  border: 1px solid rgba(255, 255, 255, 0.13);
  border-radius: 14px;
  padding: 14px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.disk-head {
  display: flex;
  justify-content: space-between;
  font-size: 13px;
}
.disk-name {
  font-weight: 600;
}
.bar {
  height: 6px;
  border-radius: 3px;
  background: rgba(255, 255, 255, 0.16);
  overflow: hidden;
}
.bar-fill {
  height: 100%;
  background: linear-gradient(90deg, var(--accent), #c59bff);
  border-radius: 3px;
}
.disk-free {
  font-size: 12px;
  color: var(--muted);
}
</style>
