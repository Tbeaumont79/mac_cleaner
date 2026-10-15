<script setup lang="ts">
defineProps<{ title: string; confirmLabel: string; danger?: boolean }>();
defineEmits<{ (e: "confirm"): void; (e: "cancel"): void }>();
</script>

<template>
  <div class="overlay" @click.self="$emit('cancel')">
    <div class="modal" role="dialog" aria-modal="true">
      <h2>{{ title }}</h2>
      <div class="body"><slot /></div>
      <div class="actions">
        <button type="button" class="btn-ghost" @click="$emit('cancel')">Annuler</button>
        <button type="button" :class="danger ? 'btn-danger' : 'btn-accent'" @click="$emit('confirm')">
          {{ confirmLabel }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  background: rgba(10, 0, 30, 0.55);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10;
}
.modal {
  background: linear-gradient(160deg, #4b1888, #33106a);
  border: 1px solid rgba(255, 255, 255, 0.16);
  border-radius: 18px;
  padding: 28px;
  width: min(440px, calc(100% - 48px));
  box-shadow: 0 30px 80px -20px rgba(10, 0, 30, 0.9);
}
h2 {
  margin: 0 0 12px;
  font-size: 22px;
  font-weight: 700;
  letter-spacing: -0.01em;
}
.body {
  color: var(--muted);
  font-size: 15px;
  line-height: 1.5;
}
.body :deep(p) {
  margin: 0 0 10px;
}
.body :deep(strong) {
  color: var(--text);
}
.actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 22px;
}
</style>
