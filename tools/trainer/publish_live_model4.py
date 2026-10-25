"""Publish a verified development leader for the next local playtest game.

This is playtest selection, not a claim of statistically established strength.
Only completed, comparable evaluation jobs enter selection. Never writes models.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path

from paired_eval4 import compare, read_verified
from run_guard4 import atomic_json

PROTOCOL_KEYS = ("against", "baseline", "seats", "shape", "teams", "deck_split",
                 "held_out_cards", "first_seed", "deal_offset", "deals",
                 "policy_seed", "temperature", "thinking_steps")


def digest(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def checked_evaluation(experiment, label, shape):
    path = experiment / "evaluation" / f"{label}-{shape}.json"
    job = experiment / "jobs" / f"eval-{label}-{shape}" / "job.json"
    value, provenance, checksum = read_verified(path, job)
    if value.get("refused", 0) or value.get("net_answer_caps", 0):
        raise ValueError("Refusals or capped games require review before live selection")
    # Recovery counters are already present in the reviewed starting model;
    # expose them rather than representing the preview as pure neural play.
    quality = {key: value.get(key) for key in ("stalls", "house_fallbacks", "fallback_rate")}
    return value, provenance, dict(path=str(path), sha256=checksum, job=str(job), quality=quality), json.loads(job.read_text())


def measured_candidate(experiment, progress):
    model = Path(progress["best_development_candidate"]).resolve()
    selected = [row for row in progress["rounds"] if Path(row["model"]).resolve() == model]
    label = f"round-{selected[0]['round']:02d}" if selected else "initial"
    benchmark, evidence, scores, jobs = {}, {}, [], []
    for shape in ("duels", "commander"):
        value, provenance, source, job = checked_evaluation(experiment, label, shape)
        reference, ref_provenance, _, _ = checked_evaluation(experiment, "initial", shape)
        # Verifies every deal, seat rotation and repeated house baseline, not
        # merely the number of wins or an aggregate written by the controller.
        compare(value, reference, provenance, ref_provenance, resamples=128)
        if Path(value["model"]).resolve() != model:
            raise ValueError("Selected checkpoint differs from evaluation")
        scores.append(sum(row["net_utility"] for row in value["deal_details"]) / value["deals"])
        benchmark[shape] = dict(provenance=provenance,
                               protocol={key: value[key] for key in PROTOCOL_KEYS},
                               deals=sorted((r["table"], r["seed"], r["decks"], r["focal_seat"],
                                             r["base_utility"], r["base_capped"])
                                            for r in value["deal_details"]))
        evidence[shape] = source
        jobs.append(job)
    score = sum(scores) / len(scores)
    if not math.isfinite(score) or not math.isclose(score, progress["best_development_score"], abs_tol=1e-12):
        raise ValueError("Controller rank disagrees with measured utility")
    return model, label, score, benchmark, evidence, jobs


def manifest(experiment):
    experiment = Path(experiment).resolve()
    progress = json.loads((experiment / "progress.json").read_text())
    model, label, score, benchmark, evidence, jobs = measured_candidate(experiment, progress)
    metadata = json.loads((model / "model.onnx.json").read_text())
    parity = metadata.get("parity_max_abs_diff", float("nan"))
    if not math.isfinite(parity) or not 0 <= parity <= .001 or metadata.get("checked_frames", 0) < 1:
        raise ValueError("No successful numerical export parity check")
    names = {"net.pt", "model.onnx.json", "model.onnx", "card_structure.json"}
    names.update(metadata["graphs"].values())
    names.update(("model.k0.onnx", "model.k1.onnx"))
    if any(Path(name).name != name for name in names):
        raise ValueError("Model files must be direct children of the frozen directory")
    hashes = {name: digest(model / name) for name in sorted(names)}
    if hashes["model.onnx"] != hashes[metadata["graphs"]["1"]]:
        raise ValueError("Default graph is not the evaluated K=1 graph")
    for job in jobs:
        for name in names - {"model.onnx", "card_structure.json"}:
            if job["inputs"].get(str(model / name)) != hashes[name]:
                raise ValueError("Checkpoint changed since its completed comparison: " + name)
    return dict(schema_version=1, model=str(model), label=label.replace("round-", "v4-R"),
                score=score, benchmark_id=hashlib.sha256(json.dumps(benchmark, sort_keys=True).encode()).hexdigest(),
                sha256=hashes, evidence=evidence, source_selection=str(experiment / "progress.json"),
                scope="development leader for local playtests; not a strength promotion")


def publish(experiment, output):
    output = Path(output)
    # A stable progress leader needs no repeated multi-GB hashing on a busy PC.
    progress = json.loads((Path(experiment) / "progress.json").read_text())
    current = json.loads(output.read_text()) if output.exists() else None
    if current and current["model"] == str(Path(progress["best_development_candidate"]).resolve()):
        return False
    candidate = manifest(experiment)
    if current:
        if current["benchmark_id"] != candidate["benchmark_id"]:
            raise ValueError("Different benchmark; requires a comparable baseline before switching")
        if candidate["score"] <= current["score"]:
            return False
    atomic_json(output, candidate)
    return True


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--experiment", required=True)
    parser.add_argument("--out", required=True)
    args = parser.parse_args()
    print(json.dumps(dict(updated=publish(args.experiment, args.out))))


if __name__ == "__main__":
    main()
