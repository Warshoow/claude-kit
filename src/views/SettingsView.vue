<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { storeToRefs } from "pinia";
import {
  AlertCircle,
  CheckCircle2,
  Copy,
  Check,
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
  ToggleGroup,
  ToggleGroupItem,
} from "@/components/ui/toggle-group";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";

const aiStore = useAiStore();
const marketplaceStore = useMarketplaceStore();
const { settings, status, loading, generating } = storeToRefs(aiStore);
const { sources: marketplaceSources } = storeToRefs(marketplaceStore);

const activeTab = ref("ai");

// ── AI tab ────────────────────────────────────────────────────────

const showKey = ref(false);

const showCliFields = computed(
  () => settings.value.ai.mode === "auto" || settings.value.ai.mode === "claude-cli"
);
const showApiFields = computed(
  () => settings.value.ai.mode === "auto" || settings.value.ai.mode === "api"
);
const statusIcon = computed(() =>
  !status.value ? null : status.value.mode === "none" ? AlertCircle : CheckCircle2
);
const statusTone = computed(() =>
  !status.value ? "muted" : status.value.mode === "none" ? "warning" : "ok"
);

async function save() {
  await aiStore.saveSettings();
}

// ── Marketplaces tab ──────────────────────────────────────────────

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

// ── CLI tab ───────────────────────────────────────────────────────

const cliPlatform = ref<"linux" | "macos" | "windows">("linux");
const copied = ref(false);

const BASE = "https://github.com/Warshoow/claude-kit/releases/latest/download";

const cliBlocks = computed<{ label?: string; code: string }[]>(() => {
  switch (cliPlatform.value) {
    case "linux":
      return [{
        code:
`mkdir -p ~/.local/bin
curl -fsSL ${BASE}/ck-linux-x86_64 -o ~/.local/bin/ck
chmod +x ~/.local/bin/ck`,
      }];
    case "macos":
      return [
        {
          label: "Apple Silicon (M1 / M2 / M3)",
          code:
`curl -fsSL ${BASE}/ck-macos-arm64 -o /usr/local/bin/ck
chmod +x /usr/local/bin/ck`,
        },
        {
          label: "Intel",
          code:
`curl -fsSL ${BASE}/ck-macos-x86_64 -o /usr/local/bin/ck
chmod +x /usr/local/bin/ck`,
        },
      ];
    case "windows":
      return [{
        code:
`# PowerShell — creates %USERPROFILE%\\.claude-kit\\bin and adds it to your PATH
$bin = "$env:USERPROFILE\\.claude-kit\\bin"
New-Item -ItemType Directory -Force $bin | Out-Null
Invoke-WebRequest "${BASE}/ck-windows-x86_64.exe" -OutFile "$bin\\ck.exe"
[Environment]::SetEnvironmentVariable("Path", "$env:Path;$bin", "User")
# Re-open your terminal, then: ck --help`,
      }];
  }
});

async function copyBlock(code: string) {
  await navigator.clipboard.writeText(code);
  copied.value = true;
  setTimeout(() => { copied.value = false; }, 2000);
}

// ─────────────────────────────────────────────────────────────────

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
    <div class="flex items-center gap-3 border-b bg-card/40 px-6 py-2.5">
      <span class="text-sm font-semibold">Settings</span>
      <ToggleGroup
        type="single"
        :model-value="activeTab"
        @update:model-value="(v) => v && (activeTab = v as typeof activeTab)"
        variant="outline"
        size="sm"
      >
        <ToggleGroupItem value="ai" class="gap-1.5">
          <Sparkles class="size-3" />
          AI
        </ToggleGroupItem>
        <ToggleGroupItem value="marketplaces" class="gap-1.5">
          <Store class="size-3" />
          Marketplaces
        </ToggleGroupItem>
        <ToggleGroupItem value="cli" class="gap-1.5">
          <Terminal class="size-3" />
          CLI
        </ToggleGroupItem>
      </ToggleGroup>
      <div class="flex-1" />
      <Button
        v-if="activeTab === 'ai'"
        size="sm"
        :disabled="loading || generating"
        @click="save"
      >
        <Save />
        Save
      </Button>
    </div>

    <!-- ── AI ─────────────────────────────────────────────────── -->
    <div v-show="activeTab === 'ai'" class="flex-1 min-h-0 overflow-hidden">
        <ScrollArea class="h-full">
          <div class="mx-auto w-full max-w-2xl space-y-6 px-6 py-6">

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
              <component :is="statusIcon" class="mt-0.5 size-3.5 shrink-0" />
              <div class="space-y-0.5">
                <div class="font-medium">{{ status.message }}</div>
                <div
                  v-if="status.claude_cli_path"
                  class="font-mono text-[10.5px] opacity-70"
                >{{ status.claude_cli_path }}</div>
              </div>
            </div>

            <!-- Description -->
            <p class="text-xs text-muted-foreground">
              Powers the "Generate with AI" buttons in the asset editor and
              new-asset dialog. claude-kit auto-detects the
              <code class="rounded bg-muted px-1 py-0.5 font-mono text-[11px]">claude</code>
              CLI; if it isn't installed, configure any OpenAI-compatible
              endpoint instead.
            </p>

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
                <ToggleGroupItem value="auto" class="flex-1">Auto</ToggleGroupItem>
                <ToggleGroupItem value="claude-cli" class="flex-1">
                  <Terminal class="size-3" />
                  Claude CLI
                </ToggleGroupItem>
                <ToggleGroupItem value="api" class="flex-1">API</ToggleGroupItem>
              </ToggleGroup>
              <p class="text-[11px] text-muted-foreground">
                <strong>Auto</strong> uses the Claude CLI if found, otherwise
                falls back to your API config.
                <strong>Claude CLI</strong> forces the local binary.
                <strong>API</strong> forces the remote provider.
              </p>
            </div>

            <!-- Claude CLI path override -->
            <div v-if="showCliFields" class="space-y-1.5">
              <Label for="cli-path">Claude CLI path</Label>
              <Input
                id="cli-path"
                v-model="settings.ai.claude_cli_path"
                type="text"
                placeholder="Auto-detect"
                class="font-mono text-[12px]"
              />
              <p class="text-[11px] text-muted-foreground">
                Leave empty to auto-detect from
                <code class="font-mono text-[10.5px]">PATH</code> and
                <code class="font-mono text-[10.5px]">~/.claude/local/claude</code>.
                Set an absolute path if <code class="font-mono text-[10.5px]">claude</code>
                isn't on your PATH.
              </p>
            </div>

            <Separator v-if="showApiFields" />

            <!-- API config -->
            <div v-if="showApiFields" class="space-y-4">
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

          </div>
        </ScrollArea>
    </div>

    <!-- ── Marketplaces ───────────────────────────────────────── -->
    <div v-show="activeTab === 'marketplaces'" class="flex-1 min-h-0 overflow-hidden">
        <ScrollArea class="h-full">
          <div class="mx-auto w-full max-w-2xl space-y-6 px-6 py-6">

            <p class="text-xs text-muted-foreground">
              Sources for the Browse › Marketplace catalog. Each entry is a URL to a
              <code class="rounded bg-muted px-1 py-0.5 font-mono text-[11px]">marketplace.json</code>
              file. The official Anthropic marketplace is built-in and can't be removed.
            </p>

            <!-- Sources list -->
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
                doesn't return a valid marketplace JSON, it's rejected without saving.
              </p>
            </form>

          </div>
        </ScrollArea>
    </div>

    <!-- ── CLI ───────────────────────────────────────────────── -->
    <div v-show="activeTab === 'cli'" class="flex-1 min-h-0 overflow-hidden">
        <ScrollArea class="h-full">
          <div class="mx-auto w-full max-w-2xl space-y-6 px-6 py-6">

            <p class="text-xs text-muted-foreground">
              Apply bundles from any project terminal without opening the app.
              Download the <code class="rounded bg-muted px-1 py-0.5 font-mono text-[11px]">ck</code>
              binary for your platform and add it to your PATH.
            </p>

            <!-- Platform picker -->
            <ToggleGroup
              type="single"
              :model-value="cliPlatform"
              @update:model-value="(v) => v && (cliPlatform = v as typeof cliPlatform)"
              variant="outline"
              class="w-full"
            >
              <ToggleGroupItem value="linux" class="flex-1">Linux</ToggleGroupItem>
              <ToggleGroupItem value="macos" class="flex-1">macOS</ToggleGroupItem>
              <ToggleGroupItem value="windows" class="flex-1">Windows</ToggleGroupItem>
            </ToggleGroup>

            <!-- Command blocks -->
            <div class="space-y-3">
              <div v-for="(block, i) in cliBlocks" :key="i" class="space-y-1">
                <p v-if="block.label" class="text-[11px] font-medium text-muted-foreground">
                  {{ block.label }}
                </p>
                <div class="relative">
                  <pre class="rounded-md border bg-muted/50 px-3 py-2.5 font-mono text-[11.5px] leading-relaxed whitespace-pre-wrap break-all">{{ block.code }}</pre>
                  <button
                    type="button"
                    class="absolute right-1.5 top-1.5 grid size-6 place-items-center rounded text-muted-foreground hover:bg-accent hover:text-foreground transition-colors"
                    :title="copied ? 'Copied!' : 'Copy'"
                    @click="copyBlock(block.code)"
                  >
                    <Check v-if="copied" class="size-3 text-green-500" />
                    <Copy v-else class="size-3" />
                  </button>
                </div>
              </div>
            </div>

            <Separator />

            <!-- Usage reference -->
            <div class="space-y-1.5">
              <p class="text-[11px] font-medium text-muted-foreground">Then from any project directory</p>
              <div class="flex flex-wrap gap-2">
                <code
                  v-for="cmd in ['ck list', 'ck apply &lt;bundle&gt;', 'ck installed', 'ck clean']"
                  :key="cmd"
                  class="rounded bg-muted px-2 py-1 font-mono text-[11px]"
                  v-html="cmd"
                />
              </div>
            </div>

          </div>
        </ScrollArea>
    </div>

  </div>
</template>
