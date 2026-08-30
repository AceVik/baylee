# Godot Client

A second renderer for the same duel, not a fork of it. Everything the client
*decides* stays in `baylee-client-core`; this adds a Godot front end over it.

| Crate / dir | Contains | Depends on a renderer? |
|---|---|---|
| `baylee-client-core` | Table layout, board model, interaction, image policy | no |
| `baylee-client-godot` | `BayleeDuel` node + local host: the `GDExtension` | Godot |
| `godot/` | The Godot project: scenes, scripts, `.gdextension` | Godot |

Status: **[Implemented]** the bridge — a Godot scene deals a real duel against
the house AI, reads the board model, and answers choices. **[Spec]** everything
that makes it a *table*: card art, the 2.5D stage, input, combat declaration.

## Running it

```sh
cargo build -p baylee-client-godot   # writes target/debug/libbaylee_client_godot.dylib
godot --path godot                   # or open godot/ in the Godot editor
```

Headless, which is what CI and a quick check want:

```sh
cd godot && godot --headless --quit-after 60
```

Godot loads the library **at startup**. After a Rust change: rebuild, then
Project → Reload Current Project. An editor left open keeps running the old
library and will happily lie to you about whether a fix worked.

## Versions

Godot 4.7 and `godot = "0.5.5"` with the `api-4-7` feature — prebuilt bindings
matching the engine, so nothing needs a local Godot binary at build time and CI
builds the crate like any other. Bumping Godot means bumping that feature in
lockstep; a mismatch shows up as `Cannot get class 'BayleeDuel'` rather than a
build error.

## Three places this crate breaks a workspace rule

Each is deliberate, each is scoped as narrowly as the tooling allows.

1. **`unsafe`.** The workspace *forbids* it, and `forbid` cannot be lifted
   locally. gdext's entry point is an `unsafe impl` — Godot calls it across an
   FFI boundary. The manifest therefore downgrades forbid → **deny**, and the
   impl lives in a module (`lib.rs::entry`) that contains nothing else.
2. **MSRV.** gdext 0.5 needs Rust 1.94; the workspace promises 1.88. The crate
   declares its own `rust-version` and the MSRV CI job excludes it, so the
   promise the engine and the gateway make stays true and testable.
3. **`panic = "abort"`.** Right for the engine, wrong for an extension: gdext
   catches panics at the FFI boundary and reports them to Godot as errors,
   while `abort` turns any panic into an instant kill of the whole editor.
   Release the extension with `cargo build -p baylee-client-godot --profile
   godot-release`, which inherits release and restores unwinding.

## The `.gdextension` file is a Godot ConfigFile

Comments start with `;`. A `#` comment is a parse error that silently drops the
entries after it — the symptom is `No GDExtension library found for current OS
and architecture`, which reads like a missing build and is not one.

## Wiring a scene

Connect before you start. Godot runs a child's `_ready` *before* its parent's,
so a `BayleeDuel` with `autostart` on deals the opening hand before the scene
above it can subscribe, and the first choice is lost. `autostart` is therefore
off by default:

```gdscript
func _ready() -> void:
	duel.board_changed.connect(_on_board_changed)
	duel.choice_offered.connect(_on_choice_offered)
	duel.start_demo_duel()
```

## Known duplication — the next change

Two pieces of renderer-agnostic logic live in the Bevy crate and are copied
here rather than shared:

- `host.rs` (`DuelHost`, `LocalHost`, `HostMessage`) — `docs/client.md` already
  calls this seam renderer-independent, and it is: no Bevy type appears in it.
- `hud::threat_line` — pure formatting over `ThreatSummary`.

Both belong in `baylee-client-core`. Moving them touches `baylee-client`, so it
is a separate change; until then a fix to one copy must be made to the other.

## Verification

- `cargo test -p baylee-client-godot` plays the opening of a real duel through
  the host, and asserts an illegal action surfaces instead of freezing.
- The Godot side is checked by running the project headless (above): it prints
  turn, step, life totals, threat lines and the sorted hand.
