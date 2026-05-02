<script setup lang="ts">
import { ref } from "vue";
import { storeToRefs } from "pinia";
import { useAppStore } from "@/stores/app";
import BundlesColumn from "@/components/BundlesColumn.vue";

const store = useAppStore();
const { bundles, library, projectPath } = storeToRefs(store);
const selectedBundle = ref<string | null>(null);
</script>

<template>
  <div class="h-full">
    <BundlesColumn
      :bundles="bundles"
      :library="library"
      :selected="selectedBundle"
      :can-apply="!!projectPath"
      @select="(n) => (selectedBundle = n)"
      @create="store.createBundle"
      @delete="
        (n) => {
          if (selectedBundle === n) selectedBundle = null;
          store.deleteBundle(n);
        }
      "
      @update="store.updateBundle"
      @apply="store.applyBundle"
    />
  </div>
</template>
