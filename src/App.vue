<script setup lang="ts">
import { computed, onMounted } from "vue";
import { useRoute, useRouter } from "vue-router";
import { storeToRefs } from "pinia";
import { Folder } from "lucide-vue-next";
import { useAppStore } from "@/stores/app";
import { Button } from "@/components/ui/button";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { Toaster } from "@/components/ui/sonner";
import AssetEditor from "@/components/AssetEditor.vue";
import PluginDetail from "@/components/PluginDetail.vue";

const store = useAppStore();
const route = useRoute();
const router = useRouter();

const {
  projectPath,
  selectedPlugin,
  readme,
  readmeLoading,
  readmeError,
  importingPlugin,
  editing,
} = storeToRefs(store);

// Active top-level tab — derived from the path so nested routes like
// /bundles/:name still highlight the "bundles" tab.
const currentRoute = computed(() => {
  const p = route.path;
  if (p.startsWith("/browse")) return "browse";
  if (p.startsWith("/project")) return "project";
  return "bundles";
});

function onSwitchView(name: string | undefined) {
  if (!name) return;
  router.push({ name });
}

const importingSelectedPlugin = computed(
  () =>
    !!selectedPlugin.value && importingPlugin.value === selectedPlugin.value.name
);

onMounted(() => store.refreshAll());
</script>

<template>
  <div class="flex h-screen flex-col">
    <header
      class="flex items-center gap-4 border-b bg-card/50 px-4 py-2.5 backdrop-blur"
    >
      <h1 class="text-sm font-semibold tracking-tight">claude-kit</h1>

      <ToggleGroup
        type="single"
        :model-value="currentRoute"
        @update:model-value="onSwitchView($event as string | undefined)"
        variant="outline"
        size="sm"
      >
        <ToggleGroupItem value="bundles">My Bundles</ToggleGroupItem>
        <ToggleGroupItem value="browse">Browse</ToggleGroupItem>
        <ToggleGroupItem value="project">Project</ToggleGroupItem>
      </ToggleGroup>

      <div class="flex-1" />

      <span
        v-if="projectPath"
        class="max-w-[420px] truncate font-mono text-xs text-muted-foreground"
        :title="projectPath"
      >{{ projectPath }}</span>
      <span v-else class="text-xs italic text-muted-foreground">No project selected</span>
      <Button variant="outline" size="sm" @click="store.pickProject">
        <Folder />
        {{ projectPath ? "Change" : "Pick project" }}
      </Button>
    </header>

    <main class="flex-1 overflow-hidden">
      <RouterView />
    </main>

    <PluginDetail
      :plugin="selectedPlugin"
      :readme="readme"
      :readme-loading="readmeLoading"
      :readme-error="readmeError"
      :importing="importingSelectedPlugin"
      @close="store.closePluginDetail"
      @import="store.importMarketplacePlugin"
    />

    <AssetEditor
      :asset="editing"
      @close="store.stopEditing"
      @saved="store.onEditorSaved"
    />

    <Toaster position="bottom-right" rich-colors />
  </div>
</template>
