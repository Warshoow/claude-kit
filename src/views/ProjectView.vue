<script setup lang="ts">
import { computed, ref } from "vue";
import { useRouter } from "vue-router";
import { storeToRefs } from "pinia";
import {
  Boxes,
  ChevronRight,
  Folder,
  FolderOpen,
  Package,
  Trash2,
} from "lucide-vue-next";
import { useAppStore } from "@/stores/app";
import { assetKey, isAssetRef } from "@/lib/types";
import type { AssetKind, InstalledAsset } from "@/lib/types";
import { Terminal } from "lucide-vue-next";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";

const store = useAppStore();
const router = useRouter();
const { projectPath, installed, installedHooks, bundles } = storeToRefs(store);

// The trailing path segment, stripped of trailing slashes.
const projectName = computed(() => {
  if (!projectPath.value) return null;
  const trimmed = projectPath.value.replace(/[/\\]+$/, "");
  const parts = trimmed.split(/[/\\]/);
  return parts[parts.length - 1] || projectPath.value;
});

const appliedBundles = computed(() => {
  const installedSet = new Set(installed.value.map(assetKey));
  return bundles.value.filter(
    (b) =>
      b.assets.length > 0 &&
      // Step 1: only markdown-asset refs are tracked through
      // `list_installed`. Hook/mcp install tracking will arrive
      // alongside their CRUD UI — any such ref currently keeps the
      // bundle out of the "applied" set.
      b.assets.every(
        (a) => isAssetRef(a) && installedSet.has(assetKey(a)),
      ),
  );
});

const appliedAssetKeys = computed(() => {
  const s = new Set<string>();
  for (const b of appliedBundles.value) {
    for (const a of b.assets) {
      if (isAssetRef(a)) s.add(assetKey(a));
    }
  }
  return s;
});

const extras = computed<InstalledAsset[]>(() =>
  installed.value.filter((a) => !appliedAssetKeys.value.has(assetKey(a)))
);

const cleanOpen = ref(false);

async function confirmClean() {
  await store.cleanProject();
  cleanOpen.value = false;
}

function openBundle(name: string) {
  router.push({ name: "bundle-detail", params: { name } });
}

function openAsset(a: InstalledAsset) {
  router.push({
    name: "asset-detail",
    params: { kind: a.kind, name: a.name },
  });
}

function kindLabel(k: AssetKind): string {
  return k.charAt(0).toUpperCase() + k.slice(1, -1);
}
</script>

<template>
  <div class="flex h-full flex-col overflow-hidden">
    <!-- Empty state: no project selected -->
    <div
      v-if="!projectPath"
      class="flex h-full flex-col items-center justify-center px-6 text-center"
    >
      <div class="rounded-full bg-muted p-4">
        <Folder class="size-8 text-muted-foreground" />
      </div>
      <h2 class="mt-4 text-base font-semibold">No project selected</h2>
      <p class="mt-1 max-w-sm text-sm text-muted-foreground">
        Pick a project folder to see which bundles and assets are currently
        applied to its <code class="rounded bg-muted px-1 py-0.5 font-mono text-[11px]">.claude/</code> directory.
      </p>
      <Button class="mt-5" @click="store.pickProject">
        <FolderOpen />
        Pick a project
      </Button>
    </div>

    <template v-else>
      <!-- Header -->
      <div class="border-b bg-card/40 px-6 py-4">
        <div class="text-[11px] font-medium text-muted-foreground">Project</div>
        <div class="flex items-start justify-between gap-4">
          <div class="min-w-0 flex-1">
            <h1 class="truncate text-lg font-semibold">{{ projectName }}</h1>
            <p
              class="mt-0.5 truncate font-mono text-xs text-muted-foreground"
              :title="projectPath ?? ''"
            >{{ projectPath }}</p>
          </div>

          <div class="flex shrink-0 items-center gap-2">
            <Button variant="outline" size="sm" @click="store.pickProject">
              <FolderOpen />
              Switch project
            </Button>
            <Button
              v-if="installed.length > 0"
              variant="ghost"
              size="sm"
              class="text-destructive hover:bg-destructive/10 hover:text-destructive"
              @click="cleanOpen = true"
            >
              <Trash2 />
              Clean all
            </Button>
          </div>
        </div>

        <!-- Stats row -->
        <div class="mt-3 flex items-center gap-3 text-xs">
          <span class="inline-flex items-center gap-1.5 text-muted-foreground">
            <Package class="size-3.5" />
            {{ appliedBundles.length }} bundle{{ appliedBundles.length === 1 ? "" : "s" }}
          </span>
          <span class="text-muted-foreground/50">·</span>
          <span class="inline-flex items-center gap-1.5 text-muted-foreground">
            <Boxes class="size-3.5" />
            {{ extras.length }} extra{{ extras.length === 1 ? "" : "s" }}
          </span>
          <span class="text-muted-foreground/50">·</span>
          <span class="text-muted-foreground">
            {{ installed.length }} symlink{{ installed.length === 1 ? "" : "s" }} total
          </span>
        </div>
      </div>

      <!-- Body -->
      <ScrollArea class="flex-1 min-h-0">
        <div class="space-y-8 px-6 py-6">
          <!-- Empty project case -->
          <div
            v-if="installed.length === 0"
            class="rounded-lg border border-dashed bg-card/40 px-4 py-10 text-center"
          >
            <Boxes class="mx-auto size-8 text-muted-foreground" />
            <h3 class="mt-3 text-sm font-semibold">No assets in this project</h3>
            <p class="mt-1 text-xs text-muted-foreground">
              Apply a bundle from
              <RouterLink
                :to="{ name: 'bundles' }"
                class="text-primary hover:underline"
              >My Bundles</RouterLink>
              or install individual assets from
              <RouterLink
                :to="{ name: 'browse' }"
                class="text-primary hover:underline"
              >Browse</RouterLink>.
            </p>
          </div>

          <!-- Applied bundles -->
          <section v-else>
            <div class="mb-3 flex items-center gap-2">
              <h2 class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
                Applied bundles
              </h2>
              <Badge variant="secondary" class="font-mono text-[10px]">
                {{ appliedBundles.length }}
              </Badge>
            </div>

            <div
              v-if="appliedBundles.length === 0"
              class="rounded-md border border-dashed bg-card/40 px-3 py-3 text-xs italic text-muted-foreground"
            >
              No bundles fully applied here yet.
            </div>

            <div
              v-else
              class="grid gap-3"
              style="grid-template-columns: repeat(auto-fill, minmax(260px, 1fr))"
            >
              <article
                v-for="b in appliedBundles"
                :key="b.name"
                class="group flex cursor-pointer items-center justify-between gap-2 rounded-lg border bg-card p-3 transition-all hover:border-primary/40 hover:shadow-sm"
                tabindex="0"
                @click="openBundle(b.name)"
                @keydown.enter="openBundle(b.name)"
                @keydown.space.prevent="openBundle(b.name)"
              >
                <div class="flex min-w-0 items-center gap-2">
                  <Package class="size-4 shrink-0 text-muted-foreground" />
                  <div class="min-w-0">
                    <div class="truncate text-sm font-semibold">{{ b.name }}</div>
                    <div class="mt-0.5 text-[11px] text-muted-foreground">
                      {{ b.assets.length }} asset{{ b.assets.length === 1 ? "" : "s" }}
                    </div>
                  </div>
                </div>
                <ChevronRight
                  class="size-4 shrink-0 text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100"
                />
              </article>
            </div>
          </section>

          <Separator v-if="installed.length > 0 && (extras.length > 0 || installedHooks.length > 0)" />

          <!-- Installed hooks -->
          <section v-if="installedHooks.length > 0">
            <div class="mb-3 flex items-center gap-2">
              <h2 class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
                Hooks
              </h2>
              <Badge variant="secondary" class="font-mono text-[10px]">
                {{ installedHooks.length }}
              </Badge>
            </div>
            <div class="overflow-hidden rounded-lg border bg-card">
              <table class="w-full text-sm">
                <thead>
                  <tr class="border-b text-left text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
                    <th class="px-4 py-2.5">Plugin</th>
                    <th class="px-4 py-2.5">File</th>
                  </tr>
                </thead>
                <tbody class="divide-y">
                  <tr
                    v-for="h in installedHooks"
                    :key="`${h.plugin}/${h.filename}`"
                    class="transition-colors"
                  >
                    <td class="px-4 py-2.5 align-middle">
                      <Badge variant="outline" class="text-[10px]">{{ h.plugin }}</Badge>
                    </td>
                    <td class="px-4 py-2.5 align-middle">
                      <div class="flex items-center gap-1.5 font-mono text-xs">
                        <Terminal class="size-3 shrink-0 text-muted-foreground" />
                        {{ h.filename }}
                      </div>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </section>

          <Separator v-if="installedHooks.length > 0 && extras.length > 0" />

          <!-- Extras -->
          <section v-if="installed.length > 0">
            <div class="mb-3 flex items-center gap-2">
              <h2 class="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
                Extras
              </h2>
              <Badge variant="secondary" class="font-mono text-[10px]">
                {{ extras.length }}
              </Badge>
              <span class="ml-1 text-[11px] text-muted-foreground">
                — installed standalone, not part of an applied bundle
              </span>
            </div>

            <div
              v-if="extras.length === 0"
              class="rounded-md border border-dashed bg-card/40 px-3 py-3 text-xs italic text-muted-foreground"
            >
              No standalone assets — everything installed comes from a bundle.
            </div>

            <div v-else class="overflow-hidden rounded-lg border bg-card">
              <table class="w-full text-sm">
                <thead>
                  <tr class="border-b text-left text-[11px] font-medium uppercase tracking-wider text-muted-foreground">
                    <th class="px-4 py-2.5">Kind</th>
                    <th class="px-4 py-2.5">Name</th>
                  </tr>
                </thead>
                <tbody class="divide-y">
                  <tr
                    v-for="a in extras"
                    :key="assetKey(a)"
                    class="cursor-pointer transition-colors hover:bg-accent/40"
                    @click="openAsset(a)"
                  >
                    <td class="px-4 py-2.5 align-middle">
                      <Badge variant="outline" class="text-[10px]">
                        {{ kindLabel(a.kind) }}
                      </Badge>
                    </td>
                    <td class="px-4 py-2.5 align-middle font-medium">
                      {{ a.name }}
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </section>
        </div>
      </ScrollArea>
    </template>

    <!-- Clean confirm -->
    <AlertDialog v-model:open="cleanOpen">
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>Clean all symlinks?</AlertDialogTitle>
          <AlertDialogDescription>
            <span class="block">
              This removes the {{ installed.length }} symlink{{ installed.length === 1 ? "" : "s" }}
              that claude-kit created in
              <code class="rounded bg-muted px-1 py-0.5 font-mono text-[11px]">{{ projectName }}/.claude/</code>.
            </span>
            <span class="mt-2 block text-xs">
              Your library and bundles stay untouched. Any hand-written files in
              <code class="rounded bg-muted px-1 py-0.5 font-mono text-[11px]">.claude/</code>
              are not touched either — only links pointing back to your library are removed.
            </span>
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction
            class="bg-destructive text-white hover:bg-destructive/90"
            @click="confirmClean"
          >Clean all</AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  </div>
</template>
