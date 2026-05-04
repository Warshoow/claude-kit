import { ref, watch } from "vue";

type Theme = "dark" | "light";

const STORAGE_KEY = "claude-kit:theme";

const theme = ref<Theme>(
  (localStorage.getItem(STORAGE_KEY) as Theme) ?? "dark"
);

watch(theme, (t) => {
  document.documentElement.className = t;
  localStorage.setItem(STORAGE_KEY, t);
});

export function useTheme() {
  function toggle() {
    theme.value = theme.value === "dark" ? "light" : "dark";
  }

  return { theme, toggle };
}
