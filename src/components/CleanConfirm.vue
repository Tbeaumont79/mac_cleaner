<script setup lang="ts">
import { computed } from "vue";
import { formatBytes } from "../lib/api";
import { selectedJunk, selectedJunkBytes } from "../lib/store";
import ConfirmModal from "./ConfirmModal.vue";

defineEmits<{ (e: "confirm"): void; (e: "cancel"): void }>();

const permanent = computed(() => selectedJunk.value.filter((c) => c.permanent));
</script>

<template>
  <ConfirmModal
    :title="permanent.length ? 'Suppression définitive' : 'Confirmer le nettoyage'"
    :confirm-label="permanent.length ? 'Supprimer définitivement' : 'Nettoyer'"
    :danger="permanent.length > 0"
    @confirm="$emit('confirm')"
    @cancel="$emit('cancel')"
  >
    <p>Total à nettoyer : <strong>{{ formatBytes(selectedJunkBytes) }}</strong>.</p>
    <p v-if="permanent.length" class="warn">
      {{ permanent.map((c) => c.label).join(", ") }} sera supprimé
      <strong>pour de bon</strong>, sans passer par la corbeille. Le reste part à la corbeille.
    </p>
    <p v-else>
      Tout part à la corbeille : tu pourras le récupérer si besoin. L'espace disque
      sera libéré une fois la corbeille vidée.
    </p>
  </ConfirmModal>
</template>

<style scoped>
.warn {
  color: var(--danger);
}
</style>
