"""Publish a verified development leader for the next local playtest game.

This is playtest selection, not a claim of statistically established strength.
Only completed, comparable evaluation jobs enter selection. Never writes models.
"""
import argparse
import hashlib
import json
import math
import time
from pathlib import Path

from paired_eval4 import compare, read_verified
from run_guard4 import atomic_json

PROTOCOL_KEYS = ("against", "baseline", "seats", "shape", "teams", "deck_split",
                 "held_out_cards", "first_seed", "deal_offset", "deals",
                 "policy_seed", "temperature", "thinking_steps")


def digest(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def verified_model_inputs(value, job, hashes=None):
    """Bind recorded model inputs and the loaded graph alias to frozen bytes."""
    model = Path(value["model"]).resolve()
    hashes = {} if hashes is None else hashes
    def checked_hash(path):
        stat = path.stat()
        key = (str(path), stat.st_size, stat.st_mtime_ns, stat.st_ctime_ns)
        if key not in hashes:
            hashes[key] = digest(path)
        return hashes[key]
    recorded = {Path(path).name: checksum for path, checksum in job["inputs"].items()
                if Path(path).resolve().parent == model
                and (Path(path).name in ("net.pt", "model.onnx.json") or Path(path).suffix == ".onnx")}
    if not {"net.pt", "model.onnx.json"}.issubset(recorded):
        raise ValueError("Evaluation omitted checkpoint or metadata input hashes")
    # Verify metadata before trusting its declared graph filenames.
    for name, checksum in recorded.items():
        if checked_hash(model / name) != checksum:
            raise ValueError("Evaluated model input changed: " + name)
    metadata = json.loads((model / "model.onnx.json").read_text())
    graphs = metadata.get("graphs", {})
    if not graphs or any(Path(name).name != name for name in graphs.values()):
        raise ValueError("Evaluation metadata has invalid graph filenames")
    if not set(graphs.values()).issubset(recorded):
        raise ValueError("Evaluation omitted declared graph input hashes")
    graph = graphs.get(str(value.get("thinking_steps", 1)))
    if graph is None or checked_hash(model / "model.onnx") != recorded[graph]:
        raise ValueError("Default graph differs from the evaluated thinking graph")
    return recorded


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
    hashes = {}
    for shape in ("duels", "commander"):
        value, provenance, source, job = checked_evaluation(experiment, label, shape)
        reference, ref_provenance, _, ref_job = checked_evaluation(experiment, "initial", shape)
        verified_model_inputs(value, job, hashes)
        verified_model_inputs(reference, ref_job, hashes)
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


def confirmation_status_path(output):
    return Path(output).with_name(Path(output).name + ".confirmation-status.json")


def confirmation_assessment(experiment, model):
    """Operational veto of nonpositive completed confirmation, not significance.

    Pending confirmation and confirmation of another candidate do not veto this
    model. Raw paired jobs, frozen selection identity and evaluated hashes bind
    the result; an aggregate controller result alone is not sufficient evidence.
    """
    experiment, model = Path(experiment).resolve(), Path(model).resolve()
    selection = experiment / "confirmation-selection.json"
    if not selection.exists():
        return dict(status="pending", veto=False)
    selected = json.loads(selection.read_text())
    if Path(selected["model"]).resolve() != model:
        return dict(status="not_this_candidate", veto=False)
    ensure_jobs = [experiment / "jobs" / f"eval-confirmation-{label}-{shape}" / "job.json"
                   for shape in ("duels", "commander") for label in ("selected", "initial")]
    if not all(path.exists() for path in ensure_jobs):
        return dict(status="pending", veto=False)
    if any(json.loads(path.read_text())["status"] != "complete" for path in ensure_jobs):
        return dict(status="pending", veto=False)
    if selected.get("selected_before_confirmation") is not True:
        raise ValueError("Confirmation candidate was not fixed before confirmation")
    if digest(model / "net.pt") != selected["model_sha256"]:
        raise ValueError("Preselected confirmation checkpoint changed")
    progress = json.loads((experiment / "progress.json").read_text())
    rounds = [row for row in progress.get("rounds", []) if Path(row["model"]).resolve() == model]
    development_label = f"round-{rounds[0]['round']:02d}" if rounds else "initial"
    evidence, differences, hashes = {}, [], {}
    for shape in ("duels", "commander"):
        value, provenance, candidate, job = checked_evaluation(experiment, "confirmation-selected", shape)
        reference, ref_provenance, baseline, ref_job = checked_evaluation(experiment, "confirmation-initial", shape)
        initial, _, _, initial_job = checked_evaluation(experiment, "initial", shape)
        development, _, _, development_job = checked_evaluation(experiment, development_label, shape)
        if Path(reference["model"]).resolve() != Path(initial["model"]).resolve():
            raise ValueError("Confirmation reference differs from the original initial model")
        if Path(development["model"]).resolve() != model:
            raise ValueError("Confirmation candidate differs from its development model")
        for evaluated, completed_job in ((value, job), (reference, ref_job),
                                          (initial, initial_job), (development, development_job)):
            verified_model_inputs(evaluated, completed_job, hashes)
        if Path(value["model"]).resolve() != model or job["inputs"].get(str(model / "net.pt")) != selected["model_sha256"]:
            raise ValueError("Confirmation evaluated a different candidate checkpoint")
        paired = compare(value, reference, provenance, ref_provenance, resamples=128)
        delta = paired["paired_uncertainty"]["paired_utility_difference"]
        if not math.isfinite(delta):
            raise ValueError("Nonfinite confirmation difference")
        differences.append(delta)
        evidence[shape] = dict(candidate=candidate, reference=baseline, paired_difference=delta,
                               outcomes=paired["outcomes"])
    mean = sum(differences) / len(differences)
    return dict(status="nonpositive_confirmation" if mean <= 0 else "positive_point_estimate",
                veto=mean <= 0, mean_paired_utility_difference=mean,
                selected_model_sha256=selected["model_sha256"], evidence=evidence,
                rule="Conservative operational veto when completed equal-format confirmation mean is nonpositive; no significance threshold or established-strength claim.",
                scope="Independent seeds on development deck families; experimental preview only. No automatic rollback or interruption of a running game.")


def publish(experiment, output):
    output = Path(output)
    progress = json.loads((Path(experiment) / "progress.json").read_text())
    current = json.loads(output.read_text()) if output.exists() else None
    model = str(Path(progress["best_development_candidate"]).resolve())
    confirmation = confirmation_assessment(experiment, model)
    current_confirmation = (confirmation if current and current["model"] == model else
                            confirmation_assessment(experiment, current["model"]) if current else None)
    report = dict(checked_unix=time.time(), candidate=model, current_model=current["model"] if current else None,
                  confirmation=confirmation, current_confirmation=current_confirmation,
                  review_required=confirmation["veto"] or bool(current_confirmation and current_confirmation["veto"]),
                  updated=False, selection_unchanged=True, scope="experimental local preview; not strength promotion")
    def decision(reason):
        report["decision"] = reason
        atomic_json(confirmation_status_path(output), report)
    if confirmation["veto"]:
        decision("confirmation_veto_existing_selection_requires_review" if current and current["model"] == model
                 else "confirmation_veto_candidate_not_published")
        return False
    # Still inspect newly completed confirmation before this stable-leader
    # shortcut. An already selected failed candidate is reported, never silently
    # described as confirmed or forcibly rolled back during a human game.
    if current and current["model"] == model:
        decision("unchanged_current_selection_requires_review" if report["review_required"] else "unchanged")
        return False
    candidate = manifest(experiment)
    if current:
        if current["benchmark_id"] != candidate["benchmark_id"]:
            raise ValueError("Different benchmark; requires a comparable baseline before switching")
        if candidate["score"] <= current["score"]:
            decision("not_a_higher_development_score")
            return False
    candidate["confirmation"] = confirmation
    atomic_json(output, candidate)
    report.update(updated=True, selection_unchanged=False, review_required=False)
    decision("published_experimental_development_preview")
    return True


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--experiment", required=True)
    parser.add_argument("--out", required=True)
    args = parser.parse_args()
    print(json.dumps(dict(updated=publish(args.experiment, args.out))))


if __name__ == "__main__":
    main()
