<script setup lang="ts">
import DefaultTheme from "vitepress/theme-without-fonts";
import { withBase } from "vitepress";

const { Layout } = DefaultTheme;
</script>

<template>
  <Layout>
    <template #layout-top>
      <div class="cl-preview-bar">
        <div class="cl-preview-bar__inner">
          <span class="cl-preview-bar__label">Developer preview</span>
          <span class="cl-preview-bar__text">
            v1.1.2 — installation, packaging, and some platform support are
            still in progress.
          </span>
          <a class="cl-preview-bar__link" :href="withBase('/status')">
            Read the status
            <span aria-hidden="true">→</span>
          </a>
        </div>
      </div>
    </template>
  </Layout>
</template>
