//! `[apps.<name>]` — what each app of a product is built with, declared once.
//!
//! One product can legitimately be an Astro marketing site, a React Router
//! dashboard and a Hono API, so the choice is per **app**, not per product
//! (`docs/specs/2026-11-13-one-product-many-shapes.md`):
//!
//! ```toml
//! [apps.web]
//! framework = "astro"
//! editor    = "keystatic"
//!
//! [apps.api]
//! framework = "hono"
//! orm       = "drizzle"
//! ```
//!
//! **The choice is data, and so is the rule.** Each field has a table of what is
//! implemented and what is only intended, the way `[adapters]` does
//! (`adapter::CONTRACTS`): a name under *intended* is a known direction that
//! nothing builds yet, and says so, instead of generating something that half
//! works. `fid doctor` reports every problem here with its reason.
//!
//! **A default is a recommendation, never a behaviour.** Nothing here switches an
//! existing app, and a product that declares no `[apps]` is read exactly as it was
//! before this block existed.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::config::Config;

/// One app's choices. Every field is optional: an app declares only what it
/// chooses, and a missing field is the capability's own default.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct App {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub framework: String,
    /// The content editor for this app's files: `keystatic`, `tina`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub editor: String,
    /// Zero or more native shells that wrap the built app.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shell: Vec<String>,
    /// The backend this app talks to, when it is not its own: `hono`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub api: String,
    /// Typed queries over the `database` contract: `drizzle`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub orm: String,
}

/// What a field can be: `implemented` is selectable today, `intended` is a named
/// direction nothing builds yet.
pub struct Choices {
    pub field: &'static str,
    pub implemented: &'static [&'static str],
    pub intended: &'static [&'static str],
}

pub const FRAMEWORK: Choices = Choices {
    field: "framework",
    implemented: &[
        "next",
        "sveltekit",
        "astro",
        "hono",
        "react-router",
        "tanstack-start",
    ],
    intended: &[],
};
pub const EDITOR: Choices = Choices {
    field: "editor",
    implemented: &["keystatic", "tina", "pages"],
    intended: &[],
};
pub const SHELL: Choices = Choices {
    field: "shell",
    implemented: &["tauri"],
    intended: &["capacitor", "electron"],
};
pub const API: Choices = Choices {
    field: "api",
    implemented: &["hono"],
    intended: &[],
};
pub const ORM: Choices = Choices {
    field: "orm",
    implemented: &["drizzle"],
    intended: &[],
};

impl Choices {
    fn check(&self, app: &str, value: &str, out: &mut Vec<String>) {
        if value.is_empty() || self.implemented.contains(&value) {
            return;
        }
        let selectable = if self.implemented.is_empty() {
            "nothing yet".to_string()
        } else {
            self.implemented.join(", ")
        };
        if self.intended.contains(&value) {
            out.push(format!(
                "[apps.{app}] {} = \"{value}\" names an intended choice that nothing builds \
                 yet. Selectable today: {selectable}.",
                self.field
            ));
        } else {
            out.push(format!(
                "[apps.{app}] {} = \"{value}\" is not a known choice. Selectable today: \
                 {selectable}; planned: {}.",
                self.field,
                if self.intended.is_empty() {
                    "none".to_string()
                } else {
                    self.intended.join(", ")
                }
            ));
        }
    }
}

/// Frameworks that render pages. `hono` is a backend, not one of these.
fn renders_pages(framework: &str) -> bool {
    !framework.is_empty() && framework != "hono"
}

/// What is wrong with the declared apps, each problem with its reason. Empty when
/// nothing is declared.
pub fn problems(cfg: &Config, apps: &BTreeMap<String, App>) -> Vec<String> {
    let mut out = Vec::new();
    for (name, a) in apps {
        FRAMEWORK.check(name, &a.framework, &mut out);
        EDITOR.check(name, &a.editor, &mut out);
        for s in &a.shell {
            SHELL.check(name, s, &mut out);
        }
        API.check(name, &a.api, &mut out);
        ORM.check(name, &a.orm, &mut out);

        // An editor edits a site's files; a backend has none.
        if !a.editor.is_empty() && !renders_pages(&a.framework) && !a.framework.is_empty() {
            out.push(format!(
                "[apps.{name}] editor = \"{}\" on framework = \"{}\": an editor edits a \
                 site's content files, and a backend has none. Put the editor on the app \
                 that renders pages.",
                a.editor, a.framework
            ));
        }
        // A shell wraps a built web app.
        if !a.shell.is_empty() && a.framework == "hono" {
            out.push(format!(
                "[apps.{name}] shell = {:?} on framework = \"hono\": a shell wraps a built \
                 web app, and a backend is not one.",
                a.shell
            ));
        }
        // An app that has its own backend does not name another.
        if !a.api.is_empty() && a.framework == "hono" {
            out.push(format!(
                "[apps.{name}] is a Hono backend and also names api = \"{}\": an API names \
                 no API. Declare it on the app that calls it.",
                a.api
            ));
        }
        // An ORM is typed queries over a database; with none selected there is
        // nothing to query.
        if !a.orm.is_empty()
            && matches!(cfg.adapters.get("database"), None | Some("none") | Some(""))
        {
            out.push(format!(
                "[apps.{name}] orm = \"{}\" but `[adapters] database` is \"none\": an ORM is \
                 a layer over the `database` contract. Select a database first.",
                a.orm
            ));
        }
        // The deploy shape and the framework are one fact seen twice: say so
        // when they disagree.
        if name == "web" {
            let shape = cfg.deploy.shape.as_str();
            let want = match a.framework.as_str() {
                "next" => Some("nextjs"),
                "sveltekit" => Some("sveltekit"),
                "astro" => Some("astro"),
                "react-router" => Some("reactrouter"),
                "tanstack-start" => Some("tanstackstart"),
                _ => None,
            };
            if let Some(want) = want {
                if !shape.is_empty() && shape != want {
                    out.push(format!(
                        "[apps.web] framework = \"{}\" but [deploy] shape = \"{shape}\": the app \
                         deploys as \"{want}\". Set shape = \"{want}\", or change the framework.",
                        a.framework
                    ));
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(extra: &str) -> Config {
        toml::from_str(&format!(
            "[product]\nname = \"p\"\nversion = \"0.1.0\"\n{extra}"
        ))
        .unwrap()
    }

    fn check(extra: &str) -> Vec<String> {
        let c = cfg(extra);
        problems(&c, &c.apps)
    }

    #[test]
    fn nothing_declared_is_nothing_wrong() {
        assert!(check("").is_empty());
    }

    #[test]
    fn what_is_built_passes() {
        assert!(check(
            "[apps.web]\nframework = \"next\"\neditor = \"keystatic\"\nshell = [\"tauri\"]\n"
        )
        .is_empty());
        assert!(check("[apps.web]\nframework = \"sveltekit\"\neditor = \"tina\"\n").is_empty());
        assert!(check("[apps.web]\nframework = \"astro\"\neditor = \"pages\"\n").is_empty());
    }

    #[test]
    fn an_intended_choice_says_nothing_builds_it() {
        let p = check("[apps.web]\nframework = \"next\"\nshell = [\"electron\"]\n").join("\n");
        assert!(p.contains("intended choice that nothing builds yet"), "{p}");
        assert!(p.contains("Selectable today: tauri"), "{p}");
    }

    #[test]
    fn an_unknown_choice_lists_what_is_known() {
        let p = check("[apps.web]\nframework = \"rails\"\n").join("\n");
        assert!(p.contains("not a known choice"), "{p}");
        assert!(p.contains("planned: none"), "{p}");
    }

    #[test]
    fn a_combination_that_does_not_fit_fails_with_the_reason() {
        let table: [(&str, &str); 4] = [
            (
                "[apps.api]\nframework = \"hono\"\neditor = \"keystatic\"\n",
                "a backend has none",
            ),
            (
                "[apps.api]\nframework = \"hono\"\nshell = [\"tauri\"]\n",
                "a backend is not one",
            ),
            (
                "[apps.api]\nframework = \"hono\"\napi = \"hono\"\n",
                "an API names no API",
            ),
            (
                "[apps.web]\nframework = \"next\"\norm = \"drizzle\"\n",
                "layer over the `database` contract",
            ),
        ];
        for (toml, reason) in table {
            let p = check(toml).join("\n");
            assert!(p.contains(reason), "{toml}\n→ {p}");
        }
    }

    #[test]
    fn an_orm_over_a_selected_database_is_fine() {
        let p = check(
            "[adapters]\ndatabase = \"d1\"\n[apps.api]\nframework = \"hono\"\norm = \"drizzle\"\n",
        );
        assert!(p.is_empty(), "{p:?}");
    }

    #[test]
    fn the_framework_and_the_deploy_shape_must_agree() {
        let p =
            check("[deploy]\nshape = \"sveltekit\"\n[apps.web]\nframework = \"next\"\n").join("\n");
        assert!(p.contains("the app deploys as \"nextjs\""), "{p}");
        assert!(
            check("[deploy]\nshape = \"nextjs\"\n[apps.web]\nframework = \"next\"\n").is_empty()
        );
        assert!(check("[apps.web]\nframework = \"next\"\n").is_empty());
    }
}
