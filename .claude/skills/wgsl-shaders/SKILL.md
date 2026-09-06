---
name: wgsl-shaders
description: WGSL and the wgpu/Bevy shader pipeline — custom materials, material keys, portability limits, and how this repo keeps shader and model in step. Use when writing or debugging a shader, adding data to a material, or diagnosing something that renders wrong or renders nothing.
---

# WGSL and shaders

Verified 2026-09-06. WGSL is the WebGPU shading language, still at W3C
Candidate Recommendation. `wgpu` (latest published 30.0.1) translates WGSL via
**naga** into the platform's native shading language, and also accepts SPIR-V
and GLSL. Bevy 0.19 pins its own wgpu version — take the version from the Bevy
dependency, not from wgpu's latest release.

**Naga is not the full WGSL specification.** It tracks the spec with a lag and
there is no maintained diff. A construct that validates in a browser may fail
natively, and vice versa. When something rejects, suspect naga coverage before
suspecting yourself, and prefer conservative WGSL.

## Portability constraints that bite

- Storage buffers are unavailable or limited on some targets (notably WebGL2
  fallbacks); uniform buffers have tight size and alignment rules.
- `std140`-style alignment: `vec3<f32>` aligns to 16 bytes. Padding mistakes
  produce garbage, not an error.
- Texture and sampler binding limits are per-platform; Bevy 0.19 tightened
  sampler-limit checking and added Metal partial bindless.
- No recursion, no dynamic array sizes, integer division by zero is defined but
  not what you want.

## Bevy custom materials

Implement the `Material` trait and point `fragment_shader()` at a WGSL file
under `assets/`, then add `MaterialPlugin::<M>::default()`. Two things to plan
for: the shader is loaded as an *asset* (a typo is a runtime failure, not a
compile error), and **anything that changes the material key creates a new
material** — which is the mechanism to exploit, not to avoid.

This repo packs a card's power/toughness/loyalty into one `u32` on the material
key beside the glow bits, so a creature that takes damage simply becomes a
different material and redraws with no second pass. Numbers too large to pack
clamp rather than wrap. Two more `u32`s carry counter chip tints and counts,
because a tint and four counts do not fit in one.

## Keep the shader and the model from drifting

The pattern used here, and worth copying: the shader draws, a **renderer-free
module** decides *where and what*, and a **test reads the WGSL source and fails
when the two disagree**. That is what keeps a hand-tuned constant in a shader
from silently diverging from the layout module that positions against it.

## Debugging

Measure before theorising, and start outside the shader:

1. A **clear colour** touches no material, texture or shader. Rendering a known
   colour and reading the pixel back tells you whether the problem is the shader
   at all. In this repo that measurement found a colour-space discrepancy
   (a red clear rendering as `(62, 19, 21)` instead of `(234, 51, 35)`) and,
   separately, a full-screen opaque overlay that was hiding the whole table.
2. Preview composited values **offline** — arithmetic in a scratch script over
   the measured background — instead of rebuilding per attempt.
3. Geometry needs tests about geometry. A card quad here once shipped as a
   bowtie while the test asserting its *transform* passed the whole time.
