# Verification · 26 September 2026

## Dedicated room and fixed login foreground

- Created a room without a preselected deck, then configured its name, three
  seats, three free mulligans and the five-land starting template through real
  pointer/keyboard input. Selected a different deck inside the room.
- Native final passes completed at **1920×1080** and **932×430** logical pixels.
  Smartphone support targets landscape. Both login notice/source bounds stayed
  inside the viewport; the final lobby and room contain no legal footer, as
  requested. Login, account creation and gateway selection remain scrollable.
- Iteration exposed a room scroll surface that ignored wheel input over blank
  space. The page now receives those gestures; the full configuration and deck
  selection flow passed in the short landscape viewport after the fix.
- The gateway status/warning share one column, and login fields have a common
  inset surface. Pointer parallax is removed from the frame, cat and lantern,
  while distant scenery retains it.
- Changed-crate library suites: **5,750 passed, 4 intentional ignores**; gateway
  HTTP/database/engine room tests: **9 passed**. Final affected library rerun:
  **2,232 passed, 4 intentional ignores**. After the last layout adjustment,
  all **165 lobby tests** passed. Workspace all-target clippy with warnings
  denied, formatting and browser-target compilation passed.
- Gateway tests cover host-only writes, password entry, occupied-seat protection,
  atomic invalid AI edits, earliest-joined host succession, readiness reset and
  configured life, permanents and free mulligans reaching the actual engine.
- Planechase remains explicitly unavailable: there is no planar rules engine.

## Login and lobby layout refinement

Three native visual passes, with a final typography pass, using a disposable
loopback gateway and a separate client profile:

1. Centred the logo/form composition on tall displays, widened desktop forms,
   kept the gateway-save label on one line, bounded the lobby to 1480 px and
   separated its heading, search tools and room controls. Replaced bare empty
   notices with original card-back ornaments, explanations and useful actions.
2. Compared actual 1920×1080 and 2560×1440 renders. Increased panel density,
   removed premature hosting controls without a deck, and verified the search
   reset by clicking through no-results back to the unfiltered empty listing.
3. The 390×844 render exposed overlapping header/guest bands. Prevented header
   shrinking and made phones scroll the full lobby, with content-sized panels.
   Moved the interior panel's engraved shoulder to its rim so larger headings
   no longer cross a decorative rule.

- Final notice/source bounds remain inside and centred in all eight native
  viewport sizes: **2560×1440, 1920×1080, 1280×768, 900×700, 390×844,
  320×568, 844×390 and 320×320**. The smallest view also passed guest entry,
  scrolling to search, submitting it and clearing it. Its footer remains fixed.
- Measured search/input/button centre heights match within 1 logical pixel on
  desktop. A populated fixture with one deck and one open table also verified
  password, chair count and room-submit alignment at 1280 and 1920 px.
- Client: **1,056 tests passed, 2 intentional ignores**. Client core:
  **1,005 passed**. Clippy (all client targets, dev-control/dev-reload, warnings
  denied), formatting and wasm compilation passed. The new regression presses
  the no-results recovery and checks that no hosting action is offered without
  a deck. Existing arrival-input and offline-game tests pass as well.
- No account data or real tables were changed during visual testing. These are
  native simulated viewport checks, not a mobile-device performance benchmark.

## Spatial garden / shared interiors follow-up

- Kept the painted architecture and added analytical 3D water intersections,
  travelling wave normals, perspective reflection/refraction and a wind-driven
  cloud volume that occludes the luminous moon. This is a hybrid scene, not a
  fully modelled landscape. Waterfalls, fireflies and parallax remain animated.
- Native login, guest arrival, lobby and a new deck were exercised against a
  disposable local gateway. Lobby and editor now share the garden, dimmed behind
  translucent reading surfaces; the same virtual clock continues across them.
- Notice and source-link bounds are fully inside the viewport and horizontally
  centred (within 1 logical pixel) at **1280×768, 390×844, 320×568, 844×390 and
  320×320**. The quoted notice's words and punctuation remain pinned in a test;
  only whitespace changes to format the sentences symmetrically.
- Native moving frame pair showed channel difference peaks [170, 164, 177].
  Pausing the virtual clock produced a pixel-identical frame pair. The actual
  reduced-motion preference also produced identical portrait frames with the
  virtual clock running, including the new water and cloud geometry.
- Final client unit suite: **1,055 passed, 2 intentionally ignored**. Native
  clippy (all targets, dev-control/dev-reload, warnings denied), shader validation
  and wasm target check passed. Actual native GPU rendering was reviewed in
  all three screens; real mobile-GPU performance remains unmeasured.

## Painterly / piano follow-up

- Replaced the photographic guardian after two repaint passes; replaced the
  logo, app icon, browser icons and macOS icon together. Native 1280×768 and
  phone renders reviewed for a consistent painted light and readable wordmark.
- Replaced the black footer plate with blue translucent mist extending 52 px
  into the scene. Both legal text bounds passed again at 1280×768, 390×844,
  320×568, 844×390 and 320×320; copy retains its reserved scrolling boundary.
- Client unit suite: **1,055 passed, 2 intentionally ignored**. All six core
  music tests passed; native clippy with warnings denied and wasm check passed.
- Removed flute, added 15 recorded piano samples (five registers, pp/mf/f).
  Prepared sample hashes are pinned in the manifest. Spectral harmonics verified
  C2–C6 roots, avoiding confusing the strong second/third partial with the root.
- Revised runtime score rendered 150 s in 33.08 s (debug). Peak 0.389,
  maximum adjacent-sample step 0.069; sanctuary RMS 0.057, quiet battle 0.079,
  intense battle 0.109. Musical changes still use the same uninterrupted clock.

## Initial sanctuary milestone

- Native Client + Client Core, all targets, dev-control/dev-reload: **2,113
  tests passed, 2 explicitly ignored audio export tests**.
- Native clippy with warnings denied; browser target compilation checked.
- After the final scroll repair, all 164 lobby tests passed, including the
  nested-scroll regression; native clippy remained clean.
- Rendered notice and source-link bounds measured at 1708×1032, 1280×768,
  390×844, 320×568, 844×390 and 320×320 logical pixels. Both text boxes stayed
  wholly inside every viewport. The form scrolls independently of the footer.
- The actual reduced-motion preference also produced a pixel-identical pair
  in the final portrait render, with the virtual clock running.
- Moving frame pair: 5,171,413 / 7,050,624 pixels changed; channel peaks
  [166, 189, 201]. Paused pair: **zero** changed pixels.
- Actual pointer clicks verified music mute and ten-percent volume steps in
  both the gear menu and the running offline table. Opening the menu itself
  preserved volume/mute. Login was exercised against a local disposable test gateway.
- The exact runtime sampler rendered 150 seconds in 13.35 seconds in a debug
  build. WAV peak remained below 0.43; maximum adjacent-sample step 0.086.
  Sanctuary RMS 0.064, restrained battle 0.097, intense battle 0.133 (before
  user master gain). All score-state unit checks passed without clipping.

The original generated painting was refined into independent near/far layers.
Baylee was refined twice for a fitted red body, then a petite 2.5 kg silhouette,
short coat, tapered tail and closer photographic likeness. A rendered pass
removed blocky procedural sky wisps and unnecessary darkening above the new
opaque footer. Short viewports use a height-limited logo. Nested lists at their
scroll limit now hand the gesture to the outer form; a dedicated regression
test covers filled, scrolling and exhausted inner lists. Empty space between
fields also receives scroll gestures. A final native 844×390 pointer/wheel
check exposed the last action while both legal text boxes stayed fixed.

These are native desktop render and automated audio checks. Real mobile-GPU
performance and browser playback latency have not been measured; the browser
build compiles. The listening demo is intended for artistic review, not a
claim of having recorded a live orchestra session.
