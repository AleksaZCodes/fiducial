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
//! assert!(can(&owner, Action::Write, &thermostat, &grants, None));
//!
//! let stranger = Principal::User(UserId::new([9; 16]));
//! assert!(!can(&stranger, Action::Read, &thermostat, &grants, None));
//! ```
//!
//! The last argument is the current time, and `None` means *this caller has no
//! clock*. A permanent grant is unaffected by that; a grant with an expiry is
//! **refused**, because a caller that cannot tell the time cannot honour one.
//! A device whose RTC has not synced is exactly such a caller, and treating
//! "no clock" as "not expired" would make expiry evaporate in the one
//! environment least able to notice.
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

/// Milliseconds since the Unix epoch.
///
/// A plain `u64` rather than a date type: this crate is `no_std` and must
/// build for a microcontroller, where there is no calendar and no allocator.
/// Milliseconds because that is what `Date.now()` gives the TypeScript mirror,
/// and a unit conversion at the boundary is a place for the two to disagree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Timestamp(pub u64);

/// How many delegation links deep a grant chain may go.
///
/// Bounded because the chain is walked recursively and a cycle — A delegated
/// by B, B delegated by A — is a grant table anyone with `Admin` can write.
/// An unbounded walk there is a stack overflow in the authorization path,
/// which on firmware is a crash and everywhere is a denial of service.
pub const MAX_DELEGATION_DEPTH: u8 = 4;

/// One principal's role over one resource — the user↔device link that had
/// nowhere to live before this crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Grant {
    pub principal: Principal,
    pub resource: Resource,
    pub role: Role,
    /// When this grant stops being valid, if ever.
    ///
    /// `None` is a permanent grant, which is what an owner holds. A support
    /// engineer's access is the case this exists for: a grant that has to be
    /// remembered to be revoked is one that is not revoked.
    pub expires_at: Option<Timestamp>,
    /// Who handed this grant out, when it was not the platform.
    ///
    /// `None` means the grant is direct — seeded by provisioning or written by
    /// an operator. `Some(p)` means `p` delegated it, and the grant is only as
    /// good as `p`'s own authority over the same resource *right now*: revoke
    /// the manager and every grant they issued stops working, without anyone
    /// having to find them.
    pub delegated_by: Option<Principal>,
}

impl Grant {
    #[inline]
    pub const fn new(principal: Principal, resource: Resource, role: Role) -> Self {
        Self {
            principal,
            resource,
            role,
            expires_at: None,
            delegated_by: None,
        }
    }

    /// The same grant, valid only until `at`.
    #[inline]
    pub const fn expiring_at(mut self, at: Timestamp) -> Self {
        self.expires_at = Some(at);
        self
    }

    /// The same grant, derived from `delegator`'s authority rather than direct.
    #[inline]
    pub const fn delegated_by(mut self, delegator: Principal) -> Self {
        self.delegated_by = Some(delegator);
        self
    }

    /// `true` when this grant has not expired as of `now`.
    ///
    /// **Fails closed without a clock.** A grant that expires cannot be
    /// honoured by a caller that does not know the time, and a device whose
    /// RTC has not synced is exactly such a caller. Treating "no clock" as
    /// "not expired" would make expiry a suggestion that evaporates in the one
    /// environment least able to notice.
    #[inline]
    pub fn is_live(&self, now: Option<Timestamp>) -> bool {
        match (self.expires_at, now) {
            (None, _) => true,
            (Some(expiry), Some(now)) => now < expiry,
            (Some(_), None) => false,
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
pub fn can(
    principal: &Principal,
    action: Action,
    resource: &Resource,
    grants: &[Grant],
    now: Option<Timestamp>,
) -> bool {
    explain(principal, action, resource, grants, now).allowed
}

/// Why a principal may or may not act — the audit record.
///
/// [`can`] answers the gate's question and throws the reason away. An audit
/// log needs the reason, and reconstructing it afterwards from the grant table
/// is guesswork: the table has moved on by the time anyone reads the log.
///
/// This is the whole audit mechanism in this crate. There is deliberately no
/// sink, writer or buffer here — `no_std` has nowhere to put one, and a
/// Worker, a desktop app and a device each have a different right answer for
/// where a record goes. The crate produces the record; the host stores it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decision {
    pub allowed: bool,
    pub reason: Reason,
    /// The grant that decided it, when one did.
    pub via: Option<Grant>,
}

/// The specific ground for a decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// A live grant covered it.
    Granted,
    /// A device reading its own state, which needs no grant.
    DeviceReadingItself,
    /// Anonymous, or an all-zero sentinel id.
    NotIdentified,
    /// No grant covers this principal and resource at all.
    NoGrant,
    /// A grant covers it, but not at this level of power.
    RoleTooWeak,
    /// A covering grant existed and has expired.
    Expired,
    /// A covering grant exists but the caller has no clock to judge its
    /// expiry, so it cannot be honoured.
    NoClock,
    /// A covering grant was delegated by someone who no longer has the
    /// authority to have delegated it.
    DelegatorLacksAuthority,
    /// The delegation chain is longer than [`MAX_DELEGATION_DEPTH`].
    DelegationTooDeep,
}

/// [`can`], with the reason attached.
pub fn explain(
    principal: &Principal,
    action: Action,
    resource: &Resource,
    grants: &[Grant],
    now: Option<Timestamp>,
) -> Decision {
    let deny = |reason: Reason| Decision {
        allowed: false,
        reason,
        via: None,
    };

    if !principal.is_identified() {
        return deny(Reason::NotIdentified);
    }

    if let (Principal::Device(actor), Resource::Device(target)) = (principal, resource) {
        if actor == target && action == Action::Read {
            return Decision {
                allowed: true,
                reason: Reason::DeviceReadingItself,
                via: None,
            };
        }
    }

    // The most specific denial wins, so the reason reported is the one a
    // reader can act on: "expired" tells them to renew, where "no grant"
    // would send them to create one that is already there.
    let mut best_denial = Reason::NoGrant;
    let mut note = |reason: Reason| {
        let rank = |r: Reason| match r {
            Reason::NoGrant => 0,
            Reason::RoleTooWeak => 1,
            Reason::DelegationTooDeep => 2,
            Reason::DelegatorLacksAuthority => 3,
            Reason::NoClock => 4,
            Reason::Expired => 5,
            _ => 0,
        };
        if rank(reason) > rank(best_denial) {
            best_denial = reason;
        }
    };

    for grant in grants {
        if !grant.covers(principal, resource) {
            continue;
        }
        if !grant.role.allows(action) {
            note(Reason::RoleTooWeak);
            continue;
        }
        if !grant.is_live(now) {
            note(if now.is_none() {
                Reason::NoClock
            } else {
                Reason::Expired
            });
            continue;
        }
        match delegation_ok(grant, resource, grants, now, 0) {
            Ok(()) => {
                return Decision {
                    allowed: true,
                    reason: Reason::Granted,
                    via: Some(*grant),
                }
            }
            Err(reason) => note(reason),
        }
    }

    deny(best_denial)
}

/// Whether a grant's delegation chain still holds.
///
/// A delegated grant is worth exactly what its delegator's own authority is
/// worth *at evaluation time*. That is the point of recording the delegator
/// rather than flattening the grant on creation: revoking a manager revokes
/// everything they handed out, without anyone having to go and find it.
///
/// The delegator must be able to `Admin` the resource — the action that
/// [`Action::Admin`] is defined as, "change who else may act". A `Member`
/// cannot mint grants they could not use.
fn delegation_ok(
    grant: &Grant,
    resource: &Resource,
    grants: &[Grant],
    now: Option<Timestamp>,
    depth: u8,
) -> Result<(), Reason> {
    let Some(delegator) = grant.delegated_by else {
        return Ok(());
    };
    if depth >= MAX_DELEGATION_DEPTH {
        return Err(Reason::DelegationTooDeep);
    }

    // Walked inline rather than by calling `explain` again, so the depth
    // counter is threaded through. A cycle in the table is otherwise
    // unbounded recursion in the authorization path.
    if !delegator.is_identified() {
        return Err(Reason::DelegatorLacksAuthority);
    }
    for g in grants {
        if !g.covers(&delegator, resource) || !g.role.allows(Action::Admin) || !g.is_live(now) {
            continue;
        }
        if delegation_ok(g, resource, grants, now, depth + 1).is_ok() {
            return Ok(());
        }
    }
    Err(Reason::DelegatorLacksAuthority)
}

/// The strongest role `principal` holds over `resource`, if any.
///
/// Separate from [`can`] because a UI answers a different question than a
/// gate does: "what may I show them?" needs the role, not one verdict.
pub fn effective_role(
    principal: &Principal,
    resource: &Resource,
    grants: &[Grant],
    now: Option<Timestamp>,
) -> Option<Role> {
    if !principal.is_identified() {
        return None;
    }
    grants
        .iter()
        .filter(|g| g.covers(principal, resource))
        .filter(|g| g.is_live(now))
        .filter(|g| delegation_ok(g, resource, grants, now, 0).is_ok())
        .map(|g| g.role)
        .max()
}

// ── Caching ──────────────────────────────────────────────────────────────────

/// A fixed-size cache of recent decisions.
///
/// `can` is O(grants), and a Worker answering it per request against a table
/// fetched per request is the shape that makes authorization the slow part of
/// a page. This caches the verdict, not the grants.
///
/// **It holds no clock and does not expire entries by time.** A cache that
/// decided when its own contents were stale would be a second, quieter copy of
/// the expiry rule, and the two would disagree. Instead the generation counter
/// is bumped by [`Self::invalidate`] whenever the grant table changes, and the
/// caller is expected to bump it when crossing whatever time boundary its
/// grants use. `N` is a const parameter because `no_std` has no allocator.
/// One remembered verdict: the generation it was decided in, the question, and
/// the answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CachedVerdict {
    generation: u64,
    principal: Principal,
    action: Action,
    resource: Resource,
    allowed: bool,
}

#[derive(Debug)]
pub struct DecisionCache<const N: usize> {
    entries: [Option<CachedVerdict>; N],
    generation: u64,
    next: usize,
}

impl<const N: usize> Default for DecisionCache<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> DecisionCache<N> {
    pub const fn new() -> Self {
        Self {
            entries: [None; N],
            generation: 0,
            next: 0,
        }
    }

    /// Drop every cached verdict. Call this when the grant table changes.
    ///
    /// A bump rather than a clear: entries from the old generation are simply
    /// never matched, so invalidation is O(1) and cannot half-finish.
    pub fn invalidate(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    /// The cached verdict, if this exact question was asked since the last
    /// invalidation.
    pub fn get(&self, principal: &Principal, action: Action, resource: &Resource) -> Option<bool> {
        self.entries.iter().flatten().find_map(|e| {
            (e.generation == self.generation
                && e.principal == *principal
                && e.action == action
                && e.resource == *resource)
                .then_some(e.allowed)
        })
    }

    /// Answer from cache, or evaluate and remember.
    pub fn can(
        &mut self,
        principal: &Principal,
        action: Action,
        resource: &Resource,
        grants: &[Grant],
        now: Option<Timestamp>,
    ) -> bool {
        if let Some(hit) = self.get(principal, action, resource) {
            return hit;
        }
        let verdict = can(principal, action, resource, grants, now);
        if N > 0 {
            // Round-robin eviction. LRU needs bookkeeping proportional to the
            // cache, and for a table this size the difference does not pay for
            // the code that would have to be right.
            self.entries[self.next] = Some(CachedVerdict {
                generation: self.generation,
                principal: *principal,
                action,
                resource: *resource,
                allowed: verdict,
            });
            self.next = (self.next + 1) % N;
        }
        verdict
    }
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
            &[],
            None
        ));
    }

    #[test]
    fn an_owner_may_administer_their_device() {
        let g = [owner_grant()];
        assert!(can(
            &Principal::User(USER),
            Action::Admin,
            &Resource::Device(DEV),
            &g,
            None
        ));
    }

    #[test]
    fn a_stranger_may_not_read_someone_elses_device() {
        let g = [owner_grant()];
        assert!(!can(
            &Principal::User(OTHER),
            Action::Read,
            &Resource::Device(DEV),
            &g,
            None
        ));
    }

    #[test]
    fn a_grant_does_not_leak_to_another_device() {
        let g = [owner_grant()];
        assert!(!can(
            &Principal::User(USER),
            Action::Read,
            &Resource::Device(DEV2),
            &g,
            None
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
            &g,
            None
        ));
        assert!(!can(
            &Principal::User(USER),
            Action::Write,
            &Resource::Device(DEV),
            &g,
            None
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
            &g,
            None
        ));
        assert!(!can(
            &Principal::User(USER),
            Action::Admin,
            &Resource::Device(DEV),
            &g,
            None
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
            &g,
            None
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
            &g,
            None
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
            &[],
            None
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
            &[],
            None
        ));
    }

    #[test]
    fn a_device_may_not_read_another_device_without_a_grant() {
        assert!(!can(
            &Principal::Device(DEV),
            Action::Read,
            &Resource::Device(DEV2),
            &[],
            None
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
            &g,
            None
        ));
    }

    #[test]
    fn effective_role_returns_the_strongest_grant() {
        let g = [
            Grant::new(Principal::User(USER), Resource::Device(DEV), Role::Viewer),
            Grant::new(Principal::User(USER), Resource::Platform, Role::Admin),
        ];
        assert_eq!(
            effective_role(&Principal::User(USER), &Resource::Device(DEV), &g, None),
            Some(Role::Admin)
        );
    }

    #[test]
    fn effective_role_is_none_without_a_grant() {
        assert_eq!(
            effective_role(&Principal::User(USER), &Resource::Device(DEV), &[], None),
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

    // ── Expiry ───────────────────────────────────────────────────────────

    const NOW: Timestamp = Timestamp(1_000_000);
    const PAST: Timestamp = Timestamp(999_000);
    const FUTURE: Timestamp = Timestamp(1_001_000);

    #[test]
    fn an_expired_grant_does_not_grant() {
        let g = [owner_grant().expiring_at(PAST)];
        assert!(!can(
            &Principal::User(USER),
            Action::Read,
            &Resource::Device(DEV),
            &g,
            Some(NOW)
        ));
    }

    #[test]
    fn an_unexpired_grant_still_grants() {
        let g = [owner_grant().expiring_at(FUTURE)];
        assert!(can(
            &Principal::User(USER),
            Action::Read,
            &Resource::Device(DEV),
            &g,
            Some(NOW)
        ));
    }

    #[test]
    fn expiry_without_a_clock_fails_closed() {
        // A device whose RTC has not synced cannot honour an expiry. Treating
        // "no clock" as "not expired" would make expiry evaporate in the one
        // environment least able to notice.
        let g = [owner_grant().expiring_at(FUTURE)];
        assert!(!can(
            &Principal::User(USER),
            Action::Read,
            &Resource::Device(DEV),
            &g,
            None
        ));
        assert_eq!(
            explain(
                &Principal::User(USER),
                Action::Read,
                &Resource::Device(DEV),
                &g,
                None
            )
            .reason,
            Reason::NoClock
        );
    }

    #[test]
    fn a_permanent_grant_is_unaffected_by_a_missing_clock() {
        let g = [owner_grant()];
        assert!(can(
            &Principal::User(USER),
            Action::Read,
            &Resource::Device(DEV),
            &g,
            None
        ));
    }

    #[test]
    fn an_expired_grant_is_not_an_effective_role() {
        let g = [owner_grant().expiring_at(PAST)];
        assert_eq!(
            effective_role(
                &Principal::User(USER),
                &Resource::Device(DEV),
                &g,
                Some(NOW)
            ),
            None
        );
    }

    // ── Delegation ───────────────────────────────────────────────────────

    #[test]
    fn a_delegated_grant_works_while_its_delegator_has_authority() {
        let g = [
            owner_grant(),
            Grant::new(Principal::User(OTHER), Resource::Device(DEV), Role::Admin)
                .delegated_by(Principal::User(USER)),
        ];
        assert!(can(
            &Principal::User(OTHER),
            Action::Admin,
            &Resource::Device(DEV),
            &g,
            Some(NOW)
        ));
    }

    #[test]
    fn revoking_the_delegator_revokes_the_delegation() {
        // The whole point of recording the delegator rather than flattening
        // the grant: revoking a manager revokes everything they handed out,
        // without anyone having to go and find it.
        let g = [
            Grant::new(Principal::User(OTHER), Resource::Device(DEV), Role::Admin)
                .delegated_by(Principal::User(USER)),
        ];
        assert!(!can(
            &Principal::User(OTHER),
            Action::Read,
            &Resource::Device(DEV),
            &g,
            Some(NOW)
        ));
    }

    #[test]
    fn a_member_cannot_delegate_what_they_do_not_have() {
        let g = [
            Grant::new(Principal::User(USER), Resource::Device(DEV), Role::Member),
            Grant::new(Principal::User(OTHER), Resource::Device(DEV), Role::Admin)
                .delegated_by(Principal::User(USER)),
        ];
        assert!(!can(
            &Principal::User(OTHER),
            Action::Read,
            &Resource::Device(DEV),
            &g,
            Some(NOW)
        ));
    }

    #[test]
    fn a_delegation_cycle_terminates_rather_than_overflowing() {
        // A table anyone with Admin can write. An unbounded walk here is a
        // stack overflow in the authorization path — a crash on firmware.
        let g = [
            Grant::new(Principal::User(USER), Resource::Device(DEV), Role::Admin)
                .delegated_by(Principal::User(OTHER)),
            Grant::new(Principal::User(OTHER), Resource::Device(DEV), Role::Admin)
                .delegated_by(Principal::User(USER)),
        ];
        assert!(!can(
            &Principal::User(USER),
            Action::Read,
            &Resource::Device(DEV),
            &g,
            Some(NOW)
        ));
    }

    #[test]
    fn an_expired_delegator_cannot_sustain_a_delegation() {
        let g = [
            owner_grant().expiring_at(PAST),
            Grant::new(Principal::User(OTHER), Resource::Device(DEV), Role::Admin)
                .delegated_by(Principal::User(USER)),
        ];
        assert!(!can(
            &Principal::User(OTHER),
            Action::Read,
            &Resource::Device(DEV),
            &g,
            Some(NOW)
        ));
    }

    // ── Audit ────────────────────────────────────────────────────────────

    #[test]
    fn the_reason_reported_is_the_one_a_reader_can_act_on() {
        // "expired" sends them to renew; "no grant" would send them to create
        // one that is already there.
        let g = [owner_grant().expiring_at(PAST)];
        let d = explain(
            &Principal::User(USER),
            Action::Read,
            &Resource::Device(DEV),
            &g,
            Some(NOW),
        );
        assert!(!d.allowed);
        assert_eq!(d.reason, Reason::Expired);
    }

    #[test]
    fn an_allowed_decision_names_the_grant_that_decided_it() {
        let g = [owner_grant()];
        let d = explain(
            &Principal::User(USER),
            Action::Admin,
            &Resource::Device(DEV),
            &g,
            Some(NOW),
        );
        assert!(d.allowed);
        assert_eq!(d.reason, Reason::Granted);
        assert_eq!(d.via, Some(owner_grant()));
    }

    #[test]
    fn a_role_that_is_merely_too_weak_says_so() {
        let g = [Grant::new(
            Principal::User(USER),
            Resource::Device(DEV),
            Role::Viewer,
        )];
        assert_eq!(
            explain(
                &Principal::User(USER),
                Action::Admin,
                &Resource::Device(DEV),
                &g,
                Some(NOW)
            )
            .reason,
            Reason::RoleTooWeak
        );
    }

    // ── Cache ────────────────────────────────────────────────────────────

    #[test]
    fn a_cached_verdict_matches_an_uncached_one() {
        let g = [owner_grant()];
        let mut cache: DecisionCache<8> = DecisionCache::new();
        let p = Principal::User(USER);
        let r = Resource::Device(DEV);

        let direct = can(&p, Action::Write, &r, &g, Some(NOW));
        assert_eq!(cache.can(&p, Action::Write, &r, &g, Some(NOW)), direct);
        assert_eq!(cache.can(&p, Action::Write, &r, &g, Some(NOW)), direct);
        assert_eq!(cache.get(&p, Action::Write, &r), Some(direct));
    }

    #[test]
    fn invalidating_drops_every_cached_verdict() {
        let mut cache: DecisionCache<8> = DecisionCache::new();
        let p = Principal::User(USER);
        let r = Resource::Device(DEV);

        assert!(cache.can(&p, Action::Read, &r, &[owner_grant()], Some(NOW)));
        cache.invalidate();
        assert_eq!(cache.get(&p, Action::Read, &r), None);
        // And the re-evaluation sees the new table rather than the old answer.
        assert!(!cache.can(&p, Action::Read, &r, &[], Some(NOW)));
    }

    #[test]
    fn the_cache_distinguishes_actions_and_resources() {
        let g = [Grant::new(
            Principal::User(USER),
            Resource::Device(DEV),
            Role::Viewer,
        )];
        let mut cache: DecisionCache<8> = DecisionCache::new();
        let p = Principal::User(USER);

        assert!(cache.can(&p, Action::Read, &Resource::Device(DEV), &g, Some(NOW)));
        assert!(!cache.can(&p, Action::Write, &Resource::Device(DEV), &g, Some(NOW)));
        assert!(!cache.can(&p, Action::Read, &Resource::Device(DEV2), &g, Some(NOW)));
    }

    #[test]
    fn a_full_cache_keeps_answering_correctly() {
        // Round-robin eviction must not turn into a wrong answer, only a miss.
        let g = [owner_grant()];
        let mut cache: DecisionCache<2> = DecisionCache::new();
        let p = Principal::User(USER);
        for _ in 0..10 {
            assert!(cache.can(&p, Action::Read, &Resource::Device(DEV), &g, Some(NOW)));
            assert!(!cache.can(&p, Action::Read, &Resource::Device(DEV2), &g, Some(NOW)));
        }
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
