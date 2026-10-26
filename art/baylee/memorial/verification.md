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
