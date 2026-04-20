<script setup lang="ts">
import { computed } from "vue";
import type { Bundle, InstalledAsset } from "../lib/types";
import { assetKey } from "../lib/types";

const props = defineProps<{
  projectPath: string | null;
  installed: InstalledAsset[];
  bundles: Bundle[];
}>();

defineEmits<{
  clean: [];
}>();

/**
 * Detect which bundles are "fully applied" — i.e. all their assets
 * are currently installed in the project.
 */
const appliedBundles = computed(() => {
  const installedSet = new Set(props.installed.map(assetKey));
  return props.bundles.filter(
    (b) =>
      b.assets.length > 0 && b.assets.every((a) => installedSet.has(assetKey(a)))
  );
});

const appliedBundleAssets = computed(() => {
  const s = new Set<string>();
  for (const b of appliedBundles.value) {
    for (const a of b.assets) s.add(assetKey(a));
  }
  return s;
});

const extras = computed(() =>
  props.installed.filter((a) => !appliedBundleAssets.value.has(assetKey(a)))
);
</script>

<template>
  <div class="column">
    <div class="col-header">
      This project
      <span class="count">{{ installed.length }}</span>
    </div>
    <div class="col-body">
      <div v-if="!projectPath" class="empty">
        No project selected.<br />Pick one from the top bar.
      </div>

      <template v-else>
        <div class="kind-label">Applied bundles ({{ appliedBundles.length }})</div>
        <div v-if="appliedBundles.length === 0" class="empty">(none)</div>
        <div
          v-for="b in appliedBundles"
          :key="b.name"
          class="bundle-item"
          style="cursor: default"
        >
          <div class="bundle-title">
            <span class="bundle-name">{{ b.name }}</span>
            <span class="bundle-count">{{ b.assets.length }}</span>
          </div>
        </div>

        <div class="kind-label" style="margin-top:12px">
          Extras ({{ extras.length }})
        </div>
        <div v-if="extras.length === 0" class="empty">(none)</div>
        <div
          v-for="a in extras"
          :key="assetKey(a)"
          class="asset-item"
        >
          <div class="asset-body">
            <div class="asset-name">{{ a.kind }}/{{ a.name }}</div>
          </div>
        </div>
      </template>
    </div>

    <div v-if="projectPath && installed.length > 0" class="col-actions">
      <button class="danger" @click="$emit('clean')">
        Clean all ({{ installed.length }})
      </button>
    </div>
  </div>
</template>
