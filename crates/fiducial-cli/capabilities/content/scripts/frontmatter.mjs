// The frontmatter reader, shared by every deriver that reads an entry.
//
// It reads the subset of YAML a writer emits for frontmatter: scalars, quoted
// scalars, wrapped (folded) scalars, block sequences, flow sequences, and
// block sequences of flat mappings (`- label: x` followed by `  value: y`).
// Not a YAML implementation: no anchors, no nested mappings below a list item,
// no block scalars (`|`, `>`), because nothing writes those here and a parser
// that pretended to handle them would be the more dangerous failure.
//
// Anything it cannot read, it reports. Silence made the first version look like
// it worked.

/** A single scalar: quoted string, number, boolean, or plain text. */
export function parseScalar(raw) {
  const v = raw.trim();
  if (v === "") return "";
  if (
    (v.startsWith('"') && v.endsWith('"') && v.length > 1) ||
    (v.startsWith("'") && v.endsWith("'") && v.length > 1)
  ) {
    return v.slice(1, -1).replace(/\\"/g, '"');
  }
  if (v.startsWith("[") && v.endsWith("]")) {
    const inner = v.slice(1, -1).trim();
    return inner ? inner.split(",").map((p) => parseScalar(p)) : [];
  }
  if (v === "true" || v === "false") return v === "true";
  if (/^-?\d+(\.\d+)?$/.test(v)) return Number(v);
  return v;
}

const KEY = /^([A-Za-z_][\w-]*):(?:\s+(.*)|\s*)$/;
const DASH_KEY = /^(\s*)-\s+([A-Za-z_][\w-]*):(?:\s+(.*)|\s*)$/;
const DASH = /^(\s*)-(?:\s+(.*)|\s*)$/;

/**
 * `---` block → an object, plus the body after it.
 *
 * `onError` is called with a message rather than thrown, so each deriver can
 * fail in its own voice with its own file name attached.
 */
export function frontmatter(md, onError = () => {}) {
  const m = md.match(/^---\r?\n([\s\S]*?)\r?\n---\r?\n?([\s\S]*)$/);
  if (!m) {
    onError("no frontmatter. An entry starts with a --- block.");
    return { meta: {}, body: md.trim() };
  }

  // A writer folds a long value onto indented lines, quoted or not. Unfold
  // first, so a value is read whole whichever kind of line it started on. A line
  // starts something new when it is a dash, a top-level key, or a key at the
  // indent of the mapping the last dash opened; anything else continues.
  const lines = [];
  let objectIndent = -1;
  for (const raw of m[1].split("\n")) {
    const line = raw.replace(/\r$/, "");
    const text = line.trim();
    if (!text || text.startsWith("#")) continue;
    const indent = line.length - line.trimStart().length;
    const dashKey = DASH_KEY.exec(line);
    if (dashKey) {
      objectIndent = indent + 2;
      lines.push(line);
    } else if (DASH.test(line)) {
      objectIndent = -1;
      lines.push(line);
    } else if (indent === 0 && KEY.test(line)) {
      objectIndent = -1;
      lines.push(line);
    } else if (objectIndent >= 0 && indent === objectIndent && KEY.test(text)) {
      lines.push(line);
    } else if (lines.length > 0) {
      lines[lines.length - 1] += ` ${text}`;
    } else {
      lines.push(line);
    }
  }

  const meta = {};
  let key = null;
  let current = null;

  for (const line of lines) {
    const dashKey = DASH_KEY.exec(line);
    if (dashKey && key) {
      if (!Array.isArray(meta[key])) meta[key] = meta[key] === "" ? [] : [meta[key]];
      current = { [dashKey[2]]: parseScalar(dashKey[3] ?? "") };
      meta[key].push(current);
      continue;
    }

    const dash = DASH.exec(line);
    if (dash && key) {
      current = null;
      if (!Array.isArray(meta[key])) meta[key] = meta[key] === "" ? [] : [meta[key]];
      meta[key].push(parseScalar(dash[2] ?? ""));
      continue;
    }

    const indented = /^\s+/.test(line);
    const kv = KEY.exec(line.trim());
    if (indented && kv && current) {
      current[kv[1]] = parseScalar(kv[2] ?? "");
      continue;
    }

    const top = KEY.exec(line);
    if (top) {
      key = top[1];
      current = null;
      const raw = (top[2] ?? "").trim();
      // `alt: The Acme icon: a white drop` is not YAML: a plain scalar may not
      // contain ": ". Quote the value.
      if (raw && !/^["'[|>]/.test(raw) && (raw.includes(": ") || raw.endsWith(":"))) {
        onError(
          `\`${key}\` contains a colon and is not quoted, which is not valid YAML.\n` +
            `  Write it as: ${key}: "${raw.replace(/"/g, '\\"')}"`,
        );
      }
      meta[key] = parseScalar(raw);
      continue;
    }

    onError(`cannot read frontmatter line: ${JSON.stringify(line)}`);
  }

  return { meta, body: m[2].trim() };
}
