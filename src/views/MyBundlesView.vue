<script setup lang="ts">
import { computed, ref } from "vue";
import { useRouter } from "vue-router";
import { storeToRefs } from "pinia";
import { Plus, Package, Boxes, ChevronRight } from "lucide-vue-next";
import { useAppStore } from "@/stores/app";
import { assetKey } from "@/lib/types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

const store = useAppStore();
const router = useRouter();
const { bundles, installedKeys } = storeToRefs(store);

const createOpen = ref(false);
const newName = ref("");
const newDesc = ref("");
const submitting = ref(false);

// A bundle is "applied" when every one of its assets is currently installed
// in the active project. Use it to surface a small badge on the card.
const appliedBundleNames = computed(() => {
  const names = new Set<string>();
  for (const b of bundles.value) {
    if (b.assets.length === 0) continue;
    if (b.assets.every((a) => installedKeys.value.has(assetKey(a)))) {
      names.add(b.name);
    }
  }
  return names;
});

function openCreate() {
  newName.value = "";
  newDesc.value = "";
  createOpen.value = true;
}

async function submitCreate() {
  const name = newName.value.trim();
  if (!name || submitting.value) return;
  submitting.value = true;
  try {
    await store.createBundle(name, newDesc.value.trim() || undefined);
    createOpen.value = false;
    router.push({ name: "bundle-detail", params: { name } });
  } finally {
    submitting.value = false;
  }
}

function openBundle(name: string) {
  router.push({ name: "bundle-detail", params: { name } });
}
</script>

<template>
  <div class="flex h-full flex-col overflow-hidden">
    <!-- Header -->
    <div class="flex items-center gap-3 border-b bg-card/40 px-6 py-3">
      <h2 class="text-sm font-semibold">My Bundles</h2>
      <span class="text-xs text-muted-foreground">{{ bundles.length }}</span>
      <div class="flex-1" />
      <Button size="sm" @click="openCreate">
        <Plus />
        Create bundle
      </Button>
    </div>

    <!-- Grid -->
    <ScrollArea class="flex-1 min-h-0">
      <div
        v-if="bundles.length === 0"
        class="flex h-full min-h-[400px] flex-col items-center justify-center px-6 py-16 text-center"
      >
        <div class="rounded-full bg-muted p-4">
          <Boxes class="size-8 text-muted-foreground" />
        </div>
        <h3 class="mt-4 text-base font-semibold">No bundles yet</h3>
        <p class="mt-1 max-w-sm text-sm text-muted-foreground">
          Bundles let you group skills, commands and agents that go together,
          and apply them to a project in one click.
        </p>
        <Button class="mt-5" @click="openCreate">
          <Plus />
          Create your first bundle
        </Button>
      </div>

      <div
        v-else
        class="grid gap-4 p-6"
        style="grid-template-columns: repeat(auto-fill, minmax(280px, 1fr))"
      >
        <article
          v-for="b in bundles"
          :key="b.name"
          class="group flex cursor-pointer flex-col rounded-xl border bg-card p-4 transition-all hover:-translate-y-0.5 hover:border-primary/40 hover:shadow-md focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background"
          tabindex="0"
          @click="openBundle(b.name)"
          @keydown.enter="openBundle(b.name)"
          @keydown.space.prevent="openBundle(b.name)"
        >
          <header class="flex items-start justify-between gap-2">
            <div class="flex min-w-0 items-center gap-2">
              <Package class="size-4 shrink-0 text-muted-foreground" />
              <h3 class="truncate text-sm font-semibold">{{ b.name }}</h3>
            </div>
            <ChevronRight
              class="size-4 shrink-0 text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100"
            />
          </header>

          <p
            v-if="b.description"
            class="mt-2 line-clamp-2 text-xs leading-relaxed text-muted-foreground"
          >{{ b.description }}</p>
          <p v-else class="mt-2 text-xs italic text-muted-foreground/60">
            No description
          </p>

          <div class="mt-3 flex items-center gap-2 pt-1">
            <Badge variant="secondary" class="font-mono text-[10px]">
              {{ b.assets.length }} asset{{ b.assets.length === 1 ? "" : "s" }}
            </Badge>
            <Badge
              v-if="appliedBundleNames.has(b.name)"
              variant="outline"
              class="text-[10px] font-medium uppercase tracking-wider text-primary"
            >Applied</Badge>
          </div>
        </article>
      </div>
    </ScrollArea>

    <!-- Create dialog -->
    <Dialog v-model:open="createOpen">
      <DialogContent class="sm:max-w-md">
        <DialogHeader>
          <DialogTitle>Create a bundle</DialogTitle>
          <DialogDescription>
            A bundle groups library assets you want to apply together to a project.
          </DialogDescription>
        </DialogHeader>
        <form class="space-y-3" @submit.prevent="submitCreate">
          <div class="space-y-1.5">
            <Label for="bundle-name">Name</Label>
            <Input
              id="bundle-name"
              v-model="newName"
              placeholder="e.g. python-backend"
              autofocus
              required
            />
          </div>
          <div class="space-y-1.5">
            <Label for="bundle-desc">
              Description
              <span class="text-muted-foreground">(optional)</span>
            </Label>
            <Input
              id="bundle-desc"
              v-model="newDesc"
              placeholder="What does this bundle pull together?"
            />
          </div>
        </form>
        <DialogFooter>
          <Button variant="outline" @click="createOpen = false">Cancel</Button>
          <Button
            :disabled="!newName.trim() || submitting"
            @click="submitCreate"
          >{{ submitting ? "Creating…" : "Create bundle" }}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  </div>
</template>
