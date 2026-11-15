import { Marked } from "marked";

const escapeHtml = (value: string) =>
  value.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

// Content is written by people, but raw HTML in it is still escaped: the editor
// is a Markdown editor, and markup that arrives through it is a typo, or worse.
const marked = new Marked({ gfm: true, renderer: { html: ({ text }) => escapeHtml(text) } });

/** An entry's body, as HTML. Server-side only: nothing here ships to the browser. */
export function renderBody(markdown: string): string {
  return marked.parse(markdown, { async: false });
}
