<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import type { LaunchItem, TaskId } from "../lib/api";
import {
  launchItemBusy,
  launchItems,
  launchItemsLoaded,
  loadSpeed,
  runSelectedTasks,
  selectedTasks,
  setLaunchItem,
  speedError,
  taskResults,
  tasks,
  tasksRunning,
  toggleTask,
} from "../lib/store";

const toast = ref<string | null>(null);

const needsPassword = computed(() =>
  tasks.value.some((t) => t.admin && selectedTasks.value.has(t.id)),
);

const enabledCount = computed(() => launchItems.value.filter((i) => i.enabled).length);

const footer = computed(() => {
  if (tasksRunning.value) return "Exécution en cours…";
  if (toast.value) return toast.value;
  const n = selectedTasks.value.size;
  if (!n) return "Aucune tâche sélectionnée";
  const base = `${n} tâche${n > 1 ? "s" : ""} à exécuter`;
  return needsPassword.value ? `${base} · mot de passe administrateur requis` : base;
});

function state(id: TaskId): { label: string; tone: string } | null {
  const r = taskResults.value.get(id);
  if (!r) return null;
  return r.ok ? { label: "Exécutée", tone: "ok" } : { label: r.error || "Échec", tone: "bad" };
}

async function toggleItem(item: LaunchItem, event: Event) {
  const el = event.target as HTMLInputElement;
  // Vue ne réécrit pas `checked` si l'état n'a pas changé : on le remet à la main en cas d'échec.
  if (!(await setLaunchItem(item, el.checked))) el.checked = item.enabled;
}

function lockReason(item: LaunchItem): string | undefined {
  if (item.scope !== "User") return "Géré par l'administrateur";
  if (!item.editable) return "Non modifiable ici (lien symbolique ou nom inhabituel)";
  return undefined;
}

function toggle(id: TaskId) {
  toast.value = null;
  toggleTask(id);
}

async function run() {
  toast.value = null;
  const results = await runSelectedTasks();
  if (!results.length) return;
  const ok = results.filter((r) => r.ok).length;
  toast.value =
    ok === results.length
      ? `${ok} tâche${ok > 1 ? "s" : ""} exécutée${ok > 1 ? "s" : ""}`
      : `${ok} sur ${results.length} tâches exécutées`;
}

function scopeLabel(item: LaunchItem): string | null {
  if (item.scope === "User") return null;
  return item.scope === "SystemDaemon" ? "Système" : "Tous les utilisateurs";
}

onMounted(() => {
  if (!launchItemsLoaded.value) loadSpeed();
});
</script>

<template>
  <div class="view">
    <header class="view-header">
      <div>
        <h1>Performance</h1>
        <p>Tâches de maintenance pour garder un Mac réactif.</p>
      </div>
    </header>

    <div class="view-body">
      <p v-if="speedError" class="error">{{ speedError }}</p>

      <div class="grid">
        <label v-for="t in tasks" :key="t.id" class="task">
          <input
            type="checkbox"
            :checked="selectedTasks.has(t.id)"
            :disabled="tasksRunning"
            @change="toggle(t.id)"
          />
          <div class="text">
            <span class="name">
              {{ t.label }}
              <span v-if="t.admin" class="tag" title="Demande le mot de passe administrateur">Admin</span>
            </span>
            <span class="desc">{{ t.description }}</span>
            <span v-if="state(t.id)" class="state" :class="state(t.id)!.tone">{{ state(t.id)!.label }}</span>
          </div>
        </label>
      </div>

      <div class="section-title">
        Éléments d'ouverture
        <span v-if="launchItemsLoaded" class="count">· {{ enabledCount }} actif{{ enabledCount > 1 ? "s" : "" }}</span>
      </div>
      <p v-if="launchItemsLoaded && launchItems.length === 0" class="message">
        Aucun élément tiers ne se lance automatiquement.
      </p>
      <div class="items">
        <label v-for="item in launchItems" :key="item.plist_path" class="row item" :title="item.plist_path">
          <div class="text">
            <span class="name">
              {{ item.label }}
              <span v-if="scopeLabel(item)" class="tag">{{ scopeLabel(item) }}</span>
            </span>
            <span class="desc path">{{ item.program ?? "Programme non indiqué" }}</span>
          </div>
          <input
            class="switch"
            type="checkbox"
            role="switch"
            :checked="item.enabled"
            :disabled="!item.editable || launchItemBusy !== null"
            :title="lockReason(item)"
            @change="toggleItem(item, $event)"
          />
        </label>
      </div>
    </div>

    <footer class="view-footer">
      <span class="info">{{ footer }}</span>
      <button
        type="button"
        class="btn-accent"
        :disabled="tasksRunning || selectedTasks.size === 0"
        @click="run"
      >
        Exécuter
      </button>
    </footer>
  </div>
</template>

<style scoped>
.grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 12px;
}
.task {
  display: flex;
  align-items: flex-start;
  gap: 14px;
  background: var(--surface);
  border: 1px solid var(--surface-border);
  border-radius: 14px;
  padding: 18px;
  cursor: pointer;
}
.task input {
  margin-top: 2px;
}
.text {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}
.name {
  font-weight: 600;
  font-size: 15px;
}
.desc {
  font-size: 13px;
  color: var(--muted);
  line-height: 1.45;
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
  color: var(--text);
  background: var(--surface-strong);
  vertical-align: 1px;
}
.state {
  margin-top: 6px;
  font-size: 12px;
  font-weight: 700;
}
.state.warn {
  color: var(--warning);
}
.state.ok {
  color: var(--success);
}
.state.bad {
  color: var(--danger);
}
.count {
  text-transform: none;
  letter-spacing: 0;
}
.items {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.item {
  cursor: default;
}
</style>
