import { defineStore } from "pinia";
import { api } from "@/lib/api";
import { toast } from "vue-sonner";
import { open } from "@tauri-apps/plugin-dialog";
import { useAppStore } from "./app";

export const usePluginStore = defineStore("plugin", () => {

    const appStore = useAppStore();

    async function importLocalPlugin() {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Pick a plugin folder",
    });
    if (typeof selected !== "string") return;
    try {
      const res = await api.importPlugin(selected);
      await appStore.refreshLibrary();
      toast.success(
        `Imported ${res.imported.length}`,
        res.skipped.length
          ? { description: `Skipped ${res.skipped.length} (already exist)` }
          : undefined
      );
    } catch (e) {
      toast.error("Import failed", { description: String(e) });
    }
  }

  return {
    importLocalPlugin
  }
});
