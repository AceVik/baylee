# Baylee's sanctuary and battle score

Original 32-bar 6/8 theme, now performed with thirty real instrument recordings
from **VSCO 2 Community Edition** (CC0), recorded by Sam Gossner and Simon
Dalzell; sample editing by Elan Hickler / Soundemote. Source:
https://versilian-studios.com/vsco-community/

The runtime is `crates/baylee-client-core/src/music/{score,orchestra}.rs`.
All states use one clock, harmony, sampler and reverberation network. Musical
changes are scheduled at bar lines; existing voices keep their release tails.
Tempo evolves without changing sample pitch. No track crossfade is involved.

`samples.json` pins every upstream WAV to a commit and SHA-256 and records
verified MIDI roots (the library uses more than one octave naming convention).
`prepare.py` rebuilds the embedded PCM bank with Python/numpy and ffmpeg:
mono 22,050 Hz PCM16, silent lead trimmed, peak-normalized, DC removed, short
edge fades, at most eight seconds. No source recording is edited by hand.
Prepared samples retain CC0; see the full bundled `LICENSE-CC0.txt`.

The piano carries the sanctuary theme, with fifteen recordings across
five registers and three touch layers (pp/mf/f). Nearest-register selection and
power-blended touch layers retain the recorded hammer character. Flute is
removed from both the score and the shipped bank. The piano has a close, clear
room send; strings sit farther back in the same continuous concert room.

The arrangement has four evolving two-hand patterns, bass inversions, added
ninths, suspended notes resolving inside the bar, phrase pickups, and a
contrary-motion cello/viola response that alternates across 32-bar cycles.
Harp answers on offbeats. In battle, horns take the melody and piano
accompaniment recedes; bowed ostinato and timpani establish a pulse even
at low activity, with snare and stronger brass entering as action grows.

Outcomes use a separate four-bar arrangement, independent of the current
piano pattern. Victory has an original ascending horn fanfare over D–G–A–D,
spiccato strings, timpani and cymbal arrivals. Defeat uses quiet piano and a
descending cello answer without brass or percussion; draw holds open fifths.
Each cadence completes before returning to the lobby, then a lingering result
screen settles into quiet strings. No third-party melody was used.

Render the same live sampler to WAV for listening review:

```sh
cargo run -p baylee-client-core --example music_demo -- /tmp/baylee-orchestra.wav
```

This is a sampled performance of real instruments, not a live orchestra session.
The score/compositor are project code (AGPL-3.0-only). No third-party composition
or game soundtrack is included.
