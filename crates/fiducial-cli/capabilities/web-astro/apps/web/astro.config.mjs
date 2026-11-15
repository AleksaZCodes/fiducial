import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "astro/config";

// A static site: `astro build` writes plain files to `dist/`, and Workers Static
// Assets serves them (`[deploy] shape = "astro"`). There is no server at
// runtime, so there is nothing here to configure an adapter for.
//
// `trailingSlash: "always"` with `build.format: "directory"` makes every page
// `<path>/index.html`, which a static file server finds without a rewrite.
export default defineConfig({
  output: "static",
  trailingSlash: "always",
  build: { format: "directory" },
  vite: { plugins: [tailwindcss()] },
});
