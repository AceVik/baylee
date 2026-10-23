"""CPU producer: prepare at most two immutable complete-game packages ahead.

The streaming converter publishes each verified game atomically. This process
groups only finished training games, prepares compressed random-access caches,
then publishes ready.json. GPU consumers mark consumed.json after completion.
Validation remains an independent fixed dataset and is never put in a package.
"""
from __future__ import annotations
import argparse
import hashlib
import json
import os
from pathlib import Path
import time

from recover_packages4 import verify_rows
from run_guard4 import atomic_json
from trajectory_store4 import TrajectoryStore4


def publish(source, output, entries, header):
    output.mkdir(exist_ok=False)
    (output/"train").mkdir()
    for entry in entries:
        name = entry["shard"]
        if Path(name).name != name or entry["split"] != "train":
            raise ValueError("Only local training game shards may enter a package")
        if (entry["structure_sha256"] != header["structure_sha256"] or
                not entry.get("replay_hashes_checked") or entry["unencoded_decisions"]):
            raise ValueError("Unverified game or incompatible rule catalog")
        os.link(source/"train"/name, output/"train"/name)
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
    ap.add_argument("--timeout",type=float,default=21600)
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
            if final["games_rejected"] or final["unencoded_decisions"]:
                raise ValueError("Review converter rejections before continuing")
        if not pending and complete:
            atomic_json(args.out/"complete.json",{"packages":len(existing),"games":len(used)})
            return
        selected=[]; frames=0
        for entry in pending:
            selected.append(entry);frames+=entry["steps"]
            if frames>=args.frames:break
        if queued<args.ahead and (frames>=args.frames or complete and selected):
            publish(args.source,args.out/f"package-{len(existing):04}",selected,header)
        else:time.sleep(2)


if __name__=="__main__":main()
