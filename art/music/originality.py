"""Interval-signature check for the score's melodies (design §2.4).

    python3 art/music/originality.py        # from the repository root

No run of six or more directed semitone intervals in any melody of
`crates/baylee-client-core/src/music/score/manuscript.rs` may coincide with the
opening of a tune in `avoid.json` (any of its settings), transposed anywhere.
The same check runs in CI as the test `no_melody_echoes_a_tune_we_must_not`;
this script is the composer's, printing the closest call for every melody.

`avoid.json`: each opening's intervals, `verified` when they were read from a
fetched public-domain notation (08.10.2026: seventeen of twenty-three; the
film and game themes are interval shapes only, never a recording or a score,
and Ederlezi's notation was not found), with the source it was read from.
"""
import json
import re
import sys
from pathlib import Path

RUN = 6
MELODIES = Path("crates/baylee-client-core/src/music/score/manuscript.rs")
AVOID = Path("art/music/avoid.json")


def melodies(text):
    """Original numeric MIDI manuscripts, with rests removed."""
    out = {}
    for m in re.finditer(r"const ([A-Z_0-9]+): Phrase = &\[(.*?)\n\];", text, re.S):
        out[m.group(1)] = [int(n) for n in re.findall(r"\((\d+), \d+\)", m.group(2)) if int(n)]
    assert len(out) == 25, f"expected 25 complete phrases, found {len(out)}"
    return out


def intervals(pitches):
    return [b - a for a, b in zip(pitches, pitches[1:])]


def longest_common_run(a, b):
    best = 0
    for i in range(len(a)):
        for j in range(len(b)):
            k = 0
            while i + k < len(a) and j + k < len(b) and a[i + k] == b[j + k]:
                k += 1
            best = max(best, k)
    return best


def main():
    tunes = []
    for tune in json.loads(AVOID.read_text()):
        for setting in [tune["intervals"], *tune.get("alt_intervals", [])]:
            if setting:
                tunes.append((tune["name"], setting))
    bad = 0
    for name, pitches in melodies(MELODIES.read_text()).items():
        ours = intervals(pitches)
        run, tune = max((longest_common_run(ours, theirs), name) for name, theirs in tunes)
        flag = "MATCH" if run >= RUN else "ok"
        bad += flag == "MATCH"
        print(f"{name:16s} longest shared run {run} ({tune}) {flag}")
    name, theirs = tunes[0]
    assert longest_common_run(theirs[:RUN], theirs) >= min(RUN, len(theirs)), "the check fires"
    print(f"an injected opening of {name}: caught")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
