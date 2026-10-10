# Five original suites in B♭ Dorian

The active score is `crates/baylee-client-core/src/music/score/`: five original
manuscripts, each arranged for all eight situations. The active orchestra uses
**our own instrument models at native 48 kHz**, not samples. Models approximate
harp, zither, lyre, violin, viola, cello, bowed bass and trombone; they should
not be described as real acoustic recordings. Code/compositions follow the
repository's AGPL-3.0-only licence. No additional sound-library terms apply.

## Audition

```sh
cargo run --release -p baylee-client-core --example music_demo -- /tmp/baylee-dorian 180
python3 art/music/originality.py
```

Forty PCM24 stereo WAVs, five off-beat transition tours, and a JSON measurement
report come from the exact runtime renderer, before the client's master gain
and cue ducking. Native rate is 48,000 Hz. `ember`, `glass`, `thorn`, `tide`,
`star` correspond to Glutpfad, Mondglas, Dornenkrone, Nebelhafen, Sternfall.
Movement names are title, lobby, standard, combat, endgame, victory, defeat,
draw. The tours also exercise combat cancellation and result dismissal.

Originality checks compare written melodies against `avoid.json`'s existing
interval signatures. They catch six consecutive shared intervals; they do not
establish legal or musical originality against all music in existence.

## Live preview (dev-control only)

With the client running on a dedicated development port:

```sh
curl -s -X POST localhost:28770/music -d '{"theme":"star","movement":"endgame"}'
curl -s -X POST localhost:28770/music -d '{"auto":true}'
```

The first command overrides the director, with the ordinary musical transition
and existing volume controls. The second restores game-driven music. It does
not change saved settings. `/state` reports the requested `score`; the WAV
render's `measurements.json` reports the actually admitted musical position.
The route exists only in dev-control builds and still binds only to loopback.

## Composition review, 10 October 2026

The review environment can render PCM and inspect scores/signals but does not
accept audio input. Therefore these passes are **not claimed as listening
reviews**. Listening files are supplied for human review.

1. Original manuscripts and eight-movement orchestration for each suite; render
   each movement and the transitions at 48 kHz. First model pass: 16-second
   excerpts per movement and 64-second tours.
2. Different harmonic routes and cue contours per suite; clearer compound
   metre and phrasing. Re-render all five suites and compare measurements.
3. Final balance/transition corrections and complete-form render; numerical
   headroom, tuning, interval, instrumentation and interruption tests.

Validation results and per-suite measurements are kept in `review.json`.
The final 45 renders peak at 0.4773 (more than 6 dB below full scale); every
transition tour retains its quickly dismissed draw cue after the third pass.

## Historical recordings (not used by the active score)

The remainder documents the previous bank for reproducibility. Its files are
retained but are no longer embedded into the score. The preparation script is
**not** a way to regenerate the current score; it regenerates the old 44.1-kHz
bank. Do not present these sources as meeting the new ≥48-kHz source brief.

## The three sources (all CC0 1.0; `docs/legal.md` §5 quotes each)

- **VCSL**, Versilian Community Sample Library, `sgossner/VCSL` at
  `c1ea7bcc3c7309650ab0da9d15c9cd1fbc4a4c7e`: alto and tenor recorders,
  bowed psaltery (pluck and long bow), folk harp, strumstick, Renaissance
  organ 8', tubular bells, Nepalese hand bells, sleigh bells, finger cymbals,
  frame drums, bass drum (the davul), timpani (the nakers), rope-tension
  snare, tambourine.
- **VSCO 2 Community Edition**, `sgossner/VSCO-2-CE` at
  `440300901dfe9275fd84e0b7763af1f8443ae62e` (recordings Sam Gossner and Simon
  Dalzell, editing Elan Hickler / Soundemote): solo violin, solo contrabass
  (arco and pizzicato), the violin, viola and cello sections (sustained and
  spiccato), French horn, tenor trombone, tuba, a timpani roll, and from its
  "VSCO 1 Percussion" folder a large ethnic drum (the taiko), a bass drum and
  its roll, a suspended cymbal's roll.
- **FreePats Bagpipe**, `freepats/bagpipe` at
  `496f2f6e82f226d650e270c0f0ad5febbebec249` (recorded August 2020 by Gilles
  Sadowski on a bagpipe in G by Rémy Dubois and Olle Geris): the chanter
  (F4 G4 A4 B♭4 C5 D5 E5 F5 G5) and both drones.

Each source's licence file ships beside the bank, byte for byte, with a
`NOTICE` naming the sources and recordists.

## How the bank is made

- **44,100 Hz mono PCM16**, peak-normalised, DC removed, leading silence
  trimmed. Plucked strings keep two seconds, bells 3.2, drums one.
- **Mono, measured**: most VCSL/VSCO files are spaced microphone pairs whose
  channels correlate weakly or negatively (`stereo_corr`); summing them
  comb-filters. A file correlating at 0.5 or more is averaged, any other ships
  its louder channel. The sampler pans every voice and the room is stereo, so
  no file ships in stereo; the organ (0.84–0.96) and the bells (−0.10–0.75)
  showed no image worth doubling their size for.
- **Pitch from measurement, the name as prior**: VCSL names and the VSCO
  contrabass, cello and horn names read one octave low (file `C4` sounds
  MIDI 72); a pitched recording that measures more than 60 cents off its
  expected note is refused. The chanter is recorded 24–65 cents flat, note
  by note: the SFZ's `tune=` is its `cents`, and the measurement must agree
  within 15 cents (E5, which the sketches misread, measures −63.4 against the
  SFZ's −65).
- **Loops**: a sustain's loop starts 0.3 s after its onset (0.6 s for the
  organ and the pipe's drones; the chanter keeps the SFZ's loop start) and
  ends, about 1.25 s later (2.4 s for drones), where a window best matches its
  start; a 60 ms crossfade is baked in, and the sample one past the loop's
  end is the loop's first, so playback reads straight across the seam
  (`a_loop_seam_is_no_jump`).
- 133 recordings, 19.6 MB (round 2 added the orchestral body and dropped
  the sleigh bells, a tambourine shake, the two highest bowed-psaltery and
  solo-violin notes). No choir: neither source has one. The completeness test
  (`bank_is_complete_and_embedded`) holds every row to its embedded bytes,
  every `.pcm` to a row, every source to the three above at the revisions
  `docs/legal.md` pins, and the whole under 20 MB.

## The melodies

Every melody is written for Baylee (`score/melodies.rs`): the table's theme
and its variants in C Dorian, the front door's in B♭ Lydian, tension's line
and the chanter's answer in G Aeolian, the hunt and three horn calls in F,
the climax in G Aeolian, and the three endings. The styles the brief names
(British and Slavic folk, medieval European music, fantasy scores) are
references in words only. `avoid.json` holds 23 openings; 17 were read from
fetched public-domain notation on 08.10.2026 (thesession.org, Mutopia,
Wikipedia's scores, ABC collections; Korobeiniki, Scarborough Fair, Skye
Boat Song, Loch Lomond, Auld Lang Syne, Kalinka, Palästinalied and the
Saltarello in every setting the sources disagree on). Ederlezi's notation was
not found, and the five film and game themes are interval shapes only, never
a recording or a score: those six are marked `verified: false`.

To hear it: `cargo run --release -p baylee-client-core --example music_demo -- <dir>`.
