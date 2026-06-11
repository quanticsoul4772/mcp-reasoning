# Deep-dive — The Deterministic / Symbolic Layer

**Status:** Deep-dive / proposal. **Parent:**
[`OFFLOAD_LANDSCAPE.md`](OFFLOAD_LANDSCAPE.md) §B. Referenced by
[`CORRECTIVE_SELECTION.md`](CORRECTIVE_SELECTION.md) (the "checkable-ness" routing
signal), [`MEMORY_LAYER.md`](MEMORY_LAYER.md) (verify-before-store), and the Verify
spike. **One line:** *a large class of claims and tasks is checkable by *execution*,
not judgment — and for those a solver beats a probabilistic critic, because there is
no judge to fool, no calibration knob, and no sycophancy.*

## Why this is the most reliable layer

The Verify spike ran into a precision/recall calibration problem because an LLM
critic is *itself* fallible. But when a claim can be turned into code or a formal
form, you don't *judge* it — you **execute** it. Program-Aided Language Models (PAL)
make the case empirically: translate the problem to code, run it, and accuracy jumps
**+15 pts on GSM8K, +40 on GSM-Hard, +11 on BIG-Bench-Hard** over chain-of-thought —
because the model decomposes well but errs in *execution*, and the interpreter does
the execution exactly ([PAL](https://arxiv.org/abs/2211.10435)). The output is
deterministic, reproducible, and **unforgeable**: a sycophantic or adversarial model
cannot talk its way past a solver, and cannot fabricate a result it didn't actually
compute. No LLM-judge has that property.

## What's checkable (the routing-in signal)

"Checkable-ness" is the deterministic routing signal from
[`CORRECTIVE_SELECTION.md`](CORRECTIVE_SELECTION.md) §2 — and it's the cleanest signal
in the whole system because it's a property of the claim, not a guess about the model.

| Claim/task shape | Engine |
|---|---|
| arithmetic / quantitative | code execution / CAS (SymPy) |
| logic / constraints | SMT (Z3), SAT |
| code correctness | run tests / type-check / static analysis |
| planning | classical planner (PDDL / Fast-Downward; LLM+P) |
| **format / schema** | schema validation — *this is the constrained-output gate* |
| units / dates / conversions | deterministic libraries |
| contract / instruction compliance | formal spec check |

**Not** checkable — judgment, values, open-ended questions, rich world-knowledge /
common-sense (PAL's own stated weakness). Those stay with the LLM critic and the
cognitive correctives. Deciding which side a claim is on is itself a judgment (see
hard problems).

## Architecture: translate → execute → feed back

The PAL pattern: the LLM **translates** (NL → code/formal), the engine **executes**,
the result returns. The property that makes the loop reliable: **the engine's errors
are ground truth.** "Doesn't compile / infeasible / type error / test failed" is
*correct* — unlike an LLM-critic's verdict, which can be wrong. So:

```
NL claim → [LLM translate] → formal/code → [engine execute]
                ↑                                   │
                └──── re-translate on a REAL ───────┘
                      violation (not a guessed one)
```

This is the symbolic feedback loop: re-prompt on *actual* constraint violations, not
on a critic's opinion — closing the gap LLM+P leaves open (it had no closed loop, so
a bad translation failed the whole pipeline).

## The failure moves to translation (the one honest catch)

Deterministic offload does not remove the failure — it **moves** it. Execution is now
exact; the failure surface becomes the **NL → formal translation** (autoformalization).
The model can formalize the *wrong problem* — syntactically valid, semantically
unfaithful — and the solver will faithfully solve the wrong thing. Worse, faithfulness
is hard to check: LLM judges "miss subtle errors precisely where unfaithful
specifications fail" ([autoformalization survey](https://arxiv.org/pdf/2505.23486),
[Verus-SpecGym](https://arxiv.org/html/2605.26457)). Defenses:

- **Back-translation / round-trip** — formal → NL, compare to the original (cosine /
  LLM). Cheap, catches gross mistranslation.
- **The solver's free signals** — syntactic validity and infeasibility cost nothing
  and are reliable.
- **Multiple independent formalizations + agreement** (ensemble — ties to Verify).
- **Keep the formal target small and typed** so there's less room for semantic drift.

Honest framing: you've traded a probabilistic *execution* failure for a probabilistic
*translation* failure — but translation is a **narrower, more checkable** surface
(round-trip, type-check, infeasibility) than open reasoning, and the execution half is
now exact. A net win *where it applies*.

## Safety: executing model-generated code is non-negotiable to sandbox

The §K tie-in, and a hard prerequisite. LLM-generated code is an **arbitrary code
execution** risk (`os.system`, `subprocess`), a DoS risk (resource exhaustion), and a
filesystem/network risk. Treat **all** generated code as potentially malicious. The
requirements are not optional: strong isolation (Docker / microVM / E2B), whitelisted
libraries only, filesystem/OS/network disabled by default, and timeouts + resource
quotas ([secure code execution](https://huggingface.co/docs/smolagents/tutorials/secure_code_execution),
[SandboxEval](https://arxiv.org/pdf/2504.00018)). A verify-by-execution layer without
a sandbox *is* a remote-code-execution hole.

## How it ties to the rest

- **Verify** — route checkable claims here (deterministic, no calibration); the
  LLM-critic is the *fallback* for judgment claims. This is the two-mode Verify the
  landscape calls for, and it's why the spike's calibration problem isn't the whole
  story: for a big slice of claims, you don't calibrate, you execute.
- **Memory** — gates the write path. **Verify-by-execution before storing a skill** is
  the strongest form of "curated, not credulous": a stored skill that is *literally
  executable and tested* can't be a poisoned fiction. This is the cleanest version of
  the MEMORY_LAYER synthesis.
- **Planning** — symbolic planning (LLM+P) is this layer applied to plans:
  guaranteed-valid output.
- **Watchdog** — prefers deterministic checks where the signal allows; a solver has no
  judge-bias of its own (no "who watches the watchdog" regress).
- **Constrained output** — schema validation is this layer at the smallest scale, and
  it's the architectural bet from `NEXT_REASONING_SERVER.md` §1.

## Hard problems

1. **Translation faithfulness** — the moved failure, and the central risk. Round-trip
   and ensemble help, but a confidently-wrong formalization is the new way to be wrong.
2. **The "looks checkable but isn't" trap** — over-formalizing a fuzzy problem forces
   a *false precision* (turning a judgment call into a solver query that answers the
   wrong question crisply). Knowing *when* to use this layer is itself a judgment.
3. **Sandbox cost / ops** — isolated execution per check has real latency and
   infrastructure cost.
4. **Solver brittleness** — invalid formal output, solver timeouts on hard instances,
   limited expressiveness.
5. **The boundary call** — checkable-vs-not is an LLM decision (fallible); err toward
   the LLM critic when unsure, because a *false* "this is checkable" is the costliest
   mistake (crisp wrong answer).

## Open questions

- Which engines to host first? Python sandbox + schema validator covers the most;
  SymPy / Z3 / a planner extend it. Start narrow.
- Round-trip translation check: always, or only on a low-confidence signal?
- The checkable-ness classifier — heuristic, learned, or an LLM call with a bias
  toward "not checkable" (safer default)?
- Sandbox tier — in-process restricted vs container vs microVM — the cost/security
  knee.
