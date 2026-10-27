"""Stage official anonymized gateway exports for reviewed, train-only use.

This imports records, not tensors or policy targets. It never reads a database,
verifies engine replay, converts a dataset, or starts training. The exporter is
the authority for removing participant names; its provenance cannot be proven
from a file alone. This entry point additionally rejects detectable identifiers.
"""
import argparse
from collections import Counter
import fcntl
import gzip
import hashlib
import json
import os
from pathlib import Path
import re
import tempfile
import zlib


CONSENT = "Locally requested on 2026-10-25: human live games may be used for training."
SCHEMA = "anonymized-live-records4-v1"
SHA = re.compile(r"[0-9a-f]{64}\Z")
UUID = re.compile(r"[0-9a-fA-F]{8}(?:-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12}")
FIELDS = {
    "header": {"kind", "record", "build", "preset", "hash", "seen"},
    "input": {"kind", "n", "at", "seat", "by", "action", "hash", "seen"},
    "chair": {"kind", "n", "at", "seat", "change"},
    "declared_mind": {"kind", "n", "at", "seat", "mind"},
    "end": {"kind", "n", "at", "winners", "reason", "turn"},
}
FORBIDDEN = {"game_id", "game_uuid", "account", "account_id", "account_uuid",
             "user_id", "username", "display_name", "email", "token", "session",
             "session_id", "session_token", "invite_key", "timestamp", "created_at"}
AUTOMATION = {"SetPriorityHold", "SetAbilityYield", "SetAbilityPolicy"}
UNIT_ACTIONS = {"MulliganKeep", "MulliganTake", "PassPriority", "Concede", "CancelCast", "OfferDraw"}
VALUE_ACTIONS = set("""ChooseManaAbility ChooseDamageSource ChooseDamageEffect AllocatePrevention
TakeGrantedAction PlayLand CastSpell ActivateManaAbility ActivateAbility DeclareAttackers
DeclareBlockers ChooseObjects ChooseTargets ChooseTargetBatch Suspend Arrange ChooseColor
ChooseSubtype ChooseCardName ChooseMode ChooseNumber ChoosePlayer YesNo PayWindow""".split()) | AUTOMATION


def require(condition, message):
    if not condition:
        raise ValueError(message)


def _object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, "duplicate JSON key")
        result[key] = value
    return result


def load_json(text):
    def invalid(_):
        raise ValueError("nonfinite JSON number")
    return json.loads(text, object_pairs_hook=_object, parse_constant=invalid)


def check_private(value, key=""):
    if isinstance(value, dict):
        if key == "Human":
            require(all(type(child) is int and child == 0 for child in value.values()),
                    "Human account identifiers were not anonymized")
            return
        for name, child in value.items():
            require(name.lower() not in FORBIDDEN, "identity or timestamp field present")
            require(not UUID.search(name), "identifier in JSON key")
            check_private(child, name)
    elif isinstance(value, list):
        for child in value:
            check_private(child, key)
    elif isinstance(value, str) and key != "scryfall_id":
        require(not UUID.search(value), "non-card UUID present")
        require(not re.search(r"[^\s@]+@[^\s@]+\.[^\s@]+", value), "address present")
        require(not re.search(r"\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}", value),
                "wall-clock timestamp present")


def human_role(lines, human_seat):
    current = None
    declared = set()
    input_roles = set()
    for row in lines:
        if row.get("seat") != human_seat:
            continue
        if row["kind"] == "declared_mind":
            mind = row.get("mind")
            require(isinstance(mind, dict) and isinstance(mind.get("kind"), str),
                    "invalid declared mind")
            current = mind["kind"]
            declared.add(current)
        elif row["kind"] == "input":
            input_roles.add(current)
    roles = declared | input_roles
    if roles == {"human"}:
        return "confirmed_human"
    if roles == {"house"}:
        return "excluded_house"
    return "quarantine_unknown_or_mixed"


def validate(raw, neural_seat):
    lines = [load_json(line) for line in raw.decode("utf-8").splitlines() if line.strip()]
    require(len(lines) >= 3, "record is empty or incomplete")
    require(all(isinstance(row, dict) for row in lines), "record line is not an object")
    require(lines[0].get("kind") == "header" and lines[-1].get("kind") == "end",
            "only complete Header-to-End records are accepted")
    require(lines[0].get("record") == 1, "unsupported record version")
    check_private(lines)
    preset = lines[0].get("preset", {})
    require(isinstance(preset, dict), "invalid preset")
    seats = preset.get("seats", [])
    require(isinstance(seats, list) and len(seats) == 2, "exactly two seats required")
    for seat in seats:
        require(isinstance(seat, dict) and "controller" in seat, "missing seat controller")
        controller = seat["controller"]
        if isinstance(controller, dict) and "Human" in controller:
            human = controller["Human"]
            require(isinstance(human, dict) and all(type(v) is int and v == 0 for v in human.values()),
                    "Human account identifiers were not anonymized")
    counts = [Counter(), Counter()]
    actions = [Counter(), Counter()]
    for index, row in enumerate(lines):
        kind = row.get("kind")
        require(kind in FIELDS and not (set(row) - FIELDS[kind]), "unknown record fields/kind")
        require(FIELDS[kind] - {"seen", "turn"} <= set(row), "missing record fields")
        require((kind == "header") == (index == 0), "duplicate or misplaced Header")
        require((kind == "end") == (index == len(lines)-1), "duplicate or misplaced End")
        if kind != "header":
            require(type(row.get("at")) is int and row["at"] == 0, "host clock not anonymized")
            require(type(row.get("n")) is int and row["n"] == index-1, "broken record sequence")
        if "seat" in row:
            require(type(row["seat"]) is int and row["seat"] in (0, 1), "invalid seat")
        if kind in ("header", "input"):
            require(isinstance(row.get("hash"), str) and re.fullmatch(r"[0-9a-f]{16}", row["hash"]),
                    "missing snapshot digest")
        if kind == "input":
            origin = row.get("by")
            require(origin in ("seat", "clock", "house", "stand_in"), "unknown input origin")
            require("seat" in row and "action" in row, "incomplete input")
            action = row["action"]
            require(isinstance(action, str) or (isinstance(action, dict) and len(action) == 1),
                    "invalid action representation")
            action_kind = action if isinstance(action, str) else next(iter(action))
            require(action_kind in (UNIT_ACTIONS if isinstance(action, str) else VALUE_ACTIONS),
                    "unknown or malformed action kind")
            automation = action_kind in AUTOMATION
            counts[row["seat"]][origin] += 1
            actions[row["seat"]]["automation" if automation else "decision"] += 1
    end = lines[-1]
    winners = end.get("winners")
    require(isinstance(winners, list) and all(type(s) is int and s in (0, 1) for s in winners),
            "invalid completed outcome")
    require(isinstance(end.get("reason"), str), "missing completion reason")
    return {"neural_seat": neural_seat, "human_seat": 1-neural_seat,
            "human_role_status": human_role(lines, 1-neural_seat),
            "behavior_logprob_available": False,
            "inputs": sum(sum(c.values()) for c in counts),
            "origins_by_seat": [dict(c) for c in counts],
            "actions_by_seat": [dict(c) for c in actions], "winners": winners,
            "terminal_turn": end.get("turn"), "replay_verified": False}


def exclusions(path):
    if path is None:
        return {}
    text = Path(path).read_text()
    if text.lstrip().startswith("{"):
        result = load_json(text)
    else:
        result = {}
        for line in text.splitlines():
            line = line.split("#", 1)[0].strip()
            if line:
                fields = line.split(maxsplit=1)
                result[fields[0]] = fields[1] if len(fields) == 2 else "known_smoke"
    require(isinstance(result, dict), "exclusion file must be SHA/reason mapping or text lines")
    for digest, reason in result.items():
        require(SHA.fullmatch(digest) is not None, "invalid excluded SHA256")
        require(isinstance(reason, str) and re.fullmatch(r"[a-z0-9_-]{1,80}", reason),
                "exclusion reasons must be anonymous lowercase labels")
    return result


def atomic_bytes(path, data, immutable=False):
    fd, temporary = tempfile.mkstemp(prefix=".import-", dir=path.parent)
    try:
        with os.fdopen(fd, "wb") as handle:
            handle.write(data)
            handle.flush()
            os.fsync(handle.fileno())
        if immutable:
            os.chmod(temporary, 0o444)
            os.link(temporary, path)  # exclusive publication; never replace content
        else:
            os.replace(temporary, path)
    finally:
        Path(temporary).unlink(missing_ok=True)


def import_records(export_dir, out, neural_seat=0, exclude_sha256=None):
    require(neural_seat in (0, 1), "invalid neural seat")
    export_dir, out = Path(export_dir), Path(out)
    require(export_dir.is_dir() and not export_dir.is_symlink(), "export directory required")
    require(not out.is_symlink() and out.resolve() != export_dir.resolve()
            and export_dir.resolve() not in out.resolve().parents,
            "output must be outside the export directory")
    files = sorted(export_dir.iterdir())
    require(files and all(re.fullmatch(r"\d{6}\.jsonl\.gz", p.name) and p.is_file()
                          and not p.is_symlink() for p in files),
            "expected only numbered official gateway export gzip records")
    excluded = exclusions(exclude_sha256)
    # Validate the entire incoming batch before any manifest or record publication.
    incoming = {}
    for path in files:
        raw = gzip.decompress(path.read_bytes())
        metadata = validate(raw, neural_seat)
        incoming[hashlib.sha256(raw).hexdigest()] = (path, metadata)
    out.mkdir(parents=True, exist_ok=True)
    with (out / ".import.lock").open("a+b") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        return _publish(out, incoming, neural_seat, excluded)


def _publish(out, incoming, neural_seat, excluded):
    manifest_path = out / "manifest.json"
    if manifest_path.exists():
        manifest = load_json(manifest_path.read_text())
        require(manifest.get("schema") == SCHEMA and manifest.get("neural_seat") == neural_seat,
                "incompatible existing import manifest")
    else:
        manifest = {"schema": SCHEMA, "split": "train", "neural_seat": neural_seat,
                    "consent": CONSENT, "source": "official_anonymized_gateway_export",
                    "allowed_initial_uses": ["outcomes", "auxiliary", "tactical_review"],
                    "policy_targets": "none_until_review", "ppo_eligible": False,
                    "neural_policy": "greedy_without_behavior_logprob_not_for_imitation",
                    "human_policy": "not_assumed_good_teacher_needs_review",
                    "conversion_done": False, "training_done": False,
                    "records": {}, "excluded": {}, "quarantined": {}}
    records = manifest["records"]
    old_excluded = manifest["excluded"]
    old_excluded.update(excluded)
    quarantined = manifest.setdefault("quarantined", {})
    for digest, (_, metadata) in incoming.items():
        role = metadata["human_role_status"]
        if role == "excluded_house":
            old_excluded.setdefault(digest, "house_opponent")
        elif role != "confirmed_human" and digest not in old_excluded:
            quarantined[digest] = "unknown_or_mixed_opponent_role"
    for digest in old_excluded:
        quarantined.pop(digest, None)
    for digest in list(records):
        require(SHA.fullmatch(digest) is not None, "invalid existing content digest")
        stored = out / "records" / f"{digest}.jsonl.gz"
        require(hashlib.sha256(gzip.decompress(stored.read_bytes())).hexdigest() == digest,
                "immutable stored record failed integrity")
        if digest in old_excluded or digest in quarantined:
            del records[digest]
    record_dir = out / "records"
    record_dir.mkdir(exist_ok=True)
    for digest, (source, metadata) in sorted(incoming.items()):
        if digest in old_excluded or digest in quarantined:
            continue
        # Bound memory to one record even during a large incremental export.
        raw = gzip.decompress(source.read_bytes())
        require(hashlib.sha256(raw).hexdigest() == digest, "export changed during import")
        stored = record_dir / f"{digest}.jsonl.gz"
        if stored.exists():
            require(gzip.decompress(stored.read_bytes()) == raw, "stored content conflict")
        else:
            atomic_bytes(stored, gzip.compress(raw, mtime=0), immutable=True)
        records[digest] = {**metadata, "path": f"records/{digest}.jsonl.gz", "split": "train"}
    encoded = (json.dumps(manifest, sort_keys=True, indent=2) + "\n").encode()
    atomic_bytes(manifest_path, encoded)
    return {"records": len(records), "excluded": len(old_excluded),
            "excluded_house": sum(reason == "house_opponent" for reason in old_excluded.values()),
            "quarantined": len(quarantined),
            "conversion_done": False, "training_done": False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--export-dir", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--neural-seat", type=int, choices=(0, 1), default=0)
    parser.add_argument("--exclude-sha256", "--exclude", type=Path,
                        help="SHA256 [anonymous_reason] lines, or JSON SHA256/reason mapping")
    args = parser.parse_args()
    try:
        result = import_records(args.export_dir, args.out, args.neural_seat, args.exclude_sha256)
    except (ValueError, OSError, EOFError, KeyError, TypeError, zlib.error):
        # Do not echo malformed record contents, identifiers, or source filenames.
        parser.exit(1, "live record import refused: invalid export, exclusion, or existing store\n")
    print(json.dumps(result, sort_keys=True))


if __name__ == "__main__":
    main()
