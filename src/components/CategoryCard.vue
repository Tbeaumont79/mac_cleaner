<script setup lang="ts">
import { type CategoryScan, formatBytes } from "../lib/api";

defineProps<{
  scan: CategoryScan;
  selected: boolean;
  disabled: boolean;
}>();

defineEmits<{ (e: "toggle"): void }>();
</script>

<template>
  <label class="card" :class="{ selected, empty: scan.size_bytes === 0 }">
    <input
      type="checkbox"
      :checked="selected"
      :disabled="disabled || scan.size_bytes === 0"
      @change="$emit('toggle')"
    />
    <div class="info">
      <div class="label">
        {{ scan.label }}
        <span v-if="scan.permanent" class="badge">définitif</span>
      </div>
      <div class="desc">{{ scan.description }}</div>
    </div>
    <div class="size">
      <div class="bytes">{{ formatBytes(scan.size_bytes) }}</div>
      <div class="count" v-if="scan.size_bytes > 0">
        {{ scan.file_count.toLocaleString("fr-FR") }} fichiers
      </div>
      <div class="count" v-else>—</div>
    </div>
  </label>
</template>

<style scoped>
.card {
  display: flex;
  align-items: center;
  gap: 0.85rem;
  padding: 0.85rem 1rem;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--card-bg);
  cursor: pointer;
  transition: border-color 0.15s, background 0.15s;
}
.card:hover {
  border-color: var(--accent);
}
.card.selected {
  border-color: var(--accent);
  background: var(--card-selected);
}
.card.empty {
  opacity: 0.55;
  cursor: default;
}
input[type="checkbox"] {
  width: 18px;
  height: 18px;
  accent-color: var(--accent);
  cursor: inherit;
}
.info {
  flex: 1;
  min-width: 0;
}
.label {
  font-weight: 600;
}
.badge {
  display: inline-block;
  margin-left: 0.4rem;
  padding: 0.05rem 0.4rem;
  font-size: 0.65rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  color: #fff;
  background: #d33;
  border-radius: 6px;
  vertical-align: middle;
}
.desc {
  font-size: 0.8rem;
  color: var(--muted);
  margin-top: 0.15rem;
}
.size {
  text-align: right;
  white-space: nowrap;
}
.bytes {
  font-variant-numeric: tabular-nums;
  font-weight: 600;
}
.count {
  font-size: 0.75rem;
  color: var(--muted);
}
</style>
