//! Conformance vectors for the authorization rule.
//!
//! `can()` exists twice: here in Rust (firmware, desktop, CLI) and in
//! TypeScript (`@fiducial/identity`, for the Worker and the app). Two
//! implementations of one rule is exactly the drift this platform exists to
//! prevent — and an authorization rule is the worst possible place for it,
//! because a divergence does not look like a bug, it looks like access.
//!
//! So the decision table is **generated from this crate**, committed to
//! `docs/identity/vectors.json`, and asserted by both suites. Same mechanism
//! `docs/protocol/vectors.json` uses for the wire format.
//!
//! Regenerate after an intentional rule change:
//!
//! ```sh
//! FIDUCIAL_WRITE_VECTORS=1 cargo test -p fiducial-identity --test vectors
//! ```
//!
//! CI runs it *without* the env var, so a rule change that skipped
//! regeneration fails the build rather than reaching main with the two
//! levels disagreeing.

use fiducial_identity::{
    can, effective_role, explain, Action, DeviceId, Grant, Principal, Reason, Resource, Role,
    ServiceId, Timestamp, UserId,
};

const VECTORS_PATH: &str = "../../docs/identity/vectors.json";

// Fixed identities, so the committed vectors are stable and readable.
const ALICE: [u8; 16] = [0x0a; 16];
const BOB: [u8; 16] = [0x0b; 16];
const DEV_A: [u8; 8] = [0xda; 8];
const DEV_B: [u8; 8] = [0xdb; 8];
const SVC: [u8; 8] = [0x5c; 8];

/// A fixed "current time" for the vectors: 2026-09-15T00:00:00Z in millis.
///
/// Fixed rather than `now()` — a vector file that changes every time it is
/// generated cannot be a freshness gate.
const NOW: Timestamp = Timestamp(1_789_516_800_000);
const EARLIER: Timestamp = Timestamp(1_789_516_800_000 - 86_400_000);
const LATER: Timestamp = Timestamp(1_789_516_800_000 + 86_400_000);

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn principal_json(p: &Principal) -> serde_json::Value {
    match p {
        Principal::Anonymous => serde_json::json!({ "kind": "anonymous" }),
        Principal::User(u) => serde_json::json!({ "kind": "user", "id": hex(u.as_bytes()) }),
        Principal::Device(d) => serde_json::json!({ "kind": "device", "id": hex(d.as_bytes()) }),
        Principal::Service(s) => serde_json::json!({ "kind": "service", "id": hex(s.as_bytes()) }),
    }
}

fn resource_json(r: &Resource) -> serde_json::Value {
    match r {
        Resource::Device(d) => serde_json::json!({ "kind": "device", "id": hex(d.as_bytes()) }),
        Resource::User(u) => serde_json::json!({ "kind": "user", "id": hex(u.as_bytes()) }),
        Resource::Platform => serde_json::json!({ "kind": "platform" }),
    }
}

fn action_name(a: Action) -> &'static str {
    match a {
        Action::Read => "read",
        Action::Write => "write",
        Action::Admin => "admin",
    }
}

fn reason_name(r: Reason) -> &'static str {
    match r {
        Reason::Granted => "granted",
        Reason::DeviceReadingItself => "device_reading_itself",
        Reason::NotIdentified => "not_identified",
        Reason::NoGrant => "no_grant",
        Reason::RoleTooWeak => "role_too_weak",
        Reason::Expired => "expired",
        Reason::NoClock => "no_clock",
        Reason::DelegatorLacksAuthority => "delegator_lacks_authority",
        Reason::DelegationTooDeep => "delegation_too_deep",
    }
}

fn role_name(r: Role) -> &'static str {
    match r {
        Role::Viewer => "viewer",
        Role::Member => "member",
        Role::Admin => "admin",
        Role::Owner => "owner",
    }
}

/// One named scenario: a grant table, and every (principal, action, resource)
/// triple evaluated against it.
struct Scenario {
    name: &'static str,
    why: &'static str,
    grants: Vec<Grant>,
    probes: Vec<(Principal, Action, Resource)>,
    /// The clock the scenario is evaluated at. `None` models a caller with no
    /// clock — a device whose RTC has not synced — which is the case expiry
    /// has to fail closed for.
    now: Option<Timestamp>,
}

fn scenarios() -> Vec<Scenario> {
    let alice = Principal::User(UserId::new(ALICE));
    let bob = Principal::User(UserId::new(BOB));
    let dev_a = Principal::Device(DeviceId::new(DEV_A));
    let dev_b = Principal::Device(DeviceId::new(DEV_B));
    let svc = Principal::Service(ServiceId::new(SVC));
    let r_dev_a = Resource::Device(DeviceId::new(DEV_A));
    let r_dev_b = Resource::Device(DeviceId::new(DEV_B));

    let every_action = [Action::Read, Action::Write, Action::Admin];

    // Probe every principal against both devices, at every action — the
    // cross product is small enough to enumerate, and enumerating it is what
    // makes a divergence between the two implementations impossible to miss.
    let all_probes = |resources: &[Resource]| -> Vec<(Principal, Action, Resource)> {
        let mut out = Vec::new();
        for p in [alice, bob, dev_a, dev_b, svc, Principal::Anonymous] {
            for r in resources {
                for a in every_action {
                    out.push((p, a, *r));
                }
            }
        }
        out
    };

    vec![
        Scenario {
            name: "no_grants",
            why: "deny is the default; a device may still read itself",
            grants: vec![],
            probes: all_probes(&[r_dev_a, r_dev_b]),
            now: Some(NOW),
        },
        Scenario {
            name: "alice_owns_device_a",
            why: "the user↔device link: ownership covers one device, not the fleet",
            grants: vec![Grant::new(alice, r_dev_a, Role::Owner)],
            probes: all_probes(&[r_dev_a, r_dev_b]),
            now: Some(NOW),
        },
        Scenario {
            name: "bob_views_device_a",
            why: "a shared device: read yes, write no",
            grants: vec![
                Grant::new(alice, r_dev_a, Role::Owner),
                Grant::new(bob, r_dev_a, Role::Viewer),
            ],
            probes: all_probes(&[r_dev_a]),
            now: Some(NOW),
        },
        Scenario {
            name: "member_may_command",
            why: "a household member sends commands but cannot re-grant",
            grants: vec![Grant::new(bob, r_dev_a, Role::Member)],
            probes: all_probes(&[r_dev_a]),
            now: Some(NOW),
        },
        Scenario {
            name: "service_admin_platform",
            why: "a backend service holds a platform-wide grant",
            grants: vec![Grant::new(svc, Resource::Platform, Role::Admin)],
            probes: all_probes(&[r_dev_a, r_dev_b]),
            now: Some(NOW),
        },
        Scenario {
            name: "zero_sentinel_is_not_an_identity",
            why: "an unprovisioned device must not authenticate as device zero",
            grants: vec![Grant::new(
                Principal::Device(DeviceId::ZERO),
                Resource::Platform,
                Role::Owner,
            )],
            probes: vec![
                (
                    Principal::Device(DeviceId::ZERO),
                    Action::Read,
                    Resource::Device(DeviceId::ZERO),
                ),
                (Principal::Device(DeviceId::ZERO), Action::Read, r_dev_a),
                (
                    Principal::User(UserId::ZERO),
                    Action::Read,
                    Resource::User(UserId::ZERO),
                ),
                (
                    Principal::Service(ServiceId::ZERO),
                    Action::Admin,
                    Resource::Platform,
                ),
            ],
            now: Some(NOW),
        },
        Scenario {
            name: "device_reports_to_its_owner",
            why: "the whole loop: device reads itself, owner reads and commands it",
            grants: vec![Grant::new(alice, r_dev_a, Role::Owner)],
            probes: vec![
                (dev_a, Action::Read, r_dev_a),
                (dev_a, Action::Write, r_dev_a),
                (dev_a, Action::Read, r_dev_b),
                (alice, Action::Read, r_dev_a),
                (alice, Action::Write, r_dev_a),
                (alice, Action::Admin, r_dev_a),
            ],
            now: Some(NOW),
        },
        Scenario {
            name: "expired_grant_is_not_a_grant",
            why: "a support engineer's access lapses without anyone remembering to revoke it",
            grants: vec![
                Grant::new(alice, r_dev_a, Role::Owner),
                Grant::new(bob, r_dev_a, Role::Admin).expiring_at(EARLIER),
            ],
            probes: all_probes(&[r_dev_a]),
            now: Some(NOW),
        },
        Scenario {
            name: "unexpired_grant_still_works",
            why: "the same grant, before its expiry — the other half of the pair",
            grants: vec![
                Grant::new(alice, r_dev_a, Role::Owner),
                Grant::new(bob, r_dev_a, Role::Admin).expiring_at(LATER),
            ],
            probes: all_probes(&[r_dev_a]),
            now: Some(NOW),
        },
        Scenario {
            name: "no_clock_refuses_an_expiring_grant",
            why: "a device with no synced RTC cannot honour an expiry, so it must not \
                  honour the grant; the permanent grant beside it is unaffected",
            grants: vec![
                Grant::new(alice, r_dev_a, Role::Owner),
                Grant::new(bob, r_dev_a, Role::Admin).expiring_at(LATER),
            ],
            probes: all_probes(&[r_dev_a]),
            now: None,
        },
        Scenario {
            name: "delegated_grant_follows_its_delegator",
            why: "alice owns the device and delegates admin to bob; bob's grant is worth \
                  exactly what alice's authority is worth",
            grants: vec![
                Grant::new(alice, r_dev_a, Role::Owner),
                Grant::new(bob, r_dev_a, Role::Admin).delegated_by(alice),
            ],
            probes: all_probes(&[r_dev_a]),
            now: Some(NOW),
        },
        Scenario {
            name: "revoking_the_delegator_revokes_the_delegation",
            why: "alice's own grant is gone, so bob's delegated grant stops working \
                  without anyone having to find it",
            grants: vec![Grant::new(bob, r_dev_a, Role::Admin).delegated_by(alice)],
            probes: all_probes(&[r_dev_a]),
            now: Some(NOW),
        },
        Scenario {
            name: "a_member_cannot_delegate_what_they_lack",
            why: "delegation needs Admin — the action defined as changing who else may \
                  act — so a Member cannot mint grants they could not use",
            grants: vec![
                Grant::new(alice, r_dev_a, Role::Member),
                Grant::new(bob, r_dev_a, Role::Admin).delegated_by(alice),
            ],
            probes: all_probes(&[r_dev_a]),
            now: Some(NOW),
        },
        Scenario {
            name: "a_delegation_cycle_terminates",
            why: "alice delegated by bob, bob delegated by alice — a table anyone with \
                  Admin can write, and an unbounded walk there is a crash in the \
                  authorization path",
            grants: vec![
                Grant::new(alice, r_dev_a, Role::Admin).delegated_by(bob),
                Grant::new(bob, r_dev_a, Role::Admin).delegated_by(alice),
            ],
            probes: all_probes(&[r_dev_a]),
            now: Some(NOW),
        },
        Scenario {
            name: "an_expired_delegator_cannot_sustain_a_delegation",
            why: "expiry and delegation compose: the chain is only live while every \
                  link in it is",
            grants: vec![
                Grant::new(alice, r_dev_a, Role::Owner).expiring_at(EARLIER),
                Grant::new(bob, r_dev_a, Role::Admin).delegated_by(alice),
            ],
            probes: all_probes(&[r_dev_a]),
            now: Some(NOW),
        },
    ]
}

fn render() -> String {
    let cases: Vec<serde_json::Value> = scenarios()
        .iter()
        .map(|s| {
            let grants: Vec<serde_json::Value> = s
                .grants
                .iter()
                .map(|g| {
                    serde_json::json!({
                        "principal": principal_json(&g.principal),
                        "resource": resource_json(&g.resource),
                        "role": role_name(g.role),
                        "expires_at": g.expires_at.map(|t| t.0),
                        "delegated_by": g.delegated_by.as_ref().map(principal_json),
                    })
                })
                .collect();

            let probes: Vec<serde_json::Value> = s
                .probes
                .iter()
                .map(|(p, a, r)| {
                    serde_json::json!({
                        "principal": principal_json(p),
                        "action": action_name(*a),
                        "resource": resource_json(r),
                        "allowed": can(p, *a, r, &s.grants, s.now),
                        "effective_role": effective_role(p, r, &s.grants, s.now).map(role_name),
                        "reason": reason_name(explain(p, *a, r, &s.grants, s.now).reason),
                    })
                })
                .collect();

            serde_json::json!({
                "name": s.name,
                "why": s.why,
                "now": s.now.map(|t| t.0),
                "grants": grants,
                "probes": probes,
            })
        })
        .collect();

    let doc = serde_json::json!({
        "note": "Generated from crates/fiducial-identity. Do not edit by hand — \
                 run FIDUCIAL_WRITE_VECTORS=1 cargo test -p fiducial-identity --test vectors",
        "roles": ["viewer", "member", "admin", "owner"],
        "actions": ["read", "write", "admin"],
        "rules": [
            "deny by default: an empty grant table permits nothing",
            "an unidentified principal is refused (anonymous, or an all-zero sentinel id)",
            "a device may read itself with no grant; it may not write itself",
            "a Platform-scoped grant covers every resource",
            "a role permits every action at or below its own level",
        ],
        "scenarios": cases,
    });

    let mut s = serde_json::to_string_pretty(&doc).expect("serializing vectors");
    s.push('\n');
    s
}

#[test]
fn vectors_are_fresh() {
    let rendered = render();

    if std::env::var("FIDUCIAL_WRITE_VECTORS").is_ok() {
        let path = std::path::Path::new(VECTORS_PATH);
        std::fs::create_dir_all(path.parent().expect("vectors dir")).expect("creating dir");
        std::fs::write(path, &rendered).expect("writing vectors");
        return;
    }

    let committed = std::fs::read_to_string(VECTORS_PATH).unwrap_or_else(|e| {
        panic!(
            "reading {VECTORS_PATH}: {e}\n\nGenerate it with:\n  \
             FIDUCIAL_WRITE_VECTORS=1 cargo test -p fiducial-identity --test vectors"
        )
    });

    assert_eq!(
        committed, rendered,
        "\ndocs/identity/vectors.json is stale — the authorization rule changed \
         without regenerating it. If the change was intentional, run:\n  \
         FIDUCIAL_WRITE_VECTORS=1 cargo test -p fiducial-identity --test vectors\n\
         and check what moved: every diff here is a change in who may do what.\n"
    );
}

/// The vectors must actually discriminate.
///
/// A table of all-`true` or all-`false` verdicts would pass in both languages
/// while proving nothing — the failure mode `docs/protocol/vectors.json` was
/// verified against by flipping a CRC polynomial bit and watching 31 tests
/// fail. Here the equivalent check is structural: the table has to contain
/// both verdicts, at every action.
#[test]
fn vectors_contain_both_verdicts_for_every_action() {
    for action in ["read", "write", "admin"] {
        let mut allowed = false;
        let mut denied = false;
        for s in scenarios() {
            for (p, a, r) in &s.probes {
                if action_name(*a) != action {
                    continue;
                }
                if can(p, *a, r, &s.grants, s.now) {
                    allowed = true;
                } else {
                    denied = true;
                }
            }
        }
        assert!(
            allowed && denied,
            "`{action}` must appear both allowed and denied in the vectors, \
             or the table proves nothing about it"
        );
    }
}
