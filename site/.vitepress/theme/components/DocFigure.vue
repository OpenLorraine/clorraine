<script setup lang="ts">
import { withBase } from "vitepress";

/**
 * A figure for a real capture or a documentation image. The source is a
 * public path such as `/screenshots/a500-boing.webp`; `withBase` adds the
 * deployment base so the image also works under `/clorraine/`.
 */
withDefaults(
  defineProps<{
    src: string;
    alt: string;
    caption?: string;
    line?: boolean;
  }>(),
  {
    caption: "",
    line: false,
  },
);
</script>

<template>
  <figure class="cl-doc-figure" :class="{ 'cl-doc-figure--line': line }">
    <img :src="withBase(src)" :alt="alt" loading="lazy" decoding="async" />
    <figcaption v-if="caption">{{ caption }}</figcaption>
  </figure>
</template>
