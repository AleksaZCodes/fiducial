/**
 * Tests for the System One contract.
 * Imports from ../dist — built by tsc before this test runs.
 *
 * The wire fixtures are the *documented* examples, copied verbatim from
 * OpenRouter's OpenAPI spec for `POST /api/alpha/decisions` and TypeSafe's API
 * reference. That is the point of them: a hand-written fixture only proves the
 * adapter agrees with whatever its author assumed, and the bug this file exists
 * to catch is the adapter disagreeing with the vendor.
 */

import { describe, it } from 'node:test'
import assert from 'node:assert/strict'
import {
  NoneSystemOne,
  OpenRouterSystemOne,
  TypeSafeSystemOne,
  SystemOneError,
  OPENROUTER_DEFAULT_MODEL,
  TYPESAFE_DEFAULT_MODEL,
  noulOf,
  choiceOf,
  scoreOf,
  createNoneAdapters,
} from '../dist/index.js'

/** The response example from OpenRouter's spec, verbatim. */
const OR_RESPONSE = {
  id: 'gen-dec-1789738314-X5e5eKGQdvR9rblyX250',
  model: 'typesafe/jev-1.13-20260917',
  provider: 'TypeSafe',
  answers: {
    is_bug: { type: 'noul', noul: 0.96 },
    team: {
      type: 'choice',
      choice: 'payments',
      confidence: 0.75,
      probabilities: { account: 0, frontend: 0.16, payments: 0.84 },
    },
    urgency: {
      type: 'score',
      score: 1.99,
      confidence: 0.99,
      legend: {
        0: 'Can wait for the next release',
        1: 'Should be fixed this week',
        2: 'Blocking revenue right now',
      },
      probabilities: { 0: 0, 1: 0.01, 2: 0.99 },
    },
  },
  usage: { cost: 0.000019992, input_tokens: 476, output_tokens: 70 },
}

/** The request example from OpenRouter's spec, verbatim. */
const QUESTIONS = {
  is_bug: {
    type: 'noul',
    instructions: 'Is the customer reporting a software defect?',
    criteria: {
      true: 'The customer describes broken or unexpected product behavior.',
      false: 'The customer is asking a question or requesting a feature.',
    },
  },
  team: {
    type: 'choice',
    instructions: 'Which team should own this ticket?',
    criteria: {
      account: 'Login, permissions, or profile issues.',
      frontend: 'Rendering, layout, or browser compatibility issues.',
      payments: 'Checkout, billing, or payment processing issues.',
    },
  },
  urgency: {
    type: 'score',
    instructions: 'How urgent is this ticket?',
    criteria: [
      'Can wait for the next release',
      'Should be fixed this week',
      'Blocking revenue right now',
    ],
  },
}

const STATE = {
  customer_tier: 'enterprise',
  ticket: 'My checkout page shows a blank screen after I click Pay.',
}

/** A fetch stub that records calls and replays queued responses. */
function stubFetch(queue) {
  const calls = []
  const impl = async (url, init) => {
    calls.push({ url, init, body: JSON.parse(init.body) })
    const next = queue.shift()
    if (typeof next === 'function') return next()
    const { status = 200, body = OR_RESPONSE, headers = {} } = next ?? {}
    return new Response(JSON.stringify(body), {
      status,
      headers: { 'content-type': 'application/json', ...headers },
    })
  }
  return { impl, calls }
}

describe('NoneSystemOne', () => {
  it('throws with the fix in the message', async () => {
    const s = new NoneSystemOne()
    await assert.rejects(
      () => s.decide({ state: 'x', questions: {} }),
      (e) => {
        assert.ok(e instanceof SystemOneError)
        assert.equal(e.kind, 'unauthorized')
        assert.match(e.message, /systemOne = "openrouter"/)
        return true
      },
    )
  })

  it('is in the None adapter set', () => {
    assert.ok(createNoneAdapters().systemOne instanceof NoneSystemOne)
  })
})

describe('OpenRouterSystemOne', () => {
  it('throws at construction without a key, naming the command', () => {
    assert.throws(
      () => new OpenRouterSystemOne({}),
      (e) => {
        assert.equal(e.kind, 'unauthorized')
        assert.match(e.message, /wrangler secret put OPENROUTER_API_KEY/)
        return true
      },
    )
  })

  it('posts to the decisions router, not chat completions', async () => {
    const { impl, calls } = stubFetch([{}])
    const s = new OpenRouterSystemOne({ OPENROUTER_API_KEY: 'k' }, undefined, impl)
    await s.decide({ state: STATE, questions: QUESTIONS })

    assert.equal(calls[0].url, 'https://openrouter.ai/api/alpha/decisions')
    assert.equal(calls[0].init.headers.Authorization, 'Bearer k')
    // Regression guard: the whole point is that this is NOT chat completions.
    assert.ok(!calls[0].url.includes('chat/completions'))
    assert.ok(!('messages' in calls[0].body))
  })

  it('sends state and questions unchanged, and the prefixed default model', async () => {
    const { impl, calls } = stubFetch([{}])
    const s = new OpenRouterSystemOne({ OPENROUTER_API_KEY: 'k' }, undefined, impl)
    await s.decide({ state: STATE, questions: QUESTIONS })

    assert.equal(calls[0].body.model, OPENROUTER_DEFAULT_MODEL)
    assert.equal(calls[0].body.model, 'typesafe/jev-latest')
    assert.deepEqual(calls[0].body.state, STATE)
    assert.deepEqual(calls[0].body.questions, QUESTIONS)
  })

  it('parses the documented response, including cost, id and provider', async () => {
    const { impl } = stubFetch([{}])
    const s = new OpenRouterSystemOne({ OPENROUTER_API_KEY: 'k' }, undefined, impl)
    const r = await s.decide({ state: STATE, questions: QUESTIONS })

    assert.equal(r.model, 'typesafe/jev-1.13-20260917')
    assert.equal(r.id, 'gen-dec-1789738314-X5e5eKGQdvR9rblyX250')
    assert.equal(r.provider, 'TypeSafe')
    assert.equal(r.usage.inputTokens, 476)
    assert.equal(r.usage.outputTokens, 70)
    assert.equal(r.usage.cost, 0.000019992)

    assert.equal(noulOf(r.answers, 'is_bug'), 0.96)
    assert.equal(choiceOf(r.answers, 'team').choice, 'payments')
    assert.equal(choiceOf(r.answers, 'team').confidence, 0.75)
    assert.equal(scoreOf(r.answers, 'urgency').score, 1.99)
    assert.equal(scoreOf(r.answers, 'urgency').legend['2'], 'Blocking revenue right now')
  })

  it('passes sessionId as session_id, and omits it when unset', async () => {
    const { impl, calls } = stubFetch([{}, {}])
    const s = new OpenRouterSystemOne({ OPENROUTER_API_KEY: 'k' }, undefined, impl)
    await s.decide({ state: 'x', questions: {}, sessionId: 'derive-run-7' })
    await s.decide({ state: 'x', questions: {} })

    assert.equal(calls[0].body.session_id, 'derive-run-7')
    assert.ok(!('session_id' in calls[1].body))
  })

  it('honours a per-call model override', async () => {
    const { impl, calls } = stubFetch([{}])
    const s = new OpenRouterSystemOne({ OPENROUTER_API_KEY: 'k' }, 'typesafe/jev-1.13', impl)
    await s.decide({ state: 'x', questions: {}, model: 'typesafe/jev-latest' })
    assert.equal(calls[0].body.model, 'typesafe/jev-latest')
  })

  it('retries a 429 and succeeds', async () => {
    const { impl, calls } = stubFetch([
      { status: 429, body: { error: { message: 'Rate limit exceeded' } } },
      {},
    ])
    const s = new OpenRouterSystemOne({ OPENROUTER_API_KEY: 'k' }, undefined, impl, {
      baseDelayMs: 1,
    })
    const r = await s.decide({ state: 'x', questions: {} })
    assert.equal(calls.length, 2)
    assert.equal(r.usage.inputTokens, 476)
  })

  it('retries a 529 then gives up with the last error', async () => {
    const overloaded = { status: 529, body: { error: { message: 'Overloaded' } } }
    const { impl, calls } = stubFetch([overloaded, overloaded, overloaded])
    const s = new OpenRouterSystemOne({ OPENROUTER_API_KEY: 'k' }, undefined, impl, {
      attempts: 3,
      baseDelayMs: 1,
    })
    await assert.rejects(
      () => s.decide({ state: 'x', questions: {} }),
      (e) => {
        assert.equal(e.kind, 'overloaded')
        assert.equal(e.status, 529)
        return true
      },
    )
    assert.equal(calls.length, 3)
  })

  it('does NOT retry a 422, and classifies it as invalid_request', async () => {
    const { impl, calls } = stubFetch([
      { status: 422, body: { error: { message: 'questions.q: missing criteria' } } },
    ])
    const s = new OpenRouterSystemOne({ OPENROUTER_API_KEY: 'k' }, undefined, impl, {
      baseDelayMs: 1,
    })
    await assert.rejects(
      () => s.decide({ state: 'x', questions: {} }),
      (e) => {
        assert.equal(e.kind, 'invalid_request')
        assert.match(e.message, /missing criteria/)
        return true
      },
    )
    // A validation error is deterministic — retrying it is pure latency.
    assert.equal(calls.length, 1)
  })

  it('classifies 402 as insufficient_credits and does not retry', async () => {
    const { impl, calls } = stubFetch([
      { status: 402, body: { error: { message: 'Insufficient credits' } } },
    ])
    const s = new OpenRouterSystemOne({ OPENROUTER_API_KEY: 'k' }, undefined, impl, {
      baseDelayMs: 1,
    })
    await assert.rejects(
      () => s.decide({ state: 'x', questions: {} }),
      (e) => {
        assert.equal(e.kind, 'insufficient_credits')
        return true
      },
    )
    assert.equal(calls.length, 1)
  })

  it('retries a transport failure', async () => {
    const { impl, calls } = stubFetch([
      () => {
        throw new Error('ECONNRESET')
      },
      {},
    ])
    const s = new OpenRouterSystemOne({ OPENROUTER_API_KEY: 'k' }, undefined, impl, {
      baseDelayMs: 1,
    })
    const r = await s.decide({ state: 'x', questions: {} })
    assert.equal(calls.length, 2)
    assert.equal(r.provider, 'TypeSafe')
  })

  it('rejects a 200 whose body carries an error instead of answers', async () => {
    const { impl } = stubFetch([{ status: 200, body: { error: { message: 'upstream died' } } }])
    const s = new OpenRouterSystemOne({ OPENROUTER_API_KEY: 'k' }, undefined, impl, {
      baseDelayMs: 1,
    })
    await assert.rejects(
      () => s.decide({ state: 'x', questions: {} }),
      (e) => {
        assert.match(e.message, /upstream died/)
        return true
      },
    )
  })
})

describe('TypeSafeSystemOne', () => {
  const native = {
    body: {
      model: 'jev-1.13.0',
      answers: { q: { type: 'noul', noul: 0.5 } },
      usage: { input_tokens: 1, output_tokens: 2 },
    },
  }

  it('posts to the native endpoint with the unprefixed model', async () => {
    const { impl, calls } = stubFetch([native])
    const s = new TypeSafeSystemOne({ TYPESAFE_API_KEY: 'tk' }, undefined, impl)
    const r = await s.decide({ state: 'x', questions: {} })

    assert.equal(calls[0].url, 'https://api.typesafe.ai/v1/systemone')
    assert.equal(calls[0].init.headers.Authorization, 'Bearer tk')
    assert.equal(calls[0].body.model, TYPESAFE_DEFAULT_MODEL)
    assert.equal(calls[0].body.model, 'jev-latest')
    // No cost on the direct API — absent, not a fabricated zero.
    assert.equal(r.usage.cost, undefined)
  })

  it('ignores sessionId rather than sending a field the API has no use for', async () => {
    const { impl, calls } = stubFetch([native])
    const s = new TypeSafeSystemOne({ TYPESAFE_API_KEY: 'tk' }, undefined, impl)
    await s.decide({ state: 'x', questions: {}, sessionId: 'ignored' })
    assert.ok(!('session_id' in calls[0].body))
  })

  it('throws at construction without a key', () => {
    assert.throws(
      () => new TypeSafeSystemOne({}),
      (e) => {
        assert.match(e.message, /TYPESAFE_API_KEY/)
        return true
      },
    )
  })
})

describe('answer helpers', () => {
  const answers = OR_RESPONSE.answers

  it('name the key and the actual type on a mismatch', () => {
    assert.throws(() => noulOf(answers, 'team'), /`team` is a choice, expected a noul/)
    assert.throws(() => scoreOf(answers, 'is_bug'), /`is_bug` is a noul, expected a score/)
  })

  it('name the available keys when one is missing', () => {
    assert.throws(() => noulOf(answers, 'typo'), /no answer under `typo`/)
    assert.throws(() => noulOf(answers, 'typo'), /is_bug, team, urgency/)
  })
})
