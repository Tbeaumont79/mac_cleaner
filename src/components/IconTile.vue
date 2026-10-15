<script setup lang="ts">
import { computed } from "vue";
import { GRADIENTS, ICONS, type IconName } from "../lib/icons";

const props = withDefaults(defineProps<{ name: IconName; size?: "sm" | "md" }>(), {
  size: "sm",
});

const dims = computed(() =>
  props.size === "md" ? { tile: 46, radius: 13, icon: 22, stroke: 1.9 } : { tile: 28, radius: 8, icon: 16, stroke: 2 },
);
</script>

<template>
  <span
    class="tile"
    :style="{
      width: `${dims.tile}px`,
      height: `${dims.tile}px`,
      borderRadius: `${dims.radius}px`,
      background: GRADIENTS[name],
    }"
  >
    <svg
      :width="dims.icon"
      :height="dims.icon"
      viewBox="0 0 24 24"
      fill="none"
      stroke="#FFFFFF"
      :stroke-width="dims.stroke"
      stroke-linecap="round"
      stroke-linejoin="round"
      aria-hidden="true"
    >
      <path :d="ICONS[name]" />
    </svg>
  </span>
</template>

<style scoped>
.tile {
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  box-shadow: 0 4px 10px -4px rgba(0, 0, 0, 0.5);
}
</style>
