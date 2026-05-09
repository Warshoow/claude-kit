<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { storeToRefs } from "pinia";
import { diffLines, type Change } from "diff";
import {
  AlertCircle,
  ArrowLeft,
  ArrowUpCircle,
  Check,
  Loader2,
  Plus,
  Save,
} from "lucide-vue-next";
import { toast } from "vue-sonner";
import { useAppStore } from "@/stores/app";
import { useMarketplaceStore } from "@/stores/marketplace";
import { api } from "@/lib/api";
import type {
  AssetKind,
  AssetWrite,
  NewAssetEntry,
  Plugin,
  UpdatePreview,
} from "@/lib/types";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Checkbox } from "@/components/ui/checkbox";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";

const props = defineProps<{ name: string }>();

const router = useRouter();
const store = useAppStore();
const marketplaceStore = useMarketplaceStore();
const { marketplace } = storeToRefs(marketplaceStore);

type Phase = "loading" | "review" | "applying" | "error" | "no-update";

interface Hunk {
  index: number;
  partIndices: number[];
  removed: string[];
  added: string[];
  accepted: boolean;
}

interface ModifiedAssetState {
  kind: AssetKind;
  name: string;
  original: string;
  proposed: string;
  parts: Change[];
  hunks: Hunk[];
}

interface AddedAssetState {
  kind: AssetKind;
  name: string;
  content: string;
  /** Whether this new file should be added to the library on apply. */
  accepted: boolean;
}

const phase = ref<Phase>("loading");
const error = ref<string | null>(null);
const preview = ref<UpdatePreview | null>(null);
const modifiedStates = ref<ModifiedAssetState[]>([]);
const addedStates = ref<AddedAssetState[]>([]);

// Resolve the marketplace Plugin record from the cached catalog. If the
// user landed here via a deep link before the marketplace was fetched,
// kick off a load and we'll re-resolve after.
const plugin = computed<Plugin | null>(
  () => marketplace.value?.plugins.find((p) => p.name === props.name) ?? null
);

const totalHunks = computed(() =>
  modifiedStates.value.reduce((n, m) => n + m.hunks.length, 0)
);
const acceptedHunks = computed(() =>
  modifiedStates.value.reduce(
    (n, m) => n + m.hunks.filter((h) => h.accepted).length,
    0
  )
);
const acceptedAdds = computed(
  () => addedStates.value.filter((a) => a.accepted).length
);

const willWriteCount = computed(() => {
  // A modified asset gets written if any of its hunks were accepted OR
  // any were rejected — either way, the merged content goes to disk.
  // Skip a modified asset only if every hunk is accepted AND the
  // merged content equals the proposed (which is what we'd write
  // anyway), but for simplicity we always write modified entries.
  // Net effect: count of modified files + accepted adds.
  return modifiedStates.value.length + acceptedAdds.value;
});

// ── Hunk computation (same shape as the harmonizer) ────────────────

function computeModified(pair: { original: string; proposed: string }): {
  parts: Change[];
  hunks: Hunk[];
} {
  const parts = diffLines(pair.original, pair.proposed);
  const hunks: Hunk[] = [];
  let current: { partIndices: number[]; removed: string[]; added: string[] } | null = null;

  parts.forEach((p, i) => {
    if (p.added || p.removed) {
      if (!current) current = { partIndices: [], removed: [], added: [] };
      current.partIndices.push(i);
      const lines = p.value.split("\n");
      if (lines.length > 0 && lines[lines.length - 1] === "") lines.pop();
      if (p.added) current.added.push(...lines);
      else current.removed.push(...lines);
    } else if (current) {
      hunks.push({
        index: hunks.length,
        partIndices: current.partIndices,
        removed: current.removed,
        added: current.added,
        accepted: true,
      });
      current = null;
    }
  });
  if (current) {
    hunks.push({
      index: hunks.length,
      partIndices: (current as { partIndices: number[] }).partIndices,
      removed: (current as { removed: string[] }).removed,
      added: (current as { added: string[] }).added,
      accepted: true,
    });
  }
  return { parts, hunks };
}

function reconstruct(state: ModifiedAssetState): string {
  let out = "";
  let hunkIdx = 0;
  let inHunk = false;
  for (const p of state.parts) {
    if (!p.added && !p.removed) {
      if (inHunk) {
        hunkIdx++;
        inHunk = false;
      }
      out += p.value;
    } else {
      inHunk = true;
      const hunk = state.hunks[hunkIdx];
      if (!hunk) continue;
      if (p.added && hunk.accepted) out += p.value;
      else if (p.removed && !hunk.accepted) out += p.value;
    }
  }
  return out;
}

// ── Phase actions ──────────────────────────────────────────────────

async function loadPreview() {
  phase.value = "loading";
  error.value = null;
  try {
    if (!marketplace.value) await marketplaceStore.loadMarketplace();
    const pl = plugin.value;
    if (!pl) throw new Error(`Plugin "${props.name}" not found in marketplace`);

    const result = await api.previewPluginUpdate(pl);
    preview.value = result;

    if (result.modified.length === 0 && result.added.length === 0) {
      phase.value = "no-update";
      return;
    }

    modifiedStates.value = result.modified.map((p) => {
      const { parts, hunks } = computeModified(p);
      return {
        kind: p.kind,
        name: p.name,
        original: p.original,
        proposed: p.proposed,
        parts,
        hunks,
      };
    });
    addedStates.value = result.added.map((a: NewAssetEntry) => ({
      kind: a.kind,
      name: a.name,
      content: a.content,
      accepted: true,
    }));

    phase.value = "review";
  } catch (e) {
    error.value = String(e);
    phase.value = "error";
  }
}

async function applyAll() {
  if (!preview.value) return;
  phase.value = "applying";

  const writes: AssetWrite[] = [];
  for (const m of modifiedStates.value) {
    writes.push({ kind: m.kind, name: m.name, content: reconstruct(m) });
  }
  for (const a of addedStates.value) {
    if (a.accepted) {
      writes.push({ kind: a.kind, name: a.name, content: a.content });
    }
  }

  try {
    await api.applyPluginUpdate(
      preview.value.plugin_name,
      preview.value.new_git_ref,
      writes,
      preview.value.new_version,
      undefined,
    );
    await Promise.all([store.refreshLibrary(), store.refreshHooksMcp?.()]);
    toast.success(`"${preview.value.plugin_name}" updated`);
    router.push({
      name: "browse-library-plugin",
      params: { plugin: preview.value.plugin_name },
    });
  } catch (e) {
    error.value = String(e);
    phase.value = "error";
  }
}

function backToPlugin() {
  router.push({ name: "plugin-detail", params: { name: props.name } });
}

function toggleAllInModified(state: ModifiedAssetState, value: boolean) {
  for (const h of state.hunks) h.accepted = value;
}

function kindLabel(k: AssetKind): string {
  if (k === "skills") return "Skill";
  if (k === "commands") return "Command";
  return "Agent";
}

onMounted(() => loadPreview());
</script>

<template>
  <div class="flex h-full flex-col overflow-hidden">
    <!-- Header -->
    <div class="border-b bg-card/40 px-6 py-4">
      <button
        class="mb-2 inline-flex items-center gap-1 text-xs text-muted-foreground transition-colors hover:text-foreground"
        @click="backToPlugin"
      >
        <ArrowLeft class="size-3" />
        Back to plugin
      </button>

      <div class="flex items-start justify-between gap-4">
        <div class="min-w-0 flex-1">
          <div class="mb-1 text-[11px] font-medium text-muted-foreground">
            Update plugin
          </div>
          <div class="flex items-center gap-2">
            <ArrowUpCircle class="size-4 shrink-0 text-amber-500" />
            <h1 class="truncate text-lg font-semibold">{{ name }}</h1>
            <Badge
              v-if="preview?.current_version || preview?.new_version"
              variant="outline"
              class="text-[10px] uppercase tracking-wider"
            >
              v{{ preview.current_version ?? "?" }} → v{{ preview.new_version ?? "?" }}
            </Badge>
          </div>
        </div>

        <div
          v-if="phase === 'review'"
          class="flex shrink-0 items-center gap-2 text-xs text-muted-foreground"
        >
          <Badge variant="outline" class="text-[11px]">
            {{ acceptedHunks }} / {{ totalHunks }} hunks
          </Badge>
          <Badge v-if="addedStates.length" variant="secondary" class="text-[11px]">
            +{{ acceptedAdds }} new
          </Badge>
        </div>
      </div>
    </div>

    <!-- Body -->
    <div class="relative flex-1 min-h-0 overflow-hidden">
      <!-- Loading -->
      <div
        v-if="phase === 'loading'"
        class="flex h-full flex-col items-center justify-center gap-3 text-sm text-muted-foreground"
      >
        <Loader2 class="size-6 animate-spin text-primary" />
        <div>Downloading upstream and computing diff…</div>
        <p class="max-w-md text-center text-xs">
          We fetch the latest tarball and compare each asset to your local
          copy. Nothing is written until you accept changes.
        </p>
      </div>

      <!-- No update needed -->
      <div
        v-else-if="phase === 'no-update'"
        class="flex h-full flex-col items-center justify-center gap-3 px-6 text-center"
      >
        <div class="rounded-full bg-primary/10 p-3">
          <Check class="size-6 text-primary" />
        </div>
        <h3 class="text-base font-semibold">Already up to date</h3>
        <p class="max-w-md text-sm text-muted-foreground">
          Every asset of this plugin in your library matches the upstream
          version. No changes to apply.
        </p>
        <Button variant="outline" @click="backToPlugin">Back</Button>
      </div>

      <!-- Error -->
      <div
        v-else-if="phase === 'error'"
        class="flex h-full flex-col items-center justify-center gap-4 px-6 text-center"
      >
        <div class="rounded-full bg-destructive/10 p-3">
          <AlertCircle class="size-6 text-destructive" />
        </div>
        <h3 class="text-base font-semibold">Update failed</h3>
        <code class="max-w-lg break-words rounded-md border bg-card/40 px-3 py-2 text-xs">
          {{ error }}
        </code>
        <div class="flex gap-2">
          <Button variant="outline" @click="backToPlugin">Back</Button>
          <Button @click="loadPreview">Retry</Button>
        </div>
      </div>

      <!-- Applying -->
      <div
        v-else-if="phase === 'applying'"
        class="flex h-full items-center justify-center gap-2 text-sm text-muted-foreground"
      >
        <Loader2 class="size-4 animate-spin" />
        Writing files and refreshing origins…
      </div>

      <!-- Review -->
      <ScrollArea v-else-if="phase === 'review'" class="h-full">
        <div class="mx-auto w-full max-w-3xl space-y-4 px-6 py-5">
          <p class="text-xs text-muted-foreground">
            Accepted hunks overwrite the original lines; rejected ones keep
            them. Files are only touched on disk after you click Apply.
          </p>

          <!-- Modified files -->
          <Card v-for="state in modifiedStates" :key="`mod-${state.kind}/${state.name}`">
            <CardHeader class="space-y-2 pb-3">
              <div class="flex items-center justify-between gap-2">
                <CardTitle class="flex items-center gap-2 text-sm">
                  <Badge variant="outline" class="text-[10px]">
                    {{ kindLabel(state.kind) }}
                  </Badge>
                  <span class="truncate font-mono">{{ state.name }}</span>
                </CardTitle>
                <div class="flex items-center gap-2 text-[11px] text-muted-foreground">
                  <span>
                    {{ state.hunks.filter((h) => h.accepted).length }} /
                    {{ state.hunks.length }} hunks
                  </span>
                  <button
                    class="text-[10.5px] underline underline-offset-2 hover:text-foreground"
                    @click="toggleAllInModified(state, true)"
                  >Accept all</button>
                  <span class="opacity-40">·</span>
                  <button
                    class="text-[10.5px] underline underline-offset-2 hover:text-foreground"
                    @click="toggleAllInModified(state, false)"
                  >Reject all</button>
                </div>
              </div>
            </CardHeader>

            <CardContent class="space-y-3 pb-4">
              <div
                v-for="(hunk, hi) in state.hunks"
                :key="hunk.index"
                class="overflow-hidden rounded-md border"
                :class="hunk.accepted ? '' : 'opacity-50'"
              >
                <header class="flex items-center gap-2 border-b bg-muted/40 px-3 py-1.5">
                  <Checkbox
                    :model-value="hunk.accepted"
                    @update:model-value="(v) => (hunk.accepted = !!v)"
                  />
                  <span class="text-[10.5px] font-medium uppercase tracking-wider text-muted-foreground">
                    Hunk {{ hi + 1 }}
                  </span>
                  <span class="text-[10.5px] text-muted-foreground">
                    <span class="text-destructive">−{{ hunk.removed.length }}</span>
                    /
                    <span class="text-emerald-600 dark:text-emerald-400">+{{ hunk.added.length }}</span>
                  </span>
                </header>
                <pre class="m-0 overflow-x-auto bg-card font-mono text-[11.5px] leading-snug"><div
                    v-for="(line, i) in hunk.removed"
                    :key="`r${i}`"
                    class="px-3 bg-destructive/10 text-destructive"
                  ><span class="select-none">- </span>{{ line }}</div><div
                    v-for="(line, i) in hunk.added"
                    :key="`a${i}`"
                    class="px-3 bg-emerald-500/10 text-emerald-700 dark:text-emerald-400"
                  ><span class="select-none">+ </span>{{ line }}</div></pre>
              </div>
            </CardContent>
          </Card>

          <!-- New files in upstream -->
          <Card v-if="addedStates.length">
            <CardHeader class="pb-3">
              <CardTitle class="flex items-center justify-between gap-2 text-sm">
                <div class="flex items-center gap-2">
                  <Plus class="size-3.5 text-emerald-600 dark:text-emerald-400" />
                  <span>New in upstream</span>
                </div>
                <span class="text-[11px] font-normal text-muted-foreground">
                  {{ acceptedAdds }} / {{ addedStates.length }} selected
                </span>
              </CardTitle>
            </CardHeader>
            <CardContent class="space-y-1.5">
              <label
                v-for="a in addedStates"
                :key="`add-${a.kind}/${a.name}`"
                class="flex cursor-pointer items-start gap-3 rounded-md p-2 transition-colors hover:bg-accent/40"
              >
                <Checkbox
                  :model-value="a.accepted"
                  @update:model-value="(v) => (a.accepted = !!v)"
                  class="mt-0.5"
                />
                <div class="min-w-0 flex-1">
                  <div class="flex items-center gap-2">
                    <Badge variant="outline" class="text-[10px]">
                      {{ kindLabel(a.kind) }}
                    </Badge>
                    <span class="font-mono text-sm">{{ a.name }}</span>
                  </div>
                </div>
              </label>
            </CardContent>
          </Card>

          <Separator />
        </div>
      </ScrollArea>
    </div>

    <!-- Footer (review phase) -->
    <div
      v-if="phase === 'review'"
      class="flex items-center gap-3 border-t bg-card/40 px-6 py-2.5"
    >
      <span class="text-xs text-muted-foreground">
        {{ willWriteCount }} file{{ willWriteCount === 1 ? "" : "s" }} will be written
      </span>
      <div class="flex-1" />
      <Button variant="outline" @click="backToPlugin">Cancel</Button>
      <Button :disabled="willWriteCount === 0" @click="applyAll">
        <Save />
        Apply update
      </Button>
    </div>
  </div>
</template>

<style scoped>
pre {
  white-space: pre-wrap;
  word-break: break-word;
}
</style>
