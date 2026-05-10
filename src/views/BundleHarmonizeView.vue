<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { storeToRefs } from "pinia";
import { diffLines, diffWordsWithSpace, type Change } from "diff";
import {
  AlertCircle,
  ArrowLeft,
  Check,
  Loader2,
  Save,
  Sparkles,
} from "lucide-vue-next";
import { toast } from "vue-sonner";
import { useAppStore } from "@/stores/app";
import { useAiStore } from "@/stores/ai";
import { api } from "@/lib/api";
import type { AssetKind, Bundle, HarmonizationResult } from "@/lib/types";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
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
const aiStore = useAiStore();
const { bundles } = storeToRefs(store);
const { status: aiStatus, generating } = storeToRefs(aiStore);

type Phase = "setup" | "generating" | "review" | "applying" | "error";

interface Hunk {
  index: number;
  /** Indices of the diff `parts` array this hunk owns. */
  partIndices: number[];
  removed: string[];
  added: string[];
  accepted: boolean;
}

interface AssetState {
  kind: AssetKind;
  name: string;
  original: string;
  proposed: string;
  parts: Change[];
  hunks: Hunk[];
  /** True when original === proposed — surfaced as "no change" cards. */
  unchanged: boolean;
}

const phase = ref<Phase>("setup");
const error = ref<string | null>(null);
const instruction = ref("");
const assetStates = ref<AssetState[]>([]);
const applying = ref(false);

const bundle = computed<Bundle | null>(
  () => bundles.value.find((b) => b.name === props.name) ?? null
);

const aiReady = computed(
  () => aiStatus.value !== null && aiStatus.value.mode !== "none"
);

const totalHunks = computed(() =>
  assetStates.value.reduce((n, a) => n + a.hunks.length, 0)
);
const acceptedHunks = computed(() =>
  assetStates.value.reduce(
    (n, a) => n + a.hunks.filter((h) => h.accepted).length,
    0
  )
);

const changedAssetsCount = computed(
  () => assetStates.value.filter((a) => !a.unchanged).length
);

// ── Hunk computation ───────────────────────────────────────────
//
// The diff library splits content into a sequence of parts, each
// either added/removed/equal. A "hunk" is a contiguous run of
// {added, removed} parts — equal parts close a hunk.

function computeAssetState(r: HarmonizationResult): AssetState {
  const unchanged = r.original === r.proposed;
  if (unchanged) {
    return {
      kind: r.kind,
      name: r.name,
      original: r.original,
      proposed: r.proposed,
      parts: [],
      hunks: [],
      unchanged: true,
    };
  }

  const parts = diffLines(r.original, r.proposed);
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

  return {
    kind: r.kind,
    name: r.name,
    original: r.original,
    proposed: r.proposed,
    parts,
    hunks,
    unchanged: false,
  };
}

/**
 * Walk the diff parts and rebuild the file content from accepted hunks.
 * Reject = keep the original lines (i.e. include `removed` parts and
 * skip `added` ones); accept = the inverse.
 */
function reconstruct(state: AssetState): string {
  if (state.unchanged) return state.original;
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

// ── Phase actions ──────────────────────────────────────────────

async function runHarmonize() {
  if (!aiReady.value) return;
  phase.value = "generating";
  error.value = null;
  try {
    const results = await api.harmonizeBundle(
      props.name,
      instruction.value.trim() || undefined
    );
    assetStates.value = results.map(computeAssetState);
    phase.value = "review";
  } catch (e) {
    error.value = String(e);
    phase.value = "error";
  }
}

async function applyAll() {
  applying.value = true;
  phase.value = "applying";
  let failures = 0;
  // Track which assets actually got rewritten so we can mark only those
  // as harmonized — leaving an asset where every hunk was rejected (or
  // was unchanged to begin with) untagged.
  const touched: { kind: AssetKind; name: string }[] = [];
  for (const a of assetStates.value) {
    const finalContent = reconstruct(a);
    if (finalContent === a.original) continue; // nothing accepted for this asset
    try {
      await api.writeAsset(a.kind, a.name, finalContent);
      touched.push({ kind: a.kind, name: a.name });
    } catch (e) {
      failures++;
      console.error(`writeAsset ${a.kind}/${a.name} failed`, e);
    }
  }

  // Stamp the harmonized timestamp on every successfully-written asset.
  // Failure here is non-fatal — the content is already on disk; missing
  // the mark just means the badge won't show. We log + continue.
  if (touched.length) {
    try {
      await api.markAssetsHarmonized(touched);
    } catch (e) {
      console.warn("mark_assets_harmonized failed:", e);
    }
  }

  await store.refreshLibrary();
  applying.value = false;
  if (failures > 0) {
    toast.error(`${failures} asset${failures === 1 ? "" : "s"} failed to save`);
  } else {
    toast.success("Bundle harmonized");
  }
  router.push({ name: "bundle-detail", params: { name: props.name } });
}

function backToBundle() {
  router.push({ name: "bundle-detail", params: { name: props.name } });
}

function toggleAllInAsset(state: AssetState, value: boolean) {
  for (const h of state.hunks) h.accepted = value;
}

function kindLabel(k: AssetKind): string {
  if (k === "skills") return "Skill";
  if (k === "commands") return "Command";
  return "Agent";
}

/**
 * Word-level diff for rendering inside a hunk. The line-level diff
 * (used to slice the file into hunks) marks an entire line as
 * removed+added when even a single word changes — which makes typo
 * fixes look like a complete rewrite. Re-running diff at the word
 * granularity lets us highlight only the words that actually changed,
 * with everything else as quiet context.
 *
 * Pure additions and pure deletions are returned as a single chunk
 * — there's no other side to align against, so word-diff would just
 * recolour everything anyway.
 */
function inlineWordDiff(hunk: Hunk): Change[] {
  const removedText = hunk.removed.join("\n");
  const addedText = hunk.added.join("\n");
  if (!removedText) {
    return [{ value: addedText, added: true, removed: false, count: 0 }];
  }
  if (!addedText) {
    return [{ value: removedText, added: false, removed: true, count: 0 }];
  }
  return diffWordsWithSpace(removedText, addedText);
}

onMounted(async () => {
  if (!bundles.value.length) await store.refreshBundles();
  await aiStore.refreshStatus();
});
</script>

<template>
  <div class="flex h-full flex-col overflow-hidden">
    <!-- Header -->
    <div class="border-b bg-card/40 px-6 py-4">
      <button
        class="mb-2 inline-flex items-center gap-1 text-xs text-muted-foreground transition-colors hover:text-foreground"
        @click="backToBundle"
      >
        <ArrowLeft class="size-3" />
        Back to bundle
      </button>

      <div class="flex items-start justify-between gap-4">
        <div class="min-w-0 flex-1">
          <div class="mb-1 text-[11px] font-medium text-muted-foreground">
            Harmonize
          </div>
          <div class="flex items-center gap-2">
            <Sparkles class="size-4 shrink-0 text-primary" />
            <h1 class="truncate text-lg font-semibold">{{ name }}</h1>
          </div>
          <p
            v-if="bundle?.description"
            class="mt-1 text-sm text-muted-foreground"
          >{{ bundle.description }}</p>
        </div>

        <div
          v-if="phase === 'review'"
          class="flex shrink-0 items-center gap-2 text-xs text-muted-foreground"
        >
          <Badge variant="outline" class="text-[11px]">
            {{ acceptedHunks }} / {{ totalHunks }} hunks
          </Badge>
          <Badge variant="secondary" class="text-[11px]">
            {{ changedAssetsCount }}
            asset{{ changedAssetsCount === 1 ? "" : "s" }} changed
          </Badge>
        </div>
      </div>
    </div>

    <!-- Body -->
    <div class="relative flex-1 min-h-0 overflow-hidden">
      <!-- Setup phase -->
      <ScrollArea v-if="phase === 'setup'" class="h-full">
        <div class="mx-auto w-full max-w-2xl space-y-5 px-6 py-6">
          <Card>
            <CardHeader>
              <CardTitle class="flex items-center gap-2">
                <Sparkles class="size-4" />
                Bundle harmonizer
              </CardTitle>
            </CardHeader>
            <CardContent class="space-y-4">
              <p class="text-sm text-muted-foreground">
                The AI rewrites every asset in
                <span class="font-mono">{{ name }}</span>
                so the bundle reads like one author wrote it: consistent
                tone, terminology, frontmatter format, and structure. Each
                proposed change is shown to you as a diff —
                <strong>nothing is written to disk</strong> until you accept
                the hunks you want and click Apply.
              </p>

              <div v-if="bundle" class="text-xs text-muted-foreground">
                {{ bundle.assets.length }}
                asset{{ bundle.assets.length === 1 ? "" : "s" }} will be sent
                to the model.
              </div>

              <div class="space-y-1.5">
                <Label for="harmonize-instruction">
                  Extra instruction
                  <span class="text-muted-foreground">(optional)</span>
                </Label>
                <Textarea
                  id="harmonize-instruction"
                  v-model="instruction"
                  rows="3"
                  placeholder="e.g. Use a more terse, technical tone. Standardize on the term 'agent' instead of 'assistant'."
                />
              </div>

              <div
                v-if="!aiReady"
                class="flex items-start gap-2.5 rounded-md border border-amber-500/40 bg-amber-500/10 p-2.5 text-[11px] text-amber-700 dark:text-amber-300"
              >
                <AlertCircle class="mt-0.5 size-3.5 shrink-0" />
                <div class="flex-1 space-y-1">
                  <div>{{ aiStatus?.message ?? "AI backend not detected." }}</div>
                  <button
                    type="button"
                    class="font-medium underline underline-offset-2"
                    @click="router.push({ name: 'settings' })"
                  >Open Settings</button>
                </div>
              </div>
              <p v-else class="text-[11px] text-muted-foreground">
                {{ aiStatus?.message }}
              </p>

              <div class="flex justify-end gap-2 pt-2">
                <Button variant="outline" @click="backToBundle">Cancel</Button>
                <Button
                  :disabled="!aiReady || generating || !bundle?.assets.length"
                  @click="runHarmonize"
                >
                  <Sparkles />
                  Harmonize
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      </ScrollArea>

      <!-- Generating phase -->
      <div
        v-else-if="phase === 'generating'"
        class="flex h-full flex-col items-center justify-center gap-3 text-sm text-muted-foreground"
      >
        <Loader2 class="size-6 animate-spin text-primary" />
        <div>Harmonizing {{ bundle?.assets.length }} asset{{ bundle?.assets.length === 1 ? "" : "s" }}…</div>
        <p class="max-w-md text-center text-xs">
          This sends every asset's content to the model in one call —
          can take 10-30 seconds depending on the bundle size.
        </p>
      </div>

      <!-- Error phase -->
      <div
        v-else-if="phase === 'error'"
        class="flex h-full flex-col items-center justify-center gap-4 px-6 text-center"
      >
        <div class="rounded-full bg-destructive/10 p-3">
          <AlertCircle class="size-6 text-destructive" />
        </div>
        <h3 class="text-base font-semibold">Harmonization failed</h3>
        <code class="max-w-lg break-words rounded-md border bg-card/40 px-3 py-2 text-xs">
          {{ error }}
        </code>
        <div class="flex gap-2">
          <Button variant="outline" @click="phase = 'setup'">Back</Button>
          <Button @click="runHarmonize">Retry</Button>
        </div>
      </div>

      <!-- Applying phase -->
      <div
        v-else-if="phase === 'applying'"
        class="flex h-full items-center justify-center gap-2 text-sm text-muted-foreground"
      >
        <Loader2 class="size-4 animate-spin" />
        Applying changes…
      </div>

      <!-- Review phase -->
      <ScrollArea v-else-if="phase === 'review'" class="h-full">
        <div class="mx-auto w-full max-w-3xl space-y-4 px-6 py-5">
          <p class="text-xs text-muted-foreground">
            Each hunk is a contiguous block of changes. Uncheck any you don't
            want to apply — rejected hunks keep the original lines.
          </p>

          <Card v-for="state in assetStates" :key="`${state.kind}/${state.name}`">
            <CardHeader class="space-y-2 pb-3">
              <div class="flex items-center justify-between gap-2">
                <CardTitle class="flex items-center gap-2 text-sm">
                  <Badge variant="outline" class="text-[10px]">
                    {{ kindLabel(state.kind) }}
                  </Badge>
                  <span class="truncate font-mono">{{ state.name }}</span>
                </CardTitle>
                <div
                  class="flex items-center gap-2 text-[11px] text-muted-foreground"
                >
                  <template v-if="state.unchanged">
                    <Badge variant="secondary" class="text-[10px]">Unchanged</Badge>
                  </template>
                  <template v-else>
                    <span>
                      {{ state.hunks.filter((h) => h.accepted).length }} /
                      {{ state.hunks.length }} hunks
                    </span>
                    <button
                      class="text-[10.5px] underline underline-offset-2 hover:text-foreground"
                      @click="toggleAllInAsset(state, true)"
                    >Accept all</button>
                    <span class="opacity-40">·</span>
                    <button
                      class="text-[10.5px] underline underline-offset-2 hover:text-foreground"
                      @click="toggleAllInAsset(state, false)"
                    >Reject all</button>
                  </template>
                </div>
              </div>
            </CardHeader>

            <CardContent v-if="!state.unchanged" class="space-y-3 pb-4">
              <div
                v-for="(hunk, hi) in state.hunks"
                :key="hunk.index"
                class="overflow-hidden rounded-md border"
                :class="hunk.accepted ? '' : 'opacity-50'"
              >
                <header
                  class="flex items-center gap-2 border-b bg-muted/40 px-3 py-1.5"
                >
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
                <pre class="m-0 overflow-x-auto whitespace-pre-wrap bg-card px-3 py-2 font-mono text-[11.5px] leading-snug"><template
                    v-for="(part, i) in inlineWordDiff(hunk)"
                    :key="i"
                  ><span
                    v-if="part.added"
                    class="rounded-sm bg-emerald-500/25 text-emerald-700 dark:text-emerald-300"
                  >{{ part.value }}</span><span
                    v-else-if="part.removed"
                    class="rounded-sm bg-destructive/25 text-destructive line-through opacity-80"
                  >{{ part.value }}</span><span
                    v-else
                  >{{ part.value }}</span></template></pre>
              </div>
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
        {{ acceptedHunks }} / {{ totalHunks }} hunk{{ totalHunks === 1 ? "" : "s" }} accepted
      </span>
      <div class="flex-1" />
      <Button variant="outline" :disabled="applying" @click="backToBundle">
        Cancel
      </Button>
      <Button :disabled="applying || !totalHunks" @click="applyAll">
        <Loader2 v-if="applying" class="animate-spin" />
        <Save v-else />
        Apply {{ acceptedHunks }} hunk{{ acceptedHunks === 1 ? "" : "s" }}
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
