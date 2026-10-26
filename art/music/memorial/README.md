# In sunlight · Baylee's remembrance

Three complete nineteenth-century compositions, performed anew by the project's
persistent 48-kHz orchestra, with no third-party performance recording:

1. Johannes Brahms (1833–1897), **Wiegenlied**, Op.49 No.4.
2. Robert Schumann (1810–1856), **Mondnacht**, Op.39 No.5.
3. Franz Schubert (1797–1828), **Des Baches Wiegenlied**, D.795 No.20.

The underlying works are public domain. The modern digital editions come from
[OpenScore Lieder](https://github.com/OpenScore/Lieder), revision
`38c5db510224d9facdc4b08d741fc788cfb58ea8`. Its README states: “These scores are
released under Creative Commons Zero (CC0).” Checked 26 October 2026.
The exact source files, their CC0 legal text and self-created MuseScore 4.4.4
MIDI exports are in `sources/`; pinned URLs and SHA-256 hashes are in `sources.json`.
`performance.json` records source note counts, MIDI hashes, durations and the
compiled output hash. All 4,506 notes, including the editions' repeats, are retained.

Regenerate with `uv run --with mido==1.3.3 art/music/memorial/prepare.py`.
For fresh MIDI exports, use MuseScore's `-j` batch mode: one object per source,
with `in` pointing at its MSCX and `out` at its `.mid`. No synthesizer rendering
is involved in MIDI export. Re-exporting in another MuseScore version can change
articulation/tempo; review the resulting event manifest and audio before accepting.

The arrangement preserves every source pitch, rest, duration, dynamic and tempo
change. Brahms is stretched by 1.12 and Schubert by 1.04; Schumann retains the
edition's timing. Voices use gentle phrase-level dynamics and deterministic
millisecond onset variation. Violin/flute carry the vocal part instrumentally;
harp/piano and cello/piano retain the two accompaniment staves. No words are sung.
The pieces have space to decay between them; the full cycle lasts 668.24 seconds.

Recorded harp, violin and cello use the existing CC0 VCSL/VSCO 2 CE bank with
its original licences and source-rate metadata (44.1 kHz, resampled for Studio
playback); piano and flute are the project's native 48-kHz additive instruments.
The [publisher](https://versilian-studios.com/vsco-community/) states:
“Licensed under CC0 (Creative Commons Zero) you can do whatever you want with these samples.”
Checked 26 October 2026. Existing bank attribution remains in `NOTICE` and
`docs/legal.md`. No Wizards sound, music, sample or imagery is used.
