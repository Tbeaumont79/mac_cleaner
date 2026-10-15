<script setup lang="ts">
import { onMounted } from "vue";
import { refreshDisk, view } from "./lib/store";
import AppSidebar from "./components/AppSidebar.vue";
import JunkView from "./components/JunkView.vue";
import AppsView from "./components/AppsView.vue";
import PerformanceView from "./components/PerformanceView.vue";
import LargeFilesView from "./components/LargeFilesView.vue";
import SmartScan from "./components/SmartScan.vue";

const TITLES = {
  scan: "Analyse intelligente",
  junk: "Nettoyage",
  speed: "Performance",
  files: "Fichiers volumineux",
  apps: "Applications",
};

onMounted(refreshDisk);
</script>

<template>
  <div class="window">
    <AppSidebar />
    <main class="main">
      <div class="titlebar" data-tauri-drag-region>{{ TITLES[view] }}</div>
      <SmartScan v-if="view === 'scan'" />
      <JunkView v-else-if="view === 'junk'" />
      <PerformanceView v-else-if="view === 'speed'" />
      <LargeFilesView v-else-if="view === 'files'" />
      <AppsView v-else />
    </main>
  </div>
</template>

<style scoped>
.window {
  height: 100%;
  display: flex;
}
.main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}
.titlebar {
  height: 40px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-soft);
}
</style>
