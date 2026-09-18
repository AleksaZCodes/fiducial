//! `fid-design` — derive the theme stylesheet from `design-system.md`.
//!
//! # Why the declaration is a Markdown file with TOML inside it
//!
//! A design system has two halves that cannot be separated without one of them
//! rotting. There is prose — *why this face, what this product is not, what the
//! annotation budget is* — and there are values. Put the values in
//! `fiducial.toml` and the prose in a document, and you have two declarations of
//! the same taste that disagree the first time either moves.
//!
//! So `design-system.md` holds both: prose everywhere, and fenced TOML blocks
//! tagged `fid:<section>` carrying the machine-readable part. The document a
//! person reads and the input a pipeline parses are one file, and the reason
//! sits directly above the value it explains.
//!
//! ```text
//! ```toml fid:color
//! [light]
//! primary = "oklch(0.56 0.15 48)"
//! ```
//! ```
//!
//! Untagged fences are ignored, so an example snippet in the prose is not
//! mistaken for a declaration.
//!
//! # What it writes
//!
//! One file: `apps/web/src/app/tokens.css` (wherever the pipeline points it).
//! That file is derived and guard-blocked. `globals.css` stays product-owned
//! and imports it — because the base layer, the `@import`s and the commentary
//! around them are prose too, and a generator that owned the whole stylesheet
//! would delete them on the next run.
//!
//! # The contrast check
//!
//! `fid:contrast` declares pairs and their minimum ratios, and this executor
//! computes them. A palette that fails AA fails `fid derive` — which is the
//! difference between a design system that documents a rule and one that holds
//! it. "Re-measure when you change a colour, do not eyeball it" is advice until
//! something measures.

use std::collections::BTreeMap;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

// ── The declaration ──────────────────────────────────────────────────────────

/// One type role: the family, the weights to load, and what follows it.
#[derive(Debug, Clone, Deserialize)]
pub struct TypeRole {
    /// Exact family name as the foundry spells it. Never a category.
    pub family: String,
    /// Weights to load. Loading more is waste; fewer breaks the scale.
    #[serde(default)]
    pub weights: Vec<u32>,
    /// Stack appended after the family, for the swap window and for failure.
    #[serde(default)]
    pub fallback: Vec<String>,
    /// The variable a font loader (next/font, `@font-face`) binds the family to.
    ///
    /// When present it goes *first* in the stack, ahead of the quoted family
    /// name — so a self-hosted face is used when it has loaded and the same
    /// family resolves by name if it has not.
    #[serde(default)]
    pub var: Option<String>,
}

/// One step of the named scale.
#[derive(Debug, Clone, Deserialize)]
pub struct ScaleStep {
    /// Which of the four roles renders it.
    pub role: String,
    pub size: String,
    pub leading: String,
    pub weight: String,
    #[serde(default)]
    pub tracking: Option<String>,
    /// Uppercase this step. Eyebrows, and nothing else so far.
    #[serde(default)]
    pub uppercase: bool,
    /// `balance` for headings, `pretty` for running text, unset otherwise.
    #[serde(default)]
    pub wrap: Option<String>,
}

/// Steps that shrink below a breakpoint.
///
/// Display type is the one part of a scale that cannot be responsive by
/// accident: a 3.75rem headline on a 375px screen is six lines, and in Serbian
/// it is eight.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct Clamp {
    /// Max-width the overrides apply below.
    pub below: String,
    /// Step name → smaller size.
    #[serde(flatten)]
    pub sizes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Shape {
    /// `round`, `chamfer`, or `square`.
    pub strategy: String,
    pub panel: String,
    pub control: String,
    pub chip: String,
    pub border: String,
    pub hair: String,
    pub doodle_stroke: String,
    /// Radius under the `round` strategy. Ignored otherwise — see `radius()`.
    #[serde(default)]
    pub radius: Option<String>,
}

impl Shape {
    /// What `--radius` resolves to.
    ///
    /// Forced to 0 unless the strategy is `round`: under `chamfer` and `square`
    /// the corner comes from the `.cham-*` classes, and a live radius would
    /// round the chamfer's own clip. The two treatments fight and the chamfer
    /// loses.
    fn radius(&self) -> &str {
        if self.strategy == "round" {
            self.radius.as_deref().unwrap_or("0.625rem")
        } else {
            "0rem"
        }
    }
}

/// One contrast pair to measure, named by token.
#[derive(Debug, Clone, Deserialize)]
pub struct ContrastPair {
    pub fg: String,
    pub bg: String,
    pub min: f64,
    /// `light`, `dark`, or `both` (the default).
    #[serde(default)]
    pub theme: Option<String>,
    /// What the pair is, for the failure message.
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ContrastBlock {
    #[serde(default)]
    pairs: Vec<ContrastPair>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ColorBlock {
    #[serde(default)]
    light: BTreeMap<String, String>,
    #[serde(default)]
    dark: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ScaleBlock {
    #[serde(default)]
    clamp: Option<Clamp>,
    #[serde(flatten)]
    steps: BTreeMap<String, ScaleStep>,
}

/// Everything `design-system.md` declares that a machine can act on.
#[derive(Debug, Clone)]
pub struct DesignSystem {
    light: BTreeMap<String, String>,
    dark: BTreeMap<String, String>,
    roles: BTreeMap<String, TypeRole>,
    steps: BTreeMap<String, ScaleStep>,
    clamp: Option<Clamp>,
    shape: Shape,
    contrast: Vec<ContrastPair>,
    /// Step order as written, so the generated CSS reads like the document.
    step_order: Vec<String>,
}

// ── Parsing ──────────────────────────────────────────────────────────────────

/// Pull every ` ```toml fid:<tag> ` block out of a Markdown document.
///
/// Deliberately only matches fences whose info string carries the `fid:` tag:
/// a design system is full of example snippets, and one of them being read as
/// a declaration is a worse failure than a missing block, because it is silent.
fn fenced_blocks(md: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut lines = md.lines().peekable();

    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();
        let Some(info) = trimmed.strip_prefix("```") else {
            continue;
        };
        let Some(tag) = info.split_whitespace().find_map(|w| w.strip_prefix("fid:")) else {
            continue;
        };
        let mut body = String::new();
        for l in lines.by_ref() {
            if l.trim_start().starts_with("```") {
                break;
            }
            body.push_str(l);
            body.push('\n');
        }
        // Last block wins is wrong here: a duplicated tag means the document
        // says a thing twice, and the second copy will not be the one edited.
        if out.insert(tag.to_string(), body).is_some() {
            out.insert(
                format!("__dup__{tag}"),
                format!("`fid:{tag}` is declared more than once"),
            );
        }
    }
    out
}

/// Preserve the order step names appear in the `fid:scale` block.
///
/// `BTreeMap` sorts, and a scale read back alphabetically (`body, display,
/// eyebrow, h1…`) is a scale nobody can check against the document.
fn declared_order(body: &str) -> Vec<String> {
    body.lines()
        .filter_map(|l| {
            let t = l.trim();
            if t.starts_with('#') || t.starts_with('[') {
                return None;
            }
            t.split_once('=').map(|(k, _)| k.trim().to_string())
        })
        .filter(|k| k != "clamp")
        .collect()
}

pub fn parse(md: &str) -> Result<DesignSystem> {
    let blocks = fenced_blocks(md);

    for key in blocks.keys() {
        if let Some(tag) = key.strip_prefix("__dup__") {
            bail!(
                "design-system.md declares `fid:{tag}` more than once — \
                 the second copy is the one nobody will edit"
            );
        }
    }

    let need = |tag: &str| -> Result<&String> {
        blocks.get(tag).ok_or_else(|| {
            anyhow::anyhow!(
                "design-system.md has no ```toml fid:{tag} block.\n  \
                 The document is the declaration; a missing section is not a \
                 default, it is an unanswered question."
            )
        })
    };

    let color: ColorBlock =
        toml::from_str(need("color")?).context("parsing the `fid:color` block")?;
    let roles: BTreeMap<String, TypeRole> =
        toml::from_str(need("type")?).context("parsing the `fid:type` block")?;
    let scale_body = need("scale")?;
    let scale: ScaleBlock = toml::from_str(scale_body).context("parsing the `fid:scale` block")?;
    let shape: Shape = toml::from_str(need("shape")?).context("parsing the `fid:shape` block")?;
    let contrast: ContrastBlock = match blocks.get("contrast") {
        Some(b) => toml::from_str(b).context("parsing the `fid:contrast` block")?,
        None => ContrastBlock::default(),
    };

    for r in ["display", "body", "script", "mono"] {
        if !roles.contains_key(r) {
            bail!(
                "the `fid:type` block is missing the `{r}` role.\n  \
                 All four are required: display, body, script, mono."
            );
        }
    }
    if roles["display"].family == roles["body"].family {
        bail!(
            "display and body are both `{}`.\n  \
             Four roles exist so the headline is not the paragraph at a larger \
             size; collapsing them gives one voice at six sizes.",
            roles["display"].family
        );
    }

    for (name, step) in &scale.steps {
        if !roles.contains_key(&step.role) {
            bail!(
                "scale step `{name}` names role `{}`, which is not one of \
                 display, body, script, mono",
                step.role
            );
        }
    }

    let mut step_order = declared_order(scale_body);
    // Anything the order scan missed (a step written as its own `[table]`)
    // still has to be emitted — appended rather than dropped.
    for k in scale.steps.keys() {
        if !step_order.contains(k) {
            step_order.push(k.clone());
        }
    }
    step_order.retain(|k| scale.steps.contains_key(k));

    Ok(DesignSystem {
        light: color.light,
        dark: color.dark,
        roles,
        steps: scale.steps,
        clamp: scale.clamp,
        shape,
        contrast: contrast.pairs,
        step_order,
    })
}

// ── Colour maths ─────────────────────────────────────────────────────────────

/// Parse `oklch(L C H)`, `oklch(L C H / A)`, `#RGB` or `#RRGGBB` into linear-ish
/// sRGB in 0..1.
///
/// Returns `None` for anything else — a `var()` reference, a named colour, a
/// gradient. Those are legal in the declaration; they just cannot be measured,
/// and a pair naming one is reported rather than silently passed.
fn to_srgb(value: &str) -> Option<[f64; 3]> {
    let v = value.trim();
    if let Some(hex) = v.strip_prefix('#') {
        let expand = |c: char| -> Option<u8> { c.to_digit(16).map(|d| (d * 17) as u8) };
        let (r, g, b) = match hex.len() {
            3 => {
                let mut it = hex.chars();
                (
                    expand(it.next()?)?,
                    expand(it.next()?)?,
                    expand(it.next()?)?,
                )
            }
            6 => (
                u8::from_str_radix(&hex[0..2], 16).ok()?,
                u8::from_str_radix(&hex[2..4], 16).ok()?,
                u8::from_str_radix(&hex[4..6], 16).ok()?,
            ),
            _ => return None,
        };
        return Some([r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0]);
    }

    let inner = v.strip_prefix("oklch(")?.strip_suffix(')')?;
    // Alpha is dropped: a contrast ratio against a translucent colour depends on
    // what is behind it, which this file cannot know. Measuring the opaque
    // colour is the honest approximation and it is the conservative direction.
    let head = inner.split('/').next()?;
    let mut parts = head.split_whitespace();
    let l: f64 = parts.next()?.parse().ok()?;
    let c: f64 = parts.next()?.parse().ok()?;
    let h: f64 = parts.next()?.parse().ok()?;

    let hr = h.to_radians();
    let (a, b) = (c * hr.cos(), c * hr.sin());

    let l_ = l + 0.396_337_777_4 * a + 0.215_803_757_3 * b;
    let m_ = l - 0.105_561_345_8 * a - 0.063_854_172_8 * b;
    let s_ = l - 0.089_484_177_5 * a - 1.291_485_548_0 * b;
    let (lc, mc, sc) = (l_ * l_ * l_, m_ * m_ * m_, s_ * s_ * s_);

    let lin = [
        4.076_741_662_1 * lc - 3.307_711_591_3 * mc + 0.230_969_929_2 * sc,
        -1.268_438_004_6 * lc + 2.609_757_401_1 * mc - 0.341_319_396_5 * sc,
        -0.004_196_086_3 * lc - 0.703_418_614_7 * mc + 1.707_614_701_0 * sc,
    ];

    Some(lin.map(|x| {
        let x = x.clamp(0.0, 1.0);
        if x > 0.003_130_8 {
            1.055 * x.powf(1.0 / 2.4) - 0.055
        } else {
            12.92 * x
        }
    }))
}

/// WCAG relative luminance from gamma-encoded sRGB.
fn luminance(rgb: [f64; 3]) -> f64 {
    let lin = rgb.map(|c| {
        if c <= 0.040_45 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    });
    0.2126 * lin[0] + 0.7152 * lin[1] + 0.0722 * lin[2]
}

/// WCAG 2.1 contrast ratio. Order-independent.
fn contrast_ratio(a: [f64; 3], b: [f64; 3]) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

/// Render an sRGB triple as `#RRGGBB`, for the generated file's comments.
fn to_hex(rgb: [f64; 3]) -> String {
    let b = rgb.map(|c| (c * 255.0).round().clamp(0.0, 255.0) as u8);
    format!("#{:02X}{:02X}{:02X}", b[0], b[1], b[2])
}

// ── Contrast check ───────────────────────────────────────────────────────────

/// One measured pair.
pub struct Measurement {
    pub theme: &'static str,
    pub fg: String,
    pub bg: String,
    pub ratio: Option<f64>,
    pub min: f64,
    pub note: Option<String>,
}

impl Measurement {
    fn failed(&self) -> bool {
        match self.ratio {
            Some(r) => r < self.min,
            // Unmeasurable is not a pass. The declaration asked for a number.
            None => true,
        }
    }
}

impl DesignSystem {
    fn theme(&self, which: &str) -> &BTreeMap<String, String> {
        if which == "dark" {
            &self.dark
        } else {
            &self.light
        }
    }

    /// Measure every declared pair in every theme it applies to.
    pub fn measure(&self) -> Vec<Measurement> {
        let mut out = Vec::new();
        for pair in &self.contrast {
            let themes: &[&'static str] = match pair.theme.as_deref() {
                Some("light") => &["light"],
                Some("dark") => &["dark"],
                _ => &["light", "dark"],
            };
            for t in themes {
                let table = self.theme(t);
                // A theme that declares no override for a slot inherits light's.
                let look =
                    |k: &String| -> Option<&String> { table.get(k).or_else(|| self.light.get(k)) };
                let ratio = match (look(&pair.fg), look(&pair.bg)) {
                    (Some(f), Some(b)) => match (to_srgb(f), to_srgb(b)) {
                        (Some(f), Some(b)) => Some(contrast_ratio(f, b)),
                        _ => None,
                    },
                    _ => None,
                };
                out.push(Measurement {
                    theme: t,
                    fg: pair.fg.clone(),
                    bg: pair.bg.clone(),
                    ratio,
                    min: pair.min,
                    note: pair.note.clone(),
                });
            }
        }
        out
    }

    /// Fail if any declared pair misses its minimum.
    pub fn check_contrast(&self) -> Result<usize> {
        let measured = self.measure();
        let failures: Vec<&Measurement> = measured.iter().filter(|m| m.failed()).collect();
        if failures.is_empty() {
            return Ok(measured.len());
        }
        let mut msg = String::from("the palette fails its own declared contrast pairs:\n");
        for f in &failures {
            match f.ratio {
                Some(r) => msg.push_str(&format!(
                    "    {} · {} on {} — {r:.2}, needs {:.1}\n",
                    f.theme, f.fg, f.bg, f.min
                )),
                None => msg.push_str(&format!(
                    "    {} · {} on {} — cannot be measured \
                     (unknown token, or a value that is not oklch()/hex)\n",
                    f.theme, f.fg, f.bg
                )),
            }
            if let Some(n) = &f.note {
                msg.push_str(&format!("      {n}\n"));
            }
        }
        msg.push_str(
            "  Change the colour in design-system.md, not the minimum. \
             The minimum is the requirement.",
        );
        bail!(msg)
    }
}

// ── Generation ───────────────────────────────────────────────────────────────

/// The `--color-*` slots `@theme inline` maps, in the shadcn order.
const COLOR_INLINE_ORDER: &[&str] = &[
    "background",
    "foreground",
    "card",
    "card-foreground",
    "popover",
    "popover-foreground",
    "primary",
    "primary-foreground",
    "secondary",
    "secondary-foreground",
    "muted",
    "muted-foreground",
    "accent",
    "accent-foreground",
    "destructive",
    "border",
    "input",
    "ring",
    "chart-1",
    "chart-2",
    "chart-3",
    "chart-4",
    "chart-5",
    "sidebar",
    "sidebar-foreground",
    "sidebar-primary",
    "sidebar-primary-foreground",
    "sidebar-accent",
    "sidebar-accent-foreground",
    "sidebar-border",
    "sidebar-ring",
    "doodle-ink",
    "doodle-accent",
];

impl DesignSystem {
    /// The font stack for one role.
    ///
    /// A family name is quoted; keyword fallbacks (`ui-serif`, `sans-serif`)
    /// must stay unquoted or they stop being keywords. The loader variable,
    /// when declared, comes first.
    fn stack(&self, role: &str) -> String {
        let r = &self.roles[role];
        let mut parts = Vec::new();
        if let Some(v) = &r.var {
            parts.push(format!("var({v})"));
        }
        parts.push(format!("\"{}\"", r.family));
        parts.extend(r.fallback.iter().cloned());
        parts.join(", ")
    }

    pub fn generate_css(&self, source: &str) -> String {
        let mut s = String::new();

        s.push_str(&format!(
            "/* ───────────────────────────────────────────────────────────────────────────\n\
             \x20  GENERATED by `fid derive` (pipeline: design) from {source}.\n\
             \x20  Do not edit. Every value here is declared there, with the reason next to it.\n\
             \n\
             \x20  Your own stylesheet imports this file and keeps everything that is not a\n\
             \x20  token — the base layer, the marks layer, the commentary. This file holds\n\
             \x20  only what the declaration decides.\n\
             \x20  ────────────────────────────────────────────────────────────────────────── */\n\n"
        ));

        // ── :root ────────────────────────────────────────────────────────────
        s.push_str(":root {\n");
        for (k, v) in &self.light {
            match to_srgb(v) {
                Some(rgb) => s.push_str(&format!("  --{k}: {v}; /* {} */\n", to_hex(rgb))),
                None => s.push_str(&format!("  --{k}: {v};\n")),
            }
        }

        s.push_str(
            "\n  /* Type roles. Four, because two cannot say \"the display face is not\n\
             \x20    the body face\". Declared as --type-* and exposed as font-* below:\n\
             \x20    Tailwind owns the --font-* namespace, and a variable cannot be\n\
             \x20    defined in terms of itself. */\n",
        );
        for role in ["display", "body", "script", "mono"] {
            // The weights are not a CSS value — they are what the font loader
            // has to pull. Emitting them as a comment puts the list where the
            // person wiring next/font or @font-face is actually looking, rather
            // than leaving it declared and never read.
            let w = &self.roles[role].weights;
            if w.is_empty() {
                s.push_str(&format!("  --type-{role}: {};\n", self.stack(role)));
            } else {
                let list = w.iter().map(u32::to_string).collect::<Vec<_>>().join(", ");
                s.push_str(&format!(
                    "  --type-{role}: {}; /* load {list} */\n",
                    self.stack(role)
                ));
            }
        }

        s.push_str(&format!(
            "\n  /* Shape. Corner strategy: {}. */\n",
            self.shape.strategy
        ));
        if self.shape.strategy != "round" {
            s.push_str(
                "  /* Not `round`: the corner is cut by the .cham-* classes, so the\n\
                 \x20    radius scale has to resolve to nothing or the two treatments fight. */\n",
            );
        }
        s.push_str(&format!("  --radius: {};\n", self.shape.radius()));
        // No column alignment anywhere in this file: it is read by a formatter
        // before it is read by a person, and padded colons are the first thing
        // any of them undoes.
        s.push_str(&format!("  --corner-panel: {};\n", self.shape.panel));
        s.push_str(&format!("  --corner-control: {};\n", self.shape.control));
        s.push_str(&format!("  --corner-chip: {};\n", self.shape.chip));
        s.push_str(&format!("  --border-w: {};\n", self.shape.border));
        s.push_str(&format!("  --hair-w: {};\n", self.shape.hair));
        s.push_str("\n  /* Annotation stroke. Fixed weight — a mark is drawn, not scaled. */\n");
        s.push_str(&format!(
            "  --doodle-stroke: {};\n",
            self.shape.doodle_stroke
        ));
        s.push_str("}\n");

        // ── dark ─────────────────────────────────────────────────────────────
        if !self.dark.is_empty() {
            s.push_str(
                "\n/* `:root.dark` rather than `.dark`: at equal specificity the dark block\n\
                 \x20  only wins because it comes second, and a file that depends on its own\n\
                 \x20  ordering breaks the first time something is inserted above it. */\n",
            );
            s.push_str(":root.dark {\n");
            for (k, v) in &self.dark {
                match to_srgb(v) {
                    Some(rgb) => s.push_str(&format!("  --{k}: {v}; /* {} */\n", to_hex(rgb))),
                    None => s.push_str(&format!("  --{k}: {v};\n")),
                }
            }
            s.push_str("}\n");
        }

        // ── @theme inline ────────────────────────────────────────────────────
        s.push_str("\n@theme inline {\n");
        for slot in COLOR_INLINE_ORDER {
            if self.light.contains_key(*slot) {
                s.push_str(&format!("  --color-{slot}: var(--{slot});\n"));
            }
        }
        s.push('\n');
        for role in ["display", "body", "script", "mono"] {
            s.push_str(&format!("  --font-{role}: var(--type-{role});\n"));
        }
        s.push_str(
            "  /* Anything asking for the default sans gets the body face, so an\n\
             \x20    unstyled paragraph is already right rather than already wrong. */\n\
             \x20 --font-sans: var(--type-body);\n",
        );
        s.push('\n');
        for (name, delta) in [
            ("sm", "- 4px"),
            ("md", "- 2px"),
            ("lg", ""),
            ("xl", "+ 2px"),
            ("2xl", "+ 4px"),
            ("3xl", "+ 8px"),
        ] {
            if delta.is_empty() {
                s.push_str(&format!("  --radius-{name}: var(--radius);\n"));
            } else {
                s.push_str(&format!(
                    "  --radius-{name}: calc(var(--radius) {delta});\n"
                ));
            }
        }
        s.push_str("}\n");

        // ── the named scale ──────────────────────────────────────────────────
        s.push_str(
            "\n/* The named type scale. One class per step, because an inline\n\
             \x20  `text-[2.375rem] leading-[1.06]` is a scale step that exists on one page\n\
             \x20  and nowhere else — and the next page invents a different one.\n\
             \n\
             \x20  Needs an eleventh step? Add it to design-system.md. Do not inline it. */\n",
        );
        s.push_str("@layer components {\n");
        for name in &self.step_order {
            let st = &self.steps[name];
            s.push_str(&format!("  .type-{name} {{\n"));
            s.push_str(&format!("    font-family: var(--type-{});\n", st.role));
            s.push_str(&format!("    font-size: {};\n", st.size));
            s.push_str(&format!("    line-height: {};\n", st.leading));
            s.push_str(&format!("    font-weight: {};\n", st.weight));
            if let Some(t) = &st.tracking {
                s.push_str(&format!("    letter-spacing: {t};\n"));
            }
            if st.uppercase {
                s.push_str("    text-transform: uppercase;\n");
            }
            if let Some(w) = &st.wrap {
                s.push_str(&format!("    text-wrap: {w};\n"));
            }
            s.push_str("  }\n");
        }

        if let Some(clamp) = &self.clamp {
            if !clamp.sizes.is_empty() {
                s.push_str(&format!(
                    "\n  /* The display face set large is the one part of a scale that cannot\n\
                     \x20    be responsive by accident: a 3.75rem headline on a phone is six\n\
                     \x20    lines, and in a language with longer words it is eight. */\n\
                     \x20 @media (max-width: {}) {{\n",
                    clamp.below
                ));
                for (step, size) in &clamp.sizes {
                    s.push_str(&format!(
                        "    .type-{step} {{\n      font-size: {size};\n    }}\n"
                    ));
                }
                s.push_str("  }\n");
            }
        }
        s.push_str("}\n");

        // ── the measured pairs, as a record ──────────────────────────────────
        let measured = self.measure();
        if !measured.is_empty() {
            s.push_str(
                "\n/* Measured contrast. `fid derive` recomputes these from the values above\n\
                 \x20  and fails if any falls under its declared minimum — so this block is a\n\
                 \x20  record of a check that ran, not a claim someone typed.\n\
                 *\n",
            );
            for m in &measured {
                match m.ratio {
                    Some(r) => s.push_str(&format!(
                        " *   {:<5} {} on {} — {r:.2} (min {:.1})\n",
                        m.theme, m.fg, m.bg, m.min
                    )),
                    None => s.push_str(&format!(
                        " *   {:<5} {} on {} — not measurable\n",
                        m.theme, m.fg, m.bg
                    )),
                }
            }
            s.push_str(" */\n");
        }

        s
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = r##"
# Product — Design System

Prose about the product.

```toml
# An untagged fence. This is an example in the prose and must be ignored.
primary = "#0000FF"
```

```toml fid:color
[light]
background = "oklch(0.988 0.004 65)"
foreground = "oklch(0.175 0.012 50)"
primary = "oklch(0.56 0.15 48)"
primary-foreground = "oklch(0.99 0.004 70)"
muted-foreground = "oklch(0.505 0.018 52)"
doodle-ink = "oklch(0.58 0.055 48)"

[dark]
background = "oklch(0.155 0.014 48)"
foreground = "oklch(0.965 0.006 70)"
```

```toml fid:type
display = { family = "Chakra Petch", weights = [600], fallback = ["sans-serif"], var = "--font-cp" }
body = { family = "Source Serif 4", weights = [400], fallback = ["serif"] }
script = { family = "Caveat", weights = [500], fallback = ["cursive"] }
mono = { family = "JetBrains Mono", weights = [400], fallback = ["monospace"] }
```

```toml fid:scale
display = { role = "display", size = "3.75rem", leading = "1.04", weight = "600", tracking = "-0.025em", wrap = "balance" }
body = { role = "body", size = "1rem", leading = "1.65", weight = "400", wrap = "pretty" }
eyebrow = { role = "display", size = "0.75rem", leading = "1", weight = "600", tracking = "0.18em", uppercase = true }
clamp = { below = "40rem", display = "2.5rem" }
```

```toml fid:shape
strategy = "chamfer"
panel = "1.5rem"
control = "0.75rem"
chip = "0.5rem"
border = "2px"
hair = "1px"
doodle_stroke = "2.25px"
```

```toml fid:contrast
pairs = [
  { fg = "foreground", bg = "background", min = 4.5 },
  { fg = "primary", bg = "background", min = 4.5, theme = "light" },
]
```
"##;

    fn sys() -> DesignSystem {
        parse(DOC).expect("the fixture should parse")
    }

    #[test]
    fn an_untagged_fence_is_not_a_declaration() {
        // The example block in the prose declares `primary = "#0000FF"`. If it
        // were read, the product would silently ship a blue primary.
        let s = sys();
        assert_eq!(s.light["primary"], "oklch(0.56 0.15 48)");
    }

    #[test]
    fn every_section_is_parsed() {
        let s = sys();
        assert_eq!(s.roles.len(), 4);
        assert_eq!(s.steps.len(), 3);
        assert_eq!(s.contrast.len(), 2);
        assert_eq!(s.shape.strategy, "chamfer");
        assert_eq!(s.clamp.as_ref().unwrap().below, "40rem");
    }

    #[test]
    fn the_scale_keeps_the_order_the_document_wrote_it_in() {
        // Alphabetical would be body, display, eyebrow — a scale nobody can
        // check against the document it came from.
        assert_eq!(sys().step_order, vec!["display", "body", "eyebrow"]);
    }

    #[test]
    fn a_missing_section_is_an_error_not_a_default() {
        let doc = DOC.replace("fid:shape", "shape-was-here");
        let err = parse(&doc).unwrap_err().to_string();
        assert!(err.contains("fid:shape"), "{err}");
    }

    #[test]
    fn a_section_declared_twice_is_refused() {
        let doc = format!("{DOC}\n```toml fid:shape\nstrategy = \"round\"\n```\n");
        let err = parse(&doc).unwrap_err().to_string();
        assert!(err.contains("more than once"), "{err}");
    }

    #[test]
    fn display_and_body_may_not_be_the_same_family() {
        let doc = DOC.replace("family = \"Source Serif 4\"", "family = \"Chakra Petch\"");
        let err = parse(&doc).unwrap_err().to_string();
        assert!(err.contains("six sizes"), "{err}");
    }

    #[test]
    fn a_step_naming_an_unknown_role_is_refused() {
        let doc = DOC.replace(
            r#"eyebrow = { role = "display""#,
            r#"eyebrow = { role = "label""#,
        );
        let err = parse(&doc).unwrap_err().to_string();
        assert!(err.contains("not one of"), "{err}");
    }

    #[test]
    fn oklch_matches_the_hex_the_declaration_documents() {
        // These two are checked into fon's design-system.md as hex. If the
        // conversion drifts, the generated comments start lying.
        assert_eq!(to_hex(to_srgb("oklch(0.56 0.15 48)").unwrap()), "#B85207");
        assert_eq!(to_hex(to_srgb("oklch(0.7 0.18 48)").unwrap()), "#F4741E");
    }

    #[test]
    fn hex_parses_in_both_lengths() {
        assert_eq!(to_srgb("#FFF"), to_srgb("#FFFFFF"));
        assert_eq!(to_hex(to_srgb("#B85207").unwrap()), "#B85207");
    }

    #[test]
    fn black_on_white_is_twenty_one_to_one() {
        let r = contrast_ratio(to_srgb("#000000").unwrap(), to_srgb("#FFFFFF").unwrap());
        assert!((r - 21.0).abs() < 0.01, "{r}");
    }

    #[test]
    fn contrast_is_order_independent() {
        let a = to_srgb("oklch(0.56 0.15 48)").unwrap();
        let b = to_srgb("oklch(0.988 0.004 65)").unwrap();
        assert!((contrast_ratio(a, b) - contrast_ratio(b, a)).abs() < 1e-9);
    }

    #[test]
    fn a_passing_palette_passes() {
        assert_eq!(sys().check_contrast().unwrap(), 3);
    }

    #[test]
    fn a_failing_pair_fails_the_derive() {
        // Mid-grey body text on a near-white ground: the classic quiet failure.
        let doc = DOC.replace(
            r#"foreground = "oklch(0.175 0.012 50)""#,
            r#"foreground = "oklch(0.72 0.012 50)""#,
        );
        let err = parse(&doc)
            .unwrap()
            .check_contrast()
            .unwrap_err()
            .to_string();
        assert!(err.contains("fails its own declared contrast"), "{err}");
        assert!(err.contains("foreground on background"), "{err}");
        assert!(
            err.contains("the minimum is the requirement")
                || err.contains("The minimum is the requirement"),
            "{err}"
        );
    }

    #[test]
    fn a_pair_that_cannot_be_measured_is_a_failure_not_a_pass() {
        let doc = DOC.replace(
            r#"{ fg = "foreground", bg = "background", min = 4.5 },"#,
            r#"{ fg = "nonesuch", bg = "background", min = 4.5 },"#,
        );
        let err = parse(&doc)
            .unwrap()
            .check_contrast()
            .unwrap_err()
            .to_string();
        assert!(err.contains("cannot be measured"), "{err}");
    }

    #[test]
    fn a_dark_pair_falls_back_to_light_for_slots_dark_does_not_override() {
        // `primary` is declared only in light; the dark pair must still resolve.
        let doc = DOC.replace(r#", theme = "light" "#, " ");
        let s = parse(&doc).unwrap();
        assert!(s.measure().iter().all(|m| m.ratio.is_some()));
    }

    #[test]
    fn generated_css_carries_the_four_roles_and_maps_them() {
        let css = sys().generate_css("design-system.md");
        for r in ["display", "body", "script", "mono"] {
            assert!(css.contains(&format!("--type-{r}:")), "missing --type-{r}");
            assert!(
                css.contains(&format!("--font-{r}: var(--type-{r})")),
                "missing --font-{r}"
            );
        }
    }

    #[test]
    fn no_font_var_is_defined_in_terms_of_itself() {
        // `--font-display: var(--font-display)` is a silent no-op, and every
        // heading quietly falls back to the browser default.
        for line in sys().generate_css("x").lines() {
            let t = line.trim();
            if let Some((k, v)) = t.split_once(':') {
                if k.trim().starts_with("--font-") && v.contains(&format!("var({}", k.trim())) {
                    panic!("self-referential: {t}");
                }
            }
        }
    }

    #[test]
    fn the_loader_variable_comes_before_the_family_name() {
        let css = sys().generate_css("x");
        assert!(
            css.contains(r#"--type-display: var(--font-cp), "Chakra Petch", sans-serif;"#),
            "{css}"
        );
        // A role with no `var` is just the family and its fallbacks.
        assert!(
            css.contains(r#"--type-body: "Source Serif 4", serif;"#),
            "{css}"
        );
    }

    #[test]
    fn chamfer_forces_the_radius_to_zero_but_keeps_the_scale() {
        let css = sys().generate_css("x");
        assert!(css.contains("--radius: 0rem;"));
        // A component copied in from any shadcn-shaped registry expects these.
        assert!(css.contains("--radius-lg: var(--radius);"));
    }

    #[test]
    fn round_keeps_a_real_radius() {
        let doc = DOC.replace(r#"strategy = "chamfer""#, r#"strategy = "round""#);
        let css = parse(&doc).unwrap().generate_css("x");
        assert!(!css.contains("--radius: 0rem;"), "{css}");
    }

    #[test]
    fn each_scale_step_becomes_one_class() {
        let css = sys().generate_css("x");
        assert!(css.contains(".type-display {"));
        assert!(css.contains(".type-eyebrow {"));
        assert!(css.contains("text-transform: uppercase;"));
        assert!(css.contains("@media (max-width: 40rem)"));
        // Expanded rather than single-line: the output is read by a formatter
        // before it is read by a person, and every one of them expands this.
        assert!(
            css.contains(".type-display {\n      font-size: 2.5rem;\n    }"),
            "{css}"
        );
    }

    #[test]
    fn generated_css_annotates_every_colour_with_its_hex() {
        let css = sys().generate_css("x");
        assert!(
            css.contains("--primary: oklch(0.56 0.15 48); /* #B85207 */"),
            "{css}"
        );
    }

    #[test]
    fn generated_css_records_what_was_measured() {
        let css = sys().generate_css("x");
        assert!(css.contains("Measured contrast"));
        assert!(css.contains("foreground on background"));
    }

    /// The declaration this capability ships, exactly as `fid add design`
    /// writes it into a product.
    const SHIPPED: &str = include_str!("../capabilities/design/declarations/design-system.md");

    #[test]
    fn the_shipped_default_parses() {
        // It is installed into every product that runs `fid add design`. A
        // template that does not parse is a product that cannot derive on the
        // day it is created.
        parse(SHIPPED).expect("the shipped design-system.md must parse");
    }

    #[test]
    fn the_shipped_default_passes_its_own_contrast_check() {
        // The embarrassing failure mode: ship a palette that fails the rule the
        // capability exists to enforce, so the first `fid derive` in a new
        // product errors out on our defaults rather than on the author's.
        let n = parse(SHIPPED)
            .unwrap()
            .check_contrast()
            .expect("the shipped palette must meet the pairs it declares");
        assert!(
            n >= 10,
            "only {n} pairs measured — light and dark, six pairs"
        );
    }

    #[test]
    fn the_shipped_default_declares_all_ten_scale_steps() {
        let s = parse(SHIPPED).unwrap();
        for step in [
            "display", "h1", "h2", "h3", "subhead", "body", "small", "eyebrow", "note", "mono",
        ] {
            assert!(
                s.steps.contains_key(step),
                "shipped scale is missing `{step}`"
            );
        }
    }

    #[test]
    fn the_shipped_defaults_are_not_on_the_slop_list() {
        // A default typeface nobody chose is the single most recognisable tell
        // in generated UI, and shipping one here would put it in every product.
        let s = parse(SHIPPED).unwrap();
        const SLOP: &[&str] = &[
            "inter",
            "geist",
            "roboto",
            "open sans",
            "poppins",
            "montserrat",
        ];
        for (role, r) in &s.roles {
            let fam = r.family.to_lowercase();
            assert!(
                !SLOP.contains(&fam.as_str()),
                "shipped `{role}` is {}",
                r.family
            );
        }
    }

    #[test]
    fn generated_css_names_the_file_it_came_from() {
        assert!(sys()
            .generate_css("design-system.md")
            .contains("design-system.md"));
    }
}
