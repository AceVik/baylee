"""Two arenas on the same seeds, compared game by game.

Every arena game `i` has the same seed, decks and seats in each run, so two
checkpoints are judged on the games both finished (a game stopped by the time
cap in either run is left out of both). Prints each run's win rate on those
games, the games only one of them won, and an exact two-sided sign test on
those: the paired test, far less noisy than two independent intervals.

    python tools/trainer/paired_arena.py ~/baylee-data/arena/base-v3a-1000 ~/baylee-data/arena/rl-0{1,2,3}
"""

import json
import sys
from math import comb
from pathlib import Path


def outcomes(run: Path) -> dict[int, bool]:
    out = {}
    for line in (run / "games.jsonl").open():
        g = json.loads(line)
        if g["outcome"] in ("win", "loss"):
            out[g["i"]] = g["outcome"] == "win"
    return out


def sign_test(a: int, b: int) -> float:
    """Two-sided exact binomial p of a vs b discordant games at 1/2."""
    n = a + b
    if n == 0:
        return 1.0
    k = min(a, b)
    tail = sum(comb(n, j) for j in range(k + 1)) / 2**n
    return min(1.0, 2 * tail)


def main() -> None:
    base, *others = (Path(p).expanduser() for p in sys.argv[1:])
    b = outcomes(base)
    for other in others:
        o = outcomes(other)
        both = sorted(b.keys() & o.keys())
        bw = sum(b[i] for i in both)
        ow = sum(o[i] for i in both)
        only_o = sum(o[i] and not b[i] for i in both)
        only_b = sum(b[i] and not o[i] for i in both)
        print(
            f"{other.name} vs {base.name}: {len(both)} games both finished; "
            f"{ow / len(both):.3f} vs {bw / len(both):.3f}; "
            f"won only by {other.name} {only_o}, only by {base.name} {only_b}; "
            f"sign test p {sign_test(only_o, only_b):.3f}"
        )


if __name__ == "__main__":
    main()
