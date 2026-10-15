<script setup lang="ts">
import { computed } from "vue";
import { formatBytes } from "../lib/api";
import { selectedJunk, selectedJunkBytes, selectedTasks, tasks } from "../lib/store";
import ConfirmModal from "./ConfirmModal.vue";

const props = defineProps<{ withTasks?: boolean }>();
defineEmits<{ (e: "confirm"): void; (e: "cancel"): void }>();

const permanent = computed(() => selectedJunk.value.filter((c) => c.permanent));
const chosenTasks = computed(() =>
  props.withTasks ? tasks.value.filter((t) => selectedTasks.value.has(t.id)) : [],
);
const needsPassword = computed(() => chosenTasks.value.some((t) => t.admin));
</script>

<template>
  <ConfirmModal
    :title="permanent.length ? 'Suppression définitive' : chosenTasks.length ? 'Confirmer' : 'Confirmer le nettoyage'"
    :confirm-label="permanent.length ? 'Supprimer définitivement' : chosenTasks.length ? 'Lancer' : 'Nettoyer'"
    :danger="permanent.length > 0"
    @confirm="$emit('confirm')"
    @cancel="$emit('cancel')"
  >
    <template v-if="selectedJunk.length">
      <p>Total à nettoyer : <strong>{{ formatBytes(selectedJunkBytes) }}</strong>.</p>
      <p v-if="permanent.length" class="warn">
        {{ permanent.map((c) => c.label).join(", ") }} sera supprimé
        <strong>pour de bon</strong>, sans passer par la corbeille. Le reste part à la corbeille.
      </p>
      <p v-else>
        Tout part à la corbeille : tu pourras le récupérer si besoin. L'espace disque
        sera libéré une fois la corbeille vidée.
      </p>
    </template>
    <p v-if="chosenTasks.length">
      Tâches : <strong>{{ chosenTasks.map((t) => t.label).join(", ") }}</strong>.
      <template v-if="needsPassword"> macOS demandera ton mot de passe administrateur.</template>
    </p>
  </ConfirmModal>
</template>

<style scoped>
.warn {
  color: var(--danger);
}
</style>
