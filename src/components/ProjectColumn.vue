<script setup lang="ts">
import { computed } from "vue";
import { Trash2, Package } from "lucide-vue-next";
import type { Bundle, InstalledAsset } from "@/lib/types";
import { assetKey } from "@/lib/types";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";

const props = defineProps<{
  projectPath: string | null;
  installed: InstalledAsset[];
  bundles: Bundle[];
}>();

defineEmits<{
  clean: [];
}>();

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
  <div class="flex h-full flex-col overflow-hidden">
    <!-- Header -->
    <div class="flex items-center gap-2 border-b bg-card/40 px-4 py-2.5">
      <span class="text-sm font-semibold">This project</span>
      <span class="text-xs text-muted-foreground">{{ installed.length }}</span>
    </div>

    <!-- Body -->
    <ScrollArea class="flex-1 min-h-0">
      <div v-if="!projectPath" class="px-6 py-10 text-center text-xs text-muted-foreground">
        No project selected.<br />Pick one from the top bar.
      </div>

      <template v-else>
        <div class="space-y-4 p-3">
          <!-- Applied bundles -->
          <section>
            <div
              class="px-1 pb-2 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground"
            >
              Applied bundles
              <span class="font-normal opacity-70">{{ appliedBundles.length }}</span>
            </div>
            <div
              v-if="appliedBundles.length === 0"
              class="rounded-md border border-dashed bg-card/40 px-3 py-3 text-xs italic text-muted-foreground"
            >
              No fully applied bundles.
            </div>
            <div v-else class="space-y-2">
              <div
                v-for="b in appliedBundles"
                :key="b.name"
                class="flex items-center justify-between rounded-lg border bg-card px-3 py-2.5"
              >
                <div class="flex items-center gap-2">
                  <Package class="size-3.5 text-muted-foreground" />
                  <span class="text-sm font-semibold">{{ b.name }}</span>
                </div>
                <Badge variant="secondary" class="font-mono text-[10px]">
                  {{ b.assets.length }} assets
                </Badge>
              </div>
            </div>
          </section>

          <!-- Extras -->
          <section>
            <div
              class="px-1 pb-2 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground"
            >
              Extras
              <span class="font-normal opacity-70">{{ extras.length }}</span>
            </div>
            <div
              v-if="extras.length === 0"
              class="rounded-md border border-dashed bg-card/40 px-3 py-3 text-xs italic text-muted-foreground"
            >
              No standalone assets.
            </div>
            <div v-else class="space-y-1">
              <div
                v-for="a in extras"
                :key="assetKey(a)"
                class="flex items-center gap-2 rounded-md px-2.5 py-1.5 text-sm"
              >
                <span class="font-mono text-[10px] text-muted-foreground">
                  {{ a.kind.charAt(0) }}
                </span>
                <span>{{ a.name }}</span>
              </div>
            </div>
          </section>
        </div>
      </template>
    </ScrollArea>

    <!-- Footer actions -->
    <div
      v-if="projectPath && installed.length > 0"
      class="border-t bg-card/40 p-3"
    >
      <Button
        variant="ghost"
        size="sm"
        class="w-full text-destructive hover:bg-destructive/10 hover:text-destructive"
        @click="$emit('clean')"
      >
        <Trash2 />
        Clean all ({{ installed.length }})
      </Button>
    </div>
  </div>
</template>
