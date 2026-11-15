//! `auth = "native"` — the [`Auth`] contract with no vendor and no network.
//!
//! The self-hosted target's auth (`docs/specs/2026-11-01-a-self-hosted-target.md`):
//!
//! - **accounts are declared in server configuration**, each with an
//!   [argon2id] hash, never a password. There is no sign-up and no
//!   password-reset email, because nothing here can send one;
//! - **a session is a signed token**: `base64url(user|expiry).base64url(HMAC-SHA256)`,
//!   keyed by a secret the server holds. Nothing is stored server-side, so it
//!   survives a restart and needs no table. The server puts it in an httpOnly
//!   cookie; this crate does not know about cookies.
//!
//! **The trait is request-less, this is not.** [`Auth::get_session`] has no
//! request to read a cookie from, so here it reports `None` and says why. A
//! server verifies each request with [`NativeAuth::verify`], which is the
//! operation a self-hosted server actually needs.
//!
//! [argon2id]: https://datatracker.ietf.org/doc/html/rfc9106

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::auth::{Auth, AuthError, AuthSession, AuthUser};
use crate::BoxFuture;

type HmacSha256 = Hmac<Sha256>;

/// One account declared in server configuration.
#[derive(Debug, Clone)]
pub struct Account {
    pub id: String,
    pub email: String,
    /// A PHC string from [`hash_password`], e.g. `$argon2id$v=19$…`.
    pub password_hash: String,
}

/// The shortest session secret accepted, in bytes. A short key makes the
/// signature guessable, so it is refused at construction, not at first use.
pub const MIN_SECRET_LEN: usize = 32;

pub struct NativeAuth {
    accounts: Vec<Account>,
    secret: Vec<u8>,
    ttl_secs: i64,
    /// Verified against when the email is unknown, so an unknown email and a
    /// wrong password take the same time.
    decoy: String,
}

/// Hash a password for an [`Account`]. Run it once, offline, and put the result
/// in configuration.
pub fn hash_password(password: &str) -> Result<String, AuthError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AuthError::Other(format!("hashing failed: {e}")))
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

impl NativeAuth {
    pub fn new(accounts: Vec<Account>, secret: &[u8], ttl_secs: i64) -> Result<Self, AuthError> {
        if secret.len() < MIN_SECRET_LEN {
            return Err(AuthError::Other(format!(
                "auth = \"native\": the session secret is {} bytes; it must be at least \
                 {MIN_SECRET_LEN}. Generate one with `openssl rand -base64 48` and set it as \
                 SESSION_SECRET.",
                secret.len()
            )));
        }
        for a in &accounts {
            PasswordHash::new(&a.password_hash).map_err(|e| {
                AuthError::Other(format!(
                    "auth = \"native\": account `{}` has no valid argon2 hash ({e}). Store the \
                     output of hash_password, never a password.",
                    a.email
                ))
            })?;
        }
        Ok(Self {
            accounts,
            secret: secret.to_vec(),
            ttl_secs,
            decoy: hash_password("decoy")?,
        })
    }

    fn mac(&self, payload: &[u8]) -> HmacSha256 {
        // `new_from_slice` accepts any length for HMAC.
        let mut m = HmacSha256::new_from_slice(&self.secret).expect("hmac takes any key length");
        m.update(payload);
        m
    }

    fn session_for(&self, account: &Account, at: i64) -> AuthSession {
        let exp = at + self.ttl_secs;
        let payload = format!("{}|{exp}", account.id);
        let sig = self.mac(payload.as_bytes()).finalize().into_bytes();
        let token = format!(
            "{}.{}",
            URL_SAFE_NO_PAD.encode(payload),
            URL_SAFE_NO_PAD.encode(sig)
        );
        AuthSession {
            access_token: token,
            refresh_token: None,
            expires_at: exp,
            user: AuthUser {
                id: account.id.clone(),
                email: Some(account.email.clone()),
                email_verified: Some(true),
                created_at: None,
            },
        }
    }

    /// Check a password and issue a session as of `at` (Unix seconds).
    pub fn sign_in_at(
        &self,
        email: &str,
        password: &str,
        at: i64,
    ) -> Result<AuthSession, AuthError> {
        let account = self
            .accounts
            .iter()
            .find(|a| a.email.eq_ignore_ascii_case(email));
        let hash = account.map_or(self.decoy.as_str(), |a| a.password_hash.as_str());
        let parsed = PasswordHash::new(hash).map_err(|e| AuthError::Other(e.to_string()))?;
        let ok = Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok();
        match (ok, account) {
            (true, Some(a)) => Ok(self.session_for(a, at)),
            _ => Err(AuthError::InvalidCredentials),
        }
    }

    /// Verify a token from a request's cookie, as of `at`. `None` for a forged,
    /// altered, expired or unknown-account token; never an error, so a caller
    /// treats all of them as "not signed in".
    pub fn verify_at(&self, token: &str, at: i64) -> Option<AuthSession> {
        let (payload_b64, sig_b64) = token.split_once('.')?;
        let payload = URL_SAFE_NO_PAD.decode(payload_b64).ok()?;
        let sig = URL_SAFE_NO_PAD.decode(sig_b64).ok()?;
        // Constant-time comparison.
        self.mac(&payload).verify_slice(&sig).ok()?;
        let text = String::from_utf8(payload).ok()?;
        let (id, exp) = text.split_once('|')?;
        let exp: i64 = exp.parse().ok()?;
        if exp <= at {
            return None;
        }
        let account = self.accounts.iter().find(|a| a.id == id)?;
        Some(AuthSession {
            expires_at: exp,
            ..self.session_for(account, exp - self.ttl_secs)
        })
    }

    pub fn verify(&self, token: &str) -> Option<AuthSession> {
        self.verify_at(token, now())
    }
}

const DECLARED: &str = "auth = \"native\": accounts are declared in server configuration, with \
                        an argon2 hash. Add or change one there; this server sends no email.";

impl Auth for NativeAuth {
    fn sign_up<'a>(
        &'a self,
        _: &'a str,
        _: &'a str,
    ) -> BoxFuture<'a, Result<AuthSession, AuthError>> {
        Box::pin(std::future::ready(Err(AuthError::Other(DECLARED.into()))))
    }

    fn sign_in<'a>(
        &'a self,
        email: &'a str,
        password: &'a str,
    ) -> BoxFuture<'a, Result<AuthSession, AuthError>> {
        Box::pin(std::future::ready(self.sign_in_at(email, password, now())))
    }

    fn sign_out<'a>(&'a self) -> BoxFuture<'a, Result<(), AuthError>> {
        // A signed token holds no server-side state: signing out is the server
        // clearing the cookie.
        Box::pin(std::future::ready(Ok(())))
    }

    fn get_session<'a>(&'a self) -> BoxFuture<'a, Result<Option<AuthSession>, AuthError>> {
        // Request-less, so there is no cookie to read. Use `verify` per request.
        Box::pin(std::future::ready(Ok(None)))
    }

    fn reset_password_for_email<'a>(
        &'a self,
        _: &'a str,
        _: &'a str,
    ) -> BoxFuture<'a, Result<(), AuthError>> {
        Box::pin(std::future::ready(Err(AuthError::Other(DECLARED.into()))))
    }

    fn update_password<'a>(&'a self, _: &'a str) -> BoxFuture<'a, Result<(), AuthError>> {
        Box::pin(std::future::ready(Err(AuthError::Other(DECLARED.into()))))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &[u8] = b"0123456789abcdef0123456789abcdef-secret";

    fn auth() -> NativeAuth {
        let hash = hash_password("correct horse").unwrap();
        NativeAuth::new(
            vec![Account {
                id: "u1".into(),
                email: "a@example.org".into(),
                password_hash: hash,
            }],
            SECRET,
            3600,
        )
        .unwrap()
    }

    #[test]
    fn a_right_password_signs_in_and_the_token_verifies() {
        let a = auth();
        let s = a
            .sign_in_at("A@Example.org", "correct horse", 1000)
            .unwrap();
        assert_eq!(s.user.id, "u1");
        assert_eq!(s.expires_at, 4600);
        let v = a.verify_at(&s.access_token, 2000).unwrap();
        assert_eq!(v.user.email.as_deref(), Some("a@example.org"));
        assert_eq!(v.expires_at, 4600);
    }

    #[test]
    fn wrong_password_and_unknown_email_look_the_same() {
        let a = auth();
        let e1 = a
            .sign_in_at("a@example.org", "nope", 0)
            .unwrap_err()
            .to_string();
        let e2 = a
            .sign_in_at("who@example.org", "nope", 0)
            .unwrap_err()
            .to_string();
        assert_eq!(e1, e2);
        assert!(e1.contains("invalid credentials"));
    }

    #[test]
    fn an_expired_altered_or_foreign_token_is_not_a_session() {
        let a = auth();
        let s = a.sign_in_at("a@example.org", "correct horse", 0).unwrap();
        assert!(a.verify_at(&s.access_token, 3600).is_none(), "expired");
        // Alter the payload: another user id, same signature.
        let (_, sig) = s.access_token.split_once('.').unwrap();
        let forged = format!("{}.{sig}", URL_SAFE_NO_PAD.encode("u2|99999999999"));
        assert!(a.verify_at(&forged, 0).is_none(), "altered payload");
        // Signed with another secret.
        let other =
            NativeAuth::new(vec![], b"another-secret-another-secret-another!!", 60).unwrap();
        assert!(other.verify_at(&s.access_token, 0).is_none(), "foreign key");
        assert!(a.verify_at("not a token", 0).is_none());
    }

    #[test]
    fn a_short_secret_or_a_plain_password_in_config_is_refused() {
        let e = NativeAuth::new(vec![], b"short", 60)
            .err()
            .unwrap()
            .to_string();
        assert!(e.contains("at least 32"), "{e}");
        let e = NativeAuth::new(
            vec![Account {
                id: "u".into(),
                email: "a@b".into(),
                password_hash: "hunter2".into(),
            }],
            SECRET,
            60,
        )
        .err()
        .unwrap()
        .to_string();
        assert!(e.contains("never a password"), "{e}");
    }

    #[tokio::test]
    async fn sign_up_and_reset_say_where_accounts_live() {
        let a = auth();
        let e = a.sign_up("x@y", "pw").await.unwrap_err().to_string();
        assert!(e.contains("declared in server configuration"), "{e}");
        assert!(a.get_session().await.unwrap().is_none());
    }
}
