# Unified public-domain-oriented score & hybrid 48-kHz Rust synthesizer

Baylee performs one continuous, beat-aligned stereo 48-kHz score across every
screen, unified around **D minor / D Dorian / F major / D major** so sign-in,
tavern, loading bridge, duel, and result cues flow together like one living piece:

- **Login (`Place::FrontDoor` → `Movement::Title`):** noble, atmospheric chamber
  and orchestral title suite in `score/memorial.rs` after George Frideric Handel
  (*Suite in D minor, HWV 437: IV. Sarabande*, 1733), Antonín Dvořák (*Symphony No. 9,
  Op. 95: II. Largo*, 1893) and Modest Mussorgsky (*Pictures at an Exhibition:
  Promenade*, 1874).
- **Lobby & Deckbuilder (`Place::Lobby | Place::Build` → `Movement::Lobby`):**
  cozy, intimate fingerstyle tavern music led by the physical-modelled **Acoustic
  Tavern Guitar** (`Instrument::TavernGuitar`) with folk harp, Renaissance lyre,
  cello/viola hearth warmth and subtle room murmur, after Gaspar Sanz (*Españoleta*,
  1674), Francisco Tárrega (*Lágrima*, 1891; *Adelita*, c. 1901), Fernando Sor
  (*Study Op. 35 No. 22*, 1828) and Johann Sebastian Bach (*Bourrée BWV 996*,
  c. 1712; *Prelude BWV 999*, c. 1720). See [memorial/README.md](memorial/README.md).
- **Loading / Arrival (`Place::Opening` → `Scene::Opening`):** sitting at a table,
  `DuelPhase::Opening`, and `DuelPhase::Playing` while `arrival::Journey::active()`
  still covers the screen hold a gentle D3–A3 open-fifth harmonic hush (`Tune::in_opening_hush`)
  while the Lobby guitar tail decays into the shared stereo room. **Table music
  begins cleanly at bar 0, beat 0 only after the loading cover clears.**
- **In-Game (`Place::Table` → `Scene::Table`, Calm to Epic):** `ScoreRequest` packs
  live `stack_depth` (`view.stack.len()`, 0..=7), `blockers`, and continuous
  `action_intensity` (`epicness()`) plus decaying 4.5-second spell/ability `flurry`
  into one atomic `u64` word. Across all fifteen suites, empty stack / quiet turns
  play **Calm** (`Movement::Standard`: chamber guitar, harp, flute, warm strings),
  developing boards and 2+ stack items or attackers step phrase-continuously into
  **Poised / Tense** (`Movement::Combat`: rhythmic ostinato, spiccato strings,
  French horn & cello counterpoint), and deep stacks (`3–4+`), big spells, or
  lethal danger bloom into **Epic** (`Movement::Endgame`: soaring brass & French
  horn swells, driving spiccato strings, pitched timpani, taiko/snare percussion,
  celesta shimmer), settling smoothly when the stack resolves.
- **Three Outcome Melodies (`Ending::Victory`, `Ending::Defeat`, `Ending::Draw`):**
  - **Victory:** radiant ascending **D-major** fanfare & triumphant chorale (after
    Mussorgsky, *The Great Gate of Kiev*, 1874, and Handel).
  - **Defeat:** somber descending **D-minor** cue & noble elegy (after Edvard Grieg,
    *Peer Gynt Suite No. 1, Op. 46: Åse's Death*, 1875/1888).
  - **Draw:** poised open-fifth (`D–A`) **D-Dorian** modal suspension (after Erik
    Satie, *Gnossienne No. 1*, 1890).

Five D-centric classical suites (`score/manuscript.rs`: `Ember` after Dvořák *Op. 95*,
`Glass` after Handel *HWV 437*, `Thorn` after Ludwig van Beethoven *Symphony No. 7,
Op. 92: II. Allegretto* [1812], `Tide` after Sanz/Bach, `Star` after Mussorgsky
*Pictures at an Exhibition* [1874]) pair with ten contrasting genre suites
(`score/styles/manuscript.rs`), all verified for consonant major/minor triad
membership and zero six-interval overlaps against `art/music/avoid.json`. Every
referenced composer died more than 100 years ago and every underlying work was
published before 1915 (worldwide public domain). The user's Risen files remain
private references, never copied, transcribed into the score or distributed.

## Ten further compositions and native 48-kHz synthesizer extensions

`score/styles/manuscript.rs` provides ten independent two-bar themes and fifty
authored phrases; `styles/rhythm.rs` supplies separate accompaniment idioms.
Piano nocturne, baroque chamber, folk jig, bossa nova, jazz waltz, synthwave,
chiptune, marimba minimalism, ambient and orchestral breakbeat each have all eight
movements and scale dynamically with `stack_depth` / `epicness()`. `Theme::DORIAN`
and `Theme::EXPLORATIONS` keep the two sets explicit; settings and automatic
rotation cover all fifteen.

Native 48-kHz synthesis (`orchestra/plucked.rs` and `orchestra/synth.rs`) renders
allocation-free alongside the 51 active pinned CC0 acoustic recordings:

- **Acoustic Tavern Guitar (`orchestra/plucked.rs`, `Instrument::TavernGuitar`):**
  16 polyphonic waveguide strings with symmetric 3-tap linear-phase bridge loss
  filtering and fractional Thiran all-pass tuning (`< 6` cents across `D2..D5`),
  two-pole thumb-flesh Drop-D bass vs finger-plucked nylon/gut treble excitation,
  three trapezoidal SVF soundboard body modes (`98 Hz` air cavity, `205 Hz` spruce
  top monopole, `395 Hz` wood cross-dipole) and subtle `D3`/`A3` sympathetic
  open-string resonators, alongside the 12-string Renaissance `Lyre` model.
- **Hybrid Orchestral Synthesizer (`orchestra/synth.rs`):** 14 band-limited
  wavetable patches across an 80-voice pool plus 4 drum/percussion synthesizers,
  adding warm brass ensemble swell (`FrenchHorn`), stereo-chorused bowed string
  ensemble with 5.4 Hz shimmer (`StringPad`), crystalline bell-plate (`Celesta`),
  and pitched circular-membrane kettle drum (`Timpani`, Bessel modal ratios) to
  the electric piano, clavinet, jazz guitar, warm pad, saw brass, sub-bass,
  8-bit pulse lead, marimba, glass bell, ambient drone and drum voices. Smooth
  `~0.80–0.85 s` release envelopes into the `~0.45 s` stereo room bridge every
  pulse-aligned transition without clicks or abrupt cuts.

Bank selection affects recordings only; native physical and additive models sound
the same under either setting. Output is always stereo 48 kHz.

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
cargo run --profile tools -p baylee-tools -- art originality
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

Public-domain-oriented fifteen-suite + Acoustic Tavern Guitar benchmark (`ci-release`,
3 rounds per bank): the five D-centric classical suites render 150 seconds of
multi-layer combat/orchestral audio in median **0.774 s (Studio)** / **0.798 s
(Original)** (~194× real-time), and the ten contrasting genre suites render 300
seconds in median **1.342 s (Studio)** / **1.341 s (Original)** (~224× real-time),
with deterministic floating-point checksums across all three rounds. One-time
startup preparation (including Studio sinc resampling across all table, title and
tavern pitches, 14 band-limited synth tables, percussion buffers and chamber bank
preparation) completes in **1.321 s** with zero render-time heap allocation. Every
movement across all fifteen suites and both banks peaks below **0.65** with zero
NaN or clipping. Automatic rotation remains sequential even when the arrival cue
counter wraps.

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

The existing generated bank and its source pins are retained. Native Rust
preparation replaces the former NumPy script. The following sections describe
all retained source assets; the active score uses only the
families named above. The preparation command regenerates source PCM at 44.1 kHz;
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
  trimmed. Plucked strings keep two seconds, bells 2.6, drums one.
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

## Native reproduction tools

`cargo run --profile tools -p baylee-tools -- art originality` checks the same 75 complete
phrases and the same six-directed-interval threshold, reports every nearest
match, and exits unsuccessfully for a match. Rests are removed before computing
intervals. This is still a limited signature check, not a legal originality claim.

Preparation needs native `ffmpeg`, `ffprobe` and `rustfmt`; all DSP, FFT,
channel selection, source/SFZ verification, PCM quantization and table generation
run in Rust. It needs neither Python nor NumPy. Paths use the launcher's repository
root, so the command also works through `cargo tools` from elsewhere.

`cargo run --profile tools -p baylee-tools -- art prepare` is an **offline, read-only parity
check** against all 133 source pins, manifest facts, shipped PCM bytes and the
generated Rust table. Raw recordings default to `$BAYLEE_ORCHESTRA_RAW` or
`/tmp/baylee-orchestra-raw`. A missing or changed cached file fails; no request
is sent unless `--download` is explicitly supplied. Downloads use the same
three pinned revisions and verify known hashes before entering the cache.

`--staging /tmp/baylee-bank-candidate` writes candidate `orchestra/`,
`samples.json` and `bank.rs` there, even when parity differs, then reports
those differences and exits unsuccessfully. It never publishes those candidates
into the repository bank. Inspect the candidate data before accepting changes.

`--write` publishes only after the complete bank passes the parity check.
Intentional recording or metadata changes require `--write --accept-changes`;
new rows acquire their source hashes for review at that point. A mismatch with
an existing **source** SHA-256 is always refused, including with that flag.
Files are published atomically one at a time; the entire bank is not a single
filesystem transaction. Stale PCM is pruned only during an explicit write.

`bank.rs` is generated by this tool and is never edited by hand. Its retained
historical generator header is accepted during read-only comparison; the next
explicit generation updates that header. This migration does not regenerate or
change the committed PCM, provenance manifest, or generated bank table.
