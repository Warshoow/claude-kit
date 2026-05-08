import { createRouter, createWebHashHistory, type RouteRecordRaw } from "vue-router";

const routes: RouteRecordRaw[] = [
  {
    path: "/",
    redirect: "/bundles",
  },
  {
    path: "/bundles",
    name: "bundles",
    component: () => import("@/views/MyBundlesView.vue"),
  },
  {
    path: "/bundles/:name",
    name: "bundle-detail",
    component: () => import("@/views/BundleDetailView.vue"),
    props: true,
  },
  {
    path: "/bundles/:name/harmonize",
    name: "bundle-harmonize",
    component: () => import("@/views/BundleHarmonizeView.vue"),
    props: true,
  },
  {
    path: "/browse",
    name: "browse",
    component: () => import("@/views/BrowseView.vue"),
    redirect: "/browse/library",
    children: [
      {
        path: "library",
        name: "browse-library",
        component: () => import("@/views/BrowseLibraryView.vue"),
      },
      {
        path: "library/:plugin",
        name: "browse-library-plugin",
        component: () => import("@/views/LibraryPluginDetailView.vue"),
        props: true,
      },
      {
        path: "marketplace",
        name: "browse-marketplace",
        component: () => import("@/views/BrowseMarketplaceView.vue"),
      },
    ],
  },
  {
    path: "/library/:kind/:name",
    name: "asset-detail",
    component: () => import("@/views/AssetDetailView.vue"),
    props: true,
  },
  {
    path: "/plugins/:name",
    name: "plugin-detail",
    component: () => import("@/views/PluginDetailView.vue"),
    props: true,
  },
  {
    path: "/project",
    name: "project",
    component: () => import("@/views/ProjectView.vue"),
  },
  {
    path: "/settings",
    name: "settings",
    component: () => import("@/views/SettingsView.vue"),
  },
];

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
});
