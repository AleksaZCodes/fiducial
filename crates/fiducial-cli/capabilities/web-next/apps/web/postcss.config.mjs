// Tailwind v4 runs as a PostCSS plugin. `globals.css` opens with
// `@import "tailwindcss"`, and without this file Next resolves that import as
// an ordinary module and fails the build on it.
export default {
  plugins: { "@tailwindcss/postcss": {} },
};
