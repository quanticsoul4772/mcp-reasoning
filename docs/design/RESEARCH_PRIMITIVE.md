# Spec — The Research Primitive

**Status:** Proposal. **Parent:** [`NEXT_REASONING_SERVER.md`](NEXT_REASONING_SERVER.md).
**One line:** *offload a question to a separate budget; get back a short, cited,
adversarially-verified answer — not 15 articles.*

Research is the highest-leverage capability the current server lacks, and it
exercises every value in the parent note's model at once: **parallelism** (fan out
many searches/fetches), **offloaded budget** (the work burns a separate context),
**adversarial verification** (independent critics the client can't run unanchored
in-context), and a **compact return** (a page, not a corpus). The client
*structurally cannot* run 15 parallel fetch-and-verify cycles in its own context
without blowing it up; the server can.

This spec is end-to-end: contract, pipeline, verification, controls, observability,
failure modes. Schemas are illustrative (language-neutral JSON), not final types.

---

## 1. The contract (MCP tool)

### Request

```jsonc
{
  "question": "string",            // required — the research question
  "depth": "quick|standard|deep|exhaustive",  // default "standard" — scales fan-out + rigor
  "focus": ["string"],             // optional — angles to bias toward
  "constraints": {
    "max_sources":  120,           // hard cap on fetched sources
    "domains_allow": ["string"],   // restrict to these domains (optional)
    "domains_deny":  ["string"],   // never fetch these
    "recency":      "past_year",   // freshness filter (optional)
    "budget_tokens": 400000,       // HARD ceiling — synthesize-early when hit
    "deadline_ms":   180000        // wall-clock ceiling — partial answer on expiry
  },
  "session_id": "string"           // optional — for cache reuse + Recall
}
```

Only `question` is required. Everything else has depth-derived defaults (§5).

### Response (compact — this is the whole point)

```jsonc
{
  "answer": "string",              // the executive synthesis — the payload
  "confidence": 0.0,               // 0–1, grounded in verification (§4), not vibes
  "key_findings": [{
    "claim": "string",
    "confidence": 0.0,             // post-verification
    "support": "confirmed|contested|refuted|unverified",
    "sources": ["s3", "s7"]        // citation ids — MUST resolve in `sources`
  }],
  "disagreements": [{              // where good sources conflict — surfaced, not resolved
    "claim": "string",
    "positions": [{ "stance": "string", "sources": ["s2"] }]
  }],
  "gaps": ["string"],              // what couldn't be answered / needs more
  "sources": [{
    "id": "s3", "url": "string", "title": "string",
    "fetched_at": "rfc3339", "credibility": 0.0   // est. 0–1
  }],
  "stats": {                       // honest accounting — see "no silent caps"
    "angles": 5, "searches": 5, "sources_found": 88, "sources_fetched": 41,
    "claims_extracted": 130, "claims_after_dedup": 74, "claims_verified": 74,
    "claims_dropped": 19, "tokens": 312000, "elapsed_ms": 141000,
    "stopped_early": false, "stop_reason": null   // "budget"|"deadline"|"max_sources"|null
  }
}
```

**What the response deliberately does NOT contain:** raw fetched page content, full
verifier transcripts, or per-search result lists. Those stay server-side (cached,
§7) and are retrievable by id only if a follow-up tool asks. The client gets a
page; the corpus stays offloaded.

**Cardinal rule (enforced, not hoped):** every clause in `answer` traces to a
`key_findings` entry, and every `key_findings.sources` id resolves in `sources`. A
post-synthesis **grounding gate** (§4.3) rejects+retries any output that cites a
source that wasn't fetched or makes a claim with no source. No fabricated
citations, ever — that is the one unforgivable failure for a research tool.

---

## 2. The pipeline

Five phases. The middle three are where the offload pays off — they fan out.
Parallelism is a server primitive ([parent §"value model"]); the client never
loops.

```
question
   │
 (1) SCOPE        1 LLM call  → N angles + success criteria + falsifiable sub-questions
   │
 (2) SEARCH       N parallel  → candidate URLs per angle ──┐ dedup URLs across angles
   │                                                       │
 (3) FETCH+EXTRACT  pipeline (per source, no barrier)  ────┘
   │              fetch → readable text → extract falsifiable claims (+source span)
   │
 (4) VERIFY       fan-out per unique claim
   │              dedup claims → K independent adversarial votes each → confidence
   │
 (5) SYNTHESIZE   1–2 LLM calls → rank, group, write compact cited answer
   │              + completeness critic (deep/exhaustive) → maybe one more round
   ▼
answer
```

### (1) Scope — *1 call*

Decompose `question` into **N search angles** (N from depth, §5), a short list of
*falsifiable sub-questions* a good answer must settle, and explicit success
criteria. Constrained output: `{ angles: [...], sub_questions: [...], success: [...] }`.
This is the only fully-sequential step; everything after fans out.

### (2) Search — *N parallel*

One search per angle against the configured provider, honoring `domains_allow/deny`
and `recency`. Collect candidate `{url, title, snippet}`; **dedup by normalized
URL** across angles. Rank candidates (snippet relevance × source heuristics) and
take the top `max_sources` for the tier.

### (3) Fetch + Extract — *pipeline, per source, no barrier*

Each source flows fetch → extract **independently** (item A can be extracting while
B is still fetching — the parent note's "pipeline by default"):

- **Fetch:** HTTP GET with per-fetch timeout, robots.txt respect, per-domain rate
  limit, redirect cap, content-type/size guard. Reduce to readable text
  (boilerplate stripped). Unreachable/paywalled/too-large → dropped, counted.
- **Extract:** *1 LLM call per source* → a list of **falsifiable claims**, each with
  the supporting span and the source id. Non-falsifiable fluff is discarded at the
  source. Constrained output: `[{ claim, span, source_id }]`.

### (4) Verify — *fan-out per unique claim*

1. **Dedup claims** semantically (embedding cosine + a merge pass) → unique claims,
   each now backed by ≥1 source.
2. For each unique claim, run **K independent adversarial verifiers** (K from depth).
   Each is a *separate* call prompted to **refute** the claim ("default to refuted
   if uncertain"); for `deep`/`exhaustive`, give each verifier a **distinct lens**
   (source credibility, internal consistency, recency, contradicting evidence)
   rather than K identical refuters — diversity catches failure modes redundancy
   can't.
3. **Verdict + confidence** (§4.1).

### (5) Synthesize — *1–2 calls*

Rank surviving claims by confidence, group by sub-question, and write the compact
`answer` with inline citations. Surface `disagreements` (claims where credible
sources conflict — *do not* pick a winner) and `gaps` (sub-questions left
`unverified`/unanswered). For `deep`/`exhaustive`, a **completeness critic** pass
asks "what sub-question is unsettled, what angle wasn't searched, what claim is
unverified?" — if it finds something material and budget remains, loop back to (2)
for one more round on just those gaps (bounded, §5).

---

## 3. What runs where (concurrency)

| Phase | Shape | Barrier? |
|---|---|---|
| Scope | 1 call | — |
| Search | N concurrent | barrier (need all candidates to dedup URLs) |
| Fetch+Extract | pipeline per source | **no barrier** — slowest single source ≠ sum |
| Verify | fan-out per claim (K votes each, concurrent) | barrier before synthesis |
| Synthesize | 1–2 calls (+ optional critic loop) | — |

Concurrency is capped server-side (a worker pool); passing 120 sources doesn't mean
120 simultaneous fetches. The Search→dedup and Verify→synthesize barriers are the
only two genuine barriers; everything else pipelines.

---

## 4. Verification & confidence (the trust model)

### 4.1 Per-claim verdict

For a claim with `v` verifiers, `r` of them refuting, `n` independent sources,
mean source credibility `c̄`:

- `support = refuted`   if `r ≥ ceil(v/2)` (majority refute) → **dropped from the answer body**, kept in stats.
- `support = contested` if refuters and supporters are close, or sources disagree → surfaced in `disagreements`.
- `support = confirmed` if `r` small and `n ≥ 2` independent sources agree.
- `support = unverified` if `n = 1` and verifiers are split → reported as a gap-ish low-confidence note, never stated as fact.

`confidence ≈ w1·(1 − r/v) + w2·agreement(sources) + w3·c̄ + w4·min(n, 3)/3`,
clamped 0–1. Weights are config, tuned **offline** (parent §"improvement is
offline"), never at runtime.

### 4.2 Overall confidence

`response.confidence` = coverage-weighted mean of the `key_findings` confidences,
penalized by unanswered `sub_questions`. A research answer that settled 3 of 7
sub-questions cannot report high confidence even if those 3 are airtight.

### 4.3 Grounding gate (hard)

Before returning, validate: every `answer` clause maps to a `key_findings` entry;
every cited id exists in `sources`; no `sources` entry is uncited dead weight.
Failure → one constrained-output retry with the violation fed back; second failure
→ return with the offending claims demoted to `gaps` and `stopped_early`/`stop_reason`
set. The tool never emits an ungrounded claim.

---

## 5. Depth tiers (the one knob that scales everything)

| | angles | max_sources | verifier votes K | completeness loops | typical budget |
|---|---|---|---|---|---|
| **quick** | 3 | 8 | 1 | 0 | ~40k tok |
| **standard** | 5 | 25 | 2 | 0 | ~120k tok |
| **deep** | 8 | 60 | 3 (diverse lenses) | 1 | ~350k tok |
| **exhaustive** | 12 | 120 | 3–5 (diverse) | 2 | ~800k tok |

`budget_tokens`/`deadline_ms` always override the tier: hitting either triggers a
**graceful early synthesize** over whatever is verified so far, with `stopped_early`
and `stop_reason` set and the unfinished sub-questions listed in `gaps`. **No silent
truncation** — if it stopped short, the response says so.

---

## 6. Controls & safety

- **Hard ceilings:** `budget_tokens` and `deadline_ms` are enforced ceilings, not
  hints — the worker stops spawning and synthesizes early when either is hit.
- **Network egress is gated**, off by default like every capability: a configured
  search provider + fetch allowlist; no provider configured ⇒ the tool returns a
  clear "research disabled" config error (not a silent degrade).
- **Fetch hygiene:** robots.txt, per-domain rate limit, per-fetch timeout, redirect
  - size + content-type guards, `domains_deny` honored absolutely.
- **Redaction:** the answer is *derived*; `sources` carry URL+title only; no raw
  page bodies, secrets, or verifier transcripts cross the wire. Reuse the parent's
  redaction discipline.
- **Determinism aids:** caches (§7) make re-runs cheap and partially reproducible;
  true determinism isn't promised (the web moves), and `stats` records what was seen.

---

## 7. Persistence, caching, and the Recall tie-in

- **Source cache:** fetched readable text keyed by content-hash + `fetched_at`;
  re-fetch only past a TTL. Cheap re-runs and cross-question reuse.
- **Claim cache:** extracted claims keyed by source-hash, so re-verification doesn't
  re-extract.
- **Result cache:** the whole `response` keyed by `(normalized question, depth,
  constraints)` with a TTL → an identical re-ask is near-free.
- **Recall hook:** when `session_id` is set, a completed research result is written
  to the memory store ([parent "Recall"/durable memory]) so a later, related
  question can surface "you already researched X; here's the prior answer + its
  sources" — turning the dead `relate` capability into something effortless.

---

## 8. Observability (it's a parallel pipeline — make it visible)

This primitive is the dashboard's best demo: a wide fan-out with a clear spine.
Emit activity events per stage so it animates:

- `Research Started` (question, depth) → `Scope Completed` (N angles)
- `Search` ×N (one pulse per angle) → `Sources Found` (count)
- `Fetch`/`Extract` per source (Worker-style nodes) → claims-extracted counter
- `Verify` per claim with vote tallies → confirmed/contested/refuted counters
- `Synthesize Started/Completed` → final confidence + token/elapsed
- Stream the same milestones as MCP progress notifications so the calling client
  sees a live "researching… 41/60 sources, verifying 38/74 claims" instead of a
  3-minute silent hang.

Every event carries tokens/cost/latency from the start (parent §"observability
designed in"). A `--demo` replay of a canned research run makes it presentable
without live spend.

---

## 9. Failure modes & degradation

- **Search returns nothing** → report an honest gap; never hallucinate sources.
- **All sources for a sub-question refuted/contested** → mark it `contested` and
  surface positions; do not pick a side.
- **Budget/deadline hit mid-flight** → graceful early synthesis over verified
  claims; unfinished sub-questions → `gaps`; `stopped_early=true`.
- **A source fails to fetch** → drop it, count it, continue (one bad URL never
  fails the run).
- **Extract/verify call errors** → retry (bounded), then drop that claim; a dropped
  claim is counted, never silently swallowed.
- **Grounding-gate failure** → §4.3 (retry once, then demote to gaps). The tool
  would rather return "I couldn't ground this" than a confident fabrication.

---

## 10. Why this can't just be a client-side loop

A capable client *could* call a search tool and read results. What it cannot
cheaply do, in its own context: run **N searches + M fetches + (claims × K)
independent verification calls** — easily hundreds of LLM/tool calls — without (a)
destroying its own context window, (b) serializing what should be parallel, and (c)
anchoring its "verification" on the same context that produced the claims. The
server does all three on a separate budget and hands back a page. That gap *is* the
product.

---

## 11. Open questions

- **Search provider abstraction:** wrap one provider or several behind a trait?
  (Trait — providers are swappable and rate/cost differ.)
- **Claim dedup quality:** embedding-cosine + merge is cheap but lossy; how
  aggressive before distinct claims get collapsed? Needs a tuned threshold + tests.
- **Credibility scoring:** heuristic (domain reputation, recency, corroboration) vs
  a learned signal. Start heuristic; it feeds `confidence`, so it must be
  conservative and explainable.
- **Verifier independence vs cost:** K diverse-lens verifiers per claim is the
  expensive part. Where's the knee? (Tier it; measure offline.)
- **Result-cache staleness:** a cached answer to "latest X" is wrong tomorrow. TTL
  by question class? Recency-sensitive questions get short/zero TTL.
