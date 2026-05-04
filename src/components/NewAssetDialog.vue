<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { useAppStore } from "@/stores/app";
import { useLibraryStore } from "@/stores/library";
import type { AssetKind } from "@/lib/types";
import { assetKey } from "@/lib/types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
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

const props = defineProps<{ open: boolean }>();
const emit = defineEmits<{ "update:open": [value: boolean] }>();

const store = useAppStore();
const libraryStore = useLibraryStore();
const router = useRouter();

const kind = ref<AssetKind>("skills");
const name = ref("");
const description = ref("");
const submitting = ref(false);

// Reset form whenever the dialog reopens — avoids leaking the previous attempt.
watch(
  () => props.open,
  (v) => {
    if (v) {
      kind.value = "skills";
      name.value = "";
      description.value = "";
      submitting.value = false;
    }
  }
);

const trimmedName = computed(() => name.value.trim());

const collision = computed(() => {
  if (!trimmedName.value) return false;
  const k = `${kind.value}:${trimmedName.value}`;
  return store.library.some((a) => assetKey(a) === k);
});

const nameError = computed<string | null>(() => {
  if (!trimmedName.value) return null;
  if (trimmedName.value.length > 64) return "Too long (max 64 chars)";
  if (!/^[A-Za-z0-9_-]+$/.test(trimmedName.value)) {
    return "Only letters, digits, '-' and '_'";
  }
  if (collision.value) return `A ${kind.value.slice(0, -1)} with that name already exists`;
  return null;
});

const canSubmit = computed(
  () => !!trimmedName.value && !nameError.value && !submitting.value
);

async function submit() {
  if (!canSubmit.value) return;
  submitting.value = true;
  const ok = await libraryStore.createAsset(
    kind.value,
    trimmedName.value,
    description.value.trim() || undefined
  );
  submitting.value = false;
  if (!ok) return;
  emit("update:open", false);
  router.push({
    name: "asset-detail",
    params: { kind: kind.value, name: trimmedName.value },
  });
}
</script>

<template>
  <Dialog :open="open" @update:open="emit('update:open', $event)">
    <DialogContent class="sm:max-w-md">
      <DialogHeader>
        <DialogTitle>New asset</DialogTitle>
        <DialogDescription>
          Create an empty skill, command, or agent in your library. You can
          edit the content right after.
        </DialogDescription>
      </DialogHeader>

      <form class="space-y-4" @submit.prevent="submit">
        <div class="space-y-1.5">
          <Label>Kind</Label>
          <ToggleGroup
            type="single"
            :model-value="kind"
            @update:model-value="(v) => v && (kind = v as AssetKind)"
            variant="outline"
            class="w-full"
          >
            <ToggleGroupItem value="skills" class="flex-1">Skill</ToggleGroupItem>
            <ToggleGroupItem value="commands" class="flex-1">Command</ToggleGroupItem>
            <ToggleGroupItem value="agents" class="flex-1">Agent</ToggleGroupItem>
          </ToggleGroup>
        </div>

        <div class="space-y-1.5">
          <Label for="new-asset-name">Name</Label>
          <Input
            id="new-asset-name"
            v-model="name"
            placeholder="e.g. python-refactor"
            autofocus
            required
          />
          <p
            v-if="nameError"
            class="text-[11px] text-destructive"
          >{{ nameError }}</p>
          <p
            v-else
            class="text-[11px] text-muted-foreground"
          >Letters, digits, '-' or '_'.</p>
        </div>

        <div class="space-y-1.5">
          <Label for="new-asset-desc">
            Description
            <span class="text-muted-foreground">(optional)</span>
          </Label>
          <Input
            id="new-asset-desc"
            v-model="description"
            placeholder="Short summary, shown in listings"
          />
        </div>
      </form>

      <DialogFooter>
        <Button variant="outline" @click="emit('update:open', false)">Cancel</Button>
        <Button :disabled="!canSubmit" @click="submit">
          {{ submitting ? "Creating…" : "Create" }}
        </Button>
      </DialogFooter>
    </DialogContent>
  </Dialog>
</template>
