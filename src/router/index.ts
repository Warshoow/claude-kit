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
    path: "/browse",
    name: "browse",
    component: () => import("@/views/BrowseView.vue"),
  },
  {
    path: "/project",
    name: "project",
    component: () => import("@/views/ProjectView.vue"),
  },
];

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
});
