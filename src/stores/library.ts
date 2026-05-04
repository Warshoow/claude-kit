import { defineStore } from "pinia";
import { api } from "@/lib/api";
import { Asset } from "@/lib/types";
import { toast } from "vue-sonner";
import { useAppStore } from "./app";


export const useLibraryStore = defineStore("library", () => {

    const appStore = useAppStore();

    async function createAsset(
        kind: Asset["kind"],
        name: string,
        description?: string
      ): Promise<boolean> {
        try {
          await api.createAsset(kind, name, description);
          await appStore.refreshLibrary();
          toast.success(`Created ${kind}/${name}`);
          return true;
        } catch (e) {
          toast.error("Couldn't create asset", { description: String(e) });
          return false;
        }
      }

    return {
        createAsset
    }
});
