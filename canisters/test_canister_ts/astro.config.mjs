// @ts-check
import {defineConfig} from "astro/config";
import tailwindcss from "@tailwindcss/vite";

// https://astro.build/config
export default defineConfig({
  output: "static",
  publicDir: "public",
  build: {
    assets: "assets",
  },
  vite: {
    // @ts-expect-error - Vite plugin type mismatch between Astro and Tailwind
    plugins: [tailwindcss()],
    define: {
      global: "globalThis",
    },
    optimizeDeps: {
      esbuildOptions: {
        define: {
          global: "globalThis",
        },
      },
    },
  },
});
