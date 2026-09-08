//! Core model primitives: `Fact`, `Decision`, and the `PipelineMeta` contract.
//!
//! All types are `no_std` and use only `'static` references to avoid heap
//! allocation in embedded contexts. The `alloc` feature enables richer owned
//! types later if a product needs them.
//!
//! These are the types that make "one declaration, many derivations" concrete:
//! a `Fact` is a named typed value declared once; a `Decision` is a judgment
//! with rationale and date; a `PipelineMeta` declares what a pipeline consumes
//! and produces without knowing how.

#![no_std]
#![deny(unsafe_code)]
#![warn(missing_docs)]

#[cfg(test)]
extern crate std;

// ── Fact ─────────────────────────────────────────────────────────────────────

/// A named declared value.
///
/// A `Fact` is the atom of the system: authored once, never generated,
/// never duplicated. All derived artifacts trace back to facts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fact<T> {
    /// The declared name — unique within a product.
    pub name: &'static str,
    /// The declared value.
    pub value: T,
}

impl<T> Fact<T> {
    /// Declare a fact.
    pub const fn new(name: &'static str, value: T) -> Self {
        Self { name, value }
    }
}

// ── Decision ─────────────────────────────────────────────────────────────────

/// A declared judgment with rationale and date.
///
/// Decisions are appended, never edited. A superseded decision gets a new
/// entry pointing back; the old one is never deleted. This is the complete
/// history.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Decision {
    /// ISO-8601 date, e.g. "2026-09-08".
    pub date: &'static str,
    /// One-line summary of the choice made.
    pub summary: &'static str,
    /// Why this choice, and what was rejected.
    pub rationale: &'static str,
    /// Key of the decision this supersedes, if any.
    pub supersedes: Option<&'static str>,
}

impl Decision {
    /// Declare a decision.
    pub const fn new(date: &'static str, summary: &'static str, rationale: &'static str) -> Self {
        Self {
            date,
            summary,
            rationale,
            supersedes: None,
        }
    }

    /// Declare a decision that supersedes an earlier one.
    pub const fn superseding(
        date: &'static str,
        summary: &'static str,
        rationale: &'static str,
        supersedes: &'static str,
    ) -> Self {
        Self {
            date,
            summary,
            rationale,
            supersedes: Some(supersedes),
        }
    }
}

// ── PipelineMeta ─────────────────────────────────────────────────────────────

/// The declared contract of a pipeline: what it takes and what it produces.
///
/// This is the §3.2 pipeline contract expressed as a value. The pipeline
/// implementation (executor, invocation) lives elsewhere; this is the
/// machine-readable declaration that `fid graph` and `fid derive --check` read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PipelineMeta {
    /// Unique pipeline name within a product.
    pub name: &'static str,
    /// Declared input paths (repo-relative, forward slashes).
    pub inputs: &'static [&'static str],
    /// Declared output paths (repo-relative, forward slashes).
    pub outputs: &'static [&'static str],
    /// The executor kind: "cargo-test", "shell", "python", …
    pub executor: &'static str,
}

impl PipelineMeta {
    /// Declare a pipeline.
    pub const fn new(
        name: &'static str,
        inputs: &'static [&'static str],
        outputs: &'static [&'static str],
        executor: &'static str,
    ) -> Self {
        Self {
            name,
            inputs,
            outputs,
            executor,
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fact_stores_name_and_value() {
        let f = Fact::new("battery_capacity_mah", 3000u32);
        assert_eq!(f.name, "battery_capacity_mah");
        assert_eq!(f.value, 3000);
    }

    #[test]
    fn decision_fields() {
        let d = Decision::new(
            "2026-09-08",
            "Chose Embassy over RTIC",
            "Sensor-array workload is cooperative; no firm deadlines requiring SRP locks.",
        );
        assert_eq!(d.date, "2026-09-08");
        assert!(d.supersedes.is_none());
    }

    #[test]
    fn decision_superseding() {
        let d = Decision::superseding(
            "2026-10-01",
            "Switched to RTIC for the 10 kHz IMU loop",
            "Loop period dropped below 100 µs — cooperative scheduling too marginal.",
            "chose-embassy-over-rtic",
        );
        assert_eq!(d.supersedes, Some("chose-embassy-over-rtic"));
    }

    #[test]
    fn pipeline_meta_fields() {
        let p = PipelineMeta::new(
            "types",
            &[
                "crates/fiducial-core/src/lib.rs",
                "crates/fiducial-wasm/src/lib.rs",
            ],
            &["packages/wasm-bridge/src/generated.ts"],
            "cargo-test",
        );
        assert_eq!(p.name, "types");
        assert_eq!(p.inputs.len(), 2);
        assert_eq!(p.outputs, &["packages/wasm-bridge/src/generated.ts"]);
    }
}
