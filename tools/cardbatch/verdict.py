"""Reads one `agy --output-format json` result and answers one question.

Kept separate from run.sh because the shape of that envelope is the one thing
here that belongs to somebody else's tool: when it changes, exactly this file
changes. It changed once already — see `payload`.
"""
import json
import sys


def payload(path):
    """The model's structured verdict, wherever the envelope keeps it.

    `structured_output` is the field `--json-schema` fills, and it is the only
    one worth reading. `response` beside it is the *narration* with the same
    JSON appended, and parsing that was the first version of this file: it
    threw on the prose, the throw was swallowed, and a finished card was
    reverted as if the model had refused it. A verdict that cannot be read has
    to look different from a refusal, so the fallbacks below are ordered and
    the last one is deliberate rather than accidental.
    """
    with open(path, encoding="utf-8") as handle:
        raw = json.load(handle)
    if not isinstance(raw, dict):
        return {}
    structured = raw.get("structured_output")
    if isinstance(structured, dict):
        return structured
    # A narration with the object appended. Take the last one: the model may
    # have shown its working, and the working is not the answer.
    text = raw.get("response") or raw.get("result") or raw.get("output") or ""
    if isinstance(text, dict):
        return text
    if isinstance(text, str):
        start = text.rfind('{"slug"')
        if start >= 0:
            decoder = json.JSONDecoder()
            try:
                obj, _ = decoder.raw_decode(text[start:])
            except ValueError:
                return {}
            if isinstance(obj, dict):
                return obj
    return {}


def cell(value):
    """One TSV cell: no tabs, no newlines, never empty."""
    text = " ".join(str(value or "").split())
    return text or "-"


def main():
    path, question = sys.argv[1], sys.argv[2]
    try:
        data = payload(path)
    except (OSError, ValueError):
        data = {}
    if question == "status":
        # Anything unrecognised is a refusal. A verdict that cannot be read is
        # not evidence that a card is good.
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
                    data.get("status", "unreadable"),
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
