# Client art-direction handover — 17 September 2026

## Request and working agreement

The owner requested one coordinated visual redesign: player/phase bars,
table, hand/action dock, background, library backs and the library/graveyard/
exile fans. Full artistic freedom, original textures/shaders, atmospheric
animation. Keep card artwork unfiltered and gameplay untouched. Avoid repeated
Clippy and broad test runs; Opus can handle deeper validation later.

Credit balance is not exposed to this assistant. This handover is kept in the
checkout rather than depending on another model seeing the conversation.

## Implemented direction

- Midnight mineral cloth/leather, aged champagne tooling and stationary
  white/blue/black/red/green inlays. Decorative light moves slowly; semantic
  turn, priority and action signals retain their existing meanings.
- `shaders/felt.wgsl`: new base palette, fine mineral vein, brushed rail,
  engraved edge circuits, five fixed colour sectors, medallion tooling,
  apron trim. CPU base palette and bounded hue/saturation expectations in
  `baylee-client-core/src/tabletop.rs` updated together.
- `shaders/mat.wgsl`: recessed inner frame, corner shoulders and terminated
  lane rules. Lane boundaries and card placements unchanged.
- `shaders/sky.wgsl`: subdued daytime storm palette, blue-black night,
  distant drifting mist, softer celestial bodies and background-only framing.
  No colour-grade pass over card art.
- `frontal.rs`, `shaders/frontal.wgsl`, `hud.rs`, `hud/hand.rs`,
  `hud/ledge.rs`, `hud/seatbar.rs`: shared procedural hand/rail/seat finish,
  bounded cache of ten materials (two dock + eight seats), explicit virtual
  clock, reduced-motion handling and headless fallbacks. Framed hand well,
  stationary five-colour inlays and beveled action keys.
- Split seat bars now place a 220px identity plaque (14px bold name, 26px
  life) beside the twelve-step timeline and zone/turn status. A 10px joint
  and 641px minimum timeline give a total **871×48px** ink requirement;
  transparent hitbox height stays 54px. Core density arithmetic reserves
  the plaque before stretching tiles; compact fallbacks remain available.
- `textures.rs`: resident, deterministic 252×352 original sleeve texture,
  independent of downloaded backs. `table.rs` dresses the existing shared
  hidden-card material with it, including library slabs and fans. Other UI
  backs retain their pre-existing fetched-image policy.
- `baylee-client-core/src/layout.rs`: wider fan spacing, lower rise,
  gentler tilt, symmetric yaw and a seven-card footprint clamp. Existing
  Motion/glide path remains responsible for movement.
- `docs/client.md` records the new material direction; older casino-green
  rationale below it is historical, not the current brief.

## Verification already performed

- `cargo fmt --all`.
- `cargo test -p baylee-client-core --lib fan`: **15 passed**, including
  hidden-library, fan residency, bounded silhouette and clearance checks.
- `cargo test -p baylee-client-core --lib seatbar`: **18 passed**, including
  exact total span, plaque reservation, row content and density thresholds.
- `cargo build -p baylee-client --features dev-control,dev-reload`: passed.
  Both the first combined design and final Split composition build. The
  final attempt hit a 300-second timeout after the tests; a build-only retry
  succeeded. Build-lock contention contributed to the delay. Only the known
  macOS linker compact-unwind-size warning.
- Live Metal client: new table/sky/dock/sleeves render with no shader error;
  mulligan Keep advances to priority, and library hover opens a fan of backs.
- Final identity-plaque layout visually checked at 1728×1052. At 1280×768,
  both shelves select Compact: 812×28px ink inside 938.3×44.4px near and
  841.3×35.7px far shelves. Screenshot confirms the fallback and hand fit.
- Two paused full-frame screenshots: peak RGB difference **0 / 0 / 0**.
  After advancing 120 virtual frames: **57 / 58 / 62** full-frame; the empty
  hand-dock field alone changes **17 / 18 / 19**, independently of card glows.
  These establish virtual-clock animation and pause, not a live reduced-motion
  preference test.
- No Clippy, broad workspace test suite, browser build or multiplayer run.

## Runtime evidence (local, not committed)

Images are under `target/`; full-size PNGs use physical Retina pixels.
Control coordinates are logical pixels (1728×1052, scale 2 at first capture).

- `redesign-before-small.png`: old green table and thin toolbar rows.
- `redesign-after-small.png`: first combined material redesign.
- `redesign-fan-small.png`: open library fan and playable hand.
- `redesign-final-small.png`: final desktop identity-plaque composition.
- `redesign-compact-small.png`: final 1280×768 compact fallback and dock.
- `redesign-still-a.png`, `redesign-still-b.png`, `redesign-motion.png`:
  pause and animation evidence above.

Build first, then launch (startup may spend roughly two minutes before the
Metal adapter/control socket appears):

```sh
BAYLEE_DEV_CONTROL=28771 BEVY_ASSET_ROOT=$PWD target/debug/baylee-client
```

The client opens the lobby: Offline play, then Play the house. Use the
`dev-control` skill and `/state` rather than blind key scripts. An accepted
key may appear in state one frame later. The first older binary did not hot
reload source changes; only judge a shader from a confirmed fresh build.

## Final integration and follow-ups

- The coordinated design and final identity-plaque composition are implemented,
  built and visually inspected. Sources, documentation and this handover are
  committed together; temporary captures and runtime logs are not included.
- Opus: validate rotated multiplayer shelves, long names, extreme
  life totals, portrait layout, live reduced-motion on/off and populated
  graveyard/exile fans. Desktop library fan and base fan invariants were
  verified, as was the smaller desktop fallback; the other cases were not
  exercised live.
- Existing shader/source constant checks, frontal contrast tests and new
  procedural-sleeve tests were not run in the initial focused fan command.
- The historical test name `the_baize_is_a_casino_green` remains, but its
  assertions now require the new mineral palette with bounded saturation.
  The mandatory IDE rename operation was unavailable; no manual rename was
  substituted. Rename this misleading symbol with IDE refactoring later.
- Do not remove `.junie/plans`, imported gateway data, existing untracked
  configuration or unrelated changes. No third-party art was added.