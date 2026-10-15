<script setup lang="ts">
import { computed, onMounted } from "vue";
import { openSettings, revealInFinder } from "../lib/api";
import {
  checks,
  launchItemBusy,
  protError,
  protIssues,
  protScannedAt,
  protScanning,
  scanProtection,
  setLaunchItem,
  signatureInfo,
  signedItems,
  launchItemError,
  protUncertain,
} from "../lib/store";

const STATUS = {
  Ok: { label: "Actif", tone: "ok" },
  Warning: { label: "À corriger", tone: "bad" },
  Unknown: { label: "Inconnu", tone: "warn" },
} as const;

const lastScan = computed(() =>
  protScannedAt.value
    ? protScannedAt.value.toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" })
    : null,
);

const headline = computed(() => {
  if (protScanning.value && !protScannedAt.value) return "Analyse en cours…";
  if (!protScannedAt.value) return "Pas encore analysé";
  const n = protIssues.value;
  if (n) return `${n} point${n > 1 ? "s" : ""} à vérifier`;
  return protUncertain.value ? "Aucun problème détecté" : "Défenses de macOS actives";
});

const sortedItems = computed(() => {
  const rank = { bad: 0, warn: 1, info: 2 };
  return [...signedItems.value].sort(
    (a, b) => rank[signatureInfo(a.signature).tone] - rank[signatureInfo(b.signature).tone],
  );
});

onMounted(() => {
  if (!protScannedAt.value) scanProtection();
});
</script>

<template>
  <div class="view">
    <header class="view-header">
      <div>
        <h1>Protection</h1>
        <p>Vérifie que les défenses de macOS sont actives et ce qui se lance tout seul.</p>
      </div>
    </header>

    <div class="view-body">
      <p v-if="protError" class="error">{{ protError }}</p>
      <p v-if="launchItemError" class="error">{{ launchItemError }}</p>

      <div class="hero">
        <div class="shield" :class="{ alert: protIssues > 0 && protScannedAt }">
          <svg width="46" height="46" viewBox="0 0 24 24" fill="none" stroke="#FFFFFF" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M12 3l8 3v6c0 5-3.5 8-8 9-4.5-1-8-4-8-9V6l8-3z" />
            <path v-if="protIssues === 0 || !protScannedAt" d="M8.5 12l2.5 2.5 4.5-5" />
            <path v-else d="M12 8v5M12 16.5v.01" />
          </svg>
        </div>
        <div class="hero-text">
          <span class="headline">{{ headline }}</span>
          <span v-if="lastScan" class="muted">Dernière analyse : aujourd'hui, {{ lastScan }}</span>
          <span class="muted small">Sweep vérifie la configuration de macOS, il ne remplace pas un antivirus.</span>
        </div>
        <button type="button" class="btn-ghost" :disabled="protScanning" @click="scanProtection">
          {{ protScanning ? "Analyse…" : "Analyser maintenant" }}
        </button>
      </div>

      <div class="section-title">Défenses du système</div>
      <div class="list">
        <div v-for="c in checks" :key="c.id" class="row static">
          <div class="text">
            <span class="name">{{ c.label }}</span>
            <span class="desc">{{ c.detail }}</span>
          </div>
          <button
            v-if="c.status !== 'Ok' && c.settings_url"
            type="button"
            class="link"
            @click="openSettings(c.settings_url)"
          >
            Ouvrir les réglages
          </button>
          <span class="pill" :class="STATUS[c.status].tone">{{ STATUS[c.status].label }}</span>
        </div>
      </div>

      <div class="section-title">Éléments lancés automatiquement</div>
      <p v-if="protScannedAt && signedItems.length === 0" class="message">
        Aucun élément tiers ne se lance automatiquement.
      </p>
      <div class="list">
        <div v-for="item in sortedItems" :key="item.plist_path" class="row static">
          <div class="text">
            <span class="name">
              {{ item.label }}
              <span v-if="!item.enabled" class="tag">Désactivé</span>
            </span>
            <span class="desc path" :title="item.program ?? ''">{{ item.program ?? "Programme non indiqué" }}</span>
          </div>
          <button type="button" class="link" @click="revealInFinder(item.plist_path)">Afficher</button>
          <button
            v-if="item.editable && item.enabled && signatureInfo(item.signature).tone !== 'info'"
            type="button"
            class="link"
            :disabled="launchItemBusy !== null"
            @click="setLaunchItem(item, false)"
          >
            Désactiver
          </button>
          <span class="pill" :class="signatureInfo(item.signature).tone" :title="signatureInfo(item.signature).label">
            {{ signatureInfo(item.signature).label }}
          </span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.hero {
  background: var(--surface);
  border: 1px solid var(--surface-strong);
  border-radius: 18px;
  padding: 28px;
  display: flex;
  align-items: center;
  gap: 28px;
}
.shield {
  width: 96px;
  height: 96px;
  border-radius: 50%;
  background: linear-gradient(135deg, #6fa8ff, #3e5bd9);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  box-shadow: 0 0 50px -10px #6fa8ff;
}
.shield.alert {
  background: linear-gradient(135deg, #ffc25c, #ee7a2e);
  box-shadow: 0 0 50px -10px #ffc25c;
}
.hero-text {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 6px;
  font-size: 14px;
}
.headline {
  font-size: 22px;
  font-weight: 700;
}
.list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.row.static {
  cursor: default;
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
.path {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.tag {
  display: inline-block;
  margin-left: 6px;
  padding: 1px 7px;
  font-size: 11px;
  font-weight: 700;
  border-radius: 6px;
  background: var(--surface-strong);
  vertical-align: 1px;
}
.link {
  border: 0;
  padding: 0;
  background: none;
  font-size: 13px;
  font-weight: 600;
  color: var(--accent);
  white-space: nowrap;
}
.pill {
  max-width: 220px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  padding: 4px 10px;
  border-radius: 999px;
  font-size: 12px;
  font-weight: 700;
  flex-shrink: 0;
}
.small {
  font-size: 12px;
}
.pill.info {
  color: var(--text-soft);
  background: var(--surface-strong);
}
.pill.ok {
  color: var(--success);
  background: rgba(111, 240, 207, 0.12);
}
.pill.warn {
  color: var(--warning);
  background: rgba(255, 194, 92, 0.12);
}
.pill.bad {
  color: var(--danger);
  background: rgba(255, 107, 139, 0.14);
}
</style>
