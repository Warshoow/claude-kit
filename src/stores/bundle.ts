import { defineStore } from "pinia";
import { api } from "@/lib/api";
import type { Bundle, ImportShareResult } from "@/lib/types";
import { useAppStore } from "./app";

export const useBundleStore = defineStore("bundle", () => {
  const appStore = useAppStore();

  async function createBundle(name: string, description?: string) {
    await api.createBundle(name, description);
    await appStore.refreshBundles();
  }

  async function deleteBundle(name: string) {
    await api.deleteBundle(name);
    await appStore.refreshBundles();
  }

  async function updateBundle(bundle: Bundle) {
    await api.setBundleAssets(bundle.name, bundle.assets);
    await appStore.refreshBundles();
  }

  async function encodeBundleShare(name: string): Promise<string> {
    return api.encodeBundleShare(name);
  }

  async function importBundleShare(code: string): Promise<ImportShareResult> {
    const result = await api.importBundleShare(code);
    await Promise.all([appStore.refreshLibrary(), appStore.refreshBundles()]);
    return result;
  }

  return {
    createBundle,
    deleteBundle,
    updateBundle,
    encodeBundleShare,
    importBundleShare,
  };
});
