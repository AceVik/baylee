"""Compare policies on identical deals against fixed opponents, not head-to-head.

Losing the same deal is not evidence that it was impossible to win. Every deal
remains in the aggregate; capped games remain unknown. Uncertainty is clustered
by whole tables, keeping correlated focal-seat rotations together.
"""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path

from run_guard4 import atomic_json
from summarize_eval4 import summarize


def read_verified(evaluation, job):
    path = Path(evaluation)
    value = json.loads(path.read_text())
    run = json.loads(Path(job).read_text())
    if run["status"] != "complete":
        raise ValueError("Only completed evaluation jobs can be compared")
    # eval4's scalar/list options each occupy one argument; bool flags have none.
    command = iter(run["command"][1:])
    options = {}
    for flag in command:
        options[flag] = True if flag in ("--legacy-v3", "--fair-scouting", "--no-baseline", "--held-out-cards") else next(command)
    if Path(options["--out"]).resolve() != path.resolve() or options["--model"] != value["model"]:
        raise ValueError("Evaluation does not belong to the supplied job")
    manifest = value["decks_manifest"]
    if not manifest or manifest != options.get("--decks-manifest"):
        raise ValueError("A frozen deck manifest is required for verified comparisons")
    provenance = {"engine_sha256": run["executable_sha256"],
                  "deck_manifest_sha256": run["inputs"][manifest],
                  "max_decisions": int(options.get("--max-decisions", 5000)),
                  "fair_scouting": bool(options.get("--fair-scouting", True)),
                  "holdout_tier": options.get("--holdout-tier", ""),
                  "generated": int(options.get("--generated", 32))}
    return value, provenance, hashlib.sha256(path.read_bytes()).hexdigest()


def compare(candidate, reference, candidate_provenance, reference_provenance, resamples=20000,
            allow_thinking_difference=False):
    required = {"engine_sha256", "deck_manifest_sha256", "max_decisions", "fair_scouting", "holdout_tier", "generated"}
    if not required.issubset(candidate_provenance) or candidate_provenance != reference_provenance:
        raise ValueError("Engine, deck content and evaluation limits must match")
    for key in ("against", "baseline", "seats", "shape", "teams", "deck_split", "held_out_cards",
                "first_seed", "deal_offset", "deals", "policy_seed", "temperature", "thinking_steps"):
        if key not in candidate or key not in reference:
            raise ValueError(f"Evaluation protocol is missing: {key}")
        if key == "thinking_steps" and allow_thinking_difference:
            continue
        if candidate[key] != reference[key]:
            raise ValueError(f"Evaluation protocol differs or is missing: {key}")
    if candidate.get("no_baseline") or reference.get("no_baseline"):
        raise ValueError("House duplicates are required to audit matching outcomes")

    def index(value):
        result = {}
        for row in value["deal_details"]:
            key = (row["table"], row["seed"], tuple(row["decks"]), row["focal_seat"])
            if key in result:
                raise ValueError("Duplicate table/seed/decks/focal-seat case")
            result[key] = row
        if len(result) != value["deals"]:
            raise ValueError("Evaluation is missing declared deals")
        return result

    left, right = index(candidate), index(reference)
    if left.keys() != right.keys():
        raise ValueError("Comparisons require exactly the same deals; no silent intersection")
    counts = Counter({k: 0 for k in ("better", "equal", "worse", "unresolved_cap", "both_lose", "both_win")})
    pairs = []
    for key in sorted(left):
        a = left[key]
        b = right[key]
        if (a["base_utility"], a["base_capped"]) != (b["base_utility"], b["base_capped"]):
            raise ValueError("Repeated house baseline differs on an identical deal")
        pairs.append(dict(a, base_utility=b["net_utility"], base_capped=b["net_capped"]))
        if a["net_capped"] or b["net_capped"]:
            counts["unresolved_cap"] += 1
            continue
        x, y = a["net_utility"], b["net_utility"]
        counts["better" if x > y else "worse" if x < y else "equal"] += 1
        counts["both_lose"] += int(x == y == 0)
        counts["both_win"] += int(x == y == 1)
    return {"candidate": candidate["model"], "reference": reference["model"],
            "thinking_steps": {"candidate": candidate["thinking_steps"], "reference": reference["thinking_steps"]},
            "different_thinking_budgets_allowed": allow_thinking_difference,
            "provenance": candidate_provenance, "outcomes": dict(counts),
            "execution_quality": {label: {key: value.get(key) for key in
                ("model_family", "house_fallbacks", "fallback_rate", "refused", "stalls", "net_answer_caps")}
                for label, value in (("candidate", candidate), ("reference", reference))},
            "paired_uncertainty": summarize({"deal_details": pairs}, resamples=resamples),
            "scope": "Same start/decks/focal seat/opponents/engine; policy substitution against fixed opponents, not direct head-to-head",
            "luck": "Keep every deal. Both losing does not prove unavoidability; later draws may diverge after different actions.",
            "outcome_counts": "better/equal/worse/unresolved_cap partition all cases; both_lose and both_win are subsets of equal.",
            "selection": "No automatic promotion; repeated selection on this suite needs an untouched final seed/deck holdout."}


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("candidate")
    ap.add_argument("reference")
    ap.add_argument("--candidate-job", required=True)
    ap.add_argument("--reference-job", required=True)
    ap.add_argument("--allow-thinking-difference", action="store_true", help="Explicit depth experiment; report each policy's thinking budget")
    ap.add_argument("--out", required=True)
    args = ap.parse_args()
    candidate, cp, ch = read_verified(args.candidate, args.candidate_job)
    reference, rp, rh = read_verified(args.reference, args.reference_job)
    report = compare(candidate, reference, cp, rp, allow_thinking_difference=args.allow_thinking_difference)
    report["evaluation_sha256"] = {"candidate": ch, "reference": rh}
    atomic_json(Path(args.out), report)
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
