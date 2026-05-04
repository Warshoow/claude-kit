import { defineStore } from "pinia";
import { api } from "@/lib/api";
import { Bundle } from "@/lib/types";
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


    return {
        createBundle,
        deleteBundle,
        updateBundle
    }
});
