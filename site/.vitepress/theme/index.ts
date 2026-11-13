// `theme-without-fonts` omits the default theme's Google Fonts import. The
// site serves its own Inter and JetBrains Mono files, so there is no
// third-party font request and no duplicate download.
import DefaultTheme from "vitepress/theme-without-fonts";
import type { Theme } from "vitepress";

import DocFigure from "./components/DocFigure.vue";
import ExampleCard from "./components/ExampleCard.vue";
import AmigaPlayer from "./components/AmigaPlayer.vue";
import HomePage from "./components/HomePage.vue";
import Layout from "./Layout.vue";
import PartnerCTA from "./components/PartnerCTA.vue";
import StatusBadge from "./components/StatusBadge.vue";

import "./custom.css";

/**
 * Extends the stock VitePress theme with a thin developer-preview bar, the
 * custom home page, and three small presentation components. The documentation
 * pages keep the stock theme so navigation, search, and accessibility behave
 * predictably.
 */
export default {
  extends: DefaultTheme,
  Layout,
  enhanceApp({ app }) {
    app.component("DocFigure", DocFigure);
    app.component("ExampleCard", ExampleCard);
    app.component("AmigaPlayer", AmigaPlayer);
    app.component("HomePage", HomePage);
    app.component("PartnerCTA", PartnerCTA);
    app.component("StatusBadge", StatusBadge);
  },
} satisfies Theme;
