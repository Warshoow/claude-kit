<script setup lang="ts">
import { ref, watch } from "vue";
import { useRouter } from "vue-router";
import { storeToRefs } from "pinia";
import { useAppStore } from "@/stores/app";
import {
  ToggleGroup,
  ToggleGroupItem,
} from "@/components/ui/toggle-group";
import LibraryColumn from "@/components/LibraryColumn.vue";
import DiscoverView from "@/components/DiscoverView.vue";
import type { Asset, Plugin } from "@/lib/types";

const store = useAppStore();
const router = useRouter();
const {
  library,
  installedKeys,
  projectPath,
  marketplace,
  marketplaceLoading,
  marketplaceError,
  importingPlugin,
} = storeToRefs(store);

type Source = "library" | "marketplace";
const source = ref<Source>("library");

watch(source, async (s) => {
  if (s === "marketplace") await store.loadMarketplace();
}, { immediate: true });

function onSelectAsset(a: Asset) {
  router.push({
    name: "asset-detail",
    params: { kind: a.kind, name: a.name },
  });
}

function onSelectPlugin(p: Plugin) {
  router.push({ name: "plugin-detail", params: { name: p.name } });
}
</script>

<template>
  <div class="flex h-full flex-col overflow-hidden">
    <!-- Sub-toggle: Library | Marketplace -->
    <div class="flex items-center gap-3 border-b bg-card/40 px-6 py-2.5">
      <span class="text-sm font-semibold">Browse</span>
      <ToggleGroup
        type="single"
        :model-value="source"
        @update:model-value="(v) => v && (source = v as Source)"
        variant="outline"
        size="sm"
      >
        <ToggleGroupItem value="library">Library</ToggleGroupItem>
        <ToggleGroupItem value="marketplace">Marketplace</ToggleGroupItem>
      </ToggleGroup>
    </div>

    <!-- Source-specific content -->
    <div class="flex-1 min-h-0">
      <LibraryColumn
        v-if="source === 'library'"
        :library="library"
        :installed-keys="installedKeys"
        :can-install="!!projectPath"
        @toggle="store.toggleAsset"
        @select="onSelectAsset"
        @import="store.importLocalPlugin"
      />
      <DiscoverView
        v-else
        :marketplace="marketplace"
        :loading="marketplaceLoading"
        :error="marketplaceError"
        :importing-plugin="importingPlugin"
        @refresh="store.loadMarketplace(true)"
        @import="store.importMarketplacePlugin"
        @select="onSelectPlugin"
      />
    </div>
  </div>
</template>
