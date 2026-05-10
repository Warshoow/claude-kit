<script setup lang="ts">
import DefaultTheme from "vitepress/theme";
import DownloadButton from "./DownloadButton.vue";

const { Layout } = DefaultTheme;
</script>

<template>
  <Layout>
    <!--
      Slot inserted right after the YAML-defined hero `actions`. We use
      this to render a smart, OS-aware download button that points at
      the latest GitHub release's matching installer asset (resolved at
      runtime via the GitHub API).
    -->
    <template #home-hero-actions-after>
      <DownloadButton />
    </template>
  </Layout>
</template>
