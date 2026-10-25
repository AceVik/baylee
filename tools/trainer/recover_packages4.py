"""Publish immutable packages from closed shards of a memory-limited replay.

Only an explicitly quarantined memory-limit attempt is accepted. Its executable
must match a completed template conversion. Gzip CRC, complete per-seat cursors,
terminal markers and original train/validation assignments are verified. The
last shard remains excluded because it may not have been closed. No original
file or failure manifest is changed; excluded work remains in the replay queue.
"""
from __future__ import annotations
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import shutil

import numpy as np
from run_guard4 import atomic_json
from trajectory_store4 import INDEX, build_part


def verify_rows(rows, allowed_games):
    if not len(rows):
        raise ValueError("Empty shard")
    order = np.lexsort((rows["cursor"], rows["seat"], rows["game"]))
    ordered = rows[order]
    cuts = np.flatnonzero((ordered["game"][1:] != ordered["game"][:-1]) |
                          (ordered["seat"][1:] != ordered["seat"][:-1])) + 1
    for trajectory in np.split(ordered, cuts):
        if int(trajectory[0]["game"]) not in allowed_games:
            raise ValueError("Validation or unknown game in a training package")
        if not np.array_equal(trajectory["cursor"], np.arange(len(trajectory))):
            raise ValueError("Incomplete/duplicate trajectory cursors")
        terminal = (trajectory["flags"] & 2) != 0
        if not terminal[-1] or terminal[:-1].any():
            raise ValueError("Incomplete trajectory terminal markers")
    return set(map(int, rows["game"]))


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--failed-job", type=Path, required=True)
    ap.add_argument("--template-job", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--shards", type=int, default=8)
    ap.add_argument("--max-shard-mib", type=int, default=100)
    args = ap.parse_args()
    failed = json.loads(args.failed_job.read_text())
    template = json.loads(args.template_job.read_text())
    if (failed["status"], failed["reason"], template["status"]) != ("quarantined", "memory_limit", "complete"):
        raise ValueError("Expected a memory-limited attempt and completed template")
    if failed["executable_sha256"] != template["executable_sha256"]:
        raise ValueError("Conversion executable differs from template")
    diagnostics = args.failed_job.with_name("stderr.log").read_text()
    if "UNENCODED" in diagnostics or "quarantining game" in diagnostics:
        raise ValueError("Review replay/encoding failures before publishing any recovered package")
    command = failed["command"]
    source = Path(command[command.index("--out") + 1])
    base = Path(template["command"][template["command"].index("--out") + 1])
    train_runs = command[command.index("--runs") + 1].split(",")
    val_runs = command[command.index("--val-runs") + 1].split(",")
    if set(train_runs) & set(val_runs):
        raise ValueError("Overlapping source splits")
    allowed_games = set()
    provenance = {}
    gid = 0
    for run in train_runs + val_runs:
        for line in (Path(run) / "games.jsonl").read_text().splitlines():
            if not line.strip(): continue
            game = json.loads(line)
            if game["outcome"]["kind"] not in ("won", "team_won", "draw"): continue
            gid += 1
            if run in train_runs:
                allowed_games.add(gid)
                provenance[gid] = {"run": run, "seed": game.get("seed"), "shard": game["shard"],
                                   "offset": game["offset"], "len": game["len"]}
    all_shards = sorted(list((source / "train").glob("*.bst4")) + list((source / "train").glob("*.jsonl.gz")))
    candidates = [p for p in all_shards[:-1] if p.stat().st_size <= args.max_shard_mib * 2**20]
    # Spread the quick first package across the source order; do not consume
    # only the first duel shards while later multiplayer records are ready.
    selected = [candidates[i] for i in np.linspace(0, len(candidates)-1, min(args.shards, len(candidates)), dtype=int)]
    if not selected: raise ValueError("No complete candidate shards")
    args.out.mkdir(exist_ok=False)
    train = args.out / "train"; train.mkdir()
    cache = args.out / ".trajectory-cache-v2"; cache.mkdir()
    parts = [(p, cache/f"part-{i}.bin", cache/f"part-{i}.idx") for i,p in enumerate(selected)]
    with ThreadPoolExecutor(max_workers=4) as pool:
        futures = [pool.submit(build_part, *part) for part in parts]
        for part, future in zip(parts, futures):
            print(f"verified gzip {part[0].name}: {future.result()} frames", flush=True)
    games = set(); steps = 0
    for path, _, index in parts:
        rows = np.fromfile(index, dtype=INDEX)
        local = verify_rows(rows, allowed_games)
        if games & local: raise ValueError("A game spans shards or appears twice")
        games |= local; steps += len(rows)
        os.link(path, train/path.name)
    meta = json.loads((base/"meta.json").read_text())
    for name in ("card_structure.json", "card_dsl.bin", "card_dsl_back.bin", "token_dsl.bin", "card_abilities.bin"):
        os.link(base/name, args.out/name)
    meta.update(train_shards=[p.name for p in selected], val_shards=[], train_steps=steps, val_steps=0,
                games_requested=len(games), games_converted=len(games), games_rejected=0,
                source_runs=train_runs, validation_runs=[], unencoded_decisions=0,
                coverage_matrix=None, recovery={"source_job": str(args.failed_job),
                    "source_job_sha256": hashlib.sha256(args.failed_job.read_bytes()).hexdigest(),
                    "template_job": str(args.template_job), "game_sources": [provenance[g] for g in sorted(games)],
                    "excluded_shards": [p.name for p in all_shards if p not in selected],
                    "scope": "Complete checked training subset; use separate immutable validation; not the full corpus"})
    # Identical cache identity/format to TrajectoryStore4, so training starts
    # immediately without decompressing the verified package a second time.
    meta_bytes = json.dumps(meta, indent=2).encode()
    identity = hashlib.sha256(meta_bytes)
    for p in selected:
        stat = (train/p.name).stat(); identity.update(f"{p.name}:{stat.st_size}:{stat.st_mtime_ns}".encode())
    fingerprint = identity.hexdigest(); stem = f"train-{fingerprint[:24]}"
    with (cache/f"{stem}.bin").open("wb") as data, (cache/f"{stem}.idx").open("wb") as index:
        for _, part_data, part_index in parts:
            rows = np.fromfile(part_index, dtype=INDEX); rows["offset"] += data.tell(); index.write(rows.tobytes())
            with part_data.open("rb") as inp: shutil.copyfileobj(inp,data,1<<20)
            part_data.unlink(); part_index.unlink()
    atomic_json(cache/f"{stem}.json", {"fingerprint":fingerprint,"records":steps})
    (args.out/"meta.json.partial").write_bytes(meta_bytes)
    os.replace(args.out/"meta.json.partial",args.out/"meta.json")
    print(json.dumps({"status":"ready","games":len(games),"frames":steps,"path":str(args.out)}),flush=True)


if __name__ == "__main__": main()
