# Memorial verification · 26 October 2026

The selected cloud portrait is the owner's approved V4, byte-for-byte unchanged.
All nine selected artwork/photo hashes match `assets.json`. Both brand masters
and the native icon exports were inspected. No original photograph was altered.

## Iterations in the real client

The passive macOS client ran on the built-in display, with separate XDG folders,
silent audio and loopback development endpoints. Final runs additionally denied
outbound connections except localhost. Every owned test process was terminated
and reaped; the harness stopped listening afterward.

- Checked the front and offline lobby at 1280 × 720 and 1600 × 950 logical pixels.
- Corrected cloud placement against actual panel bounds so its caption remains
  visible at the smaller size. Existing forms and lobby controls retain their layout.
- Fixed the transparent lobby root intercepting clicks on the background cloud.
  Opening the album from the offline lobby was then verified through real input.
- Verified hover, portrait/landscape photographs, arrow navigation, Tab/Enter
  opening, the modal focus trap, Escape and return to the rebuilt Baylee button.
  Added a regression for focus restoration and parking the login text caret.
- Checked the album at 1280 × 720 with XL text; all copy, navigation and Close fit.

Two complete 3200 × 1900 front-screen captures, separated by a two-second wait
plus capture overhead, showed moving light/cloud/portrait pixels. The same
check with `reduce_motion: true` produced byte-identical RGB pixels:

| Policy | Peak RGB delta (0–255) | Mean RGB delta | Changed pixels |
| --- | ---: | ---: | ---: |
| Default motion | 223 | 0.519108 | 815,563 |
| Reduced motion | 0 | 0 | 0 |

Both moving full frames were inspected. These measurements establish movement
and the still counter-test, not frame rate or absence of flicker. An optional
4-second, 12-Hz recording captured only 3 frames and dropped 45 under readback
backpressure; it is not used as animation-cadence or performance evidence.

## Automated and audio checks

- Client suite: **2,212 passed, 8 ignored**.
- Music suite: **43 passed**, including scalar/block rendering, headroom and
  exact waveform continuity between the title and lobby.
- Subsequent opener-focus regression and expanded sampled-pitch cache check passed.
- Final memorial tests: **4 passed** (album wrapping, complete event data, looping
  and continuity). Both world/portrait shaders passed the final shader check.
- Client/core Clippy with all targets and `dev-control,dev-reload`: no warnings.
- Rust formatting, Python Ruff and patch whitespace checked.

The complete 674-second, 48-kHz runtime render covers the 668.24-second musical
cycle and its tail, including a title-to-lobby transition. Peak sample amplitude
was 0.354979, RMS −25.021 dBFS and largest adjacent-sample step 0.053895; all
samples were finite and there was no clipping. This is a measured render, not
a claim of a listening-panel evaluation. Source editions, every one of the
4,506 notes, licence texts and preparation instructions are preserved under
`art/music/memorial/`.

Full workspace/database release gates were not run. This local branch has not
been pushed or landed; `cargo dev gate` and `cargo dev gate features` remain
required before doing so.

## Complete remembrance book · 27 October 2026

The approved German remembrance and its English translation are now embedded in
both products: eight chapters, 37 paragraphs each, with exact paragraph equality
checked between the native and Central Fluent files and the prepared source.
The story remains complete. The dates explicitly describe her time with the
family. The five original photo derivatives and approved cloud asset are unchanged.

The native book has a real scrollbar, independent photo navigation, chapter
boundaries, a fixed footer and serif prose. Its header reuses the existing GPU
cloud material; the three gold diamonds are geometry, avoiding a missing font
glyph found in the first render. Home/End and arrow scrolling now belong to the
focused reading column, so the chapter row can keep its own roving focus. Photo
changes preserve the scroll offset; chapter changes start at the top.

Live checks used a silent passive debug client (`dev-control,dev-reload`) on the
MacBook's verified built-in monitor (native index 0), isolated XDG folders and
loopback-only network sandbox. All eight chapters were opened through actual
virtual input, every German paragraph checked in the live tree, and the reading
column scrolled to its end. The final farewell, portrait and landscape photos,
English, 1280 × 720 with normal/XL text, Tab trapping, Escape/opener restoration
and opening the book from the offline lobby were checked. A photo change kept
the inspected paragraph at exactly y = 425 logical pixels. Close and chapter
controls stayed visible at XL size. Each owned client exited and its harness
port was verified closed. The last source-only cleanup extracted the unchanged
header/navigation construction into `album/chrome.rs`; final Clippy checks the
same construction and all test targets.

Two 2560 × 1440 book screenshots per policy, separated by two seconds:

| Policy | Peak RGB delta | Mean RGB delta | Changed pixels |
| --- | ---: | ---: | ---: |
| Default motion | 210 | 0.157544 | 63,841 |
| Reduced motion | 0 | 0 | 0 |

This verifies moving decoration and a completely still counter-test, not an
absence-of-flicker claim. The prose does not animate continuously.

Indicative six-second frame samples at 1280 × 720, normal text, album open,
virtual pointer outside, audio silent (debug/dev-control; not a release GPU
benchmark):

| Sample | FPS | Frame p50 / p95 ms | Update p50 / p95 ms | Entities | Allocated bytes/frame |
| --- | ---: | ---: | ---: | ---: | ---: |
| Previous five-photo draft | 60.01 | 16.630 / 18.447 | 5.896 / 6.578 | 774 | 346,224 |
| Complete eight-chapter book | 59.77 | 16.591 / 18.357 | 6.211 / 6.807 | 811 | 364,061 |

The baseline is the preserved earlier `client-6` memorial build; its music
revision differs (silent in both), so these samples are not a strict same-commit
A/B attribution. The complete reading tree adds 37 entities, with about 0.32 ms
higher measured median update time in this run; both remained near the 60-Hz cap.

Automated checks: 2,176 client library tests passed, 8 ignored; 5 selected core
memorial/music checks and 13 localization checks passed. Final client/core Clippy
with all targets and `dev-control,dev-reload`, formatting and patch whitespace
passed. The real render also exposed an existing initial-room-focus race: the
cloud could name the room before its seat tree existed. Initial focus now waits
for the real stop; the existing regression passes. The German sample-rate help
was reworded to preserve its existing 44,1 kHz display contract. No audio changed.

Central's matching book passed 215 unit tests, 34 browser tests and its normal
Rust/PostgreSQL/smoke/secret checks; its pre-existing crypto-bigint RustSec finding
still blocks a complete release gate (see that repository's security record).
Native full workspace/database release gates were not run. Both changes remain
local and were not pushed or deployed.

## Parchment, focus and photograph refinement — 27 October 2026

Multiple review/fix passes covered the native client and Central. Argon High
(`agy --model gemini-4-argon-high --effort high`) reviewed source and then PNG
frames/contact sheets from Dev-Control video captures. Direct MP4 ingestion in
headless agy attempted a command that its permission policy denied; no settings
were weakened. Its successful visual review used direct image tools and explicitly
did not claim video playback. Reports are included with the local evidence.

Confirmed fixes: static portrait UVs and antialiased ornament contours; removed
pointer ornament; retained cloud geometry across rebuilds; shared shader clock
and guarded idle material writes; live scrollbar memory before actions; old leaf
scroll/width/ornament parity; pre-layout full-photo geometry; initial and same-frame
rebuilt modal focus; caret parking before opening; chapter key repeat and modifier
ownership; direction retained through photo wrap; selected bookmark contrast while
hovering. Full-image loading keeps requested zoom; web image elements are keyed
per photo, and dragging from paper to backdrop cannot dismiss the book.

New owner-supplied photographs bring the album to ten. Per-photo crop anchors,
rounded preview corners, light vignette, finite slide transitions and elevation
shadows are independent of full-resolution uncropped originals. EXIF and motion
trailers are stripped from the new shipping copies; the five existing previews
are unchanged. Eight chapters and all 37 paragraphs
remain, with the family-requested gentler tumour wording in both languages.

Initial native scale-1 recordings: four 5-second captures at 800×500, 8fps, virtual speed
0.15, each 40 saved / 0 dropped. Cloud ROI x8/y185/w156/h104: max adjacent channel
delta 2/255, average absolute channel delta 0.00174; paused control entirely
pixel-identical across all raw frames. Full-frame changing pixels also include
the gateway caret, so full-frame peaks are not evidence of portrait flicker.
The five-second leaf clips end shortly before the 0.72s virtual turn completes;
a thin final strip is expected at that sampling boundary, not a stuck leaf.
Four final 6.5-second captures save 52/52 frames each without drops. Both leaf
turns settle completely. The final hover ROI has max channel delta 2/255 and
mean 0.00130; the paused control remains pixel-identical. Scrolling naturally clips partial
lines at the viewport edge; every paragraph remains reachable. The dim world
visible beyond the parchment is intentional modal context.

Live narrow/wide XL hub checks also found and fixed the cloud being blocked by
panels: the hub now reserves its own sky strip. The caption stays on one line,
and both memorial scrims stand above transient toast notifications.

Normal and XL native screens plus desktop/phone website screenshots were inspected.
Portrait and landscape originals fit without crop; zoom and bounded pan work.
The normal 1280×720 native book sample (6 seconds, no recording) measured 59.87fps,
update p50 6.289ms / p95 6.865ms, 835 entities. These are local observations, not a
cross-device performance guarantee. All agent-started clients and helper processes
were stopped and their dev-control ports checked closed.

Detailed final gate results and final revision identifiers are preserved in the
local QA report alongside recordings; no external release or main merge is implied.
