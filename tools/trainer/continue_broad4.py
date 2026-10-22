"""Queue one bounded broad-data experiment after the initial pipeline releases resources.

Uses the initial supervised checkpoint as the conservative starting point.
No automatic model promotion; all experiments and failures remain separate.
"""
import argparse
import json
from pathlib import Path
import sys
import time
import subprocess

from run_guard4 import atomic_json


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--initial", required=True)
    ap.add_argument("--corpus", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--manifest", required=True)
    args = ap.parse_args()
    initial, corpus, out = map(lambda p: Path(p).resolve(), (args.initial, args.corpus, args.out))
    out.mkdir(parents=True, exist_ok=False)
    deadline = time.monotonic() + 14 * 3600
    while True:
        jobs = [json.loads(p.read_text()) for p in (initial / "jobs").glob("*/job.json")]
        active = any(job["status"] in ("starting", "running") for job in jobs)
        finished = (initial / "result.json").exists()
        failed = any(job["status"] == "quarantined" for job in jobs)
        if not active and (finished or failed):
            break
        if time.monotonic() >= deadline:
            atomic_json(out / "queue.json", {"status": "quarantined", "reason": "initial pipeline did not finish"})
            return
        atomic_json(out / "queue.json", {"status": "waiting_for_initial_pipeline", "initial": str(initial)})
        time.sleep(30)
    if not (corpus / "result.json").exists() or not (initial / "bc" / "net.pt").exists():
        atomic_json(out / "queue.json", {"status": "quarantined", "reason": "No validated corpus or BC checkpoint"})
        return
    dataset = json.loads((corpus / "result.json").read_text())["dataset"]
    command = [sys.executable, "tools/trainer/pipeline4.py", "--data", dataset,
               "--out", str(out / "experiment"), "--manifest", args.manifest,
               "--init", str(initial / "bc"), "--hours", "1", "--epochs", "2", "--supervised-only"]
    atomic_json(out / "queue.json", {"status": "running", "command": command})
    # The pipeline supervises each stage in its own process group. Do not
    # wrap that supervisor in another group deadline that could kill it while
    # leaving its deliberately independent training child running.
    with (out / "pipeline.log").open("wb") as log:
        result = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=False).returncode
    atomic_json(out / "queue.json", {"status": "complete" if result == 0 else "quarantined"})


if __name__ == "__main__":
    main()
