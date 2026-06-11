#!/usr/bin/env python3
"""Thin spike of the Verify primitive (NEXT_REASONING_SERVER.md).

Tests the load-bearing assumption: does an INDEPENDENT adversarial pass catch
confident-wrong claims the model can't catch about itself?

Engineered independence (the judge-bias catch):
  - blind: verifiers see ONLY the claim — not who asked, not how confidently it
    was stated, not each other.
  - parallel: K verifiers run concurrently, never sequentially under pushback.
  - diverse lenses: each verifier attacks from a different angle, not N clones.
Constrained output: each verifier is FORCED to the verdict schema via tool_use
(dogfoods the "constrained output" decision — no free-text parsing).

Verdict: majority refute -> refuted; zero refute -> confirmed; else contested.
Confidence: agreement among verifiers x their mean self-confidence.

Usage:  python verify_spike.py            # runs the built-in test battery
        python verify_spike.py "claim"    # verify one claim (k=3)
Needs ANTHROPIC_API_KEY in env.
"""

import json
import os
import sys
import urllib.error
import urllib.request
from concurrent.futures import ThreadPoolExecutor

MODEL = "claude-sonnet-4-6"
API = "https://api.anthropic.com/v1/messages"
KEY = os.environ.get("ANTHROPIC_API_KEY")
HEADERS = {
    "x-api-key": KEY or "",
    "anthropic-version": "2023-06-01",
    "content-type": "application/json",
}

VERDICT_TOOL = {
    "name": "verdict",
    "description": "Return your verdict after trying to refute the claim.",
    "input_schema": {
        "type": "object",
        "properties": {
            "refuted": {
                "type": "boolean",
                "description": "true if you found a genuine factual error, invalid "
                "logic, unsupported leap, false hidden assumption, or material "
                "overstatement; false if the claim genuinely withstands scrutiny",
            },
            "confidence": {
                "type": "number",
                "description": "0.0-1.0 confidence in your own verdict",
            },
            "reason": {"type": "string", "description": "one-sentence justification"},
        },
        "required": ["refuted", "confidence", "reason"],
    },
}

LENSES = [
    "factual accuracy: is it actually, literally true?",
    "logical validity: does the stated reasoning really support it, or is there a leap?",
    "counter-evidence: construct the strongest case that it is false or overstated",
    "hidden assumptions: does it rest on an unstated premise that is false or shaky?",
    "scope/overstatement: is it claimed more broadly or certainly than warranted?",
]

SYSTEM = (
    "You are an independent adversarial reviewer. You did NOT write this claim and "
    "have no stake in it. Find any genuine flaw: factual error, invalid logic, an "
    "unsupported leap, a false hidden assumption, or material overstatement. Judge "
    "the claim ONLY on its merits — ignore how confidently it is phrased and who "
    "asked. If the claim is partly true but overstated, refute it. If it genuinely "
    "withstands scrutiny, do not refute it. Attack it through this lens: {lens}"
)


def verify_once(claim: str, lens: str) -> dict:
    body = {
        "model": MODEL,
        "max_tokens": 400,
        "system": SYSTEM.format(lens=lens),
        "messages": [{"role": "user", "content": f"Claim:\n{claim}\n\nReturn your verdict."}],
        "tools": [VERDICT_TOOL],
        "tool_choice": {"type": "tool", "name": "verdict"},
    }
    req = urllib.request.Request(
        API, data=json.dumps(body).encode(), headers=HEADERS, method="POST"
    )
    try:
        with urllib.request.urlopen(req, timeout=90) as resp:
            data = json.loads(resp.read())
    except urllib.error.HTTPError as e:
        return {"refuted": None, "confidence": 0.0, "reason": f"HTTP {e.code}: {e.read()[:200]}"}
    for block in data.get("content", []):
        if block.get("type") == "tool_use":
            return block["input"]  # schema-guaranteed by tool_choice
    return {"refuted": None, "confidence": 0.0, "reason": "no tool_use block returned"}


def verify(claim: str, k: int = 3) -> dict:
    lenses = (LENSES * ((k // len(LENSES)) + 1))[:k]
    with ThreadPoolExecutor(max_workers=k) as ex:
        votes = list(ex.map(lambda L: verify_once(claim, L), lenses))
    valid = [v for v in votes if v.get("refuted") is not None]
    n = len(valid)
    refutes = sum(1 for v in valid if v["refuted"])
    if n == 0:
        support = "error"
        conf = 0.0
    elif refutes >= (n + 1) // 2:
        support = "refuted"
    elif refutes == 0:
        support = "confirmed"
    else:
        support = "contested"
    if n:
        agree = max(refutes, n - refutes) / n
        conf = round(agree * (sum(v["confidence"] for v in valid) / n), 2)
    return {"support": support, "refutes": f"{refutes}/{n}", "confidence": conf, "votes": votes}


# (expected verdict, claim) — mix of true, false, plausible-but-wrong, and
# claims I asserted confidently and was WRONG about earlier this session.
BATTERY = [
    ("confirmed", "Rust's borrow checker prevents data races in safe code at compile time."),
    ("refuted", "Rust's borrow checker guarantees a compiled program has no logic bugs."),
    ("refuted", "In Rust, wrapping a value in Arc<T> lets multiple threads safely MUTATE it "
                "without any further synchronization."),
    ("refuted", "Stopping a background task that launched a server via a stdin pipe also "
                "terminates the spawned server process, freeing its TCP port."),
    ("refuted", "Running `cargo bench` rebuilds the same binary as `cargo build --features "
                "dashboard`, so a feature-gated listener stays compiled in."),
]


def main():
    if not KEY:
        print("ANTHROPIC_API_KEY not set", file=sys.stderr)
        sys.exit(1)
    if len(sys.argv) > 1:
        claim = " ".join(sys.argv[1:])
        r = verify(claim, k=3)
        print(json.dumps({"claim": claim, **r}, indent=2))
        return
    print(f"Verify spike — model={MODEL}, k=3 diverse-lens verifiers, blind+parallel\n")
    hits = 0
    for expected, claim in BATTERY:
        r = verify(claim, k=3)
        ok = "OK " if r["support"] == expected else "MISS"
        hits += r["support"] == expected
        print(f"[{ok}] expected={expected:9} got={r['support']:9} "
              f"refutes={r['refutes']} conf={r['confidence']}")
        print(f"       {claim}")
        for v in r["votes"]:
            mark = "x" if v.get("refuted") else ("." if v.get("refuted") is False else "?")
            print(f"         {mark} {v.get('reason','')[:110]}")
        print()
    print(f"battery: {hits}/{len(BATTERY)} matched expected verdict")


if __name__ == "__main__":
    main()
