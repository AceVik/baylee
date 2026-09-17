#!/usr/bin/env python3
"""The Gemini test lane: one engine test per card, for cards it did not write.

The counterpart of `tests_deepseek.py`, and the point of having both is in
`README.md`: a card and its test written by the same model share that
model's misreading, so each lane gets the other's cards.

It writes nothing into the repository — each answer lands in
`<worklist-dir>/<stem>-tests/<slug>.rs`, and the coordinator assembles,
compiles and fixes.

    python3 scripts/llm/tests_gemini.py <worklist.txt> [minutes]
"""

import pathlib
import sys

import lane

PROMPT = pathlib.Path(__file__).resolve().parent / "prompts/tests-gemini.md"


def main(list_file: str, minutes: int = 45) -> int:
    rels = lane.worklist(list_file)
    stem, parent = lane.outputs(list_file)
    out_dir = parent / f"{stem}-tests"
    out_dir.mkdir(parents=True, exist_ok=True)
    before = {p.name for p in out_dir.glob("*.rs")}

    prompt = PROMPT.read_text(encoding="utf-8").replace("<SCRATCH>", str(out_dir))
    prompt += "\n".join(f"{i:2}. `{rel}`" for i, rel in enumerate(rels, 1)) + "\n"
    (parent / f"{stem}-tests-sent.md").write_text(prompt, encoding="utf-8")

    proc, seconds, steps, sent = lane.run_gemini(prompt, minutes)

    answer = parent / f"{stem}-tests-answer.md"
    answer.write_text(proc.stdout or proc.stderr or "", encoding="utf-8")
    lane.report_gemini("Tests", "Test", len(rels), proc, seconds, steps, sent, answer)

    # Counted off the directory rather than off the session's own table: a
    # refusal and a file it forgot to save read the same in that table, and
    # only one of them is a correct outcome.
    written = {p.name for p in out_dir.glob("*.rs")} - before
    print(f"  {len(written)} von {len(rels)} Dateien geschrieben")
    return 0


if __name__ == "__main__":
    if len(sys.argv) < 2:
        raise SystemExit(__doc__)
    sys.exit(main(sys.argv[1], int(sys.argv[2]) if len(sys.argv) > 2 else 45))
