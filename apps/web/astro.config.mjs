import react from "@astrojs/react";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "astro/config";

export default defineConfig({
  // TODO: set once the domain is chosen (DECISIONS.md D-006).
  site: "https://quran-reader.example",
  integrations: [react()],
  vite: {
    plugins: [tailwindcss()],
  },
});
