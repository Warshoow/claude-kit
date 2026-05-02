<script setup lang="ts">
import { storeToRefs } from "pinia";
import { useAppStore } from "@/stores/app";
import ProjectColumn from "@/components/ProjectColumn.vue";

const store = useAppStore();
const { projectPath, installed, bundles } = storeToRefs(store);
</script>

<template>
  <div class="h-full">
    <ProjectColumn
      :project-path="projectPath"
      :installed="installed"
      :bundles="bundles"
      @clean="store.cleanProject"
    />
  </div>
</template>
