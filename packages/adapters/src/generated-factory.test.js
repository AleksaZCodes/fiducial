/**
 * The generated adapter factory is typechecked, not just string-matched.
 *
 * `fid derive` writes `src/adapters.generated.ts` into a product from the
 * `[adapters]` block in its `fiducial.toml`. Every existing test on that file
 * — in `crates/fiducial-cli/tests/adapters_pipeline.rs` — reads it as **text**
 * and asserts `contains("new NoneEmail(env)")`.
 *
 * That is the failure mode this platform has already been bitten by twice. A
 * `contains()` check passes on code that does not compile: `NoneEmail` and
 * `NoneDiagnostics` shipped for a whole phase with no explicit constructor, so
 * the `new NoneEmail(env)` the generator emitted failed to typecheck under
 * `strict` — and every test was green, because every test was reading a string.
 * The identity work hit the same shape from the other side, where a migration
 * asserted as text turned out to be SQLite-only syntax that no Postgres server
 * would run.
 *
 * So this suite runs the real generator and hands the result to `tsc`. It is
 * the same argument as `verify-postgres.sh`: the only way to know generated
 * code works is to run the thing that consumes it.
 *
 * ## This suite is `test:generated`, not `test`
 *
 * It needs the real `fid` binary *and* the built package, so it is a separate
 * script for exactly the reason `@fiducial/identity`'s `test:schema` is — the
 * generic JS/TS CI job builds no Rust, and a suite that cannot run where it is
 * invoked is worse than one that is named.
 *
 *     cargo build -p fiducial-cli --bin fid
 *     pnpm --filter @fiducial/adapters build
 *     pnpm --filter @fiducial/adapters test:generated
 */

import { describe, it, before } from 'node:test'
import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import {
  mkdtempSync,
  mkdirSync,
  existsSync,
  readFileSync,
  writeFileSync,
  symlinkSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const here = dirname(fileURLToPath(import.meta.url))
const packageRoot = join(here, '..')
const repoRoot = join(packageRoot, '../..')
const fid = join(repoRoot, 'target/debug/fid')
const tsc = join(repoRoot, 'node_modules/.bin/tsc')

/**
 * Every contract set to a vendor that is really implemented.
 *
 * Hardcoding the vendor names here is deliberate and is the point of the test:
 * if `adapter.rs` promotes a candidate to an implementation, the generator
 * starts emitting an import for it, and nothing else in the suite would ever
 * compile that import. A new vendor is added to this list, or it is not
 * covered.
 */
const REAL_VENDORS = {
  database: 'supabase',
  storage: 'supabase-storage',
  deploy: 'cloudflare',
  email: 'resend',
  botProtection: 'turnstile',
  queue: 'cloudflare-queues',
  newsletter: 'resend',
  auth: 'supabase',
  ai: 'openrouter',
  // `errors` has no real vendor yet — `none` is a real implementation, so it
  // belongs in the typechecked set rather than omitted.
  errors: 'none',
}

// The Cloudflare-native pair: D1 for database, R2 for storage.
const CLOUDFLARE_VENDORS = {
  ...REAL_VENDORS,
  database: 'd1',
  storage: 'r2',
}

/** Scaffold a product, select `vendors`, derive, and return its root. */
function productWith(vendors) {
  if (!existsSync(fid)) {
    throw new Error(
      `this suite typechecks the real generated factory, and ${fid} is not built.\n` +
        'Run: cargo build -p fiducial-cli --bin fid',
    )
  }
  if (!existsSync(join(packageRoot, 'dist/index.d.ts'))) {
    throw new Error(
      'this suite resolves @fiducial/adapters from dist/, which is not built.\n' +
        'Run: pnpm --filter @fiducial/adapters build',
    )
  }

  const dir = mkdtempSync(join(tmpdir(), 'adapters-factory-'))
  execFileSync(fid, ['new', 'p'], { cwd: dir, stdio: 'ignore' })
  const root = join(dir, 'p')
  execFileSync(fid, ['add', 'adapters'], { cwd: root, stdio: 'ignore' })

  const block = Object.entries(vendors)
    .map(([contract, vendor]) => `${contract} = "${vendor}"`)
    .join('\n')
  const config = join(root, 'fiducial.toml')
  // `ai = "openrouter"` needs a model: a gateway routes by model id, so derive
  // refuses without one. That refusal is tested in adapters_pipeline.rs; here
  // the point is to compile the factory it produces.
  const ai = vendors.ai === 'openrouter' ? '\n[ai]\nmodel = "anthropic/claude-opus-5"\n' : ''
  writeFileSync(config, `${readFileSync(config, 'utf8')}\n[adapters]\n${block}\n${ai}`)

  execFileSync(fid, ['derive'], { cwd: root, stdio: 'ignore' })

  // Resolve `@fiducial/adapters` the way a real product does — through
  // node_modules and the package's own `exports` map, not a tsconfig `paths`
  // alias. The subpath imports the generator emits (`/database`, `/auth`) are
  // exports-map entries, so an alias would typecheck a resolution no consumer
  // actually performs.
  const scope = join(root, 'node_modules/@fiducial')
  mkdirSync(scope, { recursive: true })
  symlinkSync(packageRoot, join(scope, 'adapters'), 'dir')

  writeFileSync(
    join(root, 'tsconfig.json'),
    JSON.stringify(
      {
        compilerOptions: {
          target: 'ES2022',
          module: 'NodeNext',
          moduleResolution: 'NodeNext',
          strict: true,
          noEmit: true,
          skipLibCheck: true,
        },
        include: ['src'],
      },
      null,
      2,
    ),
  )

  return root
}

/** Run `tsc --noEmit` in `root`, returning its combined output. */
function typecheck(root) {
  try {
    execFileSync(tsc, ['--noEmit', '--project', root], { encoding: 'utf8', stdio: 'pipe' })
    return ''
  } catch (e) {
    return `${e.stdout ?? ''}${e.stderr ?? ''}`.trim()
  }
}

describe('the generated adapter factory compiles', () => {
  let allNone
  let allReal
  let allCloudflare

  before(() => {
    allNone = productWith(Object.fromEntries(Object.keys(REAL_VENDORS).map((c) => [c, 'none'])))
    allReal = productWith(REAL_VENDORS)
    allCloudflare = productWith(CLOUDFLARE_VENDORS)
  })

  it('with every contract set to none', () => {
    const errors = typecheck(allNone)
    assert.equal(errors, '', `tsc rejected the all-none factory:\n${errors}`)
  })

  it('with every contract set to a real vendor (supabase database + storage)', () => {
    const errors = typecheck(allReal)
    assert.equal(errors, '', `tsc rejected the all-real-vendor factory:\n${errors}`)
  })

  it('with cloudflare-native vendors (d1 + r2)', () => {
    const errors = typecheck(allCloudflare)
    assert.equal(errors, '', `tsc rejected the cloudflare-vendor factory:\n${errors}`)
  })

  it('constructs every contract, so the typecheck actually covered them', () => {
    // A factory that emitted nothing would typecheck trivially.
    const factory = readFileSync(join(allReal, 'src/adapters.generated.ts'), 'utf8')
    for (const contract of ['database', 'storage', 'email', 'diagnostics', 'botProtection', 'queue', 'newsletter', 'ai']) {
      assert.match(factory, new RegExp(`${contract}:\\s*new `), `${contract} is not constructed`)
    }
    assert.match(factory, /export function createAuth\(/)
  })
})
