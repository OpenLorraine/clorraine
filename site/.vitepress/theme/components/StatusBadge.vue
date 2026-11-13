<script setup lang="ts">
/**
 * A compact state pill. Colour is always paired with a text label so the
 * meaning survives for readers who cannot distinguish the colours.
 */
withDefaults(
  defineProps<{
    state?: "preview" | "ready" | "progress" | "planned";
    label?: string;
  }>(),
  {
    state: "preview",
    label: "",
  },
);

const fallback: Record<string, string> = {
  preview: "Developer preview",
  ready: "Ready",
  progress: "In progress",
  planned: "Planned",
};
</script>

<template>
  <span class="cl-badge" :class="`cl-badge--${state}`">
    <span class="cl-badge__dot" aria-hidden="true"></span>
    <span>{{ label || fallback[state] }}</span>
  </span>
</template>
