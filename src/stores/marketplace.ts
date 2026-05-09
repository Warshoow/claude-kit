import { defineStore } from "pinia";
import { computed, ref } from "vue";
import { api } from "@/lib/api";
import type { Marketplace, MarketplaceSource, Plugin } from "@/lib/types";
import { toast } from "vue-sonner";
import { useAppStore } from "./app";

/**
 * One marketplace's catalog plus the source that produced it. We keep
 * an `error` field so a partial fetch (one source down, others fine)
 * doesn't tank the whole Browse page — failing sources just show as
 * empty with a note.
 */
export interface CatalogEntry {
  source: MarketplaceSource;
  marketplace: Marketplace;
  error: string | null;
}

export const useMarketplaceStore = defineStore("marketplace", () => {
  const appStore = useAppStore();

  const sources = ref<MarketplaceSource[]>([]);
  const catalogs = ref<CatalogEntry[]>([]);
  const marketplaceLoading = ref(false);
  const importingPlugin = ref<string | null>(null);

  /**
   * Backward-compat alias. Returns the official catalog if available,
   * otherwise the first one. Existing views that read a single
   * `marketplace.value` keep working until we migrate them.
   */
  const marketplace = computed<Marketplace | null>(() => {
    const builtin = catalogs.value.find((c) => c.source.builtin);
    return builtin?.marketplace ?? catalogs.value[0]?.marketplace ?? null;
  });

  /** Flattened plugin list with each plugin's source attached. */
  const allPlugins = computed<Array<{ plugin: Plugin; source: MarketplaceSource }>>(
    () => {
      const out: Array<{ plugin: Plugin; source: MarketplaceSource }> = [];
      for (const c of catalogs.value) {
        for (const p of c.marketplace.plugins) {
          out.push({ plugin: p, source: c.source });
        }
      }
      return out;
    }
  );

  const marketplaceError = computed<string | null>(() => {
    const errs = catalogs.value
      .filter((c) => c.error)
      .map((c) => `${c.source.name}: ${c.error}`);
    return errs.length ? errs.join(" · ") : null;
  });

  async function loadSources() {
    sources.value = await api.listMarketplaceSources();
  }

  async function loadMarketplace(force = false) {
    if (catalogs.value.length && !force) return;
    marketplaceLoading.value = true;
    try {
      await loadSources();
      const fetched = await Promise.all(
        sources.value.map(async (source): Promise<CatalogEntry> => {
          try {
            const m = await api.listMarketplacePlugins(source.url);
            return { source, marketplace: m, error: null };
          } catch (e) {
            // Fabricate an empty Marketplace so downstream code has a
            // stable shape; the error is surfaced via marketplaceError.
            return {
              source,
              marketplace: { name: source.name, plugins: [] } as Marketplace,
              error: String(e),
            };
          }
        })
      );
      catalogs.value = fetched;
    } finally {
      marketplaceLoading.value = false;
    }
  }

  async function addSource(url: string): Promise<boolean> {
    try {
      const newSource = await api.addMarketplaceSource(url);
      // Fetch its catalog immediately so the user sees plugins right
      // after registering. A failure here keeps the source registered
      // (it's still in settings) but with an empty catalog + error.
      try {
        const m = await api.listMarketplacePlugins(newSource.url);
        catalogs.value.push({ source: newSource, marketplace: m, error: null });
      } catch (e) {
        catalogs.value.push({
          source: newSource,
          marketplace: { name: newSource.name, plugins: [] } as Marketplace,
          error: String(e),
        });
      }
      sources.value = await api.listMarketplaceSources();
      toast.success(`Added marketplace "${newSource.name}"`);
      return true;
    } catch (e) {
      toast.error("Couldn't add marketplace", { description: String(e) });
      return false;
    }
  }

  async function removeSource(url: string): Promise<boolean> {
    try {
      await api.removeMarketplaceSource(url);
      catalogs.value = catalogs.value.filter((c) => c.source.url !== url);
      sources.value = await api.listMarketplaceSources();
      toast.success("Marketplace removed");
      return true;
    } catch (e) {
      toast.error("Couldn't remove marketplace", { description: String(e) });
      return false;
    }
  }

  async function importMarketplacePlugin(plugin: Plugin, sourceName?: string) {
    if (importingPlugin.value) return;
    importingPlugin.value = plugin.name;
    try {
      const res = await api.importMarketplacePlugin(plugin, sourceName);
      await Promise.all([appStore.refreshLibrary(), appStore.refreshHooksMcp()]);
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
    sources,
    catalogs,
    marketplace,
    marketplaceLoading,
    marketplaceError,
    importingPlugin,
    allPlugins,
    loadSources,
    loadMarketplace,
    addSource,
    removeSource,
    importMarketplacePlugin,
  };
});
