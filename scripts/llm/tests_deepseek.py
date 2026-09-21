#!/usr/bin/env python3
"""The DeepSeek test lane: one engine test per card, for cards it did not write.

The card half is the other script; this is the half the project insists on.
`xtask validate` compares a card against its printing and says nothing at all
about what the engine does with it, so a card is only finished once it has
been played once.

**Which cards go in here is the point.** A card and its test written by the
same model share that model's misreading — measured on Mikaeus, whose
generated test asserted that a keyword no rule reads worked — so this lane
gets the *other* lane's cards. See `README.md`.

Nothing is written into the repository: each answer lands in
`<worklist-dir>/<stem>-tests/<door>/<slug>.rs`, routed to the `card_tests`
module its card belongs to, and the coordinator assembles, compiles and
fixes. A model that cannot run cargo must not be the last reader of a test.

    python3 scripts/llm/tests_deepseek.py <worklist.txt> [workers]
"""

import concurrent.futures as cf
import pathlib
import re
import sys

import lane

PACK_FILES = [
    "crates/baylee-engine/src/engine/testkit.rs",
    "crates/baylee-engine/src/engine/card_tests/mod.rs",
    "crates/baylee-engine/src/choice.rs",
    "crates/baylee-engine/src/engine/card_tests/artifacts.rs",
]

RULES = (pathlib.Path(__file__).resolve().parent / "prompts/tests-deepseek.md").read_text(
    encoding="utf-8"
)


def one(prefix: str, rel: str, out_dir: pathlib.Path) -> str:
    slug = pathlib.Path(rel).stem
    text = (lane.ROOT / rel).read_text(encoding="utf-8")
    try:
        # 64000, the same as the card lane and for the same measured
        # reason. At 32000, 7 of round H's 43 came back with nothing at all
        # — six `stop_reason='max_tokens'` and one truncated code fence —
        # and all seven were written on a retry with 64000 and nothing else
        # changed, so the budget was the refusal in every case. A test that
        # reasons about a board before it writes spends most of its room
        # there, and DeepSeek bills the tokens it generates, so a low cap
        # buys nothing and loses cards.
        answer = lane.ask_deepseek(prefix, RULES, text, max_tokens=64000)
    except Exception as exc:  # noqa: BLE001 — the reason is the report
        return f"{slug} | error | {exc}"
    m = re.search(r"```(?:rust)?\n(.*?)```", answer, re.S)
    if not m:
        why = re.search(r"SKIP:\s*(.*)", answer, re.S)
        if why:
            return f"{slug} | skip | {' '.join(why.group(1).split())[:160]}"
        return (
            f"{slug} | error | unreadable answer: {' '.join(answer.split())[:120]}"
        )
    body = m.group(1)
    if "#[test]" not in body:
        return f"{slug} | error | no #[test] in the answer"
    where = out_dir / lane.door(rel)
    where.mkdir(parents=True, exist_ok=True)
    (where / f"{slug}.rs").write_text(body, encoding="utf-8")
    return f"{slug} | written | {lane.door(rel)}"


def main(list_file: str, workers: int = 5) -> int:
    rels = lane.worklist(list_file)
    stem, parent = lane.outputs(list_file)
    out_dir = parent / f"{stem}-tests"
    out_dir.mkdir(parents=True, exist_ok=True)
    prefix = lane.pack(PACK_FILES, heading="# Testkit, Helfer, Taxonomie und eine fertige Testdatei\n")
    print(f"{len(rels)} Tests, Kontext {len(prefix) / 1024:.0f} KB, {workers} parallel")
    lines = []
    with cf.ThreadPoolExecutor(max_workers=workers) as pool:
        for line in pool.map(lambda r: one(prefix, r, out_dir), rels):
            print(" ", line)
            lines.append(line)
    report = parent / f"{stem}-tests-report.md"
    report.write_text("\n".join(lines) + "\n", encoding="utf-8")
    written = sum(1 for l in lines if "| written |" in l)
    print(f"{written} von {len(rels)} geschrieben; Bericht in {report}")
    return 0


if __name__ == "__main__":
    if len(sys.argv) < 2:
        raise SystemExit(__doc__)
    sys.exit(main(sys.argv[1], int(sys.argv[2]) if len(sys.argv) > 2 else 5))
