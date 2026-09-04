"""Reads one `agy --output-format json` result and answers one question.

Kept separate from run.sh because the shape of that envelope is the one thing
here that belongs to somebody else's tool: when it changes, exactly this file
changes.
"""
import json
import sys


def payload(path):
    """The model's structured verdict, wherever the envelope keeps it."""
    with open(path, encoding="utf-8") as handle:
        raw = json.load(handle)
    # An envelope with a `result` is the documented shape; a bare object is
    # what `--json-schema` alone produces. Accept either rather than guess.
    for key in ("result", "response", "output"):
        if isinstance(raw, dict) and key in raw:
            raw = raw[key]
            break
    if isinstance(raw, str):
        raw = json.loads(raw)
    return raw if isinstance(raw, dict) else {}


def cell(value):
    """One TSV cell: no tabs, no newlines, never empty."""
    text = " ".join(str(value or "").split())
    return text or "-"


def main():
    path, question = sys.argv[1], sys.argv[2]
    data = payload(path)
    if question == "status":
        # Anything unrecognised is a refusal. A verdict that cannot be read
        # is not evidence that a card is good.
        status = data.get("status")
        print(status if status in {"implemented", "partial", "refused"} else "refused")
        return
    if question == "row":
        slug, name = sys.argv[3], sys.argv[4]
        print(
            "\t".join(
                cell(v)
                for v in (
                    slug,
                    name,
                    data.get("status", "refused"),
                    data.get("oracle_sentence"),
                    data.get("cannot_say"),
                    data.get("nearest_existing"),
                )
            )
        )
        return
    raise SystemExit(f"unknown question: {question}")


if __name__ == "__main__":
    main()
