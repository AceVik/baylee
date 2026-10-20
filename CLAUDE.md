# CLAUDE.md

@AGENTS.md

## For Claude Code

- The map above is the whole contract: read the document an area names
  before changing it. Every rule this file used to hold now lives in one of
  them (`docs/dev/`, `docs/architecture.md`, `docs/cards.md`,
  `crates/baylee-client/CLAUDE.md`); the long version of 24.09.2026 is
  `docs/history/CLAUDE-2026-09-24.md`.
- Project skills are in `.claude/skills/`: `dev-control` drives, films and
  inspects the client, and `bevy`, `wgsl-shaders`, `mtg-rules`, `rust-2024`,
  `game-ux`, `game-design`, `game-visual-design`, `game-animation`,
  `mmo-architecture` and `blender` carry the background for their fields.
- `crates/baylee-client/CLAUDE.md` loads only under `crates/baylee-client/`;
  for work in `crates/baylee-client-core/`, open it by hand.
