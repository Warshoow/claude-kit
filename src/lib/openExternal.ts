import { openUrl } from "@tauri-apps/plugin-opener";

/**
 * Click handler for any container that may render <a> links — typically
 * rendered markdown (READMEs, asset previews) but also explicit `<a>`
 * tags we drop into the UI for "View on GitHub" / "Homepage" / etc.
 *
 * Without this, an external link in the Tauri WebView either navigates
 * the app away from its routes (with no back button) or pops a new
 * webview window that doesn't render properly. We intercept the click,
 * prevent the default navigation, and ask the OS shell to open the URL
 * in the host browser via `tauri-plugin-opener`.
 *
 * Same-page anchors (`#section`) and app-relative paths (`/foo`) are
 * left alone — those are intra-page navigation that the WebView handles
 * fine.
 */
export function handleExternalLink(e: MouseEvent) {
  // Ignore modified clicks (Ctrl/Cmd-click etc.) — let the user override
  // if they're doing something deliberate.
  if (e.ctrlKey || e.metaKey || e.shiftKey || e.altKey) return;

  const target = e.target as Element | null;
  const anchor = target?.closest("a");
  if (!anchor) return;
  const href = anchor.getAttribute("href");
  if (!href) return;

  // In-page anchor (table-of-contents jump) — let the browser handle it.
  if (href.startsWith("#")) return;
  // App-relative path — also let it through (router will handle).
  if (href.startsWith("/") && !href.startsWith("//")) return;
  // `mailto:` and `javascript:` schemes are best handled by the OS / no-op.
  if (href.startsWith("javascript:")) {
    e.preventDefault();
    return;
  }

  e.preventDefault();
  openUrl(href).catch((err) => {
    // Swallowing here is intentional — opener failures (e.g. missing
    // capability, malformed URL) shouldn't break the click target. We
    // log so the issue is observable in dev.
    console.warn("openUrl failed for", href, err);
  });
}
