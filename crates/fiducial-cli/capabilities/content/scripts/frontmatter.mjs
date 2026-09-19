// The frontmatter reader, shared by every deriver that reads an entry.
//
// ## Why this is more than three lines
//
// It used to be one regex per line: `key: value`, take the rest of the line.
// That reads what a person writes by hand and nothing else. The content editor
// (`pnpm cms`) writes YAML the way a YAML library writes it, and the first
// entry saved through it came back as:
//
//     summary: A sentence long enough that the writer wrapped it
//       across two lines
//     tags:
//       - brand
//       - logo
//
// Both are ordinary YAML and both were unreadable: the summary lost everything
// after the first line, and `tags` came out as an empty string, which failed
// the schema check with a message about a field the author never touched.
//
// So this reads the subset a YAML writer actually emits for frontmatter:
// scalars, quoted scalars, wrapped (folded) scalars, block sequences and flow
// sequences. Not a YAML implementation — no anchors, no nested mappings, no
// block scalars (`|`, `>`), because nothing writes those here and a parser
// that pretends to handle them would be the more dangerous failure.
//
// Anything it cannot read, it reports. Silence is what made the first version
// look like it worked.

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

  const meta = {};
  const lines = m[1].split("\n");
  let key = null;

  for (const line of lines) {
    if (!line.trim() || line.trim().startsWith("#")) continue;

    // `  - item` continues the sequence under the key above it.
    const item = line.match(/^\s+-\s*(.*)$/);
    if (item && key) {
      if (!Array.isArray(meta[key])) meta[key] = meta[key] === "" ? [] : [meta[key]];
      meta[key].push(parseScalar(item[1]));
      continue;
    }

    const kv = line.match(/^([A-Za-z_][\w-]*):\s*(.*)$/);
    if (kv) {
      key = kv[1];
      meta[key] = parseScalar(kv[2]);
      continue;
    }

    // An indented line that is not a sequence item is the rest of a wrapped
    // scalar: YAML folds it onto the line above with a single space.
    if (/^\s+\S/.test(line) && key && typeof meta[key] === "string") {
      meta[key] = `${meta[key]} ${line.trim()}`.trim();
      continue;
    }

    onError(`cannot read frontmatter line: ${JSON.stringify(line)}`);
  }

  return { meta, body: m[2].trim() };
}
