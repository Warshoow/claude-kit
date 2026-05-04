<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { storeToRefs } from "pinia";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Folder, Minus, Square, X, Copy, Sun, Moon } from "lucide-vue-next";
import appIcon from "@/assets/icon.png";
import { useTheme } from "@/composables/useTheme";
import { useAppStore } from "@/stores/app";
import { Button } from "@/components/ui/button";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { Toaster } from "@/components/ui/sonner";

const store = useAppStore();
const route = useRoute();
const router = useRouter();

const { projectPath } = storeToRefs(store);
const { theme, toggle: toggleTheme } = useTheme();

// Detect platform for window-chrome decisions. macOS gets the native
// traffic lights overlaying the topbar (via titleBarStyle: "Overlay"
// in tauri.conf.json), so we leave space on the left and skip our own
// window controls. Other platforms run with decorations: false (set
// in the Rust setup hook), so our topbar must provide min/max/close.
const isMacOS = /Mac|iPhone|iPad/.test(navigator.userAgent);

const isMaximized = ref(false);

async function refreshMaxState() {
  try {
    isMaximized.value = await getCurrentWindow().isMaximized();
  } catch {
    /* ignored — running outside Tauri (e.g. plain vite dev) */
  }
}

async function minimize() {
  try { await getCurrentWindow().minimize(); } catch { /* noop */ }
}
async function toggleMaximize() {
  try {
    await getCurrentWindow().toggleMaximize();
    await refreshMaxState();
  } catch { /* noop */ }
}
async function closeWindow() {
  try { await getCurrentWindow().close(); } catch { /* noop */ }
}

// Manual drag handler — `data-tauri-drag-region` alone can be flaky on some
// Linux/WebKit2GTK setups (notably WSLg). Calling startDragging() ourselves
// on left mousedown is the bullet-proof path.
function isInteractive(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;
  return !!el?.closest('button, a, input, textarea, select, [role="group"], [role="button"]');
}

function onTopbarMouseDown(e: MouseEvent) {
  if (e.button !== 0) return;
  if (isInteractive(e.target)) return;
  try {
    getCurrentWindow().startDragging();
  } catch { /* outside Tauri (eg. plain vite) */ }
}

function onTopbarDblClick(e: MouseEvent) {
  if (isInteractive(e.target)) return;
  toggleMaximize();
}

// Active top-level tab — derived from the path so nested routes like
// /bundles/:name still highlight the "bundles" tab.
const currentRoute = computed(() => {
  const p = route.path;
  if (p.startsWith("/browse") || p.startsWith("/library") || p.startsWith("/plugins")) {
    return "browse";
  }
  if (p.startsWith("/project")) return "project";
  return "bundles";
});

function onSwitchView(name: string | undefined) {
  if (!name) return;
  router.push({ name });
}

onMounted(() => {
  store.refreshAll();
  refreshMaxState();
  // Track external resize → keep the maximize icon in sync.
  try {
    getCurrentWindow().onResized(() => refreshMaxState());
  } catch { /* noop */ }
});
</script>

<template>
  <div class="flex h-screen flex-col">
    <header
      data-tauri-drag-region
      class="topbar flex h-9 shrink-0 select-none items-center gap-3 border-b bg-card pr-0"
      :class="isMacOS ? 'pl-[80px]' : 'pl-3'"
      @mousedown="onTopbarMouseDown"
      @dblclick="onTopbarDblClick"
    >
      <div class="pointer-events-none flex items-center gap-1.5">
        <img :src="appIcon" alt="" class="size-4 rounded-sm" draggable="false" />
        <span class="text-[11px] font-semibold tracking-tight text-muted-foreground">claude-kit</span>
      </div>

      <ToggleGroup
        type="single"
        :model-value="currentRoute"
        @update:model-value="onSwitchView($event as string | undefined)"
        variant="outline"
        size="sm"
        class="h-7"
      >
        <ToggleGroupItem value="bundles" class="h-7 px-2.5 text-xs">My Bundles</ToggleGroupItem>
        <ToggleGroupItem value="browse" class="h-7 px-2.5 text-xs">Browse</ToggleGroupItem>
        <ToggleGroupItem value="project" class="h-7 px-2.5 text-xs">Project</ToggleGroupItem>
      </ToggleGroup>

      <div class="flex-1" />

      <button
        type="button"
        class="group flex max-w-[260px] items-center gap-1.5 rounded-md border border-transparent px-2 py-1 text-[11px] text-muted-foreground transition-colors hover:border-border hover:bg-accent/40 hover:text-foreground"
        :title="projectPath ?? 'No project selected'"
        @click="store.pickProject"
      >
        <Folder class="size-3 shrink-0" />
        <span v-if="projectPath" class="truncate font-mono">{{ projectPath }}</span>
        <span v-else class="italic">No project selected</span>
      </button>

      <button
        type="button"
        class="grid size-7 shrink-0 place-items-center rounded-md text-muted-foreground transition-colors hover:bg-secondary hover:text-foreground"
        :title="theme === 'dark' ? 'Switch to light mode' : 'Switch to dark mode'"
        @click="toggleTheme"
      >
        <Sun v-if="theme === 'dark'" class="size-3.5" />
        <Moon v-else class="size-3.5" />
      </button>

      <!-- Custom window controls (Windows / Linux only — macOS uses native traffic lights) -->
      <div v-if="!isMacOS" class="ml-1 flex h-9 shrink-0 items-stretch">
        <button
          type="button"
          class="grid w-[44px] place-items-center text-muted-foreground transition-colors hover:bg-accent/60 hover:text-foreground"
          title="Minimize"
          @click="minimize"
        >
          <Minus class="size-3.5" />
        </button>
        <button
          type="button"
          class="grid w-[44px] place-items-center text-muted-foreground transition-colors hover:bg-accent/60 hover:text-foreground"
          :title="isMaximized ? 'Restore' : 'Maximize'"
          @click="toggleMaximize"
        >
          <Copy v-if="isMaximized" class="size-3 -scale-x-100" />
          <Square v-else class="size-3" />
        </button>
        <button
          type="button"
          class="grid w-[44px] place-items-center text-muted-foreground transition-colors hover:bg-destructive hover:text-white"
          title="Close"
          @click="closeWindow"
        >
          <X class="size-3.5" />
        </button>
      </div>
    </header>

    <main class="flex-1 overflow-hidden">
      <RouterView />
    </main>

    <Toaster position="bottom-right" rich-colors />
  </div>
</template>

