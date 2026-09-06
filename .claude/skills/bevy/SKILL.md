---
name: bevy
description: Bevy 0.19 API facts — resources as components, the BSN scene macro, the render-graph-as-systems change, the new text and UI types, and what this repo's client does instead. Use when writing or changing anything in baylee-client, when a Bevy API does not exist as remembered, or when evaluating a Bevy upgrade.
---

# Bevy 0.19

Verified 2026-09-06 against docs.rs and the official release notes. The latest
published crate is **0.19.1** (2026-08-13); 0.19.0 landed 2026-06-19 and 0.18.0
in January 2026. This workspace depends on `bevy = "0.19"` with
`default-features = false`, so check the feature list in the root `Cargo.toml`
before using a subsystem — a missing feature looks like a missing API.

Bevy ships breaking changes roughly quarterly and writes a migration guide for
every release (`bevy.org/learn/migration-guides/`). **Treat any Bevy knowledge
older than the pinned version as a hypothesis**, and confirm against
`docs.rs/bevy/0.19.1` before asserting a type or method exists.

## What changed in 0.19 that most invalidates older habits

- **Resources are components.** A resource is stored as a component on a
  singleton entity. Lifecycle hooks and observers now work on resource types,
  and resources can participate in relationships.
- **The render graph is gone as a separate thing.** `RenderGraph` was replaced
  by ECS schedules: render passes are ordinary systems in the `Core3d` /
  `Core2d` schedules on the render world. The `Node` trait boilerplate that
  older tutorials show no longer applies.
- **BSN scenes.** The `bsn!` macro is the scene syntax (composable patches,
  scene functions, `#Entity` references, `.bsn` assets, `#[derive(SceneComponent)]`).
  Feathers widgets were migrated to it.
- **Text is richer and differently typed.** `TextFont` gained `weight`, `width`
  and `style`; `FontSize` is now an **enum** (`Px`, `Vh`, `Rem`, …) rather than
  an `f32` — a silent source of type errors when porting older code. `FontSource`
  accepts a handle, a family name, or a semantic category (`Monospace`).
  `LetterSpacing` is its own component, and `EditableText` provides real text
  entry with IME, selection and clipboard.
- **New rendering features:** contact shadows (per-light `contact_shadows_enabled`),
  rectangular area lights (needs the `area_light_luts` feature), parallax-corrected
  cubemaps, `Vignette` and `LensDistortion` post-processing.
- **Useful odds and ends:** `commands.delayed().secs(1.0).spawn(...)`,
  `gizmos.text()`, `InfiniteGridPlugin`, `DiagnosticsOverlay`, `TransformGizmoPlugin`,
  observers supporting `.run_if()`, and `contiguous_iter()` for SIMD-friendly bulk work.
- **`RenderErrorHandler`** lets the app respond to `DeviceLost` / `OutOfMemory` /
  `Validation` instead of panicking.

## How this repo's client is built, and why

Read `docs/client.md` first; it is normative. The shape to respect:

- **The renderer holds almost no logic.** `baylee-client-core` has the layout,
  board model, interaction state machine, mana planner, i18n and lobby
  decisions, knows no renderer, and carries the bulk of the client's tests.
  New behaviour goes there; `baylee-client` only draws it.
- **Nothing is positioned directly.** `table::sync_scene` writes a `Motion`
  target and `table::glide` moves it. Bypassing that desynchronises the scene
  from the board model.
- **The stage has no lights at all** and the camera carries
  `Tonemapping::None`, deliberately: scene lighting on card art destroys colour
  identity. Everything on the table is `unlit`.
- **Input asks actions, never keys** — the account's `Keymap` resolves them in
  `keys.rs`.
- Text on the 3D table does not exist; numerals are a 4×6 stencil in the shader.

## Debugging render bugs

Measure before theorising. A clear colour touches no material, texture or
shader, so rendering a known colour and reading the pixel back separates "the
shader is wrong" from "the whole pipeline is wrong" — that is how a
four-times-too-dark felt and a full-screen opaque overlay were found here.
Assertions on brightness must bound **both** sides; a one-sided "dark enough"
check let the felt bug through.
