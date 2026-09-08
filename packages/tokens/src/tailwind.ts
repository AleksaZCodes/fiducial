/**
 * Tailwind v4 integration — CSS only, no JS config.
 *
 * In Tailwind v4 there is no tailwind.config.ts. Everything is configured in
 * CSS. The only step needed to integrate @fiducial/tokens into a Tailwind v4
 * project is to paste the output of generateThemeCss() into your globals.css:
 *
 *   @import "tailwindcss";
 *   @import "tw-animate-css";
 *
 *   <paste generateThemeCss() output here>
 *
 * The @theme inline block inside that output registers --color-* and --radius-*
 * tokens, making bg-background, text-foreground, rounded-lg, etc. available.
 * Spacing and typography are Tailwind v4 built-ins — no registration needed.
 */
export { generateThemeCss } from './css.js'
