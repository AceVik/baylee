# In sunlight · Baylee's remembrance

A flowing grand-piano performance with restrained violin and cello, replacing
the earlier harp-led three-Lieder arrangement. Its complete cycle is 511.17
seconds. Login and lobby share the same uninterrupted performance.

## Repertoire and permissions

1. Claude Debussy (1862–1918), **Clair de lune**, Suite bergamasque, L75.
   [Mutopia edition 1778](https://www.mutopiaproject.org/cgibin/piece-info.cgi?id=1778),
   Keith OHara, after E. Fromont (1905), **Public Domain**.
2. Erik Satie (1866–1925), **Gymnopédie No. 1**.
   [Mutopia edition 37](https://www.mutopiaproject.org/cgibin/piece-info.cgi?id=37),
   Evin Robertson, **Public Domain**.

The works and these particular digital editions are public domain.
[Mutopia's terms](https://www.mutopiaproject.org/legal.html) explain the public-domain
contributions. Checked 26 October 2026. Original LilyPond scores and published
MIDI files are retained unchanged in `sources/`; exact URLs and SHA-256 hashes
are in `sources.json`. No Yiruma composition or commercial performance is used.

## Performance

Reproduce and compare every performance byte with:

```sh
cargo tools art memorial score
```

All 1,923 piano notes are retained: 1,468 in Debussy and 455 in Satie. The Satie
MIDI writes both endings without unfolding the common section. Preparation
performs the written repeat: 31 common bars plus the first eight-bar ending,
then those 31 bars plus the second ending. It lasts 78 bars / 234 quarter beats.
Debussy lasts 322.5 quarter beats in 9/8.

The performance intentionally replaces the MIDI's flat/absent dynamics with
an authored continuous tempo and velocity map. The source score's rubato,
un poco mosso, animando, calmato and morendo guide the Debussy arc. Melody
notes speak above quieter accompaniment; phrase ends breathe. Piano velocity
ranges from 29 to 103, selecting and blending two real recorded layers.
Bounded half-pedal extends note holds toward the next left-hand onset; release
envelopes preserve natural decays without accumulating long harmonic smears.
Small deterministic timing variation is fixed during preparation.

Violin doubles only selected sustained melodic phrases (79 notes); cello
supports selected written bass notes (51 notes). The opening and fast piano
figures remain solo piano. Seven seconds between works allow the room to decay.
`performance.json` records counts, durations and the event SHA-256. The runtime
reads 20-byte BAY2 events and performs the music live, with no audio parsing,
allocation, pitch resampling or network activity in the callback.

## Recorded instruments

- **Osiris Piano**, Versilian Studios / Karoryfer Samples: a Yamaha C2 grand,
  soft pedal, sustain-pedal-on, low-noise Mic A recordings, **two velocity
  layers, real stereo, 48 kHz PCM16**. The
  [publisher](https://shop.karoryfer.com/pages/free-osiris-piano) and its
  [free-sample terms](https://shop.karoryfer.com/pages/free-samples) specify
  **CC0 1.0 Universal**. Pinned source:
  `sfzinstruments/Osiris_Piano@18c6afccb60cff458edbf7c394571783e074e1e9`.
  Source SFZ key centres determine pitch; filenames use a different octave
  numbering and must not be interpreted as standard scientific pitch names.
- **VSCO 2 Community Edition**, solo violin Arco Vib piano and cello-section
  susvib v1: full stereo 44.1-kHz recordings, **CC0 1.0 Universal**.
  [Publisher](https://versilian-studios.com/vsco-community/), pinned source:
  `sgossner/VSCO-2-CE@440300901dfe9275fd84e0b7763af1f8443ae62e`.
  Credits: Sam Gossner, Simon Dalzell; editing Elan Hickler / Soundemote.
  Existing measured tuning corrections are retained where available.

The exact CC0 legal texts ship in `crates/baylee-client-core/assets/chamber/`.
No finished third-party musical performance or Wizards audio is used.

Verify the shipped bank, then optionally reconstruct every recording with:

```sh
cargo tools art memorial bank
cargo tools art memorial bank --raw /tmp/baylee-piano-revision/raw --soxr /path/to/libsoxr.dylib
```

The tools are Rust-only. The approved `instruments.json` is also the source-selection
recipe: every recording, pitch, layer, duration and tuning is explicit. Reconstruction
requires native `ffmpeg`/`ffprobe` and an explicitly selected libsoxr 0.1 shared
library. The reference build uses
[`dofuuz/soxr@a66f3eeeeb62a32403ff143b756eed92b1ec6b62`](https://github.com/dofuuz/soxr/tree/a66f3eeeeb62a32403ff143b756eed92b1ec6b62)
(0.1.3-14, LGPL-2.1-or-later), an external authoring dependency, not distributed
with the client. No Python runtime or subprocess is used.

Downloads require `--download`, use pinned revisions **and source hashes**, and
cache outside the repository under the chosen `--raw` path. Without `--raw`,
only the shipped bank is verified and no external program is required.
The score check needs neither external programs nor network access.

Reconstruction refuses differing PCM. `--output` writes a separate candidate
score file or bank directory only after parity succeeds; it never replaces the
approved playback assets. Rust zlib packing may produce different compressed
bytes, so candidate banks include updated compression hashes alongside the
unchanged PCM hashes. Existing source licence files remain in the asset folder.

Offline libsoxr VHQ prepares exact pitches at
48 kHz. Source stereo, recording levels and the natural tail are retained;
only leading silence/DC and an end taper outside the played release are removed.
High keys that decay before key-up receive trailing silence, never a piano loop. No per-note
normalization, denoising or lossy encoding is applied. These 157 clips need
no repeated bow sections; their original recordings cover every arranged hold.
Prepared PCM is 16-bit stereo, matching the piano's original depth. A 24-bit
runtime WAV does not imply 24-bit piano recordings.

`instruments.json` records every raw URL/hash, recording format, tuning,
trim, prepared PCM hash and packed hash. Four lossless zlib chunks keep each
Git blob below 32 MiB; together about 127 MiB, decoded once to about 175 MiB
before playback. The render thread uses integer positions into those exact
pitch recordings, stereo-preserving balance, a fixed voice pool, smooth piano
key release and bowed envelopes. The existing continuous room remains active.

## Audition and checks

```sh
cargo run --release -p baylee-client-core --example music_demo -- /tmp/baylee-piano-audition 520 memorial
cargo test --release -p baylee-client-core music:: --lib
```

The demo renders the actual runtime to stereo PCM24/48-kHz WAV, switching from
login to lobby at 31.13 seconds. It rejects non-finite or clipped output and
reports peaks, RMS, sample jumps and transition positions. Tests check source
bank/PCM integrity, score coverage and tail length, finite output/headroom,
scalar/block equality, release continuity and fixed voice capacity. Numerical
checks do not replace the owner's subjective audition of timbre and feeling.

The complete 520-second runtime verification, including the next loop opening,
peaks at 0.7653 (about −2.32 dBFS), with no sample reaching the 0.85 safety knee.
The mix was revised once after the first climax reached that knee; the final
piano gain preserves the dynamic arc without limiter coloration. See
`review.json` for both passes and section levels. The login-to-lobby request
at 31.13 s was admitted at 31.163 s; waveform continuity is also tested exactly.

## Native-tool migration (28 October 2026)

Both former Python preparation tools now live under
`crates/baylee-tools/src/art/memorial/`. Migration validation rebuilt all 2,053
performance events and all 157 stereo clips from the cached original sources;
every performance and PCM byte matched the approved hashes. Shipped music,
source editions and recorded-bank files were not modified. Tests exercise
truncated MIDI, repeat unfolding, source-score parity, stereo fades/padding,
and invalid PCM. Native resampler boundary/stereo tests run when
`BAYLEE_TEST_SOXR` names the reference library.
