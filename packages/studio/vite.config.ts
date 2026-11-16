import path from "node:path";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";

// Studio is a static app: `vite build` produces files any host can serve, and
// `base: "./"` lets them live under any path (a GitHub Pages project site, a
// Worker route, `fid studio`'s own server).
export default defineConfig({
  base: "./",
  plugins: [react(), tailwindcss()],
  resolve: { alias: { "@": path.resolve(import.meta.dirname, "src") } },
});
