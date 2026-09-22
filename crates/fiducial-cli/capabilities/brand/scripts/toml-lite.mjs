// TOML, only as far as `rasterize.mjs` needs it.
//
// It reads two files: `pipelines/raster.toml` (a name, an executor, an args
// array, an outputs array) and the `[raster]` block of `fiducial.toml`. That
// is tables, dotted table headers, strings, numbers, booleans, and arrays
// including ones written across several lines — which matters, because an
// `outputs` list is always written that way and a parser that stops at the
// newline reports *no outputs* rather than erroring.
//
// Deliberately not a dependency: this capability's pipeline has to run in a
// product that has installed nothing, and a TOML package would make the brand
// pipeline the only one with a node_modules prerequisite.

const balanced = (s) => {
  let depth = 0;
  let inString = false;
  for (let i = 0; i < s.length; i++) {
    const c = s[i];
    if (c === '"' && s[i - 1] !== "\\") inString = !inString;
    if (inString) continue;
    if (c === "[") depth++;
    if (c === "]") depth--;
    if (c === "#") break;
  }
  return depth <= 0;
};

function parseValue(raw) {
  const v = stripComment(raw).trim();
  if (v.startsWith("[")) {
    const inner = v.slice(1, v.lastIndexOf("]"));
    const items = [];
    let buf = "";
    let depth = 0;
    let inString = false;
    for (let i = 0; i < inner.length; i++) {
      const c = inner[i];
      if (c === '"' && inner[i - 1] !== "\\") inString = !inString;
      if (!inString) {
        if (c === "[" || c === "{") depth++;
        if (c === "]" || c === "}") depth--;
        if (c === "," && depth === 0) {
          if (buf.trim()) items.push(parseValue(buf));
          buf = "";
          continue;
        }
      }
      buf += c;
    }
    if (buf.trim()) items.push(parseValue(buf));
    return items;
  }
  if (v.startsWith("{")) {
    const table = {};
    for (const part of v.slice(1, v.lastIndexOf("}")).split(",")) {
      const eq = part.indexOf("=");
      if (eq === -1) continue;
      table[part.slice(0, eq).trim().replace(/^"|"$/g, "")] = parseValue(part.slice(eq + 1));
    }
    return table;
  }
  if (v.startsWith('"')) return v.slice(1, v.lastIndexOf('"')).replace(/\\"/g, '"');
  if (v === "true") return true;
  if (v === "false") return false;
  if (/^-?\d+(\.\d+)?$/.test(v)) return Number(v);
  return v;
}

/** Drop a trailing `# comment`, but not a `#` inside a quoted string. */
function stripComment(s) {
  let inString = false;
  for (let i = 0; i < s.length; i++) {
    if (s[i] === '"' && s[i - 1] !== "\\") inString = !inString;
    if (s[i] === "#" && !inString) return s.slice(0, i);
  }
  return s;
}

export function parseToml(src) {
  const root = {};
  let table = root;
  let pending = null;

  for (const raw of src.split("\n")) {
    const line = raw.trim();
    if (pending) {
      pending.buf += ` ${line}`;
      if (balanced(pending.buf)) {
        table[pending.key] = parseValue(pending.buf);
        pending = null;
      }
      continue;
    }
    if (!line || line.startsWith("#")) continue;

    const header = line.match(/^\[([^[\]]+)\]$/);
    if (header) {
      table = root;
      for (const part of header[1].split(".")) {
        const key = part.trim().replace(/^"|"$/g, "");
        table[key] ??= {};
        table = table[key];
      }
      continue;
    }

    const kv = raw.match(/^\s*("?[A-Za-z_][\w.-]*"?)\s*=\s*(.*)$/);
    if (!kv) continue;
    const key = kv[1].replace(/^"|"$/g, "");
    const value = kv[2].trim();
    if (!balanced(value)) {
      pending = { key, buf: value };
      continue;
    }
    table[key] = parseValue(value);
  }
  return root;
}
