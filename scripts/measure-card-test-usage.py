#!/usr/bin/env python3
"""Read only this experiment's Codex usage records; never export message text.

Usage: python3 scripts/measure-card-test-usage.py /tmp/baylee-luna64-start.json
Local telemetry is best-effort, not a billing statement. Cached input is a
subset of input; reasoning output is a subset of output, not an extra charge.
"""

import argparse
import json
from pathlib import Path


def collect(path, start, root_id, offset=0):
    with path.open("rb") as stream:
        meta = json.loads(stream.readline())["payload"]
        thread = meta["id"]
        source = meta.get("source", {})
        spawn = (
            source.get("subagent", {}).get("thread_spawn", {})
            if isinstance(source, dict)
            else {}
        )
        parent = meta.get("parent_thread_id", spawn.get("parent_thread_id"))
        if thread != root_id and parent != root_id:
            return None
        agent = meta.get("agent_path", spawn.get("agent_path", "/root"))
        stream.seek(offset)
        seen, totals, models, turns, timings = set(), {}, set(), {}, {}
        first = last = None
        for line in stream:
            try:
                event = json.loads(line)
            except (ValueError, UnicodeDecodeError):
                continue  # A currently appended partial record is retried next run.
            payload = event.get("payload", {})
            if (
                event.get("type") == "event_msg"
                and payload.get("type") == "task_complete"
                and event.get("timestamp", "") >= start
            ):
                timings[payload.get("turn_id", "unknown")] = {
                    key: payload[key]
                    for key in ("started_at", "completed_at", "duration_ms")
                    if key in payload
                }
            if event.get("type") == "turn_context" and payload.get("model"):
                models.add(payload["model"])
            if event.get("type") != "token_usage_record":
                continue
            stamp = event.get("timestamp", "")
            response = payload.get("response_id")
            if (
                stamp < start
                or payload.get("thread_id") != thread
                or not response
                or response in seen
            ):
                continue
            usage = payload.get("usage")
            if not isinstance(usage, dict):
                continue
            seen.add(response)
            turn_id = payload.get("turn_id", "unknown")
            turn = turns.setdefault(
                turn_id,
                {
                    "usage": {},
                    "first_usage": stamp,
                    "last_usage": stamp,
                    "responses": 0,
                },
            )
            turn["responses"] += 1
            turn["first_usage"] = min(turn["first_usage"], stamp)
            turn["last_usage"] = max(turn["last_usage"], stamp)
            for key, value in usage.items():
                if isinstance(value, int):
                    totals[key] = totals.get(key, 0) + value
                    turn["usage"][key] = turn["usage"].get(key, 0) + value
            first = min(first or stamp, stamp)
            last = max(last or stamp, stamp)
    if not seen:
        return None
    for turn_id, turn in turns.items():
        turn["timing"] = timings.get(turn_id)
    if "input_tokens" in totals and "cached_input_tokens" in totals:
        totals["uncached_input_tokens"] = (
            totals["input_tokens"] - totals["cached_input_tokens"]
        )
    return {
        "agent": agent,
        "thread_id": thread,
        "models": sorted(models),
        "responses": len(seen),
        "first_usage": first,
        "last_usage": last,
        "usage": totals,
        "turns": turns,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("baseline", type=Path)
    parser.add_argument(
        "--sessions", type=Path, default=Path.home() / ".codex/sessions"
    )
    args = parser.parse_args()
    baseline = json.loads(args.baseline.read_text())
    root_path = Path(baseline["root_path"])
    with root_path.open("rb") as stream:
        root_id = json.loads(stream.readline())["payload"]["id"]
    # Inspect metadata before reading bodies; skip all unrelated sessions.
    paths = set(args.sessions.glob("**/rollout-*.jsonl")) | {root_path}
    rows = []
    for path in sorted(paths):
        row = collect(
            path,
            baseline["start"],
            root_id,
            baseline["root_offset"] if path == root_path else 0,
        )
        if row:
            rows.append(row)
    print(json.dumps({"start": baseline["start"], "agents": rows}, indent=2))


if __name__ == "__main__":
    main()
