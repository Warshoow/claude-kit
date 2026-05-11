import DefaultTheme from "vitepress/theme";
import Layout from "./Layout.vue";
import Home from "./Home.vue";
import "./style.css";
import type { App } from "vue";

export default {
  ...DefaultTheme,
  Layout,
  enhanceApp({ app }: { app: App }) {
    app.component("Home", Home);
  },
};
