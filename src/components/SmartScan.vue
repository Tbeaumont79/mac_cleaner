<script setup lang="ts">
import { computed, ref } from "vue";
import { formatBytes } from "../lib/api";
import { ICONS, type IconName } from "../lib/icons";
import {
  type View,
  describeClean,
  largeFiles,
  largeFilesTotal,
  filesError,
  junkError,
  runSmartClean,
  runSmartScan,
  scannedMinMb,
  selectedJunkBytes,
  smartPhase,
  smartStep,
  smartSummary,
  trashCategory,
  view,
} from "../lib/store";
import CleanConfirm from "./CleanConfirm.vue";
import IconTile from "./IconTile.vue";

const confirmOpen = ref(false);

const orbIcon = computed(() =>
  smartPhase.value === "scanning" ? ICONS[smartStep.value] : ICONS.scan,
);

const cleanedBytes = computed(() =>
  smartSummary.value ? smartSummary.value.trashedBytes + smartSummary.value.deletedBytes : 0,
);

const title = computed(
  () =>
    ({
      idle: "Prêt pour un grand ménage ?",
      scanning: "Analyse en cours",
      results: "Analyse terminée",
      cleaning: "Nettoyage en cours",
      cleaned: cleanedWell.value ? "Ton Mac est tout propre" : "Nettoyage terminé",
    })[smartPhase.value],
);

const cleanedWell = computed(
  () => !!smartSummary.value && cleanedBytes.value > 0 && smartSummary.value.errors.length === 0,
);

const errorMessage = computed(() => junkError.value ?? filesError.value);

const subtitle = computed(() => {
  switch (smartPhase.value) {
    case "idle":
      return "Une seule analyse pour les déchets système et les fichiers volumineux de ton Mac.";
    case "scanning":
      return smartStep.value === "junk"
        ? "Recherche des fichiers inutiles…"
        : "Recherche des fichiers volumineux…";
    case "results":
      return selectedJunkBytes.value
        ? "Vérifie le détail de chaque module ou nettoie la sélection en un clic."
        : "Rien à nettoyer dans la sélection actuelle.";
    case "cleaning":
      return "Déplacement des fichiers vers la corbeille…";
    case "cleaned": {
      const s = smartSummary.value;
      if (!s) return "";
      const hint = s.trashedBytes > 0 ? ". Vide la corbeille pour récupérer l'espace disque." : ".";
      return describeClean(s) + hint;
    }
  }
});

const mainButton = computed(() => {
  switch (smartPhase.value) {
    case "idle":
      return { label: "Analyser", run: runSmartScan };
    case "results":
      return selectedJunkBytes.value
        ? { label: "Nettoyer", run: () => (confirmOpen.value = true) }
        : { label: "Nouvelle analyse", run: runSmartScan };
    case "cleaned":
      return { label: "Nouvelle analyse", run: runSmartScan };
    default:
      return null;
  }
});

function moduleStatus(step: "junk" | "files", result: string): string {
  switch (smartPhase.value) {
    case "idle":
      return "Prêt";
    case "scanning":
      if (step === smartStep.value) return "Analyse en cours…";
      return step === "junk" ? "Terminé" : "En attente";
    case "cleaning":
      return step === "junk" ? "En cours…" : result;
    default:
      return result;
  }
}

interface ModuleCard {
  icon: IconName;
  name: string;
  status: string;
  target: View;
}

const modules = computed<ModuleCard[]>(() => {
  const trash = trashCategory.value?.size_bytes ?? 0;
  const files = largeFiles.value.length;
  return [
    {
      icon: "junk",
      name: "Nettoyage",
      status: moduleStatus("junk", `${formatBytes(selectedJunkBytes.value)} à nettoyer`),
      target: "junk",
    },
    {
      icon: "files",
      name: "Fichiers volumineux",
      status: moduleStatus(
        "files",
        files
          ? `${files} fichier${files > 1 ? "s" : ""} · ${formatBytes(largeFilesTotal.value)}`
          : `Aucun au-delà de ${formatBytes(scannedMinMb.value * 1_000_000)}`,
      ),
      target: "files",
    },
    {
      icon: "trash",
      name: "Corbeille",
      status: moduleStatus("junk", trash ? `${formatBytes(trash)} à vider` : "Vide"),
      target: "junk",
    },
  ];
});

async function confirmClean() {
  confirmOpen.value = false;
  await runSmartClean();
}
</script>

<template>
  <div class="smart">
    <div class="orb" :class="smartPhase">
      <div class="ring"></div>
      <div class="core">
        <template v-if="smartPhase === 'results' || smartPhase === 'cleaned'">
          <span class="value">{{ formatBytes(smartPhase === "cleaned" ? cleanedBytes : selectedJunkBytes) }}</span>
          <span class="caption">{{ smartPhase === "cleaned" ? "nettoyés" : "à nettoyer" }}</span>
        </template>
        <svg
          v-else
          width="76"
          height="76"
          viewBox="0 0 24 24"
          fill="none"
          stroke="var(--accent)"
          stroke-width="1.3"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <path :d="orbIcon" />
        </svg>
      </div>
    </div>

    <h1>{{ title }}</h1>
    <p class="sub">{{ subtitle }}</p>
    <p v-if="errorMessage" class="error">{{ errorMessage }}</p>

    <button v-if="mainButton" type="button" class="main-btn" @click="mainButton.run">
      {{ mainButton.label }}
    </button>

    <div class="modules">
      <button
        v-for="m in modules"
        :key="m.name"
        type="button"
        class="module"
        @click="view = m.target"
      >
        <IconTile :name="m.icon" size="md" />
        <div class="module-text">
          <span class="module-name">{{ m.name }}</span>
          <span class="module-status">{{ m.status }}</span>
        </div>
      </button>
    </div>

    <CleanConfirm v-if="confirmOpen" @confirm="confirmClean" @cancel="confirmOpen = false" />
  </div>
</template>

<style scoped>
.smart {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 20px 40px 32px;
  box-sizing: border-box;
  min-height: 0;
  overflow: auto;
}
.orb {
  position: relative;
  width: 236px;
  height: 236px;
  flex-shrink: 0;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow:
    0 0 70px -6px var(--accent),
    0 0 0 10px rgba(92, 225, 255, 0.06);
}
.ring {
  position: absolute;
  inset: 0;
  border-radius: 50%;
  background: var(--accent);
}
.orb.idle .ring {
  background: rgba(92, 225, 255, 0.55);
}
.orb.scanning .ring,
.orb.cleaning .ring {
  background: conic-gradient(rgba(255, 255, 255, 0.12) 0deg, var(--accent) 140deg, rgba(255, 255, 255, 0.12) 140deg);
  animation: spin 1.4s linear infinite;
}
@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
.core {
  position: relative;
  width: 212px;
  height: 212px;
  border-radius: 50%;
  background: radial-gradient(circle at 50% 35%, #6a2db0 0%, #3a1474 70%, #2a0d58 100%);
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 6px;
  box-shadow: inset 0 0 40px rgba(92, 225, 255, 0.25);
}
.value {
  font-size: 40px;
  font-weight: 700;
  letter-spacing: -0.03em;
}
.caption {
  font-size: 13px;
  color: var(--muted);
}
h1 {
  font-size: 30px;
  font-weight: 700;
  letter-spacing: -0.02em;
  margin: 32px 0 8px;
  text-align: center;
}
.sub {
  margin: 0;
  font-size: 15px;
  color: var(--muted);
  text-align: center;
  max-width: 470px;
  line-height: 1.5;
  min-height: 22px;
}
.error {
  margin: 12px 0 0;
  text-align: center;
  max-width: 470px;
}
.main-btn {
  margin-top: 26px;
  min-width: 160px;
  min-height: 50px;
  padding: 0 32px;
  border-radius: 25px;
  font-size: 16px;
  font-weight: 700;
  background: var(--accent);
  color: var(--accent-ink);
  border: 0;
  box-shadow: 0 0 40px -6px var(--accent);
}
.modules {
  margin-top: auto;
  padding-top: 28px;
  width: 100%;
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 14px;
}
.module {
  background: var(--surface);
  border: 1px solid var(--surface-strong);
  border-radius: 16px;
  padding: 18px;
  display: flex;
  align-items: center;
  gap: 14px;
  color: var(--text);
  text-align: left;
  backdrop-filter: blur(20px);
  box-shadow: 0 10px 30px -18px rgba(10, 0, 30, 0.8);
}
.module:hover {
  background: rgba(255, 255, 255, 0.12);
}
.module-text {
  display: flex;
  flex-direction: column;
  gap: 3px;
  min-width: 0;
}
.module-name {
  font-weight: 600;
  font-size: 15px;
}
.module-status {
  font-size: 13px;
  color: var(--muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
