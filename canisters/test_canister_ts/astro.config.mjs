// @ts-check
import {defineConfig} from "astro/config";
import tailwindcss from "@tailwindcss/vite";

// https://astro.build/config
export default defineConfig({
  output: "static",
  build: {
    assets: "assets",
  },
  vite: {
    plugins: [tailwindcss()],
    define: {
      global: "globalThis",
    },
    optimizeDeps: {
      include: ["@walletconnect/ethereum-provider", "@walletconnect/modal"],
      esbuildOptions: {
        define: {
          global: "globalThis",
        },
      },
    },
    build: {
      // Ensure consistent chunking for WalletConnect dynamic imports
      rollupOptions: {
        output: {
          manualChunks: {
            walletconnect: [
              "@walletconnect/ethereum-provider",
              "@walletconnect/modal",
            ],
          },
        },
      },
    },
  },
});
