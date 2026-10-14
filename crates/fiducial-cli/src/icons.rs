//! Icons: one library per product, chosen, and every template follows it.
//!
//! A template that shows an icon does not name a library. It writes the icon
//! once per supported library, the way shadcn's own registry does:
//!
//! ```tsx
//! <IconPlaceholder lucide="CheckIcon" tabler="IconCheck" hugeicons="Tick02Icon"
//!   phosphor="CheckIcon" remixicon="RiCheckLine" className="size-4" />
//! ```
//!
//! and [`transform`] rewrites it, at the moment the file is written into a
//! product, into a real import from the library that product chose. The choice
//! is declared once, as `iconLibrary` in `apps/web/components.json` — the file
//! the shadcn CLI already reads — so `fid add`, `fid upgrade` and `npx shadcn
//! add` can never disagree about it.
//!
//! The library table and the rewrite are ported from shadcn 4.21
//! (`packages/shadcn/src/icons/libraries.ts`,
//! `packages/shadcn/src/utils/transformers/transform-icons.ts`), so a component
//! that started as `npx shadcn add` keeps the same names it had there, and
//! switching later is `npx shadcn migrate icons --from <a> --to <b>`.
//!
//! The parser handles the form templates are written in — a self-closing
//! element whose attributes are `name="value"`, `name={expr}` with balanced
//! braces, or bare `name`. A template that writes anything else gets an error
//! from the test that walks every shipped template, not a silent mis-rewrite.

use std::path::Path;

/// The libraries a product can choose, in shadcn's order.
pub const LIBRARIES: &[Library] = &[
    Library {
        id: "lucide",
        packages: &[("lucide-react", "^1.47.0")],
        imports: &[("lucide-react", &[])],
        wrapper: None,
    },
    Library {
        id: "tabler",
        packages: &[("@tabler/icons-react", "^3.47.0")],
        imports: &[("@tabler/icons-react", &[])],
        wrapper: None,
    },
    Library {
        id: "hugeicons",
        packages: &[
            ("@hugeicons/react", "^1.1.10"),
            ("@hugeicons/core-free-icons", "^4.3.4"),
        ],
        imports: &[
            ("@hugeicons/react", &["HugeiconsIcon"]),
            ("@hugeicons/core-free-icons", &[]),
        ],
        // shadcn: `<HugeiconsIcon icon={ICON} strokeWidth={2} />`
        wrapper: Some((
            "HugeiconsIcon",
            &[("icon", "{ICON}"), ("strokeWidth", "{2}")],
        )),
    },
    Library {
        id: "phosphor",
        packages: &[("@phosphor-icons/react", "^2.1.10")],
        imports: &[("@phosphor-icons/react", &[])],
        // shadcn's usage string carries `strokeWidth={2}`, but its transformer
        // drops default props whenever the component is the icon itself — so
        // the rendered output has none, and neither does this.
        wrapper: None,
    },
    Library {
        id: "remixicon",
        packages: &[("@remixicon/react", "^4.9.0")],
        imports: &[("@remixicon/react", &[])],
        wrapper: None,
    },
];

/// shadcn's default, and so the default here.
pub const DEFAULT: &str = "lucide";

pub struct Library {
    pub id: &'static str,
    /// npm packages the rewritten code imports, with the range a template adds.
    ///
    /// Each floor is a release at least a few weeks old. pnpm 11 will not
    /// install a version younger than its `minimumReleaseAge`, and when a range
    /// admits nothing older it writes an exclusion into the product's
    /// `pnpm-workspace.yaml` instead — editing a platform file on first install.
    pub packages: &'static [(&'static str, &'static str)],
    /// Import lines: module, and fixed names. An empty name list means "the
    /// icons this file uses".
    imports: &'static [(&'static str, &'static [&'static str])],
    /// When the icon is passed to a wrapper component rather than rendered:
    /// the wrapper's name and the props it gets by default (`{ICON}` is the
    /// icon).
    wrapper: Option<(&'static str, &'static [(&'static str, &'static str)])>,
}

pub fn find(id: &str) -> Option<&'static Library> {
    LIBRARIES.iter().find(|l| l.id == id)
}

pub fn ids() -> Vec<&'static str> {
    LIBRARIES.iter().map(|l| l.id).collect()
}

const COMPONENTS_JSON: &str = "apps/web/components.json";
const TAG: &str = "<IconPlaceholder";

/// The library this product declared, or the default when it declared none.
///
/// An `iconLibrary` that names something unsupported is an error rather than a
/// fallback: rewriting a product's icons into a library it did not ask for is
/// the silent wrong answer this module exists to prevent.
pub fn configured(root: &Path) -> anyhow::Result<&'static Library> {
    let path = root.join(COMPONENTS_JSON);
    let declared = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("iconLibrary")?.as_str().map(str::to_string));
    let id = declared.as_deref().unwrap_or(DEFAULT);
    find(id).ok_or_else(|| {
        anyhow::anyhow!(
            "{COMPONENTS_JSON} declares iconLibrary = \"{id}\", which is not one of: {}",
            ids().join(", ")
        )
    })
}

/// Does this template need the rewrite at all?
pub fn uses_placeholders(source: &str) -> bool {
    source
        .match_indices(TAG)
        .any(|(i, _)| source[i + TAG.len()..].starts_with(|c: char| c.is_whitespace() || c == '/'))
}

/// Rewrite every `<IconPlaceholder … />` into `lib`'s component, drop the
/// placeholder's import, and add the library's imports.
pub fn transform(source: &str, lib: &Library) -> anyhow::Result<String> {
    if !uses_placeholders(source) {
        return Ok(source.to_string());
    }

    let mut out = String::with_capacity(source.len());
    let mut icons: Vec<String> = Vec::new();
    let mut rest = source;
    while let Some(start) = rest.find(TAG) {
        out.push_str(&rest[..start]);
        let after = &rest[start + TAG.len()..];
        // Only an element: `<IconPlaceholder` followed by whitespace or `/>`.
        // Prose about the placeholder — "written as `<IconPlaceholder>`" in a
        // doc comment — is text and passes through untouched.
        if !after.starts_with(|c: char| c.is_whitespace() || c == '/') {
            out.push_str(TAG);
            rest = after;
            continue;
        }
        let (attrs, consumed) = parse_attrs(after)?;
        rest = &after[consumed..];

        let icon = attrs
            .iter()
            .find(|(name, _)| name == lib.id)
            .and_then(|(_, v)| v.as_deref())
            .map(|v| v.trim_matches('"').to_string())
            .ok_or_else(|| {
                anyhow::anyhow!("an <IconPlaceholder> names no icon for `{}`", lib.id)
            })?;
        if !icons.contains(&icon) {
            icons.push(icon.clone());
        }

        // Everything that is not a library name is the template's own prop.
        let user: Vec<&Attr> = attrs
            .iter()
            .filter(|(name, _)| find(name).is_none())
            .collect();
        let render = |(name, value): &Attr| match value {
            Some(v) => format!("{name}={v}"),
            None => name.clone(),
        };

        match lib.wrapper {
            None => {
                let props: Vec<String> = user.iter().map(|a| render(a)).collect();
                if props.is_empty() {
                    out.push_str(&format!("<{icon} />"));
                } else {
                    out.push_str(&format!("<{icon} {} />", props.join(" ")));
                }
            }
            Some((component, defaults)) => {
                let mut props: Vec<String> = defaults
                    .iter()
                    .filter(|(d, _)| !user.iter().any(|(n, _)| n == d))
                    .map(|(d, v)| format!("{d}={}", v.replace("ICON", &icon)))
                    .collect();
                props.extend(user.iter().map(|a| render(a)));
                out.push_str(&format!("<{component} {} />", props.join(" ")));
            }
        }
    }
    out.push_str(rest);

    Ok(rewrite_imports(&out, lib, &icons))
}

/// Remove the placeholder import and add the library's, after the last import,
/// matching the file's semicolon style.
fn rewrite_imports(source: &str, lib: &Library, icons: &[String]) -> String {
    let semicolon = source
        .lines()
        .find(|l| l.trim_start().starts_with("import "))
        .is_some_and(|l| l.trim_end().ends_with(';'));
    let end = if semicolon { ";" } else { "" };

    let mut lines: Vec<String> = source
        .lines()
        .filter(|l| !(l.trim_start().starts_with("import ") && l.contains("icon-placeholder")))
        .map(str::to_string)
        .collect();

    let added: Vec<String> = lib
        .imports
        .iter()
        .map(|(module, fixed)| {
            let names = if fixed.is_empty() {
                icons.join(", ")
            } else {
                fixed.join(", ")
            };
            format!("import {{ {names} }} from \"{module}\"{end}")
        })
        .collect();

    let at = lines
        .iter()
        .rposition(|l| l.trim_start().starts_with("import "))
        .map(|i| i + 1)
        .unwrap_or(0);
    for (i, line) in added.into_iter().enumerate() {
        lines.insert(at + i, line);
    }

    let mut joined = lines.join("\n");
    if source.ends_with('\n') {
        joined.push('\n');
    }
    joined
}

/// One JSX attribute: its name, and its value as written (`"…"` or `{…}`), or
/// `None` for a bare boolean attribute.
type Attr = (String, Option<String>);

/// Parse attributes up to and including the closing `/>`. Returns them and the
/// number of bytes consumed.
fn parse_attrs(s: &str) -> anyhow::Result<(Vec<Attr>, usize)> {
    let bytes = s.as_bytes();
    let mut i = 0;
    let mut attrs = Vec::new();
    loop {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if s[i..].starts_with("/>") {
            return Ok((attrs, i + 2));
        }
        if i >= bytes.len() || bytes[i] == b'>' {
            anyhow::bail!("<IconPlaceholder> must be self-closing (`… />`)");
        }
        let name_start = i;
        while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || b"-_:".contains(&bytes[i])) {
            i += 1;
        }
        if i == name_start {
            anyhow::bail!(
                "unsupported syntax in <IconPlaceholder> near `{}`",
                &s[i..s.len().min(i + 20)]
            );
        }
        let name = s[name_start..i].to_string();
        if i < bytes.len() && bytes[i] == b'=' {
            i += 1;
            let value_start = i;
            match bytes.get(i) {
                Some(b'"') | Some(b'\'') => {
                    let q = bytes[i];
                    i += 1;
                    while i < bytes.len() && bytes[i] != q {
                        i += 1;
                    }
                    i += 1;
                }
                Some(b'{') => {
                    let mut depth = 0;
                    while i < bytes.len() {
                        match bytes[i] {
                            b'{' => depth += 1,
                            b'}' => {
                                depth -= 1;
                                if depth == 0 {
                                    i += 1;
                                    break;
                                }
                            }
                            _ => {}
                        }
                        i += 1;
                    }
                }
                _ => anyhow::bail!("attribute `{name}` in <IconPlaceholder> has no value"),
            }
            attrs.push((name, Some(s[value_start..i].to_string())));
        } else {
            attrs.push((name, None));
        }
    }
}

/// Add `lib`'s packages to the web app's dependencies, where they are missing.
///
/// A line is inserted after `"dependencies": {` rather than the file being
/// re-serialised: `package.json` is the product's, and rewriting its formatting
/// to add one key would show up as an edit to every line of it.
pub fn ensure_packages(root: &Path, lib: &Library) -> anyhow::Result<Vec<&'static str>> {
    let path = root.join("apps/web/package.json");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Ok(Vec::new());
    };
    let parsed: serde_json::Value = serde_json::from_str(&text)?;
    let deps = parsed.get("dependencies");
    let missing: Vec<&(&str, &str)> = lib
        .packages
        .iter()
        .filter(|(name, _)| deps.and_then(|d| d.get(*name)).is_none())
        .collect();
    if missing.is_empty() {
        return Ok(Vec::new());
    }
    let anchor = "\"dependencies\": {";
    let Some(at) = text.find(anchor) else {
        anyhow::bail!(
            "apps/web/package.json has no \"dependencies\" object to add {} to",
            missing[0].0
        );
    };
    let insert_at = at + anchor.len();
    let lines: String = missing
        .iter()
        .map(|(name, range)| format!("\n    \"{name}\": \"{range}\","))
        .collect();
    let mut updated = text.clone();
    updated.insert_str(insert_at, &lines);
    // Inserting before an empty object leaves a trailing comma.
    let updated = updated.replace(",\n  }", "\n  }");
    serde_json::from_str::<serde_json::Value>(&updated).map_err(|e| {
        anyhow::anyhow!("adding icon packages would break apps/web/package.json: {e}")
    })?;
    std::fs::write(&path, updated)?;
    Ok(missing.iter().map(|(name, _)| *name).collect())
}

/// Set `iconLibrary` in `components.json`, keeping its formatting.
pub fn declare(root: &Path, lib: &Library) -> anyhow::Result<()> {
    let path = root.join(COMPONENTS_JSON);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("reading {COMPONENTS_JSON}: {e}"))?;
    let mut value: serde_json::Value = serde_json::from_str(&text)?;
    let current = value.get("iconLibrary").and_then(|v| v.as_str());
    if current == Some(lib.id) {
        return Ok(());
    }
    value["iconLibrary"] = serde_json::Value::String(lib.id.to_string());
    std::fs::write(&path, serde_json::to_string_pretty(&value)? + "\n")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = r#"import * as React from "react";
import { IconPlaceholder } from "@/components/icon-placeholder";
import { cn } from "@/lib/utils";

export function A() {
  return (
    <span>
      <IconPlaceholder
        lucide="CheckIcon"
        tabler="IconCheck"
        hugeicons="Tick02Icon"
        phosphor="CheckIcon"
        remixicon="RiCheckLine"
        className={cn("size-4", "x")}
      />
      <IconPlaceholder lucide="ChevronRightIcon" tabler="IconChevronRight" hugeicons="ArrowRight01Icon" phosphor="CaretRightIcon" remixicon="RiArrowRightSLine" />
    </span>
  );
}
"#;

    #[test]
    fn lucide_renders_the_icon_and_imports_it() {
        let out = transform(SRC, find("lucide").unwrap()).unwrap();
        assert!(
            out.contains(r#"<CheckIcon className={cn("size-4", "x")} />"#),
            "{out}"
        );
        assert!(out.contains("<ChevronRightIcon />"), "{out}");
        assert!(
            out.contains(r#"import { CheckIcon, ChevronRightIcon } from "lucide-react";"#),
            "{out}"
        );
        assert!(!out.contains("IconPlaceholder"), "{out}");
        assert!(!out.contains("icon-placeholder"), "{out}");
    }

    #[test]
    fn hugeicons_wraps_and_imports_both_packages() {
        let out = transform(SRC, find("hugeicons").unwrap()).unwrap();
        assert!(
            out.contains(r#"<HugeiconsIcon icon={Tick02Icon} strokeWidth={2} className={cn("size-4", "x")} />"#),
            "{out}"
        );
        assert!(out.contains(r#"import { HugeiconsIcon } from "@hugeicons/react";"#));
        assert!(out.contains(
            r#"import { Tick02Icon, ArrowRight01Icon } from "@hugeicons/core-free-icons";"#
        ));
    }

    #[test]
    fn every_library_rewrites_every_placeholder() {
        for lib in LIBRARIES {
            let out = transform(SRC, lib).unwrap();
            assert!(!out.contains("IconPlaceholder"), "{}: {out}", lib.id);
            for (module, _) in lib.imports {
                assert!(
                    out.contains(&format!("from \"{module}\"")),
                    "{}: {out}",
                    lib.id
                );
            }
        }
    }

    #[test]
    fn imports_follow_the_files_semicolon_style() {
        let no_semi = SRC.replace("\";\n", "\"\n");
        let out = transform(&no_semi, find("tabler").unwrap()).unwrap();
        assert!(
            out.contains(r#"import { IconCheck, IconChevronRight } from "@tabler/icons-react""#)
                && !out.contains(r#"@tabler/icons-react";"#),
            "{out}"
        );
    }

    #[test]
    fn prose_about_the_placeholder_is_not_an_element() {
        let src = format!("/** Icons are written as `<IconPlaceholder>`. */\n{SRC}");
        let out = transform(&src, find("lucide").unwrap()).unwrap();
        assert!(out.contains("`<IconPlaceholder>`"), "{out}");
        assert!(out.contains("<CheckIcon"), "{out}");
        assert!(!uses_placeholders("/** `<IconPlaceholder>` */"));
    }

    #[test]
    fn a_file_without_placeholders_is_untouched() {
        let src = "import x from \"y\";\nexport const a = 1;\n";
        assert_eq!(transform(src, find("lucide").unwrap()).unwrap(), src);
    }

    #[test]
    fn a_placeholder_missing_the_chosen_library_is_an_error() {
        let src = "import a from \"b\";\nconst x = <IconPlaceholder lucide=\"CheckIcon\" />;\n";
        assert!(transform(src, find("tabler").unwrap()).is_err());
    }

    /// Every shipped template that uses a placeholder names an icon for every
    /// library, in syntax the parser accepts. A template that forgot one would
    /// otherwise fail only in the product that picked that library.
    #[test]
    fn every_shipped_placeholder_covers_every_library() {
        for cap in crate::capability::builtins() {
            let entries = cap
                .templates
                .iter()
                .chain(cap.conditional_templates.values().flatten());
            for entry in entries {
                if !uses_placeholders(&entry.content) {
                    continue;
                }
                for lib in LIBRARIES {
                    transform(&entry.content, lib).unwrap_or_else(|e| {
                        panic!("{}/{} for {}: {e}", cap.id, entry.path, lib.id)
                    });
                }
            }
        }
    }

    #[test]
    fn packages_are_added_once_and_keep_the_file_valid() {
        let tmp = tempfile::tempdir().unwrap();
        let web = tmp.path().join("apps/web");
        std::fs::create_dir_all(&web).unwrap();
        std::fs::write(
            web.join("package.json"),
            "{\n  \"dependencies\": {\n    \"next\":  \"^16.0.0\"\n  }\n}\n",
        )
        .unwrap();
        let lib = find("hugeicons").unwrap();
        assert_eq!(ensure_packages(tmp.path(), lib).unwrap().len(), 2);
        assert!(ensure_packages(tmp.path(), lib).unwrap().is_empty());
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(web.join("package.json")).unwrap())
                .unwrap();
        assert!(v["dependencies"]["@hugeicons/react"].is_string());
        assert_eq!(v["dependencies"]["next"], "^16.0.0");
    }

    #[test]
    fn an_unsupported_declared_library_is_an_error_not_a_fallback() {
        let tmp = tempfile::tempdir().unwrap();
        let web = tmp.path().join("apps/web");
        std::fs::create_dir_all(&web).unwrap();
        std::fs::write(web.join("components.json"), r#"{"iconLibrary":"radix"}"#).unwrap();
        assert!(configured(tmp.path()).is_err());
        std::fs::write(web.join("components.json"), r#"{"iconLibrary":"phosphor"}"#).unwrap();
        assert_eq!(configured(tmp.path()).unwrap().id, "phosphor");
    }
}
