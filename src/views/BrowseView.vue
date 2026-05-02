<script setup lang="ts">
import { ref, watch } from "vue";
import { storeToRefs } from "pinia";
import { useAppStore } from "@/stores/app";
import {
  ToggleGroup,
  ToggleGroupItem,
} from "@/components/ui/toggle-group";
import LibraryColumn from "@/components/LibraryColumn.vue";
import DiscoverView from "@/components/DiscoverView.vue";

const store = useAppStore();
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
        @edit="store.startEditing"
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
        @select="store.selectPlugin"
      />
    </div>
  </div>
</template>
