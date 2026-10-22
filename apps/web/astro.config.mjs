import react from "@astrojs/react";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "astro/config";

// GitHub Pages (.github/workflows/site.yml) serves the site below the repository name, and
// each pull request preview below pr-preview/pr-<n>/; the workflow passes both in. Locally
// the site is at the root.
export default defineConfig({
  site: process.env.SITE_URL || "https://ensobih.github.io",
  base: process.env.SITE_BASE || "/",
  build: {
    // GitHub Pages runs Jekyll on the branch, which drops folders starting with `_`.
    assets: "assets",
  },
  integrations: [react()],
  vite: {
    plugins: [tailwindcss()],
  },
});
