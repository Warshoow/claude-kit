import { defineStore } from "pinia";
import { api } from "@/lib/api";
import { Marketplace, Plugin } from "@/lib/types";
import { ref } from "vue";
import { toast } from "vue-sonner";
import { useAppStore } from "./app";

export const useMarketplaceStore = defineStore("marketplace", () => {

    const appStore = useAppStore();

    const marketplace = ref<Marketplace | null>(null);
      const marketplaceLoading = ref(false);
      const marketplaceError = ref<string | null>(null);
      const importingPlugin = ref<string | null>(null);

      async function loadMarketplace(force = false) {
        if (marketplace.value && !force) return;
        marketplaceLoading.value = true;
        marketplaceError.value = null;
        try {
          marketplace.value = await api.listMarketplacePlugins();
        } catch (e) {
          marketplaceError.value = String(e);
        } finally {
          marketplaceLoading.value = false;
        }
      }

      async function importMarketplacePlugin(plugin: Plugin) {
        if (importingPlugin.value) return;
        importingPlugin.value = plugin.name;
        try {
          const res = await api.importMarketplacePlugin(plugin);
          await appStore.refreshLibrary();
          toast.success(
            `Imported ${res.imported.length} from ${plugin.name}`,
            res.skipped.length
              ? { description: `Skipped ${res.skipped.length} (already in library)` }
              : undefined
          );
        } catch (e) {
          toast.error("Import failed", { description: String(e) });
        } finally {
          importingPlugin.value = null;
        }
      }


    return {
        marketplace,
        marketplaceLoading,
        marketplaceError,
        importingPlugin,
        loadMarketplace,
        importMarketplacePlugin
    }
});
