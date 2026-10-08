# Baylee's score: the sample bank and its provenance

The score is Baylee's own composition and code
(`crates/baylee-client-core/src/music/`, AGPL-3.0-only): a modal score around
B♭ for old instruments, performed live by one sampler and one musical clock
(`docs/client.md` §"One orchestra follows the player"). This directory holds
what the recordings it plays are made from and how.

| File | What it is |
|---|---|
| `prepare.py` | Rebuilds the bank: downloads every recording at its pinned commit, checks its SHA-256, writes the mono 44,100 Hz PCM16 files under `crates/baylee-client-core/assets/orchestra/`, this `samples.json`, and the generated table `crates/baylee-client-core/src/music/bank.rs`. |
| `samples.json` | One row per shipped recording: source, path, revision, the original's SHA-256 and size, how it was made mono, its measured pitch and cents, the loop, attack/release/room, and the prepared PCM's SHA-256. |
| `avoid.json` | The openings of tunes the score must not echo, as directed semitone intervals, each with its source and whether it was read from a fetched public-domain notation. |
| `originality.py` | The composer's check: every melody against every opening, six shared intervals in a row fail. The test `no_melody_echoes_a_tune_we_must_not` runs the same check in CI. |

Rebuild (needs numpy and ffmpeg; from the repository root):

```sh
uv run --with numpy python3 art/music/prepare.py
python3 art/music/originality.py
```

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
  (arco and pizzicato), cello section, French horn.
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
- 99 recordings, 15.6 MB. The completeness test
  (`bank_is_complete_and_embedded`) holds every row to its embedded bytes,
  every `.pcm` to a row, every source to the three above at the revisions
  `docs/legal.md` pins, and the whole under 16 MB.

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
