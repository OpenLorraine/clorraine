<script setup lang="ts">
import { withBase } from "vitepress";

/**
 * One real acceptance capture with its machine/profile caption. The image is
 * lazy-loaded because every gallery sits below the fold.
 */
withDefaults(
  defineProps<{
    src: string;
    alt: string;
    caption: string;
    tag?: string;
  }>(),
  {
    tag: "",
  },
);
</script>

<template>
  <figure class="cl-shot">
    <div class="cl-shot__frame">
      <img :src="withBase(src)" :alt="alt" loading="lazy" decoding="async" />
    </div>
    <figcaption class="cl-shot__caption">
      <span>{{ caption }}</span>
      <span v-if="tag" class="cl-shot__tag">{{ tag }}</span>
    </figcaption>
  </figure>
</template>
