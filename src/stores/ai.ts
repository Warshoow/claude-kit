import { ref } from "vue";
import { defineStore } from "pinia";
import { toast } from "vue-sonner";
import { api } from "@/lib/api";
import type { AiStatus, AssetKind, Settings } from "@/lib/types";

const DEFAULT_SETTINGS: Settings = {
  ai: {
    mode: "auto",
    api_base_url: undefined,
    api_key: undefined,
    api_model: undefined,
  },
};

export const useAiStore = defineStore("ai", () => {
  // ── State ───────────────────────────────────────────────────────
  const status = ref<AiStatus | null>(null);
  const settings = ref<Settings>(structuredClone(DEFAULT_SETTINGS));
  const loading = ref(false);
  const generating = ref(false);

  // ── Actions ─────────────────────────────────────────────────────
  async function refreshStatus() {
    try {
      status.value = await api.aiStatus();
    } catch (e) {
      // Surface as a soft error — Generate buttons will be disabled because
      // status.mode stays "none", which is the right fallback.
      console.warn("ai_status failed", e);
      status.value = {
        mode: "none",
        api_configured: false,
        message: `Couldn't query AI status: ${String(e)}`,
      };
    }
  }

  async function loadSettings() {
    loading.value = true;
    try {
      settings.value = await api.readSettings();
    } catch (e) {
      console.warn("read_settings failed", e);
      settings.value = structuredClone(DEFAULT_SETTINGS);
    } finally {
      loading.value = false;
    }
  }

  async function saveSettings(): Promise<boolean> {
    try {
      await api.writeSettings(settings.value);
      // Saving may flip the available backend (e.g. user added an API key
      // without a CLI installed) — recompute the status so any open page
      // reflects it immediately.
      await refreshStatus();
      toast.success("Settings saved");
      return true;
    } catch (e) {
      toast.error("Couldn't save settings", { description: String(e) });
      return false;
    }
  }

  /**
   * Generate or rewrite an asset's content via the configured AI backend.
   * Returns the raw markdown produced by the model, or `null` if generation
   * failed (a toast has already been shown).
   */
  async function generateAsset(
    kind: AssetKind,
    prompt: string,
    context?: string
  ): Promise<string | null> {
    if (generating.value) return null;
    generating.value = true;
    try {
      return await api.aiGenerate(kind, prompt, context);
    } catch (e) {
      toast.error("AI generation failed", { description: String(e) });
      return null;
    } finally {
      generating.value = false;
    }
  }

  return {
    // state
    status,
    settings,
    loading,
    generating,
    // actions
    refreshStatus,
    loadSettings,
    saveSettings,
    generateAsset,
  };
});
