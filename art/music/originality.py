"""Interval-signature check for the score's melodies (design §2.4).

    python3 art/music/originality.py        # from the repository root

No run of six or more directed semitone intervals in any melody of
`crates/baylee-client-core/src/music/score/melodies.rs` may coincide with the
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
MELODIES = Path("crates/baylee-client-core/src/music/score/melodies.rs")
AVOID = Path("art/music/avoid.json")


def note_values(text):
    values = {}
    for m in re.finditer(r"pub\(super\) const ([A-Z0-9]+): u8 = (\d+);", text):
        values[m.group(1)] = int(m.group(2))
    return values


def melodies(text):
    """Each `const NAME: Phrase = &[ ... ];` as its list of pitches."""
    values = note_values(text)
    out = {}
    for m in re.finditer(r"pub\(super\) const ([A-Z_0-9]+): Phrase = &\[(.*?)\n\];", text, re.S):
        out[m.group(1)] = [values[n] for n in re.findall(r"\(([A-Z0-9]+), \d+\)", m.group(2)) if n in values]
    calls = re.search(r"pub\(super\) const CALLS: &\[Phrase\] = &\[(.*?)\n\];", text, re.S).group(1)
    # Each call is one top-level `&[...]` of the table, however rustfmt wrapped it.
    depth, start, k = 0, None, 0
    for i, ch in enumerate(calls):
        if ch == "[":
            depth += 1
            if depth == 1:
                start = i
        elif ch == "]":
            depth -= 1
            if depth == 0:
                k += 1
                call = calls[start:i]
                out[f"CALL_{k}"] = [values[n] for n in re.findall(r"\(([A-Z0-9]+), \d+\)", call)]
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
