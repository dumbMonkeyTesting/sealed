import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  base: "/app/privy/",
  plugins: [react()],
  build: {
    outDir: "../app/privy",
    emptyOutDir: true,
    cssCodeSplit: false,
    rollupOptions: {
      output: {
        entryFileNames: "privy.js",
        chunkFileNames: "privy-[name].js",
        assetFileNames: "privy[extname]",
      },
    },
  },
});
