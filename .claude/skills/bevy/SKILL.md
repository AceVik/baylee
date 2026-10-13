---
name: bevy
description: Bevy 0.20 API facts (and the 0.19 ones beneath them) — WESL shaders, flat pointer events, the deprecated Button/Interaction, Tonemapping::None as passthrough, resources as components, BSN, the render graph as systems, the text and UI types, and what this repo's client does instead. Use when writing or changing anything in baylee-client, when a Bevy API does not exist as remembered, or when evaluating a Bevy upgrade.
---

# Bevy 0.20

The workspace moved to **bevy 0.20.0** (released 2026-10-08) on 13.10.2026,
following the official migration guide
(`bevy.org/learn/migration-guides/0-19-to-0-20/`). It depends on
`bevy = "0.20"` with `default-features = false`, so check the feature list in
the root `Cargo.toml` before using a subsystem — a missing feature looks like
a missing API. bevy 0.20 and its `wesl` shader compiler declare
`rust-version = "1.97.1"`, which is the workspace MSRV.

Bevy ships breaking changes roughly quarterly and writes a migration guide for
every release. **Treat any Bevy knowledge older than the pinned version as a
hypothesis**, and confirm against `docs.rs/bevy/0.20.0` — or the source in
`~/.cargo/registry/src/*/bevy_*-0.20.0/` — before asserting a type or method
exists.

## What changed in 0.20 that bites this client

- **Shaders are WESL.** naga_oil is gone. Our shaders are
  `crates/baylee-client/src/shaders/*.wesl`; a `.wgsl` file is passed to wgpu
  verbatim, so a `#import` or `#{…}` left in one fails only when the pipeline
  is built (a missing object, not a crash — grep the log for
  `ShaderCacheError`/`ModuleNotFound`). Imports come first, end with `;`, and
  name the module by its crate path: `import bevy_pbr::render::forward_io::VertexOutput;`,
  `bevy_pbr::render::{mesh_view_bindings, mesh_functions, view_transformations}`,
  `bevy_render::globals::Globals`, `bevy_ui_render::ui_vertex_output::UiVertexOutput`,
  and ours as `baylee_client::shaders::card_common::{…}` (an
  `embedded://baylee_client/shaders/x.wesl` asset is module
  `baylee_client::shaders::x`). The material group is
  `@group(constants::MATERIAL_BIND_GROUP)`; `#ifdef X` is `@if(X)`.
  `UiVertexOutput` carries `border_radius_x`/`border_radius_y` (elliptical
  corners) instead of `border_radius`. `cardmat::tests::check_wgsl` strips the
  imports and the constant and validates the rest with naga.
- **Pointer events are flat.** `Pointer<Click>` is `PointerClick { entity,
  pointer, button, hit, duration, count }`, and so on for every kind; the
  position is `event.pointer.position` (a `Vec2`), the id `event.pointer.id`.
  Tests build them with `crate::test_pointer::aim(id, location, Click {…},
  entity)`.
- **Lifecycle observers** carry the bundle in the event: `On<Add<T>>`, not
  `On<Add, T>`.
- **`bevy::ui::widget::Button` and `bevy::ui::Interaction` are deprecated**
  and removed in the next release; `Button` is an alias of a crate-private
  type and cannot be spawned. This client spawns `legacy_ui::LegacyButton`
  (requires `Node`, `FocusPolicy::Block`, `DeprecatedInteraction`, a button
  role) and reads `LegacyInteraction`. `ui_widgets::Button` is not a drop-in:
  its observers stop the propagation of presses, releases, clicks and
  Enter/Space on a focused button. Moving to `Hovered` + `Pressed` is a
  separate change.
- **`Tonemapping::None` is a full passthrough** (no deband dither, no clamp,
  and a warning next to `DebandDither::Enabled`). The table camera carries
  `Tonemapping::Linear`, which is what 0.19's `None` was.
- **UI:** `BorderRadius` corners are `CornerRadius` (`px(4).into()`), and its
  constructors are no longer `const`. `CalculatedClip` is an enum of
  per-ancestor rects in each ancestor's local space; `shellkit::clip_bounds`
  gives the window-space band. `TextFont::default()` is `FontSize::Rem(1.)`.
  `EditableText` needs a sibling `TextInput` to be a working field (this
  client draws its own fields).
- **Extraction** names its target world: `#[extract_app(RenderApp)]` on an
  `ExtractResource`/`ExtractComponent` derive.
- **glam 0.33:** `Mat4::perspective_rh` is deprecated; use
  `bevy::math::proj::perspective` (right-handed, depth 0..1).
- Also renamed: `NextState::set_if_neq` → `set_if_different`, `MeshTag(x)` →
  `MeshTag::new(x)`, `Query::iter_many` yields `Result`s (`.matched()` for the
  old behaviour), `ScreenSpaceTransmission` is opt-in, sprites render through
  `Mesh2d`, and some built-in schedules order their sets weakly.

# Bevy 0.19 (still true underneath)

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
- **The stage has no lights at all** and the camera carries no tone curve
  (`Tonemapping::Linear`; `None` before bevy 0.20), deliberately: scene lighting on card art destroys colour
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
