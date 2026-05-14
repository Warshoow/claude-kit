<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { Loader2, Plug, AlertCircle } from "lucide-vue-next";
import { toast } from "vue-sonner";
import { api } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  ToggleGroup,
  ToggleGroupItem,
} from "@/components/ui/toggle-group";

const props = defineProps<{
  open: boolean;
  /** When set, the dialog edits this existing local MCP instead of
   *  creating a new one. The name input is then locked. */
  editName?: string;
}>();

const emit = defineEmits<{
  "update:open": [value: boolean];
  /** Fires after a successful create/update with the saved name so
   *  the parent can refresh listings or auto-pick the entry. */
  saved: [name: string];
}>();

type Mode = "form" | "json";
const mode = ref<Mode>("form");

const name = ref("");
const command = ref("");
const argsText = ref("");
const envText = ref("");
const jsonText = ref("");
const submitting = ref(false);
const error = ref<string | null>(null);

const isEdit = computed(() => !!props.editName);

watch(
  () => props.open,
  async (v) => {
    if (!v) return;
    error.value = null;
    submitting.value = false;
    if (isEdit.value) {
      name.value = props.editName!;
      // Pre-fill from the existing config — the backend returns the
      // inner shape (no `mcpServers` envelope) so we can populate the
      // form fields directly.
      try {
        const existing = (await api.readLocalMcp(props.editName!)) as
          | Record<string, unknown>
          | null;
        if (existing) {
          command.value = (existing.command as string) ?? "";
          argsText.value = Array.isArray(existing.args)
            ? (existing.args as string[]).join("\n")
            : "";
          envText.value = formatEnv(existing.env);
          jsonText.value = JSON.stringify(existing, null, 2);
        }
      } catch (e) {
        error.value = `Couldn't load existing config: ${String(e)}`;
      }
    } else {
      name.value = "";
      command.value = "";
      argsText.value = "";
      envText.value = "";
      jsonText.value = "{\n  \"command\": \"\",\n  \"args\": []\n}";
      mode.value = "form";
    }
  },
  { immediate: true },
);

function formatEnv(env: unknown): string {
  if (!env || typeof env !== "object") return "";
  return Object.entries(env as Record<string, unknown>)
    .map(([k, v]) => `${k}=${String(v)}`)
    .join("\n");
}

/** Build the server config object from the active mode's inputs. */
function buildConfig(): unknown {
  if (mode.value === "json") {
    return JSON.parse(jsonText.value);
  }
  const args = argsText.value
    .split("\n")
    .map((l) => l.trim())
    .filter(Boolean);
  const env: Record<string, string> = {};
  for (const line of envText.value.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("#")) continue;
    const eq = trimmed.indexOf("=");
    if (eq < 0) {
      throw new Error(
        `env line must be KEY=value: "${trimmed}"`,
      );
    }
    env[trimmed.slice(0, eq).trim()] = trimmed.slice(eq + 1);
  }
  const cfg: Record<string, unknown> = {
    command: command.value.trim(),
  };
  if (args.length > 0) cfg.args = args;
  if (Object.keys(env).length > 0) cfg.env = env;
  return cfg;
}

function validate(): string | null {
  if (!isEdit.value) {
    const n = name.value.trim();
    if (!n) return "Name is required.";
    if (!/^[A-Za-z0-9_-]+$/.test(n)) {
      return "Name must contain only letters, digits, '-' or '_'.";
    }
    if (n.length > 64) return "Name too long (max 64).";
    if (n === "__local__") return "'__local__' is reserved.";
  }
  if (mode.value === "form" && !command.value.trim()) {
    return "Command is required.";
  }
  return null;
}

async function submit() {
  error.value = null;
  const v = validate();
  if (v) {
    error.value = v;
    return;
  }
  let config: unknown;
  try {
    config = buildConfig();
  } catch (e) {
    error.value = String(e);
    return;
  }

  submitting.value = true;
  try {
    if (isEdit.value) {
      await api.updateLocalMcp(props.editName!, config);
      toast.success(`Updated "${props.editName}"`);
      emit("saved", props.editName!);
    } else {
      const trimmed = name.value.trim();
      await api.createLocalMcp(trimmed, config);
      toast.success(`Created MCP "${trimmed}"`);
      emit("saved", trimmed);
    }
    emit("update:open", false);
  } catch (e) {
    error.value = String(e);
  } finally {
    submitting.value = false;
  }
}

function cancel() {
  emit("update:open", false);
}
</script>

<style scoped>
.json-editor {
  font-family: ui-monospace, "SF Mono", Menlo, Consolas, monospace;
  font-size: 12.5px;
  line-height: 1.5;
  min-height: 220px;
}
</style>

<template>
  <Dialog :open="open" @update:open="emit('update:open', $event)">
    <DialogContent class="sm:max-w-lg">
      <DialogHeader>
        <DialogTitle class="flex items-center gap-2">
          <Plug class="size-4" />
          {{ isEdit ? `Edit MCP "${editName}"` : "New MCP server" }}
        </DialogTitle>
        <DialogDescription>
          Define an MCP server config. Persisted under
          <code class="text-[11px]">~/.claude-assets/library/mcp/__local__/</code>;
          available to add to any bundle.
        </DialogDescription>
      </DialogHeader>

      <form class="space-y-4" @submit.prevent="submit">
        <div class="space-y-1.5">
          <Label for="mcp-name">Server name</Label>
          <Input
            id="mcp-name"
            v-model="name"
            placeholder="e.g. my-postgres"
            :disabled="isEdit"
            autocomplete="off"
          />
          <p class="text-[11px] text-muted-foreground">
            Slug used as the server key inside <code>mcp.json</code>. Letters,
            digits, <code>-</code>, <code>_</code>, max 64 chars.
          </p>
        </div>

        <div class="space-y-1.5">
          <Label>Mode</Label>
          <ToggleGroup
            type="single"
            :model-value="mode"
            @update:model-value="(v) => v && (mode = v as Mode)"
            variant="outline"
            class="w-full"
          >
            <ToggleGroupItem value="form" class="flex-1">Form</ToggleGroupItem>
            <ToggleGroupItem value="json" class="flex-1">Raw JSON</ToggleGroupItem>
          </ToggleGroup>
        </div>

        <template v-if="mode === 'form'">
          <div class="space-y-1.5">
            <Label for="mcp-command">Command</Label>
            <Input
              id="mcp-command"
              v-model="command"
              placeholder="e.g. npx, uvx, /usr/local/bin/my-server"
              autocomplete="off"
            />
          </div>

          <div class="space-y-1.5">
            <Label for="mcp-args">Args (one per line)</Label>
            <Textarea
              id="mcp-args"
              v-model="argsText"
              rows="4"
              placeholder="-y&#10;@modelcontextprotocol/server-postgres"
              class="font-mono text-[12.5px]"
            />
          </div>

          <div class="space-y-1.5">
            <Label for="mcp-env">Env vars (KEY=value per line)</Label>
            <Textarea
              id="mcp-env"
              v-model="envText"
              rows="3"
              placeholder="POSTGRES_URL=postgres://..."
              class="font-mono text-[12.5px]"
            />
          </div>
        </template>

        <template v-else>
          <div class="space-y-1.5">
            <Label for="mcp-json">Server config (inner shape, no <code>mcpServers</code> wrapper)</Label>
            <Textarea
              id="mcp-json"
              v-model="jsonText"
              class="json-editor"
              spellcheck="false"
            />
            <p class="text-[11px] text-muted-foreground">
              We add the <code>mcpServers</code> envelope on save. Use this
              mode for advanced configs (transport: "sse", custom headers, etc.).
            </p>
          </div>
        </template>

        <div
          v-if="error"
          class="flex items-start gap-2 rounded-md border border-destructive/40 bg-destructive/10 p-2.5 text-[11.5px] text-destructive"
        >
          <AlertCircle class="mt-0.5 size-3.5 shrink-0" />
          <div class="flex-1 break-words">{{ error }}</div>
        </div>
      </form>

      <DialogFooter>
        <Button variant="outline" :disabled="submitting" @click="cancel">
          Cancel
        </Button>
        <Button :disabled="submitting" @click="submit">
          <Loader2 v-if="submitting" class="animate-spin" />
          <Plug v-else />
          {{ isEdit ? "Save" : "Create" }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
