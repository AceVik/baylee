"""Local playtest maintenance: publish a measured leader and archive completed games.

Run beside seat4; never restarts a game or modifies a checkpoint. Training reads
only the gateway's anonymized export, never its database. DATABASE_URL is passed
solely to the official exporter, from the service environment.
"""
import argparse
import json
import os
from pathlib import Path
import signal
import subprocess
import tempfile
import time

from publish_live_model4 import confirmation_status_path, publish
from run_guard4 import atomic_json


def collect(args):
    from import_live_records4 import import_records
    with tempfile.TemporaryDirectory(prefix="baylee-live-export-") as directory:
        subprocess.run([args.gateway_bin, "records", "export", "--out", directory],
                       check=True, timeout=120, stdout=subprocess.DEVNULL)
        return import_records(Path(directory), Path(args.corpus), args.neural_seat)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--experiment", required=True)
    parser.add_argument("--selection", required=True)
    parser.add_argument("--gateway-bin", required=True)
    parser.add_argument("--corpus", required=True)
    parser.add_argument("--neural-seat", type=int, default=0)
    parser.add_argument("--status", required=True)
    args = parser.parse_args()
    if not os.environ.get("DATABASE_URL"):
        raise ValueError("Official export needs DATABASE_URL in the service environment")
    stopped = False

    def stop(_number, _frame):
        nonlocal stopped
        stopped = True

    signal.signal(signal.SIGTERM, stop)
    signal.signal(signal.SIGINT, stop)
    next_selection = next_collection = 0.0
    state = dict(phase="watching", pid=os.getpid(), errors={},
                 selection=args.selection, corpus=args.corpus)
    while not stopped:
        now = time.monotonic()
        if now >= next_selection:
            try:
                if publish(args.experiment, args.selection):
                    state["selection_updated_unix"] = time.time()
                confirmation = confirmation_status_path(args.selection)
                state["selection_review"] = (json.loads(confirmation.read_text())
                                             if confirmation.exists() else None)
                state["errors"].pop("selection", None)
            except Exception as error:
                state["errors"]["selection"] = str(error)
            next_selection = now + 30
        if now >= next_collection:
            try:
                state["collection"] = collect(args)
                state["collection_checked_unix"] = time.time()
                state["errors"].pop("collection", None)
            except Exception as error:
                state["errors"]["collection"] = str(error)
            next_collection = now + 300
        state["checked_unix"] = time.time()
        atomic_json(Path(args.status), state)
        time.sleep(1)
    state["phase"] = "stopped"
    atomic_json(Path(args.status), state)


if __name__ == "__main__":
    main()
