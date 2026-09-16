//! Adapter contracts — the third concept in the capability taxonomy.
//!
//! Spec: `docs/specs/2026-09-14-capability-taxonomy.md` §3. This is
//! `MISSION.md` principle 6 — *commit to contracts, not to tools* — which until
//! now existed only as prose.
//!
//! A product names a vendor per contract:
//!
//! ```toml
//! [adapters]
//! database = "none"
//! storage  = "none"
//! errors   = "none"
//! ```
//!
//! # Why every contract ships with only `none` implemented
//!
//! The spec is blunt about the cost: *"a contract must be designed against the
//! narrowest plausible vendor or it leaks that vendor's model into every
//! consumer … a badly drawn contract is worse than no abstraction — it is
//! lock-in wearing a portability costume."*
//!
//! So this phase settles the **format** and nothing more. Every implementation
//! listed here is one that exists. The vendors each contract is *intended* to
//! carry are named in `candidates`, and selecting one fails with a message
//! saying so — deliberately, because this repository has just spent a phase
//! removing guard rules that were declared and never implemented, and an
//! adapter registry that lists `supabase` before anything speaks Supabase is
//! the identical bug with a different noun. A selectable vendor is a promise.
//!
//! The Cloudflare adapter set roadmap item moves vendors from `candidates` to
//! `implementations` as each gets something behind it. `d1` and `r2` are the
//! first two — see `docs/specs/2026-09-15-cloudflare-adapter-set.md` for what
//! shipped and what the rest of the Cloudflare set (Workers as `deploy`,
//! Access, Turnstile, Queues, Workers AI) still needs before it can move.
//!
//! # Why `none` is not a placeholder
//!
//! `errors = "none"` is a real, working no-op: diagnostics are wired in from
//! the first commit and cost nothing until pointed somewhere. A capability you
//! have to retrofit is one you will not retrofit.

/// A swappable slot behind a fixed, vendor-neutral contract.
pub struct Contract {
    /// Key in `fiducial.toml [adapters]`.
    pub name: &'static str,
    /// What the contract is for.
    pub description: &'static str,
    /// Vendors that can be selected today. Every one of these works.
    pub implementations: &'static [&'static str],
    /// Vendors this contract is meant to carry, which nothing implements yet.
    ///
    /// Listed so the intent is on the record and a reader can see where the
    /// contract is heading — and kept out of `implementations` so nothing can
    /// select one and believe it works.
    pub candidates: &'static [&'static str],
}

/// The no-op every contract ships with.
///
/// Not a placeholder: a contract selected as `none` is wired in, reported, and
/// does nothing. That is what makes it cost nothing to have from day one.
pub const NONE: &str = "none";

/// Every contract the platform defines.
pub static CONTRACTS: &[Contract] = &[
    Contract {
        name: "database",
        description: "Relational storage: queries, migrations, transactions",
        implementations: &[NONE, "d1"],
        candidates: &["supabase", "neon", "postgres"],
    },
    Contract {
        name: "storage",
        description: "Object storage: put, get, signed URLs",
        implementations: &[NONE, "r2"],
        candidates: &["s3", "supabase-storage"],
    },
    Contract {
        name: "deploy",
        description: "Where the product ships and how a release is promoted",
        implementations: &[NONE, "cloudflare"],
        candidates: &["vercel", "fly"],
    },
    Contract {
        name: "email",
        description: "Transactional email: send, template, verify a domain",
        implementations: &[NONE, "resend"],
        candidates: &["ses", "cloudflare-email"],
    },
    Contract {
        name: "newsletter",
        description: "Subscriber list management: subscribe, unsubscribe, status",
        implementations: &[NONE, "resend"],
        candidates: &[],
    },
    Contract {
        name: "errors",
        description: "Error tracking and diagnostics",
        implementations: &[NONE],
        candidates: &["sentry", "workers-analytics"],
    },
    Contract {
        name: "botProtection",
        description: "Bot / abuse challenge verification",
        implementations: &[NONE, "turnstile"],
        candidates: &["recaptcha", "hcaptcha"],
    },
    Contract {
        name: "queue",
        description: "Asynchronous job/message queue (producer side)",
        implementations: &[NONE, "cloudflare-queues"],
        candidates: &["sqs"],
    },
    Contract {
        name: "auth",
        description: "Users and authentication: sign-up, sign-in, sessions",
        implementations: &[NONE, "supabase"],
        candidates: &["clerk", "auth.js"],
    },
];

/// Look up a contract by name.
pub fn find(name: &str) -> Option<&'static Contract> {
    CONTRACTS.iter().find(|c| c.name == name)
}

impl Contract {
    /// Whether a vendor can be selected for this contract today.
    pub fn implements(&self, vendor: &str) -> bool {
        self.implementations.contains(&vendor)
    }

    /// Whether a vendor is a named future direction rather than a mistake.
    pub fn is_candidate(&self, vendor: &str) -> bool {
        self.candidates.contains(&vendor)
    }
}

/// What is wrong with one `[adapters]` entry, if anything.
///
/// Returned rather than printed so `fid doctor` and `fid dash` can report the
/// same finding in their own shapes without either of them restating the rule.
pub fn problem(contract: &str, vendor: &str) -> Option<String> {
    let Some(c) = find(contract) else {
        return Some(format!(
            "`{contract}` is not an adapter contract. Known contracts: {}",
            CONTRACTS
                .iter()
                .map(|c| c.name)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    };
    if c.implements(vendor) {
        return None;
    }
    if c.is_candidate(vendor) {
        return Some(format!(
            "`{contract} = \"{vendor}\"` names an intended vendor that nothing \
             implements yet. Selectable today: {}. Until then use \"{NONE}\", \
             which is wired in and does nothing.",
            c.implementations.join(", ")
        ));
    }
    Some(format!(
        "`{contract} = \"{vendor}\"` is not a known implementation. \
         Selectable today: {}; planned: {}.",
        c.implementations.join(", "),
        if c.candidates.is_empty() {
            "none".to_string()
        } else {
            c.candidates.join(", ")
        }
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every contract has a working selection.
    ///
    /// A contract whose `implementations` list is empty is one a product cannot
    /// legally satisfy — it would fail `fid doctor` whatever it chose.
    #[test]
    fn every_contract_can_be_satisfied() {
        for c in CONTRACTS {
            assert!(
                !c.implementations.is_empty(),
                "contract `{}` has no implementation, so nothing can select it",
                c.name
            );
        }
    }

    /// `none` is always available, so a product is never forced to adopt a
    /// vendor to have a valid config.
    #[test]
    fn none_satisfies_every_contract() {
        for c in CONTRACTS {
            assert!(c.implements(NONE), "contract `{}` has no no-op", c.name);
        }
    }

    /// A vendor is in exactly one list.
    ///
    /// The whole point of the split is that `implementations` is a promise and
    /// `candidates` is an intention. A name in both makes the promise
    /// unreadable.
    #[test]
    fn a_vendor_is_either_implemented_or_intended_never_both() {
        for c in CONTRACTS {
            for v in c.candidates {
                assert!(
                    !c.implements(v),
                    "`{v}` is listed as both implemented and planned for `{}`",
                    c.name
                );
            }
        }
    }

    #[test]
    fn an_unknown_contract_is_named_as_such() {
        let p = problem("blockchain", "none").expect("unknown contract must be a problem");
        assert!(p.contains("not an adapter contract"), "{p}");
    }

    /// The distinction that matters to whoever hits it: a planned vendor is a
    /// "not yet", a typo is a "no".
    #[test]
    fn a_planned_vendor_reads_differently_from_a_typo() {
        let planned = problem("database", "supabase").expect("not implemented yet");
        assert!(planned.contains("nothing implements yet"), "{planned}");

        let typo = problem("database", "supabse").expect("unknown");
        assert!(typo.contains("not a known implementation"), "{typo}");
    }

    #[test]
    fn a_working_selection_has_no_problem() {
        assert!(problem("errors", NONE).is_none());
    }

    #[test]
    fn email_resend_is_selectable() {
        assert!(problem("email", "resend").is_none());
    }

    #[test]
    fn newsletter_none_is_selectable() {
        assert!(problem("newsletter", NONE).is_none());
    }

    #[test]
    fn newsletter_resend_is_selectable() {
        assert!(problem("newsletter", "resend").is_none());
    }
}
