<script setup lang="ts">
import { withBase } from "vitepress";

/**
 * A linked example card with a real capture or a code illustration, a title,
 * a one-line explanation, and a category label.
 */
withDefaults(
  defineProps<{
    href: string;
    src: string;
    alt: string;
    title: string;
    description: string;
    meta?: string;
  }>(),
  {
    meta: "",
  },
);
</script>

<template>
  <a class="cl-example-card" :href="withBase(href)">
    <img :src="withBase(src)" :alt="alt" loading="lazy" decoding="async" />
    <span class="cl-example-card__body">
      <strong>{{ title }}</strong>
      <span>{{ description }}</span>
      <span v-if="meta" class="cl-example-card__meta">{{ meta }}</span>
    </span>
  </a>
</template>
