/**
 * Token verification in TypeScript — the Worker's half of `token.rs`.
 *
 * The same fixed-layout, domain-separated ed25519 token a device verifies
 * offline. Two implementations of one format is exactly the drift this
 * platform exists to prevent, so the layout constants are asserted against
 * vectors generated from the Rust crate.
 *
 * Verification only. **Issuing needs a private key, and a Worker is not where
 * one lives** — a signing key in edge code is a signing key in every deploy
 * log and every error report. The platform issues; everything else verifies.
 */

import type { Principal, Timestamp } from "./index.js";

export const SIGNATURE_LEN = 64;
export const PUBLIC_KEY_LEN = 32;
export const CLAIMS_LEN = 41;
export const TOKEN_LEN = CLAIMS_LEN + SIGNATURE_LEN;

/** Mirrors `DOMAIN` in `token.rs`. A token is not an OTA manifest. */
const DOMAIN = new TextEncoder().encode("fiducial-identity-token-v1");

/** Why a token was refused. Mirrors `TokenError` in Rust. */
export type TokenError =
  | "malformed_length"
  | "unknown_principal_kind"
  | "bad_signature"
  | "expired"
  | "not_yet_valid"
  | "no_clock"
  | "wrong_principal";

export interface Claims {
  principal: Principal;
  issuedAt: Timestamp;
  expiresAt: Timestamp;
  /** Issuer-chosen, unique per token. `bigint` because it is a full u64. */
  nonce: bigint;
}

export type TokenResult =
  | { ok: true; claims: Claims }
  | { ok: false; error: TokenError };

const hex = (bytes: Uint8Array): string =>
  Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");

/** Read the claims out of their canonical bytes. */
export function decodeClaims(bytes: Uint8Array): TokenResult {
  if (bytes.length !== CLAIMS_LEN) return { ok: false, error: "malformed_length" };
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);

  let principal: Principal;
  switch (bytes[0]) {
    case 1:
      principal = { kind: "user", id: hex(bytes.subarray(1, 17)) };
      break;
    case 2:
      principal = { kind: "device", id: hex(bytes.subarray(1, 9)) };
      break;
    case 3:
      principal = { kind: "service", id: hex(bytes.subarray(1, 9)) };
      break;
    default:
      return { ok: false, error: "unknown_principal_kind" };
  }

  // Milliseconds since the epoch fits a double exactly until year 287396, so
  // Number is safe for the timestamps. The nonce is an arbitrary u64 and is
  // not, so it stays a bigint.
  return {
    ok: true,
    claims: {
      principal,
      issuedAt: Number(view.getBigUint64(17)),
      expiresAt: Number(view.getBigUint64(25)),
      nonce: view.getBigUint64(33),
    },
  };
}

/**
 * Verify a token with a WebCrypto Ed25519 public key.
 *
 * `key` is imported by the caller (`crypto.subtle.importKey("raw", …,
 * { name: "Ed25519" }, …)`) so this never handles key material and never
 * decides key rotation policy.
 */
export async function verifyToken(
  key: CryptoKey,
  token: Uint8Array,
  now: Timestamp | null,
): Promise<TokenResult> {
  if (token.length !== TOKEN_LEN) return { ok: false, error: "malformed_length" };

  const encoded = token.subarray(0, CLAIMS_LEN);
  // Copied rather than a subarray: `subarray` shares the caller's buffer, and
  // WebCrypto's BufferSource wants a view over a plain ArrayBuffer. A copy of
  // 64 bytes also means a caller mutating its own array mid-verification
  // cannot change what was checked.
  const signature = new Uint8Array(token.subarray(CLAIMS_LEN));

  const input = new Uint8Array(DOMAIN.length + CLAIMS_LEN);
  input.set(DOMAIN, 0);
  input.set(encoded, DOMAIN.length);

  // Signature first. Reading claims out of an unverified token and acting on
  // them — even to report a better error — is how a parser becomes the attack
  // surface.
  const valid = await crypto.subtle.verify("Ed25519", key, signature, input);
  if (!valid) return { ok: false, error: "bad_signature" };

  const decoded = decodeClaims(encoded);
  if (!decoded.ok) return decoded;

  // Fails closed without a clock, exactly as the grant rule does: a bearer
  // credential whose expiry is unenforceable is a permanent one.
  if (now === null) return { ok: false, error: "no_clock" };
  if (now >= decoded.claims.expiresAt) return { ok: false, error: "expired" };
  if (now < decoded.claims.issuedAt) return { ok: false, error: "not_yet_valid" };

  return decoded;
}

/**
 * Verify, and require the token to be for `expected`.
 *
 * Forgetting this means accepting any authentic token as any principal: the
 * signature is real and the binding to *who is asking* is simply missing.
 */
export async function verifyTokenAs(
  key: CryptoKey,
  token: Uint8Array,
  expected: Principal,
  now: Timestamp | null,
): Promise<TokenResult> {
  const result = await verifyToken(key, token, now);
  if (!result.ok) return result;
  const p = result.claims.principal;
  const same =
    p.kind === expected.kind &&
    (p.kind === "anonymous" ||
      expected.kind === "anonymous" ||
      p.id === (expected as { id: string }).id);
  return same ? result : { ok: false, error: "wrong_principal" };
}
