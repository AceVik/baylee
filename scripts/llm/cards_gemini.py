#!/usr/bin/env python3
"""The Gemini card lane: one `agy` session for a whole batch, and its cost.

One session per batch and not per card: a fresh session pays for `CLAUDE.md`,
`docs/card-dsl.md` and its own orientation every time, and thirty cards
amortise that once instead of thirty times.

This lane writes card files **into the repository**, because `agy` edits the
tree itself — so do not run it while anything else is writing cards, and
never run `xtask codegen` while it is in flight (codegen rewrites every file
carrying the stub marker and would clobber the session's work).

What is measured afterwards is what actually matters on this lane: minutes,
because the subscription costs no money and refreshes a time quota instead —
and, beside them, steps and the transcript's prefix sum, which is what the
model was handed as input and what makes a session slow.

    python3 scripts/llm/cards_gemini.py <batch.json> [minutes]
"""

import json
import pathlib
import subprocess
import sys

import lane

PROMPT = pathlib.Path(__file__).resolve().parent / "prompts/cards-gemini.md"


def main(batch_file: str, minutes: int = 60) -> int:
    cards = json.loads(pathlib.Path(batch_file).read_text(encoding="utf-8"))
    stem, out_dir = lane.outputs(batch_file)
    prompt = PROMPT.read_text(encoding="utf-8") + "\n".join(
        f"{i:2}. {c['name']}\n    `{c['path']}`" for i, c in enumerate(cards, 1)
    ) + "\n"
    (out_dir / f"{stem}-sent.md").write_text(prompt, encoding="utf-8")

    proc, seconds, steps, sent = lane.run_gemini(prompt, minutes)

    answer = out_dir / f"{stem}-answer.md"
    answer.write_text(proc.stdout or proc.stderr or "", encoding="utf-8")
    lane.report_gemini("Karten", "Karte", len(cards), proc, seconds, steps, sent, answer)

    # What it actually touched, asked of git rather than of the model: the
    # session's own summary is a claim, and a card it reported as written
    # and never saved would look identical in that table.
    touched = subprocess.run(
        ["git", "status", "--porcelain", "--"] + [c["path"] for c in cards],
        cwd=lane.ROOT,
        capture_output=True,
        text=True,
        check=False,
    ).stdout.splitlines()
    written = [l.split()[-1] for l in touched if l.strip()]
    print(f"  {len(written)} von {len(cards)} Dateien angefasst")

    # The check `cards_deepseek.py` makes *before* it writes, made here
    # after: `agy` edits the tree itself, so there is no answer to refuse
    # and the file is the only thing left to ask. Round H had three cards
    # that rewrote the generator's four lines — each with a different
    # `oracle_id`, a different `scryfall_id` and a `//!` line naming a set
    # that does not exist — while writing the right card's text, which is
    # why the identity is restored and the rest is kept. What caught them
    # was the pool's own lint, and it caught exactly one: that assertion
    # panics on the first card it disagrees with, so two were invisible
    # behind the third until it was fixed. A file with no `card!(` left in
    # it is not a card and goes back to being a stub.
    for path in written:
        head = subprocess.run(
            ["git", "show", f"HEAD:{path}"],
            cwd=lane.ROOT,
            capture_output=True,
            text=True,
            check=False,
        ).stdout
        body = (lane.ROOT / path).read_text(encoding="utf-8")
        bad = lane.keeps_the_header(head, body)
        if not bad:
            continue
        repaired = lane.restore_generated(head, body)
        if repaired is None:
            (lane.ROOT / path).write_text(head, encoding="utf-8")
            print(f"  zurückgesetzt: {path} — {bad}")
        else:
            (lane.ROOT / path).write_text(repaired, encoding="utf-8")
            print(f"  Kopf wiederhergestellt: {path} — {bad}")

    return 0 if proc.returncode == 0 else 1


if __name__ == "__main__":
    if len(sys.argv) < 2:
        raise SystemExit(__doc__)
    sys.exit(main(sys.argv[1], int(sys.argv[2]) if len(sys.argv) > 2 else 60))
