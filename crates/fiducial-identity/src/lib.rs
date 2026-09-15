//! Identity, at every level of the platform.
//!
//! Before this crate, the platform had three unrelated notions of "who":
//! [`fiducial_core::DeviceId`] at L0 (firmware, protocol, OTA), a Supabase
//! user UUID inside the `auth` adapter, and nothing at all for a service
//! calling another service. Each was correct on its own and none of them
//! could be compared to another, so the one question a real product actually
//! asks — *may this actor do this to this thing?* — had no place to live and
//! would have been answered separately, and differently, at each level.
//!
//! [`Principal`] is that missing type. It spans all three, reuses `DeviceId`
//! rather than minting a second device identity (principle 1 — a fact is
//! declared once), and is `no_std` so the firmware evaluating "is this
//! command from my owner?" offline runs *the same rule* as the Worker
//! answering the same question at the edge.
//!
//! ```
//! use fiducial_identity::{Action, Grant, Principal, Resource, Role, UserId, can};
//! use fiducial_core::DeviceId;
//!
//! let owner = Principal::User(UserId::new([7; 16]));
//! let thermostat = Resource::Device(DeviceId::new([1, 2, 3, 4, 5, 6, 7, 8]));
//! let grants = [Grant::new(owner, thermostat, Role::Owner)];
//!
//! assert!(can(&owner, Action::Write, &thermostat, &grants));
//!
//! let stranger = Principal::User(UserId::new([9; 16]));
//! assert!(!can(&stranger, Action::Read, &thermostat, &grants));
//! ```
//!
//! # The decision table is a conformance artifact
//!
//! [`can`] is mirrored in TypeScript (`@fiducial/identity`) for the Worker
//! and the app. Two implementations of one rule is exactly the drift this
//! platform exists to prevent, so the rule's whole decision table is
//! generated from this crate into `docs/identity/vectors.json` and asserted
//! by **both** suites — the same mechanism `docs/protocol/vectors.json`
//! already uses for the wire format. A divergence fails a build rather than
//! quietly granting someone access on one level that the other denies.

#![no_std]
#![forbid(unsafe_code)]

pub use fiducial_core::DeviceId;

// ── Identifiers ──────────────────────────────────────────────────────────────

/// A 16-byte user identifier.
///
/// Sized for a UUID because that is what every candidate auth vendor issues
/// (Supabase, Clerk and Auth.js all hand out UUID-shaped subjects), and
/// stored as raw bytes rather than a string so this crate stays `no_std`
/// with no allocator: a device holding its owner's id in flash needs 16
/// bytes, not a heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct UserId([u8; 16]);

impl UserId {
    /// The uninitialized sentinel — all bytes zero, treated as absent.
    pub const ZERO: UserId = UserId([0u8; 16]);

    #[inline]
    pub const fn new(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    #[inline]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }

    /// `true` when all bytes are zero — the uninitialized sentinel.
    ///
    /// Mirrors [`DeviceId::is_zero`] deliberately: every id in this platform
    /// answers "am I real?" the same way, so no consumer has to remember
    /// which sentinel convention applies at which level.
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.0.iter().all(|&b| b == 0)
    }

    /// Parse the canonical hyphenated UUID form (`8-4-4-4-12` hex).
    ///
    /// The one place a string becomes a `UserId`: an auth vendor hands back
    /// a UUID string, and it is converted here, once, at the boundary.
    pub fn parse_uuid(s: &str) -> Result<Self, IdentityError> {
        let bytes = s.as_bytes();
        let mut out = [0u8; 16];
        let mut nibble = 0usize;
        let mut hi: Option<u8> = None;

        for &b in bytes {
            if b == b'-' {
                continue;
            }
            let v = match b {
                b'0'..=b'9' => b - b'0',
                b'a'..=b'f' => b - b'a' + 10,
                b'A'..=b'F' => b - b'A' + 10,
                _ => return Err(IdentityError::MalformedUuid),
            };
            match hi {
                None => hi = Some(v),
                Some(h) => {
                    if nibble >= 16 {
                        return Err(IdentityError::MalformedUuid);
                    }
                    out[nibble] = (h << 4) | v;
                    nibble += 1;
                    hi = None;
                }
            }
        }

        if nibble != 16 || hi.is_some() {
            return Err(IdentityError::MalformedUuid);
        }
        Ok(Self(out))
    }
}

/// An 8-byte service identifier — a Worker, a job runner, a backend calling
/// another backend.
///
/// Deliberately the same width as [`DeviceId`]: a service and a device are
/// both non-human actors, and a product that later promotes a "service" to
/// run on hardware should not have to change its identity's shape to do it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct ServiceId([u8; 8]);

impl ServiceId {
    pub const ZERO: ServiceId = ServiceId([0u8; 8]);

    #[inline]
    pub const fn new(bytes: [u8; 8]) -> Self {
        Self(bytes)
    }

    #[inline]
    pub const fn as_bytes(&self) -> &[u8; 8] {
        &self.0
    }

    #[inline]
    pub fn is_zero(&self) -> bool {
        self.0.iter().all(|&b| b == 0)
    }
}

/// What went wrong converting something into an identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityError {
    /// Not a canonical hyphenated UUID.
    MalformedUuid,
}

impl core::fmt::Display for IdentityError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            IdentityError::MalformedUuid => f.write_str("malformed UUID"),
        }
    }
}

impl core::error::Error for IdentityError {}

// ── Principal ────────────────────────────────────────────────────────────────

/// Who is acting — at any level of the platform.
///
/// One type for a human, a device and a service, because the authorization
/// question is the same shape for all three and answering it three times is
/// how the three answers drift apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Principal {
    /// Nobody has authenticated. Never equal to any other principal,
    /// including another `Anonymous` — see [`Principal::is_identified`].
    Anonymous,
    /// A human, from the `auth` adapter contract.
    User(UserId),
    /// A device, from `fiducial_core::DeviceId` — the same id firmware,
    /// `fiducial-protocol` and `fiducial-ota` already speak.
    Device(DeviceId),
    /// A machine actor: a Worker, a scheduled job, a backend service.
    Service(ServiceId),
}

impl Principal {
    /// `true` when this principal is a real, initialized identity.
    ///
    /// An all-zero id is the uninitialized sentinel at every level, and a
    /// device that has not yet read its UID out of OTP would otherwise
    /// authenticate as "device zero" — which every such device would share.
    /// [`can`] refuses an unidentified principal outright.
    pub fn is_identified(&self) -> bool {
        match self {
            Principal::Anonymous => false,
            Principal::User(u) => !u.is_zero(),
            Principal::Device(d) => !d.is_zero(),
            Principal::Service(s) => !s.is_zero(),
        }
    }
}

/// The principal for a signed-in user, from an auth vendor's UUID subject.
///
/// This is the whole bridge from the `auth` adapter contract to identity:
/// `AuthSession.user.id` goes in, a [`Principal`] comes out, and from there
/// the same [`can`] the firmware runs decides what they may do. Taking the
/// id rather than the session type is deliberate — it keeps this crate
/// `no_std` and dependency-free, and works for any vendor's session shape.
pub fn user_principal(uuid: &str) -> Result<Principal, IdentityError> {
    Ok(Principal::User(UserId::parse_uuid(uuid)?))
}

// ── Resources, roles, actions ────────────────────────────────────────────────

/// What is being acted upon.
///
/// A device is a resource *and* a principal — it acts (reporting telemetry)
/// and is acted upon (a user renaming it). That duality is why `Resource` is
/// its own enum rather than a reuse of `Principal`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Resource {
    /// A specific device.
    Device(DeviceId),
    /// A specific user's own account and data.
    User(UserId),
    /// Everything — the scope a platform-level admin grant covers.
    Platform,
}

/// What a principal is trying to do.
///
/// Ordered by increasing power, and [`Role`] grants are evaluated against
/// that order, so adding an action between two existing ones does not
/// require revisiting every role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Action {
    /// Read state: telemetry, settings, profile.
    Read = 0,
    /// Change state: write settings, send a command, rename.
    Write = 1,
    /// Change who else may act: grant and revoke, transfer ownership,
    /// authorize a firmware rollout.
    Admin = 2,
}

/// How much a principal may do with a resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Role {
    /// May read only.
    Viewer = 0,
    /// May read and write.
    Member = 1,
    /// May read, write, and administer.
    Admin = 2,
    /// Admin, plus the thing itself belongs to them.
    ///
    /// Distinct from `Admin` because transfer and deletion are an owner's to
    /// make, and an installer or support engineer holding `Admin` should not
    /// inherit them by accident.
    Owner = 3,
}

impl Role {
    /// The most powerful action this role permits.
    #[inline]
    pub fn permits(&self) -> Action {
        match self {
            Role::Viewer => Action::Read,
            Role::Member => Action::Write,
            Role::Admin | Role::Owner => Action::Admin,
        }
    }

    /// `true` when this role permits `action`.
    #[inline]
    pub fn allows(&self, action: Action) -> bool {
        action <= self.permits()
    }
}

// ── Grants ───────────────────────────────────────────────────────────────────

/// One principal's role over one resource — the user↔device link that had
/// nowhere to live before this crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Grant {
    pub principal: Principal,
    pub resource: Resource,
    pub role: Role,
}

impl Grant {
    #[inline]
    pub const fn new(principal: Principal, resource: Resource, role: Role) -> Self {
        Self {
            principal,
            resource,
            role,
        }
    }

    /// `true` when this grant speaks to `principal` acting on `resource`.
    ///
    /// A `Platform` grant covers every resource; a resource-specific grant
    /// covers only itself. There is deliberately no wildcard *principal*:
    /// "everyone may" is a policy decision, not an identity one, and putting
    /// it here would make every grant table a potential blanket grant.
    #[inline]
    fn covers(&self, principal: &Principal, resource: &Resource) -> bool {
        self.principal == *principal
            && (self.resource == *resource || self.resource == Resource::Platform)
    }
}

// ── The decision ─────────────────────────────────────────────────────────────

/// May `principal` perform `action` on `resource`, given `grants`?
///
/// The whole authorization rule of the platform, in one `no_std` function so
/// firmware, a Worker, a desktop app and the TypeScript mirror all reach the
/// same verdict. Deny is the default: an empty grant table permits nothing.
///
/// Two rules hold before any grant is consulted:
///
/// 1. **An unidentified principal is refused** — anonymous, or an all-zero
///    sentinel id (see [`Principal::is_identified`]).
/// 2. **A device may always read itself**, with no grant. A device reporting
///    its own telemetry is not an authorization question, and requiring a
///    grant for it would mean every device needs a provisioning round trip
///    before it can say anything at all — including the "I am here, I am
///    unclaimed" message that provisioning itself depends on.
pub fn can(principal: &Principal, action: Action, resource: &Resource, grants: &[Grant]) -> bool {
    if !principal.is_identified() {
        return false;
    }

    if let (Principal::Device(actor), Resource::Device(target)) = (principal, resource) {
        if actor == target && action == Action::Read {
            return true;
        }
    }

    grants
        .iter()
        .any(|g| g.covers(principal, resource) && g.role.allows(action))
}

/// The strongest role `principal` holds over `resource`, if any.
///
/// Separate from [`can`] because a UI answers a different question than a
/// gate does: "what may I show them?" needs the role, not one verdict.
pub fn effective_role(
    principal: &Principal,
    resource: &Resource,
    grants: &[Grant],
) -> Option<Role> {
    if !principal.is_identified() {
        return None;
    }
    grants
        .iter()
        .filter(|g| g.covers(principal, resource))
        .map(|g| g.role)
        .max()
}

#[cfg(test)]
mod tests {
    use super::*;

    const USER: UserId = UserId::new([7; 16]);
    const OTHER: UserId = UserId::new([9; 16]);
    const DEV: DeviceId = DeviceId::new([1, 2, 3, 4, 5, 6, 7, 8]);
    const DEV2: DeviceId = DeviceId::new([8, 7, 6, 5, 4, 3, 2, 1]);

    fn owner_grant() -> Grant {
        Grant::new(Principal::User(USER), Resource::Device(DEV), Role::Owner)
    }

    #[test]
    fn deny_is_the_default() {
        assert!(!can(
            &Principal::User(USER),
            Action::Read,
            &Resource::Device(DEV),
            &[]
        ));
    }

    #[test]
    fn an_owner_may_administer_their_device() {
        let g = [owner_grant()];
        assert!(can(
            &Principal::User(USER),
            Action::Admin,
            &Resource::Device(DEV),
            &g
        ));
    }

    #[test]
    fn a_stranger_may_not_read_someone_elses_device() {
        let g = [owner_grant()];
        assert!(!can(
            &Principal::User(OTHER),
            Action::Read,
            &Resource::Device(DEV),
            &g
        ));
    }

    #[test]
    fn a_grant_does_not_leak_to_another_device() {
        let g = [owner_grant()];
        assert!(!can(
            &Principal::User(USER),
            Action::Read,
            &Resource::Device(DEV2),
            &g
        ));
    }

    #[test]
    fn a_viewer_may_read_but_not_write() {
        let g = [Grant::new(
            Principal::User(USER),
            Resource::Device(DEV),
            Role::Viewer,
        )];
        assert!(can(
            &Principal::User(USER),
            Action::Read,
            &Resource::Device(DEV),
            &g
        ));
        assert!(!can(
            &Principal::User(USER),
            Action::Write,
            &Resource::Device(DEV),
            &g
        ));
    }

    #[test]
    fn a_member_may_write_but_not_administer() {
        let g = [Grant::new(
            Principal::User(USER),
            Resource::Device(DEV),
            Role::Member,
        )];
        assert!(can(
            &Principal::User(USER),
            Action::Write,
            &Resource::Device(DEV),
            &g
        ));
        assert!(!can(
            &Principal::User(USER),
            Action::Admin,
            &Resource::Device(DEV),
            &g
        ));
    }

    #[test]
    fn anonymous_is_refused_even_with_a_matching_grant() {
        let g = [Grant::new(
            Principal::Anonymous,
            Resource::Device(DEV),
            Role::Owner,
        )];
        assert!(!can(
            &Principal::Anonymous,
            Action::Read,
            &Resource::Device(DEV),
            &g
        ));
    }

    /// The uninitialized-device case that makes this rule matter: a device
    /// that has not read its UID yet would otherwise authenticate as
    /// "device zero" — an identity every unprovisioned device shares.
    #[test]
    fn a_zero_sentinel_id_is_not_an_identity() {
        let g = [Grant::new(
            Principal::Device(DeviceId::ZERO),
            Resource::Platform,
            Role::Owner,
        )];
        assert!(!can(
            &Principal::Device(DeviceId::ZERO),
            Action::Read,
            &Resource::Device(DEV),
            &g
        ));
        assert!(!Principal::User(UserId::ZERO).is_identified());
        assert!(!Principal::Service(ServiceId::ZERO).is_identified());
    }

    #[test]
    fn a_device_may_read_itself_without_a_grant() {
        assert!(can(
            &Principal::Device(DEV),
            Action::Read,
            &Resource::Device(DEV),
            &[]
        ));
    }

    /// Self-read is a read, not a blanket self-grant: a device must still be
    /// authorized to *change* itself, or a compromised device could rewrite
    /// its own configuration and call it self-service.
    #[test]
    fn a_device_may_not_write_itself_without_a_grant() {
        assert!(!can(
            &Principal::Device(DEV),
            Action::Write,
            &Resource::Device(DEV),
            &[]
        ));
    }

    #[test]
    fn a_device_may_not_read_another_device_without_a_grant() {
        assert!(!can(
            &Principal::Device(DEV),
            Action::Read,
            &Resource::Device(DEV2),
            &[]
        ));
    }

    #[test]
    fn a_platform_grant_covers_every_resource() {
        let g = [Grant::new(
            Principal::Service(ServiceId::new([1; 8])),
            Resource::Platform,
            Role::Admin,
        )];
        assert!(can(
            &Principal::Service(ServiceId::new([1; 8])),
            Action::Admin,
            &Resource::Device(DEV),
            &g
        ));
    }

    #[test]
    fn effective_role_returns_the_strongest_grant() {
        let g = [
            Grant::new(Principal::User(USER), Resource::Device(DEV), Role::Viewer),
            Grant::new(Principal::User(USER), Resource::Platform, Role::Admin),
        ];
        assert_eq!(
            effective_role(&Principal::User(USER), &Resource::Device(DEV), &g),
            Some(Role::Admin)
        );
    }

    #[test]
    fn effective_role_is_none_without_a_grant() {
        assert_eq!(
            effective_role(&Principal::User(USER), &Resource::Device(DEV), &[]),
            None
        );
    }

    #[test]
    fn owner_outranks_admin() {
        assert!(Role::Owner > Role::Admin);
        assert!(Role::Admin.allows(Action::Admin));
        assert!(Role::Owner.allows(Action::Admin));
    }

    #[test]
    fn uuid_parses_in_canonical_form() {
        let id = UserId::parse_uuid("11111111-2222-3333-4444-555555555555").unwrap();
        assert_eq!(
            id.as_bytes(),
            &[
                0x11, 0x11, 0x11, 0x11, 0x22, 0x22, 0x33, 0x33, 0x44, 0x44, 0x55, 0x55, 0x55, 0x55,
                0x55, 0x55
            ]
        );
    }

    #[test]
    fn uuid_parsing_rejects_junk() {
        assert_eq!(
            UserId::parse_uuid("nope"),
            Err(IdentityError::MalformedUuid)
        );
        assert_eq!(UserId::parse_uuid(""), Err(IdentityError::MalformedUuid));
        // One nibble short.
        assert_eq!(
            UserId::parse_uuid("11111111-2222-3333-4444-55555555555"),
            Err(IdentityError::MalformedUuid)
        );
        // One byte too many.
        assert_eq!(
            UserId::parse_uuid("11111111-2222-3333-4444-5555555555555555"),
            Err(IdentityError::MalformedUuid)
        );
    }
}

/// The crate README, compiled as a doctest.
///
/// A README is documentation, and documentation that is not executed drifts
/// from the code it describes. `#[cfg(doctest)]` means this item exists only
/// while doctests are collected, so the README is verified on every
/// `cargo test` without being rendered a second time on docs.rs.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
pub struct ReadmeDoctests;
