# Verification · 26 September 2026

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
