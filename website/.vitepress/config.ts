import { defineConfig } from "vitepress";

// Site is served from `https://<user>.github.io/claude-kit/` by default.
// Set CK_SITE_BASE in CI to override (e.g. for a custom domain → "/").
const base = process.env.CK_SITE_BASE ?? "/claude-kit/";

export default defineConfig({
  base,
  title: "claude-kit",
  description:
    "Manage your Claude Code skills, commands, agents, hooks and MCP configs as reusable bundles — applied to any project in one click.",
  lang: "en-US",
  cleanUrls: true,

  // Suppress dead-link warnings during build for the GitHub releases page,
  // which only exists once the user has tagged something.
  ignoreDeadLinks: [/^https?:\/\/github\.com\/Warshoow\/claude-kit\/releases/],

  head: [
    ["link", { rel: "icon", href: `${base}icon.png` }],
    ["meta", { name: "theme-color", content: "#F97316" }],
    ["meta", { property: "og:type", content: "website" }],
    ["meta", { property: "og:title", content: "claude-kit" }],
    [
      "meta",
      {
        property: "og:description",
        content:
          "Manage your Claude Code assets as reusable bundles — apply them to any project in one click.",
      },
    ],
    ["meta", { property: "og:image", content: `${base}icon.png` }],
  ],

  themeConfig: {
    logo: "/icon.png",

    nav: [
      { text: "GitHub", link: "https://github.com/Warshoow/claude-kit" },
      {
        text: "Releases",
        link: "https://github.com/Warshoow/claude-kit/releases",
      },
    ],

    socialLinks: [
      { icon: "github", link: "https://github.com/Warshoow/claude-kit" },
    ],

    footer: {
      message: "Released under the MIT License.",
      copyright: "© Joffrey Guilmeau",
    },

    // Hide the "Made with VitePress" footer line — we want a clean
    // single-page feel without docs sidebar leaking in.
    outline: false,
  },
});
