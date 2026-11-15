import { sveltekit } from "@sveltejs/kit/vite";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";

// Tailwind before SvelteKit: `app.css` starts with `@import "tailwindcss"`, and
// the design system's tokens and utilities are written against it.
export default defineConfig({
  plugins: [tailwindcss(), sveltekit()],
});
