"""CPU producer: prepare at most two immutable complete-game packages ahead.

The streaming converter publishes each verified game atomically. This process
groups only finished training games, prepares compressed random-access caches,
then publishes ready.json. GPU consumers mark consumed.json after completion.
Validation remains an independent fixed dataset and is never put in a package.
"""
from __future__ import annotations
import argparse
from collections import defaultdict, deque
from fractions import Fraction
import hashlib
import json
import os
from pathlib import Path
import time

from recover_packages4 import verify_rows
from run_guard4 import atomic_json
from trajectory_store4 import TrajectoryStore4


def select_entries(pending, frame_limit, seed):
    """Mix source runs by their available frame share, keeping whole games.

    Sorting all ready games by ID reintroduces source-order blocks once the CPU
    is ahead: whole duel packages followed by whole eight-seat packages. Balance
    the fraction consumed from each source instead; seeded game order removes
    within-source ordering. Readiness can affect the available mix, never labels.
    """
    groups = defaultdict(list)
    for entry in pending:
        if entry["steps"] <= 0:
            raise ValueError("Only nonempty completed games can enter a package")
        groups[str(Path(entry["source_shard"]).parent.parent)].append(entry)
    order = lambda key: hashlib.sha256(f"{seed}:{key}".encode()).digest()
    queues = {key: deque(sorted(rows, key=lambda row: order(row["game_id"])))
              for key, rows in groups.items()}
    available = {key: sum(row["steps"] for row in rows) for key, rows in groups.items()}
    consumed = {key: 0 for key in groups}
    selected, frames = [], 0
    while queues and frames < frame_limit:
        # Compare virtual finish times, including the next whole game's size.
        # Comparing only work already consumed would front-load every long game.
        source = min(queues, key=lambda key: (
            Fraction(consumed[key] + queues[key][0]["steps"], available[key]), order(key)))
        entry = queues[source].popleft()
        selected.append(entry)
        frames += entry["steps"]
        consumed[source] += entry["steps"]
        if not queues[source]:
            del queues[source]
    return selected


def publish(source, output, entries, header):
    output.mkdir(exist_ok=False)
    (output/"train").mkdir()
    src_sidecar = source / ".shard-cache-v3"
    dst_sidecar = output / ".shard-cache-v3"
    if src_sidecar.exists():
        dst_sidecar.mkdir(exist_ok=True)
    for entry in entries:
        name = entry["shard"]
        if Path(name).name != name or entry["split"] != "train":
            raise ValueError("Only local training game shards may enter a package")
        if (entry["structure_sha256"] != header["structure_sha256"] or
                not entry.get("replay_hashes_checked") or entry["unencoded_decisions"]):
            raise ValueError("Unverified game or incompatible rule catalog")
        os.link(source/"train"/name, output/"train"/name)
        if src_sidecar.exists():
            for ext in (".bin", ".idx", ".meta"):
                sp = src_sidecar / (name + ext)
                if sp.exists():
                    os.link(sp, dst_sidecar / (name + ext))
    for name in ("card_structure.json","card_dsl.bin","card_dsl_back.bin","token_dsl.bin","card_abilities.bin"):
        os.link(source/name, output/name)
    ids = [entry["game_id"] for entry in entries]
    if len(set(ids)) != len(ids): raise ValueError("Duplicate game in package")
    meta = {**header, "train_shards":[e["shard"] for e in entries], "val_shards":[],
            "train_steps":sum(e["steps"] for e in entries), "val_steps":0,
            "games_requested":len(entries), "games_converted":len(entries), "games_rejected":0,
            "unencoded_decisions":0, "coverage_matrix":None,
            "source_game_ids":ids, "source_dataset":str(source), "package_sources":entries,
            "validation_runs":[]}
    atomic_json(output/"meta.json",meta)
    store = TrajectoryStore4(output,"train",meta["train_shards"])
    if len(store.index) != meta["train_steps"] or verify_rows(store.index,set(ids)) != set(ids):
        raise ValueError("Package count or trajectory mismatch")
    store.close()
    atomic_json(output/"ready.json",{"games":len(ids),"frames":meta["train_steps"],
                "metadata_sha256":hashlib.sha256((output/"meta.json").read_bytes()).hexdigest()})
    print(json.dumps({"ready":str(output),"games":len(ids),"frames":meta["train_steps"]}),flush=True)


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--source",type=Path,required=True)
    ap.add_argument("--source-job",type=Path,required=True)
    ap.add_argument("--out",type=Path,required=True)
    ap.add_argument("--validation-data",type=Path,required=True)
    ap.add_argument("--frames",type=int,default=500000)
    ap.add_argument("--ahead",type=int,default=2)
    ap.add_argument("--seed",type=int,default=2718,help="Deterministic source-balanced package mixing")
    ap.add_argument("--timeout",type=float,default=21600)
    ap.add_argument("--allow-replay-quarantined",action="store_true",
                    help="Allow historical records quarantined by replay audit divergence when unencoded_decisions == 0")
    args=ap.parse_args()
    if args.ahead<1 or args.frames<1: ap.error("Positive queue/size limits are required")
    args.out.mkdir(exist_ok=True)
    deadline=time.monotonic()+args.timeout
    while not (args.source/"ready").exists():
        if time.monotonic()>deadline: raise TimeoutError("Converter did not publish its header")
        time.sleep(2)
    header=json.loads((args.source/"dataset_header.json").read_text())
    validation=json.loads((args.validation_data/"meta.json").read_text())
    if header["structure_sha256"] != validation["structure_sha256"]:
        raise ValueError("Validation uses a different catalog")
    if set(header["source_runs"]) & set(validation["validation_runs"]):
        raise ValueError("Training source overlaps fixed validation runs")
    while True:
        if time.monotonic()>deadline: raise TimeoutError("Package producer deadline")
        source_job=json.loads(args.source_job.read_text())
        if source_job["status"] not in ("starting","running","complete"):
            raise RuntimeError("Converter failed; preserve prepared packages and inspect its logs")
        existing=sorted(args.out.glob("package-*"))
        used=set()
        for folder in existing:
            if not (folder/"ready.json").exists():
                raise ValueError(f"Preserving incomplete package: {folder}")
            used.update(json.loads((folder/"meta.json").read_text())["source_game_ids"])
        queued=sum(not (p/"consumed.json").exists() for p in existing)
        entries=[json.loads(p.read_text()) for p in sorted((args.source/"ready").glob("*.json"))]
        pending=[e for e in entries if e["split"]=="train" and e["game_id"] not in used]
        complete=source_job["status"]=="complete"
        if complete:
            final=json.loads((args.source/"meta.json").read_text())
            if final["unencoded_decisions"]:
                raise ValueError("Review converter unencoded decisions before continuing")
            if final["games_rejected"]:
                if not args.allow_replay_quarantined:
                    raise ValueError("Review converter rejections before continuing")
                rejected=json.loads((args.source/"rejected-games.json").read_text())
                if any(not str(r.get("reason","")).startswith("replay audit: Diverged") for r in rejected):
                    raise ValueError("Non-divergence converter rejection encountered")
        if not pending and complete:
            atomic_json(args.out/"complete.json",{"packages":len(existing),"games":len(used)})
            return
        selected=select_entries(pending,args.frames,args.seed+len(existing))
        frames=sum(entry["steps"] for entry in selected)
        if queued<args.ahead and (frames>=args.frames or complete and selected):
            package_header={**header,"package_mixing":{"version":1,"seed":args.seed+len(existing),
                "strategy":"available frame share per source run; complete games; seeded within-source order"}}
            publish(args.source,args.out/f"package-{len(existing):04}",selected,package_header)
        else:time.sleep(2)


if __name__=="__main__":main()
