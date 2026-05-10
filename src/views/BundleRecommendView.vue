<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { storeToRefs } from "pinia";
import {
  AlertCircle,
  ArrowLeft,
  Check,
  Loader2,
  Package,
  Sparkles,
} from "lucide-vue-next";
import { toast } from "vue-sonner";
import { useAppStore } from "@/stores/app";
import { useBundleStore } from "@/stores/bundle";
import { useAiStore } from "@/stores/ai";
import { api } from "@/lib/api";
import type {
  AssetKind,
  RecommendationResult,
  RecommendedAsset,
  RecommendedPlugin,
} from "@/lib/types";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
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

const router = useRouter();
const store = useAppStore();
const bundleStore = useBundleStore();
const aiStore = useAiStore();
const { bundles } = storeToRefs(store);
const { status: aiStatus } = storeToRefs(aiStore);

type Phase = "setup" | "generating" | "review" | "applying" | "error";

const phase = ref<Phase>("setup");
const error = ref<string | null>(null);
const userNeed = ref("");

// Editable fields populated from the AI's response, plus per-line acceptance.
const recommendation = ref<RecommendationResult | null>(null);
const bundleName = ref("");
const bundleDescription = ref("");
const pluginAccepted = ref<Map<string, boolean>>(new Map());
const assetAccepted = ref<Map<string, boolean>>(new Map());

const aiReady = computed(
  () => aiStatus.value !== null && aiStatus.value.mode !== "none"
);

function assetMapKey(a: RecommendedAsset): string {
  return `${a.kind}:${a.name}`;
}

const acceptedPluginCount = computed(() => {
  let n = 0;
  for (const v of pluginAccepted.value.values()) if (v) n++;
  return n;
});
const acceptedAssetCount = computed(() => {
  let n = 0;
  for (const v of assetAccepted.value.values()) if (v) n++;
  return n;
});

const nameCollision = computed(() => {
  const t = bundleName.value.trim();
  if (!t) return false;
  return bundles.value.some((b) => b.name === t);
});

const nameValid = computed(() => {
  const t = bundleName.value.trim();
  if (!t) return false;
  if (t.length > 64) return false;
  return /^[A-Za-z0-9_-]+$/.test(t);
});

const canApply = computed(
  () =>
    phase.value === "review" &&
    nameValid.value &&
    !nameCollision.value &&
    acceptedAssetCount.value > 0
);

// Group assets by their owning plugin in a stable order: plugins first
// in the order recommended (so the AI's prioritisation is preserved),
// then any "leftover" assets without a matching plugin.
const assetsByPlugin = computed<Array<{ plugin: string; assets: RecommendedAsset[] }>>(() => {
  if (!recommendation.value) return [];
  const buckets = new Map<string, RecommendedAsset[]>();
  for (const a of recommendation.value.assets) {
    const key = a.plugin || "(local / pre-existing)";
    if (!buckets.has(key)) buckets.set(key, []);
    buckets.get(key)!.push(a);
  }
  // Order: recommended plugins first, then anything else alphabetically.
  const ordered: Array<{ plugin: string; assets: RecommendedAsset[] }> = [];
  for (const rp of recommendation.value.plugins_to_import) {
    const items = buckets.get(rp.name);
    if (items) {
      ordered.push({ plugin: rp.name, assets: items });
      buckets.delete(rp.name);
    }
  }
  const rest = Array.from(buckets.entries()).sort(([a], [b]) => a.localeCompare(b));
  for (const [k, v] of rest) ordered.push({ plugin: k, assets: v });
  return ordered;
});

// ── Phase actions ──────────────────────────────────────────────

async function runRecommend() {
  if (!aiReady.value || !userNeed.value.trim()) return;
  phase.value = "generating";
  error.value = null;
  try {
    const result = await api.recommendBundle(userNeed.value.trim());
    recommendation.value = result;
    bundleName.value = result.bundle_name || "";
    bundleDescription.value = result.bundle_description || "";

    // Default acceptance: every plugin that's NOT already imported = checked,
    // every asset = checked. Already-imported plugins stay unchecked because
    // there's nothing to do for them.
    pluginAccepted.value = new Map(
      result.plugins_to_import.map((p) => [p.name, !p.already_imported])
    );
    assetAccepted.value = new Map(
      result.assets.map((a) => [assetMapKey(a), true])
    );

    phase.value = "review";
  } catch (e) {
    error.value = String(e);
    phase.value = "error";
  }
}

async function apply() {
  if (!recommendation.value || !canApply.value) return;
  phase.value = "applying";

  // 1. Import plugins the user kept checked.
  const importTargets = recommendation.value.plugins_to_import.filter(
    (rp) => pluginAccepted.value.get(rp.name) && !rp.already_imported
  );
  let importFailures = 0;
  for (const rp of importTargets) {
    if (!rp.plugin) {
      importFailures++;
      continue;
    }
    try {
      await api.importMarketplacePlugin(rp.plugin);
    } catch (e) {
      console.error(`import ${rp.name} failed`, e);
      importFailures++;
    }
  }
  await store.refreshLibrary();

  // 2. Resolve checked assets against the post-import library. The model
  // occasionally invents asset names — the recommendation prompt only
  // shows it plugin names + descriptions, not each plugin's actual
  // contents, so it has to guess. Anything that doesn't materialize is
  // dropped from the bundle but reported back to the user so the gap
  // doesn't feel silent.
  const libIndex = new Set(store.library.map((a) => `${a.kind}:${a.name}`));
  const acceptedAssets = recommendation.value.assets.filter((a) =>
    assetAccepted.value.get(assetMapKey(a))
  );
  const found = acceptedAssets.filter((a) => libIndex.has(`${a.kind}:${a.name}`));
  const missing = acceptedAssets.filter((a) => !libIndex.has(`${a.kind}:${a.name}`));

  if (found.length === 0) {
    phase.value = "review";
    toast.error("Nothing to bundle", {
      description:
        "After imports, none of the accepted assets are in the library — the model likely invented those names. Try keeping more plugins checked, or pick assets manually.",
    });
    return;
  }

  // 3. Create the bundle then attach its refs.
  const trimmedName = bundleName.value.trim();
  try {
    await bundleStore.createBundle(
      trimmedName,
      bundleDescription.value.trim() || undefined
    );
    await api.setBundleAssets(
      trimmedName,
      found.map((a) => ({ kind: a.kind as AssetKind, name: a.name }))
    );
    await store.refreshBundles();
  } catch (e) {
    error.value = String(e);
    phase.value = "error";
    return;
  }

  // 4. Tell the user exactly what landed where so the gap between
  // "I checked 12 things" and "the bundle has 8" never feels silent.
  const summary = `Bundle "${trimmedName}" ready · ${found.length} asset${found.length === 1 ? "" : "s"}`;
  const notes: string[] = [];
  if (importFailures > 0) {
    notes.push(
      `${importFailures} plugin import${importFailures === 1 ? "" : "s"} failed`
    );
  }
  if (missing.length > 0) {
    const sample = missing.slice(0, 3).map((m) => `${m.kind}/${m.name}`).join(", ");
    const overflow = missing.length > 3 ? ` (+${missing.length - 3} more)` : "";
    notes.push(
      `${missing.length} suggested asset${missing.length === 1 ? "" : "s"} weren't found after import (model invented names): ${sample}${overflow}`
    );
  }

  if (notes.length === 0) {
    toast.success(summary);
  } else {
    toast.warning(summary, { description: notes.join(". ") });
  }
  router.push({ name: "bundle-detail", params: { name: trimmedName } });
}

function backToBundles() {
  router.push({ name: "bundles" });
}

function kindLabel(k: AssetKind): string {
  if (k === "skills") return "Skill";
  if (k === "commands") return "Command";
  return "Agent";
}

function pluginIsAccepted(p: RecommendedPlugin): boolean {
  return pluginAccepted.value.get(p.name) ?? false;
}

function setPluginAccepted(p: RecommendedPlugin, v: boolean) {
  pluginAccepted.value.set(p.name, v);
  // Cascade: if a plugin is unchecked, also uncheck its assets so the
  // counts stay coherent. Re-checking the plugin doesn't auto-recheck
  // assets — the user might have deliberately rejected some.
  if (!v && recommendation.value) {
    for (const a of recommendation.value.assets) {
      if (a.plugin === p.name) assetAccepted.value.set(assetMapKey(a), false);
    }
  }
}

onMounted(async () => {
  await Promise.all([
    bundles.value.length ? Promise.resolve() : store.refreshBundles(),
    aiStore.refreshStatus(),
  ]);
});
</script>

<template>
  <div class="flex h-full flex-col overflow-hidden">
    <!-- Header -->
    <div class="border-b bg-card/40 px-6 py-4">
      <button
        class="mb-2 inline-flex items-center gap-1 text-xs text-muted-foreground transition-colors hover:text-foreground"
        @click="backToBundles"
      >
        <ArrowLeft class="size-3" />
        Back to bundles
      </button>

      <div class="flex items-center gap-2">
        <Sparkles class="size-4 shrink-0 text-primary" />
        <h1 class="truncate text-lg font-semibold">Recommend a bundle</h1>
      </div>
      <p class="mt-1 text-sm text-muted-foreground">
        Describe what you want and let the model assemble a bundle from
        your library and the marketplace.
      </p>
    </div>

    <!-- Body -->
    <div class="relative flex-1 min-h-0 overflow-hidden">
      <!-- Setup -->
      <ScrollArea v-if="phase === 'setup'" class="h-full">
        <div class="mx-auto w-full max-w-2xl space-y-5 px-6 py-6">
          <Card>
            <CardHeader>
              <CardTitle class="flex items-center gap-2">
                <Sparkles class="size-4" />
                What should the bundle do?
              </CardTitle>
            </CardHeader>
            <CardContent class="space-y-4">
              <div class="space-y-1.5">
                <Label for="recommend-need">Your need</Label>
                <Textarea
                  id="recommend-need"
                  v-model="userNeed"
                  rows="5"
                  placeholder="e.g. TypeScript backend with strict types, Vitest for tests, and pre-commit hooks for lint + format."
                  autofocus
                />
                <p class="text-[11px] text-muted-foreground">
                  More specific = better. Mention languages, frameworks,
                  and conventions you care about.
                </p>
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
                <Button variant="outline" @click="backToBundles">Cancel</Button>
                <Button
                  :disabled="!aiReady || !userNeed.trim()"
                  @click="runRecommend"
                >
                  <Sparkles />
                  Recommend
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      </ScrollArea>

      <!-- Generating -->
      <div
        v-else-if="phase === 'generating'"
        class="flex h-full flex-col items-center justify-center gap-3 text-sm text-muted-foreground"
      >
        <Loader2 class="size-6 animate-spin text-primary" />
        <div>Picking plugins and assembling a bundle…</div>
        <p class="max-w-md text-center text-xs">
          The model is reading the marketplace catalog and your library —
          can take 10-30 seconds.
        </p>
      </div>

      <!-- Error -->
      <div
        v-else-if="phase === 'error'"
        class="flex h-full flex-col items-center justify-center gap-4 px-6 text-center"
      >
        <div class="rounded-full bg-destructive/10 p-3">
          <AlertCircle class="size-6 text-destructive" />
        </div>
        <h3 class="text-base font-semibold">Recommendation failed</h3>
        <code class="max-w-lg break-words rounded-md border bg-card/40 px-3 py-2 text-xs">
          {{ error }}
        </code>
        <div class="flex gap-2">
          <Button variant="outline" @click="phase = 'setup'">Back</Button>
          <Button @click="runRecommend">Retry</Button>
        </div>
      </div>

      <!-- Applying -->
      <div
        v-else-if="phase === 'applying'"
        class="flex h-full items-center justify-center gap-2 text-sm text-muted-foreground"
      >
        <Loader2 class="size-4 animate-spin" />
        Importing plugins and creating the bundle…
      </div>

      <!-- Review -->
      <ScrollArea v-else-if="phase === 'review' && recommendation" class="h-full">
        <div class="mx-auto w-full max-w-3xl space-y-5 px-6 py-5">
          <!-- Bundle metadata -->
          <Card>
            <CardHeader>
              <CardTitle class="text-sm">Bundle</CardTitle>
            </CardHeader>
            <CardContent class="space-y-3">
              <div class="space-y-1.5">
                <Label for="r-name">Name</Label>
                <Input
                  id="r-name"
                  v-model="bundleName"
                  placeholder="kebab-case-name"
                />
                <p
                  v-if="bundleName.trim() && !nameValid"
                  class="text-[11px] text-destructive"
                >Letters, digits, '-' or '_' only (max 64 chars).</p>
                <p
                  v-else-if="nameCollision"
                  class="text-[11px] text-destructive"
                >A bundle named "{{ bundleName.trim() }}" already exists.</p>
              </div>

              <div class="space-y-1.5">
                <Label for="r-desc">
                  Description
                  <span class="text-muted-foreground">(optional)</span>
                </Label>
                <Input
                  id="r-desc"
                  v-model="bundleDescription"
                  placeholder="One-sentence summary"
                />
              </div>
            </CardContent>
          </Card>

          <!-- Rationale -->
          <Card v-if="recommendation.rationale">
            <CardHeader>
              <CardTitle class="text-sm">Why this bundle</CardTitle>
            </CardHeader>
            <CardContent>
              <p class="text-sm leading-relaxed text-muted-foreground">
                {{ recommendation.rationale }}
              </p>
            </CardContent>
          </Card>

          <!-- Plugins to import -->
          <Card v-if="recommendation.plugins_to_import.length">
            <CardHeader>
              <CardTitle class="flex items-center justify-between gap-2 text-sm">
                <span>Plugins to import</span>
                <span class="text-[11px] font-normal text-muted-foreground">
                  {{ acceptedPluginCount }} / {{ recommendation.plugins_to_import.length }} selected
                </span>
              </CardTitle>
            </CardHeader>
            <CardContent class="space-y-1.5">
              <label
                v-for="p in recommendation.plugins_to_import"
                :key="p.name"
                class="flex cursor-pointer items-start gap-3 rounded-md p-2 transition-colors hover:bg-accent/40"
              >
                <Checkbox
                  :model-value="pluginIsAccepted(p)"
                  :disabled="p.already_imported"
                  @update:model-value="(v) => setPluginAccepted(p, !!v)"
                  class="mt-0.5"
                />
                <div class="min-w-0 flex-1">
                  <div class="flex items-center gap-2">
                    <Package class="size-3.5 shrink-0 text-muted-foreground" />
                    <span class="text-sm font-medium">{{ p.name }}</span>
                    <Badge
                      v-if="p.already_imported"
                      variant="outline"
                      class="border-primary/40 text-[10px] uppercase tracking-wider text-primary"
                    >
                      <Check class="size-2.5" />
                      Already in library
                    </Badge>
                  </div>
                  <p
                    v-if="p.reason"
                    class="mt-0.5 text-xs leading-snug text-muted-foreground"
                  >{{ p.reason }}</p>
                </div>
              </label>
            </CardContent>
          </Card>

          <!-- Assets to bundle -->
          <Card>
            <CardHeader>
              <CardTitle class="flex items-center justify-between gap-2 text-sm">
                <span>Assets in this bundle</span>
                <span class="text-[11px] font-normal text-muted-foreground">
                  {{ acceptedAssetCount }} / {{ recommendation.assets.length }} selected
                </span>
              </CardTitle>
            </CardHeader>
            <CardContent class="space-y-4">
              <div
                v-for="group in assetsByPlugin"
                :key="group.plugin"
                class="space-y-1"
              >
                <div
                  class="px-1 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground"
                >from {{ group.plugin }}</div>
                <label
                  v-for="a in group.assets"
                  :key="`${a.kind}/${a.name}`"
                  class="flex cursor-pointer items-start gap-3 rounded-md p-2 transition-colors hover:bg-accent/40"
                >
                  <Checkbox
                    :model-value="assetAccepted.get(assetMapKey(a)) ?? false"
                    @update:model-value="(v) => assetAccepted.set(assetMapKey(a), !!v)"
                    class="mt-0.5"
                  />
                  <div class="min-w-0 flex-1">
                    <div class="flex items-center gap-2">
                      <Badge variant="outline" class="text-[10px]">
                        {{ kindLabel(a.kind) }}
                      </Badge>
                      <span class="font-mono text-sm">{{ a.name }}</span>
                      <Badge
                        v-if="a.already_in_library"
                        variant="secondary"
                        class="text-[10px]"
                      >In library</Badge>
                    </div>
                    <p
                      v-if="a.reason"
                      class="mt-0.5 text-xs leading-snug text-muted-foreground"
                    >{{ a.reason }}</p>
                  </div>
                </label>
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
        {{ acceptedPluginCount }} plugin{{ acceptedPluginCount === 1 ? "" : "s" }}
        ·
        {{ acceptedAssetCount }} asset{{ acceptedAssetCount === 1 ? "" : "s" }}
      </span>
      <div class="flex-1" />
      <Button variant="outline" @click="phase = 'setup'">Back</Button>
      <Button :disabled="!canApply" @click="apply">
        <Check />
        Create bundle
      </Button>
    </div>
  </div>
</template>
