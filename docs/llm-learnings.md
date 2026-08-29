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

## 2026-08-29 — M3 start: baylee-ai + self-play soak

- `baylee-ai`: `HeuristicAgent` (greedy 1-ply, full pending taxonomy),
  `decks` loader (acceptance parser moved `baylee-cards-codegen` →
  `baylee-core` so runtime crates can use it), `play_game` driver with
  hash-based loop detection (key = state hash + player + turn + phase +
  step + pending kind — engine-side fields like pass counters are NOT in
  the snapshot hash, and the pending kind matters: priority-after-declare
  is rules-correct, CR 506.2).
- **Self-play found 4 real engine bugs that 59 green unit tests missed:**
  1. `resolve::exec` router missed 3 choice ops (AddManaChoice,
     AddManaCommanderIdentity, PayLifeOrEnterTapped) → latent
     `unreachable!` crash; only real gameplay reached them.
  2. Choice-mana abilities (any-color lands) hit a `debug_assert`
     expecting immediate resolution — they suspend like any resolution.
  3. Synthetic triggers (prowess/ward, index `u32::MAX`) crashed in the
     def-lookup BEFORE the synthetic branch — order matters.
  4. Wizard failures (payment/late target legality) left a consumed
     pending → infinite re-ask loop. Wizards now fizzle cleanly and resume.
- Lesson: unit tests verify mechanics in isolation; the soak is the
  integration net. Run it after every engine change.
- AI v1 deliberately does NOT activate non-mana abilities (free no-op
  ability spam loops) and checks pool before miracle yes/no.
- `tail -1` on cargo commands masks clippy failures in shell chains —
  check clippy output directly before committing.

## 2026-08-29 — mechanics roadmap + E1 (bundled small hooks)

- `docs/mechanics-roadmap.md` now inventories all mechanic families
  (A: supported, B: 12 remaining engine hooks, C: family taxonomy +
  batch order, C4: explicit long tail). Process rule: a card needing a
  missing family STARTS A FAMILY MILESTONE, never a single-card hack.
- E1 bundled all S-sized hooks in one iteration (the anti-pattern of
  one-hook-per-card is what the roadmap kills): `ActivatedConditional`
  (activation preconditions), `CostReduction` on FaceDef (with
  `state.starting_player`), `Trigger::BecomesTapped`,
  `Effect::IfControlGreatestCmc` (comparative conditions),
  `Effect::CreateEmblem` + `obj.emblem_abilities` + command-zone trigger
  scan (emblem triggers route through a DEDICATED push path —
  `push_ability_to_stack` requires card-backed sources; resolution falls
  back to `emblem_abilities` before the card lookup), `PlayerHexproof`
  (filtered in the wizard's ChoosePlayer stage).
- 8 partials upgraded: Mox Opal, Bleachbone Verge, Surgical Metamorph,
  City of Brass, Padeem, Venser −8, Everybody Lives!, Reflections of
  Littjara (token-copies were already correct).
- Coverage now: **159 Implemented, 35 Partial, 0 Unimplemented**.
- Token-efficiency note: E1 = 7 hooks + 8 cards in ONE iteration — the
  roadmap-driven batch shape works.
- DSL gotcha: inserting a variant ABOVE another variant's doc comment
  steals the comment (missing-docs error for the next variant).

## 2026-08-29 — E2 sagas (+ data-correction catch)

- Saga machinery (CR 714): lore counter + chapter trigger on ETB
  (apply_enter_modifiers) and after each draw step
  (saga_draw_step_counters at the FirstMain→Combat transition); chapter
  abilities are `AbilityDef::SagaChapter { chapter, effects, target }`
  reusing the whole trigger/target/resolution machinery; sacrifice after
  the final chapter in finish_resolution (counters >= max chapter).
- `Modifier::GrantActivated { cost, effects, mana_ability }` — granted
  abilities enumerate as synthetic index u32::MAX in compute_legal and
  resolve through the synthetic side map (`start_granted_activation`).
- `Modifier::ModifyPTPerCount { filter, p, t }` (layer 7c) +
  `Effect::CreateTokenPtPerCount` (Urza's Saga Construct).
- Per-player chains: `DestroyChosenForPlayers` (uses `sba::destroy` —
  respects indestructible, unlike the sacrifice path) and
  `DiscardForPlayers` (DiscardChain tracks the CHOOSING player for the
  graveyard, not the controller).
- `Effect::ExileSelfReturnAsFace { face }` — transform via
  `obj.pending_face_change` applied in finish_resolution (resolve has no
  lookup; face switches need the def).
- **Data catch**: the sheoldred.rs stub header had the WRONG oracle text
  (the Apocalypse's draw triggers). The real MOM Sheoldred: 4/5 menace,
  ETB edict, {4}{B} flip (sorcery, opponent gy >= 8 — new
  `ActivationCondition::OpponentGraveyardCountAtLeast`). Lesson: verify
  stub headers against Scryfall for cards that share names with other
  printings (Sheoldred × 2 in the pool).
- Coverage now: **161 Implemented, 33 Partial, 0 Unimplemented**.

## 2026-08-29 — quick-win sweep after E2 (roadmap paying off)

- After E2's GrantActivated machinery, many "partial" notes collapsed in
  one sweep: Chromatic Lantern + Great Divide Guide (mana grants),
  Luminarch Ascension (CountersOnSelf condition), Storm of Saruman
  (NthSpellCast + ward {3}), Nesting Dovehawk (populate via IsToken
  targets), Helm of the Host (token-copy mods on CreateTokenCopyOfEquipped),
  Recruiter of the Guard (ToughnessAtMost filter), Emeritus of Woe (MDFC
  back-face spell), Elspeth + Teferi (UntilYourNextTurn duration +
  SorceriesHaveFlash), Force of Negation (CounterTargetSpellToExile),
  Karmic Guide (Echo via DelayedAction::PayCostOrSacrifice + protection),
  Doubling Season (ETB loyalty placement now honors counter-doubling).
- **175 Implemented, 19 Partial, 0 Unimplemented.**
- Remaining real milestones: E3 disturb/graveyard-casting (Mirrorhall),
  E4 classes (Wizard Class), E5 mana provenance (Cavern, Path of
  Ancestry, Jasmine Dragon), plus M3/M4 protocol items (target re-choice,
  outside-game, presentation) and small riders.
- Coverage-staleness gotcha: `Coverage::Partial` strings linger after the
  machinery lands — grep stale Partials after every engine milestone
  (multi-line literals evade single-line edits; the listing loop catches
  them).

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
