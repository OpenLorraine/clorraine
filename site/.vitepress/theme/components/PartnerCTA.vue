<script setup lang="ts">
import { withBase } from "vitepress";
</script>

<template>
  <aside class="cl-partner">
    <div class="cl-partner__copy">
      <p class="cl-partner__kicker">For AmigaOS partners</p>
      <h2>Evaluating Clorraine for the Amiga platform?</h2>
      <p>
        The one-page brief covers status, licensing, provenance, compatibility,
        media and NDK handling, the roadmap, and contact details. It is written
        to be forwarded without a call.
      </p>
    </div>
    <a class="cl-btn cl-btn--primary" :href="withBase('/project/brief')">
      Read the project brief
      <span aria-hidden="true">→</span>
    </a>
  </aside>
</template>
