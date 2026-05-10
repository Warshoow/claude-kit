<script setup lang="ts">
import { computed, onMounted, ref } from "vue";

interface Asset {
  name: string;
  browser_download_url: string;
  size: number;
}

interface Release {
  tag_name: string;
  html_url: string;
  assets: Asset[];
}

type OSGuess = "windows" | "mac-arm" | "mac-intel" | "linux" | "other";

const REPO_API_URL = "https://api.github.com/repos/Warshoow/claude-kit/releases/latest";
const FALLBACK_RELEASES_URL = "https://github.com/Warshoow/claude-kit/releases/latest";

const release = ref<Release | null>(null);
const loadError = ref<string | null>(null);

/**
 * Best-guess OS family from the browser UA. We can't tell Apple Silicon
 * from Intel reliably (UA reports "Mac OS X" either way), so we default
 * to arm64 — that's the dominant fleet on macOS as of 2024+. Users on
 * Intel Macs see the suggestion and have a "Other downloads" link to
 * pick their flavour explicitly.
 */
const os = computed<OSGuess>(() => {
  if (typeof navigator === "undefined") return "other";
  const ua = navigator.userAgent;
  if (/Win(dows)?/i.test(ua)) return "windows";
  if (/Mac|iPhone|iPad/.test(ua)) return "mac-arm";
  if (/Linux/i.test(ua)) return "linux";
  return "other";
});

/**
 * Pick the best asset for the detected OS. We're tolerant of naming
 * variations because tauri-action's exact filename pattern can shift
 * between versions; the key heuristic is the file extension + arch.
 */
const primary = computed<Asset | null>(() => {
  const r = release.value;
  if (!r) return null;
  const a = r.assets;
  switch (os.value) {
    case "windows":
      // .msi is friendlier (silent install, registers in Apps & Features)
      // than the NSIS .exe — prefer it.
      return (
        a.find((x) => /\.msi$/i.test(x.name)) ??
        a.find((x) => /-setup\.exe$/i.test(x.name)) ??
        null
      );
    case "mac-arm":
      return (
        a.find((x) => /aarch64.*\.dmg$/i.test(x.name)) ??
        a.find((x) => /\.dmg$/i.test(x.name)) ??
        null
      );
    case "mac-intel":
      return (
        a.find((x) => /x64.*\.dmg$/i.test(x.name)) ??
        a.find((x) => /\.dmg$/i.test(x.name)) ??
        null
      );
    case "linux":
      // .AppImage works without root and across distros; .deb/.rpm
      // need to match the package manager so we don't auto-pick them.
      return a.find((x) => /\.AppImage$/i.test(x.name)) ?? null;
    default:
      return null;
  }
});

const primaryLabel = computed(() => {
  if (!release.value) return "Download";
  const v = release.value.tag_name.replace(/^v/, "");
  switch (os.value) {
    case "windows": return `Download for Windows · v${v}`;
    case "mac-arm": return `Download for macOS · v${v}`;
    case "mac-intel": return `Download for macOS · v${v}`;
    case "linux": return `Download for Linux · v${v}`;
    default: return `View all downloads · v${v}`;
  }
});

const primaryHref = computed(
  () => primary.value?.browser_download_url ?? release.value?.html_url ?? FALLBACK_RELEASES_URL
);

const otherDownloadsHref = computed(
  () => release.value?.html_url ?? FALLBACK_RELEASES_URL
);

onMounted(async () => {
  try {
    const res = await fetch(REPO_API_URL, {
      headers: { Accept: "application/vnd.github+json" },
    });
    if (!res.ok) {
      // 404 → no release yet (e.g. fresh fork); 403 → API rate-limited.
      // Both fall through to the static "View all downloads" link.
      throw new Error(`GitHub API returned ${res.status}`);
    }
    release.value = await res.json();
  } catch (e) {
    loadError.value = String(e);
  }
});
</script>

<template>
  <div class="download-shelf">
    <div class="cta-row">
      <a class="VPButton medium brand smart-cta" :href="primaryHref" rel="noopener">
        {{ primaryLabel }}
      </a>
      <a
        class="VPButton medium alt smart-cta"
        href="https://github.com/Warshoow/claude-kit"
        rel="noopener"
      >
        View on GitHub
      </a>
    </div>
    <p class="hint">
      <a :href="otherDownloadsHref" target="_blank" rel="noopener">
        Other platforms &amp; older versions →
      </a>
    </p>
  </div>
</template>

<style scoped>
.download-shelf {
  display: flex;
  flex-direction: column;
  align-items: stretch;
  gap: 0.65rem;
  margin-top: 1rem;
}

.cta-row {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

/* Mirror the shape VitePress uses for `actions` so our buttons sit
   next to the headline at the same visual weight as the default
   ones. Padding/line-height come from the framework's own button
   style; we just enforce a sensible width-cap and white-space. */
.smart-cta {
  padding: 0 22px;
  line-height: 38px;
  font-size: 14px;
  border-radius: 20px;
  font-weight: 600;
  white-space: nowrap;
  text-align: center;
}

.hint {
  margin: 0;
  font-size: 12px;
  color: var(--vp-c-text-2);
  text-align: left;
}

.hint a {
  color: var(--vp-c-text-2);
  text-decoration: none;
  border-bottom: 1px solid var(--vp-c-divider);
  transition: color 150ms, border-color 150ms;
}

.hint a:hover {
  color: var(--vp-c-text-1);
  border-color: var(--vp-c-text-1);
}

/* On desktop, line up the two CTAs in a row like the default hero
   actions block. On mobile keep them stacked so they remain easy to
   tap. */
@media (min-width: 640px) {
  .download-shelf {
    align-items: flex-start;
  }
  .cta-row {
    flex-direction: row;
  }
}
</style>
