# LLM Learnings — baylee

Running log of what works when delegating card implementations to local
LLMs (hardware: MacBook M1 Max 64 GB — at most ONE local model active at a
time). Maintained by the orchestrator; entries dated, newest first.

## Process rules (baseline)

1. First batch per model is verified card-by-card; afterwards only special
   cases (layers, copy, replacement, multi-choice cards) are spot-checked.
2. Fixes are applied by the orchestrator, never by the LLM — but every fix
   is analyzed for a prompt improvement and recorded below.
3. Each card task gets: the stub header (oracle text), the forge-reference
   script, one similar already-implemented exemplar, and the DSL cookbook
   excerpt for its mechanic class. Nothing else (token budget).

## Model scoreboard

| Model | Verdict | Notes |
|---|---|---|
| (unset) | | first batches pending (M2.S8) |

## Prompt learnings

(append after each batch: error class → prompt rule that prevents it)

## 2026-08-29 — orchestrator night batch (21 cards, no local LLM)

Batch: Mana Drain, Tishana's Tidebinder, Everybody Lives!, Misdirection,
Maze of Ith, Urza's Saga (partial), Venser the Sojourner (partial) plus the
14-card Avatar wave earlier that day. New machinery added en route:

- `DelayedWhen::NextFirstMain` / `DelayedAction::AddMana` (Mana Drain):
  fires when the active player LEAVES their first main phase — hook lives
  in `advance_step`, not at phase entry.
- `DelayedWhen::NextEndStep` / `DelayedAction::ReturnToBattlefield`
  (Venser +2): fires for ANY controller, not just the active player —
  unlike upkeep/first-main delayed triggers.
- Counter-ability machinery: `Effect::CounterTargetAbility`,
  `TargetSpec::AbilityOnStack`, `TargetSourceLosesAbilities` (Tishana).
- Resolution-time targeting: `RedirectTarget` + `AwaitingOp::RedirectNewTarget`
  — Misdirection's new target is chosen at resolution (CR 115.7), not at
  cast time. `Pending::ChooseTargets` has NO `prompt` field (unlike
  ChooseCards).
- Damage prevention modifiers: `PreventDamageToIt` / `PreventDamageFromIt`
  — checked directly in `combat.rs` deal-damage fns via
  `EffectFilter::ObjectIs`, not through the layers system (they're in the
  "handled elsewhere" match arm of `layers.rs` + `state.rs` modifier hash).
  Every new `Modifier` variant MUST be added to BOTH match statements or
  the build breaks with non-exhaustive errors.
- `Filter::Attacking` (Maze of Ith): evaluated against
  `state.combat.attackers` in `eval.rs`.
- No-lose suppression: `Modifier::PlayersCantLose` checked in
  `engine/mod.rs::game_result` AND `sba.rs`; `CantLoseLife` checked in the
  `LoseLife` resolve op.

Error classes hit (orchestrator-side, relevant for prompt design):

1. Brace imbalance when converting `if let` blocks to let-chains inside
   nested loops — twice (trigger.rs prowess block, resolve.rs token-copy).
   Rule: after ANY structural edit, run `cargo check -p baylee-engine`
   immediately, not after a batch of edits.
2. Truncating a fn body with a bad `edit` oldString (gain_life early
   return) — always re-read the region after a failed edit attempt.
3. `ZoneLocation::Exile` is a TUPLE variant (`Exile(owner)`) — check
   zone.rs before writing moves.
4. Clippy `doc_markdown` on new helper fns — backtick type names in docs.

Open milestones discovered tonight:

- **Sagas**: lore counters on ETB/after draw step, chapter triggers
  (ChapterUp), granted abilities (ch. I/II style "gains ..."), sacrifice
  after final chapter. Blocks: Urza's Saga chapters, any future saga cards.
- **Emblems**: `CreateEmblem` op + command-zone triggered-ability scanning
  in trigger.rs. Blocks: Venser −8, other walker ults.
- **Player hexproof** ("players gain hexproof", Everybody Lives!): player
  targeting prevention — needs protocol/rules work (M2+).

## State after M2.S8 (2026-08-29)

- DSL frozen (`docs/card-dsl.md`); the cards `AGENTS.md` playbook lives in
  `crates/baylee-cards/AGENTS.md`.
- `cargo run -p xtask -- card-batch` prepares per-card task packages in
  `target/card-batch/<slug>/` (STUB + FORGE + SCRYFALL + EXEMPLAR + PROMPT).
  `--cards "A,B"` restricts to a list; default = all unimplemented
  acceptance cards.
- `cargo run -p xtask -- validate` enforces conventions (194 conform).
- Coverage: **153 Implemented, 41 Partial, 0 Unimplemented** (was 92/19/83
  at freeze) — M2.5 acceptance coverage COMPLETE. All six subsystem
  milestones landed on 2026-08-29: MDFC (per-face abilities,
  `FaceDef.abilities` + `abilities_for_face`, `face_index`,
  `CastModeKind::Face/PlayLandFace`), miracle (`FaceDef.miracle`,
  `pending_miracle`, `CastModeKind::Miracle`, extra turns, lifelink
  counters), flashback grants (`Modifier::GrantsFlashback` +
  `Rider::Flashback`), protection (`Modifier::ProtectionFrom` at
  damage/target/block), until-EOT layer-1 copies
  (`Modifier::BecomeCopyOf` + `AbilityDef::CopyOnEnterUntilEot`),
  delve/convoke (`FaceDef.delve/convoke` + wizard payment reductions,
  `ManaCost::with_less_generic`).
- Key DSL lesson: card-level `abilities` = FRONT face only;
  `abilities_for_face(0)` falls back to card-level, back faces NEVER
  inherit (Sheoldred's saga must not inherit the front's triggers).
- FaceDef grew 4 fields post-freeze (`abilities`, `castable_from_hand`,
  `miracle`, `delve`, `convoke`) — all bulk-inserted via perl across 194
  files; mechanical multi-file literal edits are safe when the anchor
  line is uniform.
- Remaining partials (41) cluster into: sagas, emblems (command-zone
  trigger scan), ability-granting statics, activation conditions,
  mana-source provenance, search takeover, tap events, comparative
  conditions, cost reducers, disturb, player hexproof, token-spell-copies,
  colored convoke mana.
- 2026-08-29 second night batch added: choose-subtype machinery
  (`EnterModifier::ChooseSubtype`, `Pending::ChooseSubtype`,
  `obj.chosen_subtype`, `Filter::MatchesChosenTypeOfSource`),
  `AbilityDef::Ward` (synthetic trigger like prowess; ward {1}/{2}
  statics), `color_identity` + `produced_colors`/`produced_colorless` on
  `Characteristics` (precomputed at creation because resolve has no
  lookup access), `Filter::{Attacking,Monocolored,IsToken}`,
  `Effect::{SacrificeFilter, DrainAllCountersIntoSelf,
  IfEventPowerAtLeast, AddManaLandColor, AddManaCommanderIdentity,
  ExchangeControlOrSacrifice, GainLifeDoubleX}`, `Modifier::
  {OpponentsCantSearch, NoMaxHandSize}`.
- Batch order: local model implements a card → `cargo check -p baylee-cards`
  → `cargo test -p baylee-cards <slug>` → `xtask validate`. Failures retry
  once with compiler output, then escalate. One local model at a time
  (M1 Max 64 GB).
