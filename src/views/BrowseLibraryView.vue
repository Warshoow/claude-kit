<script setup lang="ts">
import { computed, ref } from "vue";
import { useRouter } from "vue-router";
import { storeToRefs } from "pinia";
import { Search, Upload, X } from "lucide-vue-next";
import { useAppStore } from "@/stores/app";
import type { Asset, AssetKind } from "@/lib/types";
import { assetKey } from "@/lib/types";
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

const store = useAppStore();
const router = useRouter();
const { library, installedKeys, projectPath } = storeToRefs(store);

const canInstall = computed(() => !!projectPath.value);
const query = ref("");

const filtered = computed(() => {
  const q = query.value.trim().toLowerCase();
  if (!q) return library.value;
  return library.value.filter((a) => {
    if (a.name.toLowerCase().includes(q)) return true;
    if (a.description?.toLowerCase().includes(q)) return true;
    if (a.tags?.some((t) => t.toLowerCase().includes(q))) return true;
    if (a.kind.toLowerCase().includes(q)) return true;
    return false;
  });
});

const grouped = computed(() => {
  const g: Record<AssetKind, Asset[]> = { skills: [], commands: [], agents: [] };
  for (const a of filtered.value) g[a.kind].push(a);
  return g;
});

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
</script>

<template>
  <TooltipProvider :delay-duration="200">
    <div class="flex h-full flex-col overflow-hidden">
      <!-- Toolbar: count + search + import -->
      <div class="flex items-center gap-3 border-b px-4 py-2">
        <span class="text-xs text-muted-foreground">
          {{ filtered.length
          }}<span v-if="filtered.length !== library.length">/{{ library.length }}</span>
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

        <div class="flex-1" />

        <Tooltip>
          <TooltipTrigger as-child>
            <Button variant="outline" size="sm" @click="store.importLocalPlugin">
              <Upload />
              Import
            </Button>
          </TooltipTrigger>
          <TooltipContent>Import assets from a plugin folder</TooltipContent>
        </Tooltip>
      </div>

      <!-- Body -->
      <ScrollArea class="flex-1 min-h-0">
        <template v-if="library.length === 0">
          <div class="px-6 py-12 text-center text-xs text-muted-foreground">
            Empty library.<br />
            Add files to
            <code class="rounded bg-muted px-1 py-0.5 font-mono text-[11px]">~/.claude-assets/library/</code>
            or import a plugin folder.
          </div>
        </template>
        <template v-else-if="filtered.length === 0">
          <div class="px-6 py-12 text-center text-xs text-muted-foreground">
            No match for "{{ query }}".
          </div>
        </template>
        <template v-else>
          <div class="space-y-3 p-3">
            <div
              v-for="kind in (['skills', 'commands', 'agents'] as AssetKind[])"
              :key="kind"
            >
              <template v-if="grouped[kind].length > 0">
                <div
                  class="px-2.5 pt-1 pb-1.5 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground"
                >
                  {{ kind }}
                  <span class="font-normal opacity-70">{{ grouped[kind].length }}</span>
                </div>
                <div class="space-y-0.5">
                  <div
                    v-for="a in grouped[kind]"
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
                      <div class="flex items-center gap-1.5">
                        <span class="text-sm font-medium leading-none">{{ a.name }}</span>
                        <Badge
                          v-if="a.origin"
                          variant="secondary"
                          class="h-4 px-1.5 text-[9px] font-medium uppercase tracking-wider"
                          :title="`from ${a.origin.marketplace} · imported ${a.origin.imported_at}`"
                        >from {{ a.origin.plugin }}</Badge>
                      </div>
                      <p
                        v-if="a.description"
                        class="mt-1 line-clamp-2 text-xs leading-snug text-muted-foreground"
                      >{{ a.description }}</p>
                    </div>
                  </div>
                </div>
              </template>
            </div>
          </div>
        </template>
      </ScrollArea>
    </div>
  </TooltipProvider>
</template>
