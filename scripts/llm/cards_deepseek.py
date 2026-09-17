#!/usr/bin/env python3
"""The DeepSeek card lane: a batch of stubs in, finished card files out.

Not an agent. The context the model would have browsed for itself is packed
and sent as one cacheable prefix — the authoring contract, the whole DSL
vocabulary, the pool's shared filters and tokens, and four finished cards to
read the house style off — so thirty cards pay for it once.

What comes back is a whole file or the word STUB. Nothing is written until
the generated header survives byte for byte: the `//!` lines, `index`,
`oracle_id`, `scryfall_id`. Those belong to a generator, and a model that
rewrote one has misunderstood the job badly enough that the rest is not worth
trusting either.

    python3 scripts/llm/cards_deepseek.py <batch.json> [workers]

`batch.json` is a list of `{"name": …, "path": …}`, the path repo-relative.
The report lands beside it as `<stem>-report.md`.
"""

import concurrent.futures as cf
import json
import pathlib
import re
import sys

import lane

PACK_FILES = [
    "docs/card-dsl.md",
    "crates/baylee-cards-dsl/src/effect.rs",
    "crates/baylee-cards-dsl/src/cost.rs",
    "crates/baylee-cards-dsl/src/ability.rs",
    "crates/baylee-cards-dsl/src/filter.rs",
    "crates/baylee-cards-dsl/src/static_ability.rs",
    "crates/baylee-cards-dsl/src/counters.rs",
    "crates/baylee-cards-dsl/src/build.rs",
    "crates/baylee-cards/src/filters.rs",
    "crates/baylee-cards/src/tokens.rs",
]

EXAMPLES = [
    "crates/baylee-cards/src/cards/lands/restricted/bleachbone_verge.rs",
    "crates/baylee-cards/src/cards/artifacts/equipment/mv_1/skullclamp.rs",
    "crates/baylee-cards/src/cards/instants/mv_1/swords_to_plowshares.rs",
    "crates/baylee-cards/src/cards/creatures/mv_1/llanowar_elves.rs",
]

RULES = (pathlib.Path(__file__).resolve().parent / "prompts/cards-deepseek.md").read_text(
    encoding="utf-8"
)


def keeps_the_header(stub: str, written: str) -> str | None:
    """The reason to refuse this answer, or None."""
    for line in stub.splitlines():
        if line.startswith("//!") and line not in written:
            return f"header line dropped: {line[:60]}"
    for field in ("index = index::", "oracle_id = ", "scryfall_id = "):
        want = next(
            (l.strip() for l in stub.splitlines() if l.strip().startswith(field)), None
        )
        if want and want not in written:
            return f"generated field rewritten: {want[:60]}"
    if "card!(" not in written:
        return "no `card!(` in the answer"
    if "GENERATED STUB" in written or "TODO(card)" in written:
        return "still marked a stub"
    return None


def one(prefix: str, card: dict) -> str:
    path = lane.ROOT / card["path"]
    stub = path.read_text(encoding="utf-8")
    try:
        answer = lane.ask_deepseek(prefix, RULES, stub, max_tokens=16000)
    except Exception as exc:  # noqa: BLE001 — the reason is the report
        return f"{card['name']} | error | {exc}"
    # The code block first, because a card that *was* written may also carry
    # the word STUB in a `// NOT SUPPORTED:` line beside what it could say.
    m = re.search(r"```(?:rust)?\n(.*?)```", answer, re.S)
    if not m:
        refusal = re.search(r"STUB:\s*(.*)", answer, re.S)
        if refusal:
            return f"{card['name']} | stub | {' '.join(refusal.group(1).split())[:200]}"
        return (
            f"{card['name']} | error | unreadable answer: "
            f"{' '.join(answer.split())[:120]}"
        )
    written = m.group(1)
    bad = keeps_the_header(stub, written)
    if bad:
        return f"{card['name']} | refused | {bad}"
    path.write_text(written, encoding="utf-8")
    kind = "partial" if "Coverage::Partial" in written else "implemented"
    return f"{card['name']} | {kind} | written"


def main(batch: str, workers: int = 5) -> int:
    cards = json.loads(pathlib.Path(batch).read_text(encoding="utf-8"))
    stem, out_dir = lane.outputs(batch)
    prefix = lane.pack(PACK_FILES, EXAMPLES)
    print(f"{len(cards)} Karten, Kontext {len(prefix) / 1024:.0f} KB, {workers} parallel")
    lines = []
    with cf.ThreadPoolExecutor(max_workers=workers) as pool:
        for line in pool.map(lambda c: one(prefix, c), cards):
            print(" ", line)
            lines.append(line)
    report = out_dir / f"{stem}-report.md"
    report.write_text("\n".join(lines) + "\n", encoding="utf-8")
    done = sum(1 for l in lines if "| implemented |" in l or "| partial |" in l)
    print(f"{done} von {len(cards)} geschrieben; Bericht in {report}")
    return 0


if __name__ == "__main__":
    if len(sys.argv) < 2:
        raise SystemExit(__doc__)
    sys.exit(main(sys.argv[1], int(sys.argv[2]) if len(sys.argv) > 2 else 5))
