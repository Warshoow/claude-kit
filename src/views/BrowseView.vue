<script setup lang="ts">
import { computed } from "vue";
import { useRoute, useRouter } from "vue-router";
import {
  ToggleGroup,
  ToggleGroupItem,
} from "@/components/ui/toggle-group";

const route = useRoute();
const router = useRouter();

type Source = "library" | "marketplace";

const source = computed<Source>(() => {
  return route.path.endsWith("/marketplace") ? "marketplace" : "library";
});

function onSwitch(v: string | undefined) {
  if (!v) return;
  router.push({ name: `browse-${v}` });
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
        @update:model-value="onSwitch($event as string | undefined)"
        variant="outline"
        size="sm"
      >
        <ToggleGroupItem value="library">Library</ToggleGroupItem>
        <ToggleGroupItem value="marketplace">Marketplace</ToggleGroupItem>
      </ToggleGroup>
    </div>

    <!-- Sub-route content -->
    <div class="flex-1 min-h-0">
      <RouterView />
    </div>
  </div>
</template>
