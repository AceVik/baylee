# Fifteen suites: five Dorian originals plus ten contrasting styles

Five independently composed two-bar title themes, each developed into eight
movements. See `score/manuscript.rs` and the table in `docs/client.md` for
motifs, phrase structure, instrumentation and faster tempos. The revised score
uses consonant major/minor triads; augmented/diminished chords and chromatic
neighbours have been removed. The user's Risen files are private references,
never copied, transcribed into the score or distributed.

## Ten further compositions

The original five remain intact. `score/styles/manuscript.rs` adds ten independent
two-bar themes and fifty authored phrases; `styles/rhythm.rs` supplies separate
accompaniment idioms. Piano nocturne, baroque chamber, folk jig, bossa nova,
jazz waltz, synthwave, chiptune, marimba minimalism, ambient and orchestral
breakbeat each have all eight movements. The table in `docs/client.md` gives
keys and tempos. `Theme::DORIAN` and `Theme::EXPLORATIONS` keep the two sets
explicit; settings and automatic rotation cover all fifteen.

`orchestra/synth.rs` supplies eleven original additive patches and three original
percussion sounds. These are native 48-kHz models, not sampled acoustic instruments
or imported synthesizer presets. Recorded layers continue to use only the same
51 pinned CC0 recordings. Bank selection affects recordings only; native models
sound the same under either setting. Output is always stereo 48 kHz.

## Two sound banks for every suite

- **Studio · 48 kHz:** real VCSL/VSCO 2 CE recordings, prepared at every used
  pitch with 32-tap windowed-sinc resampling. Natural attacks and sustain loops;
  bowed bass throughout. Prepared floating-point frames make playback cheap.
- **Original · 44.1 kHz:** the existing mono PCM16 recordings, decoded once,
  played with cubic interpolation at the same continuous 48-kHz output rate.

Both banks derive from **44.1-kHz recordings**, not native 48-kHz source audio.
Higher output rate does not add information missing from a recording. The
quality improvement over the previous version comes mainly from real acoustic
articulation replacing static oscillator timbres, plus resampling, phrasing,
and a linear mix. Lyre remains an original native 48-kHz string model.
All shipped recordings are CC0; composition/code use AGPL-3.0-only.

## Audition and measure

```sh
cargo run --profile ci-release -p baylee-client-core --example music_demo -- /tmp/baylee-music 180
cargo run --profile ci-release -p baylee-client-core --example music_bench
python3 art/music/originality.py
```

The demo renders the exact runtime code: 240 movement files and 30 off-beat
transition tours, PCM24 stereo, in `studio48` / `original441` folders. JSON
records source/output rates, peaks, RMS, adjacent steps, and admitted changes.
The benchmark prepares outside the timed render, runs 150 seconds of combat
music three times per bank for the original group, then 300 seconds for the
new ten. Append `new` or `dorian` after demo seconds to render only that group.
Preparation is reported separately. It has no file
I/O in the timed section. Comparisons include the changed score's workload.

Before the ten-style extension, measured on this Mac, optimized `ci-release`, three runs: median 0.534 seconds
(Studio) / 0.581 seconds (Original) for 150 seconds of music, versus 0.925
seconds for the earlier model score. These are whole-score comparisons, not
isolated resampler benchmarks. Preparation: 0.233 seconds. The active PCM
payload is 6,595,212 bytes, selected at compile time from the historical
19,577,282-byte bank; decoded Original PCM is 13,190,424 bytes, and 135 cached
Studio pitches occupy 36,730,796 bytes. No additional sound files are fetched.

Final fifteen-suite check: 300 seconds across the ten new combat arrangements
render in median **1.043 s (Studio)** / **1.047 s (Original)**. The original
five still render 150 seconds in **0.539 / 0.582 s**. Preparation is **0.311 s**;
195 cached recorded pitches occupy **49,907,064 bytes**. Original decoded PCM
and embedded recordings are unchanged. New synthesized wave/percussion data
add **343,512 bytes**. These are local whole-score measurements, not a promise
for every device. The new final renders peak below **0.420**, with all twenty
tours completing their ten changes. The 90 baseline PCM checks for the old five
are byte-identical. All 32 suite/rotation-bank selections persisted in the live
client; all 240 preview requests and compact-window keyboard navigation passed.
Automatic rotation remains sequential even when the arrival cue counter wraps.

Live development preview (no change to saved settings):

```sh
curl -s -X POST localhost:28770/music -d '{"theme":"star","movement":"endgame","samples":"studio48"}'
curl -s -X POST localhost:28770/music -d '{"theme":"star","movement":"endgame","samples":"original441"}'
curl -s -X POST localhost:28770/music -d '{"auto":true}'
```

Omitting `samples` selects Studio. Invalid names are rejected. `/state` reports
requested theme/bank; the export reports the actual musical position.
Both settings rows support pointer and keyboard, persist per device, and keep
old volume/theme settings when the new bank field is absent.

## Review

The environment does not accept audio input: reviews use score inspection,
signal analysis and runtime tests, **not a claimed subjective listening pass**.
For the original five, three rounds per suite/bank were rendered: 16-second sketches, 64-second
balance checks, and 180-second complete-form excerpts, each with transition
tours. Round two removed always-on saturation and softened psaltery transients;
round three distinguished the harbour motif further and interpolated sinc
phases to suppress conversion steps. The final 90 files peak below 0.530;
all ten tours retain the promptly dismissed draw cue.
The local audition pages supply both banks and all three render rounds.
The ten-style extension uses 16/64/96-second rounds plus 64-second tours.
`review-fifteen.json` records its checks, preservation evidence and measurements.
`review.json` records actual checks and measurements. The old model-only audit
is retained as `review-models.json`, with its original scope/date.

The five titles develop two-bar themes through statement, answer, return,
cadence and a contrasting bridge; tavern and table movements quote those themes
in quieter registers. Major/minor changes alter the melody and harmony together.
Every arranged pitch is tested against its intended triad, with separate
rising/falling/open-fifth result-cue checks. `avoid.json` is only a limited
interval-signature check against its listed openings; passing is not a claim
of legal originality against all existing music.

## Recording provenance and reproduction

The existing generated bank and preparation script are unchanged. The following
sections describe all retained source assets; the active score uses only the
families named above. The preparation script regenerates source PCM at 44.1 kHz;
it does not generate compositions or turn them into native 48-kHz recordings.

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
