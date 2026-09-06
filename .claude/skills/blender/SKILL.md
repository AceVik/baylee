---
name: blender
description: Blender 5.2 LTS — geometry nodes, the XPBD physics systems, and the glTF pipeline into a Bevy game. Use when authoring, exporting or debugging 3D assets, when a model imports wrong, or when deciding between an authored asset and a generated one.
---

# Blender 5.2 LTS

Verified 2026-09-06. **Blender 5.2 LTS** was released 2026-07-14 and receives
fixes until **July 2028**. It is the version to standardise on: an LTS keeps a
team's files openable for two years, and Blender's non-LTS releases routinely
change node names and operators.

## What 5.2 added that changes how work is done

- **Geometry Nodes physics.** Experimental hair and cloth built on a built-in
  **XPBD Solver** node, exposed as high-level assets you can use without
  understanding the solver underneath.
- **A list data type in Geometry Nodes.** Sequences of arbitrary length, with
  `Field to List`, `Closure to List`, `List Length`, `Get List Item`,
  `Filter List` and `Sort List`. Many loops that previously needed a script are
  now node graphs.
- **Geometry bundles.** `Get Geometry Bundle` / `Set Geometry Bundle` for
  passing grouped geometry through a graph.
- **Cycles:** texture cache support, and `Thin Wall` mode on Principled BSDF.
- **EEVEE:** a batch of usability fixes and cleanup.

## Exporting to a Bevy game

glTF 2.0 (`.glb`) is the path Bevy supports properly. Points that cause most
import problems:

- **Apply transforms** before export. A non-uniform scale on an armature or a
  parented mesh is the single most common cause of an asset that looks right in
  Blender and wrong in engine.
- **+Y up.** Blender is Z-up; the glTF exporter converts, but only if you let it
  (leave the axis conversion at its default rather than "fixing" it twice).
- **Bevy 0.19 culls skinned meshes against animated bounds**
  (`DynamicSkinnedMeshBounds`), and glTF meshes get this automatically — so an
  animated mesh disappearing at the edge of frame is now a real bug to report,
  not the expected rest-pose behaviour it used to be.
- Bake procedural materials to textures; Bevy does not evaluate Blender's shader
  nodes. Principled BSDF maps to Bevy's PBR reasonably; anything else does not.
- Name things deliberately. Object and animation names come across and become
  the handles the game code uses.

## Before authoring an asset at all: is it needed?

Two constraints in this project push toward *generating* rather than importing:

- **Legal.** Ornament is the easiest thing to borrow by accident.
  `docs/legal.md` §2 is the decision, and arithmetic borrows nothing — the table
  felt, the centre medallion and every seat mat here are computed into RGBA8
  buffers with a seeded value-noise fbm, so every player sees the same grain
  with no shipped texture and no provenance question.
- **Determinism and size.** A generated surface has no asset to load, no cache
  to invalidate, and no per-player difference.

Reach for Blender when the thing genuinely has *form* — a modelled object, a
rigged character, an animation. Reach for arithmetic when it has only pattern.
