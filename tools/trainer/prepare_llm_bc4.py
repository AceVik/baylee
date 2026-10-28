"""Freeze the earliest technically admitted collector games for explicit LLM BC.

This copies evidence without changing collector receipts or selecting by wins,
strategy or model preference. The native --llm-bc converter independently
replays the games and reconstructs executed-answer provenance before training.
"""
from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path


class InsufficientGames(ValueError):
    """The collector has not yet admitted the requested prefix."""


def digest(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def write_json(path: Path, value) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def select_games(collector: Path, count: int):
    if count < 1:
        raise ValueError("A positive game count is required")
    selected = []
    for path in sorted((collector / "attempts").glob("attempt-*/receipt.json")):
        raw = path.read_bytes()
        receipt = json.loads(raw)
        if receipt.get("status") == "admitted":
            selected.append((path, raw, receipt))
            if len(selected) == count:
                return selected
    raise InsufficientGames(f"Need {count} complete admitted games; found {len(selected)}")


def prepare(collector: Path, out: Path, count: int) -> dict:
    collector, out = collector.resolve(), out.resolve()
    selected = select_games(collector, count)
    staging = out.with_name(out.name + ".partial")
    if out.exists() or staging.exists():
        raise ValueError("Output or retained partial output already exists")
    staging.mkdir(parents=True)
    for name in ("records", "llm", "evidence"):
        (staging / name).mkdir()
    rows, sources, seen_records = [], [], set()

    def copy_bytes(raw: bytes, relative: Path) -> dict:
        destination = staging / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        with destination.open("xb") as stream:
            stream.write(raw)
        return {"path": str(out / relative), "sha256": digest(raw)}

    for receipt_path, receipt_raw, receipt in selected:
        attempt = receipt_path.parent
        game = attempt.name
        request_raw = (attempt / "request.json").read_bytes()
        technical_raw = (attempt / "technical-result.json").read_bytes()
        request, technical = json.loads(request_raw), json.loads(technical_raw)
        if request != receipt.get("request") or technical != receipt.get("technical"):
            raise ValueError(f"Receipt does not bind exact request/result: {game}")
        if request.get("attempt_id") != game or technical.get("status") != "complete":
            raise ValueError(f"Wrong attempt or incomplete result: {game}")
        if technical.get("bc_only") is not True or technical.get("ppo_eligible") is not False:
            raise ValueError("LLM collection must be BC-only")
        if any(technical.get("checks", {}).get(k) is not True for k in
               ("connection", "view_boundary", "full_record", "replay", "provenance")):
            raise ValueError(f"Incomplete technical checks: {game}")
        models = request.get("models")
        if not isinstance(models, list) or len(models) != 2:
            raise ValueError("Two declared model seats are required")
        artifacts, pins = {}, {}
        for artifact in technical["artifacts"]:
            path = Path(artifact["path"]).resolve()
            role = artifact["role"]
            if not path.is_relative_to(attempt.resolve()) or role in artifacts:
                raise ValueError("Escaping artifact or duplicate evidence role")
            if not role.replace("_", "").isalnum():
                raise ValueError("Invalid artifact role")
            raw = path.read_bytes()
            if digest(raw) != artifact["sha256"]:
                raise ValueError(f"Artifact bytes changed: {game}/{role}")
            artifacts[role] = raw
            relative = Path("evidence") / game / (role + "".join(path.suffixes))
            pins[role] = copy_bytes(raw, relative)
        compressed = artifacts["anonymized_record"]
        if digest(compressed) != technical["record_sha256"]:
            raise ValueError("Record identity differs from admitted result")
        raw_record = gzip.decompress(compressed)
        record_hash = digest(raw_record)
        if record_hash in seen_records:
            raise ValueError("Duplicate complete game in collector history")
        seen_records.add(record_hash)
        record_rows = [json.loads(line) for line in raw_record.splitlines() if line.strip()]
        if record_rows[0].get("kind") != "header" or record_rows[-1].get("kind") != "end":
            raise ValueError("Record lacks complete header/end")
        seed = technical["actual_seed"]
        if type(seed) is not int or record_rows[0]["preset"]["seed"] != seed:
            raise ValueError("Actual server seed mismatch")
        terminal = technical["terminal"]
        expected_winners = [] if terminal["draw"] else [terminal["winner_seat"]]
        if record_rows[-1]["winners"] != expected_winners:
            raise ValueError("Terminal result mismatch")
        shard = game + ".jsonl.gz"
        copy_bytes(compressed, Path("records") / shard)
        evidence = Path("evidence") / game
        manifest = dict(
            schema_version=1, contract="admitted_executed_llm_v1", source_game=game,
            record_sha256=record_hash, record_artifact_sha256=digest(compressed),
            request=copy_bytes(request_raw, evidence / "request.json"),
            admitted_receipt=copy_bytes(receipt_raw, evidence / "receipt.json"),
            technical_result=copy_bytes(technical_raw, evidence / "technical-result.json"),
            seats=[dict(seat=s, model=models[s], transcript=pins[f"transcript_seat{s}"],
                        origins=pins[f"executed_provenance_seat{s}"]) for s in (0, 1)],
        )
        write_json(staging / "llm" / (game + ".json"), manifest)
        rows.append(dict(game=game, seed=seed, outcome=dict(
            kind="draw" if terminal["draw"] else "won", winner=terminal["winner_seat"]),
            decks=request["decks"], shard=shard, offset=0, len=len(compressed)))
        sources.append(dict(attempt=game, receipt_path=str(receipt_path),
                            receipt_sha256=digest(receipt_raw), record_sha256=record_hash))
        if receipt_path.read_bytes() != receipt_raw:
            raise ValueError("Collector receipt changed while freezing")
    metadata = dict(schema_version=1, llm_bc_version=1, seats=2, source_collector=str(collector),
                    games=count, selection="earliest technically admitted games, independent of outcome",
                    sources=sources, semantic_quality_gate=False, bc_only=True, ppo_eligible=False)
    write_json(staging / "run.json", metadata)
    (staging / "games.jsonl").write_text("".join(json.dumps(row, sort_keys=True) + "\n" for row in rows))
    # Failure leaves a distinct partial directory; it is never a usable run.
    os.rename(staging, out)
    return {"root": str(out), "games": count, "run_sha256": digest((out / "run.json").read_bytes())}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--collector", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--games", type=int, required=True)
    args = parser.parse_args()
    print(json.dumps(prepare(args.collector, args.out, args.games)))


if __name__ == "__main__":
    main()
