<script setup lang="ts">
import { computed, ref } from "vue";
import { useRouter } from "vue-router";
import { storeToRefs } from "pinia";
import {
  ChevronLeft,
  FolderOpen,
  Package,
  Search,
  X,
} from "lucide-vue-next";
import { useAppStore } from "@/stores/app";
import type { Asset } from "@/lib/types";
import { assetKey } from "@/lib/types";
import { groupAssets } from "@/lib/grouping";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Checkbox } from "@/components/ui/checkbox";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";

const LOCAL_KEY = "__local__";

const props = defineProps<{ plugin: string }>();

const store = useAppStore();
const router = useRouter();
const { library, installedKeys, projectPath } = storeToRefs(store);

const canInstall = computed(() => !!projectPath.value);
const isLocal = computed(() => props.plugin === LOCAL_KEY);

const pluginAssets = computed<Asset[]>(() => {
  if (isLocal.value) {
    return library.value.filter((a) => !a.origin);
  }
  return library.value.filter((a) => a.origin?.plugin === props.plugin);
});

const marketplace = computed<string | null>(() => {
  // All assets in a non-local bucket share the same marketplace tag.
  return pluginAssets.value[0]?.origin?.marketplace ?? null;
});

const query = ref("");

const filtered = computed<Asset[]>(() => {
  const q = query.value.trim().toLowerCase();
  if (!q) return pluginAssets.value;
  return pluginAssets.value.filter((a) => {
    if (a.name.toLowerCase().includes(q)) return true;
    if (a.description?.toLowerCase().includes(q)) return true;
    if (a.tags?.some((t) => t.toLowerCase().includes(q))) return true;
    if (a.kind.toLowerCase().includes(q)) return true;
    return false;
  });
});

const groups = computed(() => groupAssets(filtered.value, "kind"));

const title = computed(() => (isLocal.value ? "Local" : props.plugin));

function openAsset(a: Asset) {
  router.push({
    name: "asset-detail",
    params: { kind: a.kind, name: a.name },
  });
}

function onCheckboxChange(a: Asset) {
  if (!canInstall.value) return;
  store.toggleAsset(a);
}

function back() {
  router.push({ name: "browse-library" });
}
</script>

<template>
  <TooltipProvider :delay-duration="200">
    <div class="flex h-full flex-col overflow-hidden">
      <!-- Header -->
      <div class="border-b bg-card/40 px-6 py-4">
        <button
          class="mb-2 inline-flex items-center gap-1 text-xs text-muted-foreground transition-colors hover:text-foreground"
          @click="back"
        >
          <ChevronLeft class="size-3" />
          Back to library
        </button>

        <div class="flex items-center gap-2">
          <FolderOpen
            v-if="isLocal"
            class="size-5 shrink-0 text-muted-foreground"
          />
          <Package v-else class="size-5 shrink-0 text-muted-foreground" />
          <h1 class="truncate text-lg font-semibold">{{ title }}</h1>
          <Badge
            v-if="marketplace"
            variant="secondary"
            class="text-[10px] uppercase tracking-wider"
          >from {{ marketplace }}</Badge>
        </div>

        <p
          v-if="isLocal"
          class="mt-1 text-xs text-muted-foreground"
        >Assets you created here, not imported from any plugin.</p>
      </div>

      <!-- Toolbar -->
      <div class="flex items-center gap-3 border-b px-4 py-2">
        <span class="text-xs text-muted-foreground">
          {{ filtered.length
          }}<span v-if="filtered.length !== pluginAssets.length">/{{ pluginAssets.length }}</span>
          asset{{ filtered.length === 1 ? "" : "s" }}
        </span>

        <div class="relative flex-1 max-w-md">
          <Search
            class="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground"
          />
          <Input
            v-model="query"
            type="text"
            placeholder="Search name, description, tag…"
            class="h-8 pl-8 pr-8 text-sm"
          />
          <button
            v-if="query"
            type="button"
            class="absolute right-1.5 top-1/2 grid size-5 -translate-y-1/2 place-items-center rounded-sm text-muted-foreground hover:bg-accent hover:text-foreground"
            @click="query = ''"
          >
            <X class="size-3" />
          </button>
        </div>
      </div>

      <!-- Body -->
      <ScrollArea class="flex-1 min-h-0">
        <template v-if="pluginAssets.length === 0">
          <div class="px-6 py-12 text-center text-xs text-muted-foreground">
            <template v-if="isLocal">No local assets yet.</template>
            <template v-else>This plugin has no assets in your library.</template>
          </div>
        </template>
        <template v-else-if="filtered.length === 0">
          <div class="px-6 py-12 text-center text-xs text-muted-foreground">
            No match for "{{ query }}".
          </div>
        </template>
        <template v-else>
          <div class="space-y-3 p-3">
            <div v-for="g in groups" :key="g.key">
              <div
                class="px-2.5 pt-1 pb-1.5 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground"
              >
                {{ g.label }}
                <span class="font-normal opacity-70">{{ g.items.length }}</span>
              </div>
              <div class="space-y-0.5">
                <div
                  v-for="a in g.items"
                  :key="assetKey(a)"
                  class="group flex cursor-pointer items-start gap-2.5 rounded-md px-2.5 py-2 transition-colors hover:bg-accent/60"
                  :class="
                    installedKeys.has(assetKey(a))
                      ? 'bg-primary/10 ring-1 ring-inset ring-primary/30'
                      : ''
                  "
                  tabindex="0"
                  @click="openAsset(a)"
                  @keydown.enter="openAsset(a)"
                  @keydown.space.prevent="openAsset(a)"
                >
                  <Tooltip>
                    <TooltipTrigger as-child>
                      <Checkbox
                        :model-value="installedKeys.has(assetKey(a))"
                        :disabled="!canInstall"
                        class="mt-0.5"
                        @update:model-value="onCheckboxChange(a)"
                        @click.stop
                      />
                    </TooltipTrigger>
                    <TooltipContent>
                      {{
                        canInstall
                          ? "Toggle install in project"
                          : "Pick a project to install"
                      }}
                    </TooltipContent>
                  </Tooltip>

                  <div class="min-w-0 flex-1">
                    <span class="text-sm font-medium leading-none">{{ a.name }}</span>
                    <p
                      v-if="a.description"
                      class="mt-1 line-clamp-2 text-xs leading-snug text-muted-foreground"
                    >{{ a.description }}</p>
                  </div>
                </div>
              </div>
            </div>
          </div>
        </template>
      </ScrollArea>
    </div>
  </TooltipProvider>
</template>
