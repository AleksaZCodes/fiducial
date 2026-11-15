import { cloudflare } from "@cloudflare/vite-plugin";
import { reactRouter } from "@react-router/dev/vite";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";

// The Cloudflare plugin runs the server build inside `workerd`, so `dev` and
// `build` see the same runtime a deploy does. React Router's server environment
// is named `ssr`, and the plugin is told so.
export default defineConfig({
  plugins: [cloudflare({ viteEnvironment: { name: "ssr" } }), tailwindcss(), reactRouter()],
  resolve: { tsconfigPaths: true },
});
