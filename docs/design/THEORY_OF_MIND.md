# Deep-dive — Theory-of-Mind as a Corrective

**Status:** Deep-dive / proposal. **Parent:**
[`OFFLOAD_LANDSCAPE.md`](OFFLOAD_LANDSCAPE.md) §A (cognitive correctives).
**Scope note:** this is a *corrective*, deliberately lighter than the load-bearing
layers (memory / watchdog / deterministic / correctives). It doesn't get its own
layer — it **sharpens primitives we already have** (Diverge/perspectives and
requirement-clarification) with a grounded methodology. **One line:** *the model
reasons from its own knowledge as if others share it; an independent
perspective-taking pass models what the other party actually knows and believes,
distinct from what the model knows.*

## The failure it corrects: egocentric anchoring (the curse of knowledge)

Theory-of-mind is modeling *other minds* — representing that someone holds beliefs,
knowledge, or intent that differ from yours, including beliefs you know to be false.
Two grounded facts shape the design:

- **The model's own ToM is brittle.** GPT-4 passes Sally-Anne-style false-belief
  tasks ([Kosinski]) but performance **collapses under minor perturbations**
  ([Ullman]; mixed BigToM/SocialIQa results) — so it's partly pattern-matching, not a
  reliable capability ([ToM in LLMs](https://www.emergentmind.com/topics/theory-of-mind-in-large-language-models)).
  *Corollary: don't trust the model's spontaneous ToM; structure it.*
- **The practical failure is egocentric anchoring** — people and LLMs use *their own*
  knowledge as the reference frame and **overestimate how much others share it** (the
  curse of knowledge, [feedback lifts it](https://www.ncbi.nlm.nih.gov/pmc/articles/PMC8107504/)).
  A sharp instance: when an LLM judges a statement false, it perceives the *speaker's*
  certainty as low **regardless of what the speaker expressed** — it projects its own
  belief onto the other mind.

## Two applications (whose mind is being modeled)

- **The user (the epistemic gap).** The model assumes the user knows what *it* knows,
  or that its reading of their intent is the right one. But the user has partial
  observability and a belief shaped by their own profile/knowledge that may diverge
  from the model's ([epistemic divergence](https://arxiv.org/pdf/2602.13832)). This
  half **is** the requirement-clarification thread: "I assumed you meant X."
- **Third parties (the task's other minds).** "How will stakeholder X react,"
  negotiation, a review of someone else's intent, white lies
  ([TactfulToM](https://arxiv.org/pdf/2509.17054)), multi-agent collaboration.

## The corrective, specced

The method that works in the research is **structured perspective-taking with a
perception → belief decomposition** (SimToM / PercepToM / TimeToM): don't infer the
other mind's belief directly (that's where projection leaks in) — first establish
*what they perceived / have access to*, then infer belief *from that*, which turns a
hard false-belief problem into a simpler true-belief judgment
([UniToMBench](https://arxiv.org/html/2506.09450)).

A perspective-taking pass, three steps:

1. **Access** — what does this party actually perceive / know / have been told? (Build
   *their* information set, explicitly *excluding* what only the model knows.)
2. **Belief** — what do they therefore believe, want, intend — reasoning *only* from
   their access set?
3. **Divergence** — where does their belief differ from ground truth / from the
   model's own — and what are their blind spots?

**Independence is the right delivery mechanism**, for the same reason as Challenge:
the model reasoning *in its own frame* is the egocentric anchor, so a separate pass
explicitly instructed to occupy the other's information set breaks the projection.
(Higher-order ToM — "A thinks B thinks…" for negotiation — is just recursion on this,
and gets fragile/expensive fast; cap the depth.)

### Sketch contract

```jsonc
// in
{ "situation": "string", "perspective_of": "user | <named party>",
  "model_knows": ["facts only the model has"], "depth": 1 }
// out
{ "access": ["what they perceive/know"],
  "belief": "what they believe/want/intend, from their access only",
  "diverges": ["where their belief != ground truth / the model's view"],
  "blind_spots": ["..."], "confidence": 0.0 }
```

## Where it lives: a corrective, not a layer

It is delivered by the **Diverge/perspectives** primitive (the old server's
`decision-perspectives` already maps stakeholder viewpoints — ToM is the cognitive
science under it), with the perception→belief decomposition as the *methodology*. Its
user-modeling half feeds **requirement-clarification**. It is **not** a standalone
layer and doesn't need one.

A clean unifying split it surfaces: ToM divides across the architecture by *whose
mind*. **ToM-of-others** is this perspective-taking corrective. **ToM-of-self** — the
model can't reliably model its *own* mind, can't tell when it's wrong (there are
documented [self-modeling deficits](https://arxiv.org/pdf/2603.26089)) — is exactly
the failure the **watchdog** exists for. Same cognitive gap, two layers, depending on
whose mind is in question.

## Hard problems

1. **The model's ToM is itself brittle**, so a perspective-taking pass can be
   *confidently wrong* about the other mind. Anchor it to *real signals* — what the
   user actually said, observable facts — not the model's guess about them; treat the
   output as a hypothesis to check, not a fact.
2. **Projection may persist** — the decomposition reduces but doesn't eliminate
   leaking the model's own knowledge into the "access" set; an independent context
   helps, blinding the pass to model-only facts helps more.
3. **Over-modeling the user** is its own failure — presuming you've inferred intent
   well enough to *skip* asking. Perspective-taking should often *raise* a clarifying
   question, not replace it (tie to requirement-clarification).
4. **Higher-order recursion** is fragile and costly; cap depth, and prefer it only
   where it pays (negotiation/adversarial).

## Open questions

- Is a dedicated perspective-taking *operation* worth it, or is it just a better
  *prompt/methodology* inside Diverge/perspectives? (Leaning: methodology, not a new
  tool — consistent with "keep the surface small.")
- How to blind the access-set construction to model-only knowledge in practice?
- When does user-ToM trigger a clarifying question vs a confident inference?
- Eval: ToM benchmarks are Sally-Anne-narrow; how do you measure the *useful* version
  (did modeling the user/stakeholder change the outcome for the better)?
