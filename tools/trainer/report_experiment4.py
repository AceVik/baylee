"""Write a local progress report and paired uncertainty for offline experiments.

Optional watch mode observes files only; it never starts training, changes a
checkpoint, retries a failed job or promotes a model.
"""
import argparse
import json
from pathlib import Path
import time

from run_guard4 import atomic_json
from summarize_eval4 import summarize


def read_json(path):
    try:
        return json.loads(path.read_text())
    except (OSError, ValueError):
        return None


def inspect(root):
    jobs = []
    for path in sorted((root / "jobs").glob("*/job.json")):
        value = read_json(path)
        if value:
            jobs.append({"stage": path.parent.name, "status": value["status"],
                         "reason": value.get("reason"), "log": str(path.parent)})
    training = root / "jobs" / "supervised" / "stdout.log"
    latest_step, validation = None, None
    if training.exists():
        for line in training.read_text(errors="replace").splitlines():
            if line.startswith("[train4] step="):
                latest_step = line
            if line.startswith("[validation4]"):
                try:
                    validation = json.loads(line[line.index("{"):])
                except ValueError:
                    pass  # A concurrently written last line may be incomplete.
    evaluations = {}
    for path in sorted((root / "evaluation").glob("*.json")):
        if path.name.endswith(".uncertainty.json"):
            continue
        value = read_json(path)
        if not value or value.get("no_baseline") or not value.get("deal_details"):
            continue
        output = path.with_suffix(".uncertainty.json")
        # Never rewrite a completed calculation on every observation tick.
        summary = read_json(output)
        if summary is None or output.stat().st_mtime < path.stat().st_mtime:
            summary = summarize(value)
            atomic_json(output, summary)
        evaluations[path.stem] = summary
    result = read_json(root / "result.json")
    active = any(j["status"] in ("starting", "running") for j in jobs)
    failed = any(j["status"] == "quarantined" for j in jobs)
    return {"root": str(root), "jobs": jobs, "latest_step": latest_step,
            "validation": validation, "evaluations": evaluations, "result": result,
            "finished": bool(not active and (result or failed))}


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--experiment", action="append", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--watch-hours", type=float, default=0)
    args = ap.parse_args()
    root = Path(args.out).resolve()
    root.mkdir(parents=True, exist_ok=True)
    deadline = time.monotonic() + args.watch_hours * 3600
    while True:
        results = [inspect(Path(p).resolve()) for p in args.experiment]
        finished = all(v["finished"] for v in results)
        expired = time.monotonic() >= deadline
        status = "complete" if finished else "one_shot" if args.watch_hours <= 0 else "deadline" if expired else "watching"
        atomic_json(root / "progress.json", {"updated_unix": time.time(), "status": status, "experiments": results})
        lines = ["# Playernet v4: lokaler Laufstatus", "",
                 time.strftime("Aktualisiert: %Y-%m-%d %H:%M:%S UTC", time.gmtime()) + f"; Bericht: {status}.", "",
                 "Automatisch aus den Experimentdateien erstellt. Keine Aussage über menschliche Spielstärke.", ""]
        for value in results:
            lines += ["## " + Path(value["root"]).name, "", "Pfad: `" + value["root"] + "`", ""]
            if value["latest_step"]:
                lines += [value["latest_step"], ""]
            if value["validation"]:
                v = value["validation"]
                lines += [f"Aktive Auswahl: {v['val_active_top1_acc']:.1%}; Brier: {v['val_mse']:.4f}; "
                          f"einfache Basis: {v['val_uniform_seat_mse']:.4f}; ECE: {v['val_ece']:.4f}.", ""]
            lines += [f"- {j['stage']}: {j['status']}" + (f" ({j['reason']})" if j['reason'] else "") for j in value["jobs"]]
            lines += [""]
            for name, ev in value["evaluations"].items():
                lines += [f"{name}: {ev['independent_tables']} unabhängige Tische; "
                          f"gepaarte Differenz {ev['paired_utility_difference']}; "
                          f"konservatives 95%-Intervall {ev['bounded_independent_table_95_percent_interval']}; "
                          f"{ev['capped_pairs']} unvollständige Paare.", ""]
        temporary = root / "progress.md.partial"
        temporary.write_text("\n".join(lines))
        temporary.replace(root / "progress.md")
        if args.watch_hours <= 0 or expired or finished:
            break
        time.sleep(30)


if __name__ == "__main__":
    main()
