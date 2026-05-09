<script setup lang="ts">
import { computed, onMounted } from "vue";
import { storeToRefs } from "pinia";
import {
  AlertCircle,
  CheckCircle2,
  Eye,
  EyeOff,
  ExternalLink,
  Lock,
  Plus,
  Save,
  Sparkles,
  Store,
  Terminal,
  Trash2,
} from "lucide-vue-next";
import { useAiStore } from "@/stores/ai";
import { useMarketplaceStore } from "@/stores/marketplace";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  ToggleGroup,
  ToggleGroupItem,
} from "@/components/ui/toggle-group";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import { ref } from "vue";

const aiStore = useAiStore();
const marketplaceStore = useMarketplaceStore();
const { settings, status, loading, generating } = storeToRefs(aiStore);
const { sources: marketplaceSources } = storeToRefs(marketplaceStore);

// Reveal toggle so the user can confirm what they typed without
// leaving the API key in plain view by default.
const showKey = ref(false);

// Add-marketplace form state.
const newMarketplaceUrl = ref("");
const addingMarketplace = ref(false);

async function submitAddMarketplace() {
  const url = newMarketplaceUrl.value.trim();
  if (!url || addingMarketplace.value) return;
  addingMarketplace.value = true;
  try {
    const ok = await marketplaceStore.addSource(url);
    if (ok) newMarketplaceUrl.value = "";
  } finally {
    addingMarketplace.value = false;
  }
}

async function removeMarketplace(url: string) {
  await marketplaceStore.removeSource(url);
}

const showApiFields = computed(
  () => settings.value.ai.mode === "auto" || settings.value.ai.mode === "api"
);

const statusIcon = computed(() => {
  if (!status.value) return null;
  return status.value.mode === "none" ? AlertCircle : CheckCircle2;
});

const statusTone = computed(() => {
  if (!status.value) return "muted";
  return status.value.mode === "none" ? "warning" : "ok";
});

async function save() {
  await aiStore.saveSettings();
}

onMounted(async () => {
  await Promise.all([
    aiStore.loadSettings(),
    aiStore.refreshStatus(),
    marketplaceStore.loadSources(),
  ]);
});
</script>

<template>
  <div class="flex h-full flex-col overflow-hidden">
    <!-- Header -->
    <div class="flex items-center gap-3 border-b bg-card/40 px-6 py-3">
      <h2 class="text-sm font-semibold">Settings</h2>
      <div class="flex-1" />
      <Button
        size="sm"
        :disabled="loading || generating"
        @click="save"
      >
        <Save />
        Save
      </Button>
    </div>

    <ScrollArea class="flex-1 min-h-0">
      <div class="mx-auto w-full max-w-2xl space-y-6 px-6 py-6">
        <!-- AI section -->
        <Card>
          <CardHeader>
            <CardTitle class="flex items-center gap-2">
              <Sparkles class="size-4" />
              AI integration
            </CardTitle>
            <CardDescription>
              Powers the "Generate with AI" buttons in the asset editor and
              new-asset dialog. claude-kit auto-detects the
              <code class="rounded bg-muted px-1 py-0.5 font-mono text-[11px]">claude</code>
              CLI; if it isn't installed, configure any OpenAI-compatible
              endpoint instead.
            </CardDescription>
          </CardHeader>
          <CardContent class="space-y-5">
            <!-- Status banner -->
            <div
              v-if="status"
              class="flex items-start gap-2.5 rounded-md border p-3 text-xs"
              :class="
                statusTone === 'warning'
                  ? 'border-amber-500/40 bg-amber-500/10 text-amber-700 dark:text-amber-300'
                  : 'border-primary/30 bg-primary/5 text-foreground'
              "
            >
              <component
                :is="statusIcon"
                class="mt-0.5 size-3.5 shrink-0"
              />
              <div class="space-y-0.5">
                <div class="font-medium">{{ status.message }}</div>
                <div
                  v-if="status.claude_cli_path"
                  class="font-mono text-[10.5px] opacity-70"
                >{{ status.claude_cli_path }}</div>
              </div>
            </div>

            <!-- Mode selector -->
            <div class="space-y-1.5">
              <Label>Backend</Label>
              <ToggleGroup
                type="single"
                :model-value="settings.ai.mode"
                @update:model-value="(v) => v && (settings.ai.mode = v as typeof settings.ai.mode)"
                variant="outline"
                class="w-full"
              >
                <ToggleGroupItem value="auto" class="flex-1">
                  Auto
                </ToggleGroupItem>
                <ToggleGroupItem value="claude-cli" class="flex-1">
                  <Terminal class="size-3" />
                  Claude CLI
                </ToggleGroupItem>
                <ToggleGroupItem value="api" class="flex-1">
                  API
                </ToggleGroupItem>
              </ToggleGroup>
              <p class="text-[11px] text-muted-foreground">
                <strong>Auto</strong> uses the Claude CLI if found, otherwise
                falls back to your API config.
                <strong>Claude CLI</strong> forces the local binary.
                <strong>API</strong> forces the remote provider.
              </p>
            </div>

            <Separator v-if="showApiFields" />

            <!-- API config -->
            <div v-if="showApiFields" class="space-y-3">
              <div class="space-y-1.5">
                <Label for="ai-base-url">API base URL</Label>
                <Input
                  id="ai-base-url"
                  v-model="settings.ai.api_base_url"
                  type="text"
                  placeholder="https://api.openai.com/v1"
                />
                <p class="text-[11px] text-muted-foreground">
                  The chat-completions endpoint will be
                  <code class="font-mono text-[10.5px]">{base_url}/chat/completions</code>.
                  Examples: <span class="font-mono">https://api.anthropic.com/v1</span>,
                  <span class="font-mono">http://localhost:11434/v1</span> (Ollama),
                  <span class="font-mono">http://localhost:1234/v1</span> (LM Studio).
                </p>
              </div>

              <div class="space-y-1.5">
                <Label for="ai-key">API key</Label>
                <div class="relative">
                  <Input
                    id="ai-key"
                    v-model="settings.ai.api_key"
                    :type="showKey ? 'text' : 'password'"
                    placeholder="sk-…"
                    class="pr-9 font-mono"
                    autocomplete="off"
                  />
                  <button
                    type="button"
                    class="absolute right-1.5 top-1/2 grid size-7 -translate-y-1/2 place-items-center rounded-sm text-muted-foreground hover:bg-accent hover:text-foreground"
                    :title="showKey ? 'Hide key' : 'Show key'"
                    @click="showKey = !showKey"
                  >
                    <EyeOff v-if="showKey" class="size-3.5" />
                    <Eye v-else class="size-3.5" />
                  </button>
                </div>
                <p class="text-[11px] text-muted-foreground">
                  Stored in plain text in
                  <code class="font-mono text-[10.5px]">~/.claude-assets/settings.json</code>.
                </p>
              </div>

              <div class="space-y-1.5">
                <Label for="ai-model">Model</Label>
                <Input
                  id="ai-model"
                  v-model="settings.ai.api_model"
                  type="text"
                  placeholder="claude-sonnet-4-5"
                />
                <p class="text-[11px] text-muted-foreground">
                  Optional. Sent as the <code class="font-mono text-[10.5px]">model</code>
                  field. Defaults to <span class="font-mono">claude-sonnet-4-5</span>.
                </p>
              </div>
            </div>
          </CardContent>
        </Card>

        <!-- Marketplaces -->
        <Card>
          <CardHeader>
            <CardTitle class="flex items-center gap-2">
              <Store class="size-4" />
              Marketplaces
            </CardTitle>
            <CardDescription>
              Sources for the Browse > Marketplace catalog. Each entry is
              just a URL to a
              <code class="rounded bg-muted px-1 py-0.5 font-mono text-[11px]">marketplace.json</code>
              file. The official Anthropic marketplace is built-in and
              can't be removed.
            </CardDescription>
          </CardHeader>
          <CardContent class="space-y-4">
            <!-- Existing marketplaces list -->
            <div class="space-y-1.5">
              <div
                v-for="src in marketplaceSources"
                :key="src.url"
                class="flex items-start gap-2 rounded-md border bg-card/40 p-2.5"
              >
                <div class="min-w-0 flex-1">
                  <div class="flex items-center gap-2">
                    <span class="truncate text-sm font-medium">{{ src.name }}</span>
                    <Badge
                      v-if="src.builtin"
                      variant="outline"
                      class="gap-1 border-primary/40 text-[10px] uppercase tracking-wider text-primary"
                    >
                      <Lock class="size-2.5" />
                      Built-in
                    </Badge>
                  </div>
                  <a
                    :href="src.url"
                    target="_blank"
                    rel="noopener"
                    class="mt-0.5 inline-flex items-center gap-1 break-all font-mono text-[11px] text-muted-foreground transition-colors hover:text-foreground"
                  >
                    {{ src.url }}
                    <ExternalLink class="size-2.5 shrink-0" />
                  </a>
                </div>
                <Button
                  v-if="!src.builtin"
                  variant="ghost"
                  size="sm"
                  class="text-destructive hover:bg-destructive/10 hover:text-destructive"
                  title="Remove this marketplace"
                  @click="removeMarketplace(src.url)"
                >
                  <Trash2 class="size-3.5" />
                </Button>
              </div>
            </div>

            <Separator />

            <!-- Add marketplace form -->
            <form class="space-y-1.5" @submit.prevent="submitAddMarketplace">
              <Label for="add-marketplace-url">Add a marketplace</Label>
              <div class="flex items-start gap-2">
                <Input
                  id="add-marketplace-url"
                  v-model="newMarketplaceUrl"
                  type="url"
                  placeholder="https://…/marketplace.json"
                  class="font-mono text-[12px]"
                />
                <Button
                  :disabled="!newMarketplaceUrl.trim() || addingMarketplace"
                  @click="submitAddMarketplace"
                >
                  <Plus />
                  {{ addingMarketplace ? "Adding…" : "Add" }}
                </Button>
              </div>
              <p class="text-[11px] text-muted-foreground">
                The URL is fetched once to read its self-declared
                <code class="font-mono text-[10.5px]">name</code> field. If it
                doesn't return a valid marketplace JSON, it's rejected
                without saving.
              </p>
            </form>
          </CardContent>
        </Card>
      </div>
    </ScrollArea>
  </div>
</template>
