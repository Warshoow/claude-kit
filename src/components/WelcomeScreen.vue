<script setup lang="ts">
import { useRouter } from "vue-router";
import { Store, Globe, Plus } from "lucide-vue-next";
import { Button } from "@/components/ui/button";
import { usePluginStore } from "@/stores/plugin";
import appIcon from "@/assets/icon.png";

const emit = defineEmits<{ dismiss: [] }>();

const router = useRouter();
const pluginStore = usePluginStore();

function goMarketplace() {
  emit("dismiss");
  router.push({ name: "browse-marketplace" });
}

function importLocal() {
  emit("dismiss");
  pluginStore.importLocalPlugin();
}

function startScratch() {
  emit("dismiss");
  router.push({ name: "browse-library" });
}
</script>

<template>
  <div class="flex h-full flex-col items-center justify-center gap-8 px-8 py-16">
    <div class="flex flex-col items-center gap-4 text-center">
      <img :src="appIcon" alt="claude-kit" class="size-20 rounded-2xl shadow-lg" draggable="false" />
      <div>
        <h1 class="text-2xl font-bold tracking-tight">Welcome to claude-kit</h1>
        <p class="mt-2 max-w-sm text-sm text-muted-foreground">
          Manage your Claude Code skills, commands and agents as reusable
          bundles — and apply them to any project in one click.
        </p>
      </div>
    </div>

    <div class="flex w-full max-w-sm flex-col gap-3">
      <Button class="h-12 w-full gap-3 text-sm" @click="goMarketplace">
        <Globe class="size-4 shrink-0" />
        <span class="flex flex-col text-left leading-tight">
          <span class="font-semibold">Browse Marketplace</span>
          <span class="text-xs font-normal opacity-80">Import plugins from the official catalog</span>
        </span>
      </Button>

      <Button variant="outline" class="h-12 w-full gap-3 text-sm" @click="importLocal">
        <Store class="size-4 shrink-0" />
        <span class="flex flex-col text-left leading-tight">
          <span class="font-semibold">Import a local plugin</span>
          <span class="text-xs font-normal opacity-70">Pick a plugin folder from your machine</span>
        </span>
      </Button>

      <Button variant="ghost" class="h-10 w-full gap-2 text-xs text-muted-foreground" @click="startScratch">
        <Plus class="size-3.5" />
        Start from scratch — create your first asset
      </Button>
    </div>
  </div>
</template>
