---
name: game-animation
description: Motion and animation for a game client — Bevy 0.19's animation system, frame-rate-independent easing, and the rules this repo's table follows. Use when adding or changing any movement, transition, tween or timed visual effect, or when an animation looks wrong at a different frame rate.
---

# Animation

Verified 2026-09-06. Bevy 0.19's animation system is three layers:
`AnimationClip` (the data), `AnimationGraph` (a DAG that blends clips), and
`AnimationPlayer` (execution). 0.19 also added `DynamicSkinnedMeshBounds`, so
animated meshes are culled against their animated bounds rather than their rest
pose — glTF meshes get this automatically.

For simple property tweens, prefer Bevy's own `EaseFunction` over a third-party
crate. `bevy_tweening` exists and is actively published, but **verify its Bevy
version compatibility before adding it** — tweening crates routinely lag a Bevy
release, and this workspace pins 0.19.

## The three rules that matter more than the API

**1. Frame-rate independence is not optional.** A per-frame `pos += speed * dt`
lerp toward a target (`pos = lerp(pos, target, 0.1)`) is frame-rate *dependent*
and moves at different speeds on a 60 Hz and a 144 Hz display. The correct form
is exponential decay:

```
factor = 1.0 - (-rate * dt).exp()
pos = pos.lerp(target, factor)
```

This repo uses exactly that in `table::glide`, and it has a second virtue: a
half-millimetre correction settles quickly while a card arriving from off-table
still takes visible time, with no separate cases.

**2. One door for all movement.** In this client, `table::sync_scene` writes a
`Motion` *target* and `glide` moves toward it. A repacked lane, a tap, a hover
and a card entering play all animate through that one path, so the scene can
never disagree with the board model. Setting a `Transform` directly is the bug,
not the shortcut.

**3. Motion is a preference, not a given.** `Preferences::reduce_motion` turns
both the card glide and the camera rig off. Anything new that moves must honour
it — and on the web, `prefers-reduced-motion` is the same promise.

## Proving an animation actually happens

A claim that something animates is easy to make and easy to get wrong. The
evidence that works: capture two frames and diff them full-frame, reporting the
peak difference per channel — and always run the counter-test (the same two
frames with the animation disabled) so a diff caused by something else does not
read as success.

## Interpolation details worth getting right

- Interpolate camera yaw **the short way round**, or a turn from 350° to 10°
  sweeps the long way.
- Rotations blend as quaternions (`slerp`), never as Euler angles.
- Colour blends look wrong in sRGB; convert to linear (or Oklab) first.
- Easing choice carries meaning: ease-out for something arriving, ease-in for
  something leaving, linear only for genuinely constant motion.
