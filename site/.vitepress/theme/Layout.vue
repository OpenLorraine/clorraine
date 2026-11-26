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
            v1.2.0 for Linux x86_64 installs with one command. Windows 11 ships
            with the next release; macOS is on the way.
          </span>
          <a class="cl-preview-bar__link" :href="withBase('/guide/quick-start')">
            Install on Linux
            <span aria-hidden="true">→</span>
          </a>
        </div>
      </div>
    </template>
  </Layout>
</template>
