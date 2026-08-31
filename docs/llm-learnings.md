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

## 2026-08-29 — E3+E4+E5 in one bundle (180/14/0)

- **E3 disturb**: `FaceDef.disturb` + graveyard face-casting; disturb
  reuses the Flashback rider for exile-on-resolution (identical rule
  shape). Mirrorhall Mimic Implemented.
- **E4 classes**: `CounterKind::Level`, `CountersOnSelfExactly`,
  `Modifier::GrantTriggered` — granted TRIGGERED abilities scan
  continuous effects (like GrantActivated for activated ones) and carry
  `synthetic_target` through the trigger queue with a ChooseTargets plan
  (PlanKind::SyntheticTriggerTarget → push_synthetic_trigger_with_targets).
  Wizard Class (all 3 levels) Implemented.
- **E5 mana provenance**: the pre-existing `RestrictedMana` pool entries
  finally got wired: `restriction_info` side table (id → source, filter,
  SpendRider), `Effect::AddManaRestricted[CommanderIdentity]`, spell-aware
  payment in finish_cast (matching entries pay first, riders apply,
  refund on failure), `Rider::Uncounterable` checked in all counter ops,
  `Filter::SharesSubtypeWithCommander`. Cavern of Souls (uncounterable),
  Path of Ancestry (scry rider), Jasmine Dragon Tea Shop (Ally-only)
  Implemented.
- **180 Implemented, 14 Partial, 0 Unimplemented (93%)**.
- Remaining partials: M3/M4 protocol items (target re-choice, outside-game,
  presentation, MP direction, commander-cast count) + riders (Spark Double
  walker-copy loyalty, Mycosynth mana-any-color, Twining Twins adventure,
  Spirit Water assist, Inspirit station, copy mods, Opposition Agent
  search takeover).
- Edit gotcha: an edit that "fails" may have partially applied — grep
  before retrying (duplicate impl blocks happened twice today).

## 2026-08-29 — M3 core: engine-server live

- Protocol v1 shipped: protobuf `Envelope` framing + **serde_json
  payloads** for the choice taxonomy (`Pending`/`PlayerAction` got serde
  derives) — avoids a full proto mapping of ~30 enum variants; typed
  mapping is protocol v2.
- `baylee-engine-server`: `Session` (engine + human seat + AI seats
  auto-driven via baylee-ai) factored socket-free for tests; tokio +
  tokio-tungstenite transport in main.rs; dev duel = acceptance decks.
- Tests: session integration (AI pumps between human choices) + real
  e2e (spawn binary, ws client, CreateGame → ChoiceRequest → answer →
  game advances).
- prost gotcha: oneof field names are snake_cased in generated code
  (`PlayerActionMsg` → `Msg::PlayerAction`).
- Workspace deps gotcha: adding a dependency to a member crate ALSO
  requires it in `[workspace.dependencies]` (baylee-ai was missing).

## 2026-08-29 — M3 views + last riders batch (185/9/0)

- Hidden-information views: per-seat `PlayerView` (public zones full,
  own hand contents, others counts-only) emitted as `StateDelta` before
  each choice; e2e asserts it.
- Adventure machinery (CR 715): `FaceDef.adventure` — adventure spells
  resolve to exile with `Rider::Adventure`; the front face casts from
  exile afterwards. Twining Twins (data-corrected vs Scryfall AGAIN —
  stub had a different card's text; flying/vigilance/ward{1} 4/4 +
  Swift Spiral = ExileAndReturnAtEndStep).
- Spell-copy mods: `Effect::CopyTargetSpell { mods }` — Double Major +
  Storm of Saruman "copy isn't legendary" done; target re-choice stays
  protocol v2.
- Spirit Water Revival: `IfKicked` branch + `ShuffleGraveyardIntoLibrary`;
  waterbend = convoke extended to artifacts.
- Mycosynth Lattice: `Modifier::ManaIsAnyColor` + `pay_wild` (cost → cmc
  against pool total).
- Spark Double: CopyOnEnter with 3 mods (both counters unconditionally —
  harmless on the wrong card type, matches play).
- **185 Implemented, 9 Partial, 0 Unimplemented (95.4%)**. The 9 left:
  Aminatou (MP direction), Commander's Insight (commander-cast count),
  Inspirit (station), Jin-Gitaxias + Storm (target re-choice), Karn
  (outside-game), Opposition Agent (search takeover), Vendilion Clique
  (presentation) — all protocol/gateway items, no engine blockers.

## 2026-08-29 — M4-core: game manager

- Multi-game hosting: `Games = Arc<Mutex<HashMap<Uuid, Session>>>` in
  the transport; one `Session` per connection-bound human seat, AI seats
  auto-driven inside the session.
- `CreateGame` now honors client presets: `preset::from_proto` converts
  the v0 `GamePresetMsg` to the core `GamePreset` (formats, house rules,
  seats, decks, prints); no preset = dev acceptance duel.
- `JoinGame { game_id }` re-attaches to a live game (v1: resends view +
  pending; full seq-resume is protocol v2).
- prost naming gotchas: oneof fields snake_case (`Msg::Join` for
  `JoinGame`), `PrintRef::new` takes `u16` (wire uses `u32` → cast).
- e2e covers: create → answer → advance → second client joins by id.

## 2026-08-29 — 191/3/0: engine card scope essentially complete

- Final engine batch: Commander's Insight (commander_casts tracking in
  finish_cast + Amount::XPlusCommanderCasts), Aminatou −6
  (Effect::ControlRotation — heads-up swap of all nonland permanents),
  Vendilion Clique (presentation is protocol, engine choice was already
  complete), Inspirit (station: TapTarget + AddType/Keyword-
  IfCountersAtLeast conditional statics + modal counter trigger),
  Opposition Agent (REAL takeover: Modifier::SearchTakeover redirects the
  search choice to the agent's controller, finds go to exile with
  Rider::PlayableFromExileFor + wild payment on takeover casts),
  General Tazri (Amount::DistinctColorsAmong).
- **191 Implemented, 3 Partial, 0 Unimplemented.** The 3: Jin-Gitaxias
  and Storm of Saruman (copy target re-choice, protocol v2), Karn
  (outside-the-game, M4 gateway sideboards). No engine blockers remain.
- M4-core game manager shipped alongside: multi-game hosting, real
  client presets via `preset::from_proto`, JoinGame re-attach.

## 2026-08-30 — user card-review batch (20+ fixes, all verified vs Scryfall)

- Data corrections: Aang and Katara {3}{G}{W}{U} (was {1}), Earth King's
  Lieutenant {1}{G} (was {3}), Elspeth NOT commander-eligible, Ondu
  Cleric (real text: gain life = number of Allies you control — the stub
  had "1 life" again, third stub-text catch after Sheoldred + Twining
  Twins), Jin-Gitaxias triggers are artifact/instant/sorcery (not
  noncreature), Emeritus of Woe was the worst: I had invented an MDFC
  back face from the stub comment — the real card is the "prepared"
  Vampire Warlock with a linked Demonic Tutor.
- Rules fixes: fetchlands enter UNTAPPED; 21 land color identities
  corrected (ability mana symbols count: shocklands/triomes/checklands
  were EMPTY); Bojuka Bog targets ANY player (not opponents);
  Heliod's Intervention target player (GainLifeFor Chosen + DoubleX);
  Cyclonic Rift = "you don't control" (not opponents-only); blink family
  returns under OWNER's control; suspend costs are PAID ({U}/{1}{B} —
  the action was free before); triomes have mana abilities + cycling as
  hand-zone DiscardSelf→draw; produced_colors includes restricted mana
  (Exotic Orchard sees Cavern's full range).
- New mechanic: **prepared** (Rider::Prepared, enters-prepared modifier,
  prepared cast of a linked registry card with unprepare,
  per_turn.creatures_died tracking + IfCreaturesDiedAtLeast re-prepare).
- Centralization: `baylee_cards::tokens` (stable token ids = art keys;
  cards reference central TokenDefs), `ALL_MANA_COLORS` +
  `ANY_COLOR_MANA` in the DSL (used by 5 cards).
- Test lesson: behavior-correct fixes break tests with hardcoded
  expectations (cleric +1/+2 life, free suspend) — update tests to the
  real rules; turn-2 belongs to p1 in heads-up (sorcery windows are
  every OTHER turn for p0 — guards must span full rounds).

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

## 2026-08-31 — 193/1/0: the copy re-choice, and two bugs it uncovered

- Target re-choice for copies shipped, and it needed **no protocol work at
  all**. The handoff had it filed under protocol v2, but `Pending` travels
  as serde_json inside the protobuf frames by deliberate design, so a new
  choice costs no proto churn. Reusing `Pending::ChooseTargets` rather than
  inventing a variant meant the AI and the client needed no change either:
  both already answer it generically. Check what a "protocol item" actually
  touches before scheduling it behind a protocol milestone.
- Writing the first real test for the two cards found the machinery around
  them broken in three ways that no existing test could see:
  1. `Pending::ChooseTargets` had no resolution path in `apply` — only cast
     wizard and `pending_plan`. Any resolution-time target choice panicked
     with "target plan set". `RedirectTarget` has emitted one since it was
     written; nothing ever answered one, so the panic sat there unseen.
  2. A copy put on the stack was journalled as `GameEvent::SpellCast`. A
     copy is *put* onto the stack, not cast (CR 707.10) — so every copy
     re-triggered "whenever you cast", and Jin-Gitaxias copied its own copy
     without end.
  3. `Trigger::NthSpellCast` existed in the DSL and in Storm of Saruman,
     but the engine's matcher had **no arm for it**. Storm's headline
     ability had never fired. The card's `// PARTIAL` note claimed "the
     second-spell copy trigger work[s]" — a coverage note asserting more
     than the code did, the opposite of the usual stale-Partial drift.
- `once_per_turn` was write-only: `ability_fires` was inserted into and
  cleared each turn, but never read. Exactly one card uses it
  (Jin-Gitaxias), which is why nothing noticed.
- Lesson, again but sharper than the stale-Partial note above: a coverage
  string is a claim, not evidence. The only thing that distinguishes
  "implemented" from "declared" is a test that plays the card. Two ~40-line
  behavioral tests turned three latent bugs into failing output in minutes.
- **193 Implemented, 1 Partial, 0 Unimplemented.** The 1 is Karn
  (outside-the-game access, still waiting on M4 gateway sideboards).

## 2026-08-31 (later) — 194/0/0, and the pattern behind all of it

Five things landed this session that were each already "in the codebase":
`once_per_turn`, `Trigger::NthSpellCast`, `ResumeGame{last_seq}`,
`HouseRules::decision_timeout_secs`, and `AIProfile::politics`. Every one was
declared in a type, carried through the preset or the proto, documented as a
feature — and read by nobody. Two of them (`politics`, the agent profile) even
carried an `#[allow(dead_code)]` that made the deadness look deliberate.

That is the failure mode of this codebase, and it is not sloppiness: it is
what happens when the *declaration* is the cheap half and the wiring is the
expensive half, and the declaration is what gets reviewed. Things to take
from it:

- **Grep for reads, not for the name.** All five would have been caught by
  asking "where is this *read*?" instead of "does this exist?". `ability_fires`
  had an insert and a clear and no `get`.
- **A settings field is a claim like a coverage string is a claim.** Neither
  is evidence. The only evidence is a test that exercises it.
- **"Needs protocol v2" was wrong four times out of five.** The copy
  re-choice, the agreed draw, the resume and the decision clock all shipped
  without touching the wire, because the taxonomy travels as JSON inside the
  protobuf frames by deliberate design. Before deferring something to a
  protocol milestone, check whether it needs the wire or only the taxonomy
  the wire carries. Only `time_extension_votes` genuinely needs new messages.

The sideboard was the same shape in reverse: `Zone::Sideboard` was parsed
correctly and then folded into the main deck (`Zone::Main | Zone::Sideboard =>
&mut main`), so every acceptance deck was ~15 cards larger than the one it
described, silently, for as long as the parser has existed. Both acceptance
decks have sideboard sections, so this was live, not theoretical.

Karn's −2 then became small: sideboard cards materialise into a
`Zone::OutsideGame` — not a zone in the rules (CR 400.1 says those cards are
in *no* zone), but they need object ids for a choice to offer them, and a home
makes them impossible to confuse with cards in the game. `Effect::WishToHand`
reads that zone plus your own exile.

- **194 Implemented, 0 Partial, 0 Unimplemented.** The acceptance pool is
  complete. What remains is engine *families* (see the roadmap), not cards.
- Regeneration was deliberately **not** built: nothing in the pool regenerates,
  and `damn.rs` promised a roadmap entry that had never been written. The
  entry now exists (C2b) with the shape and sizing. Building the machinery
  ahead of a card would have produced a sixth declared-but-dead feature —
  which, this session of all sessions, would have been a poor joke.

### The soak failure the sideboard fix exposed

Fixing the sideboard took the AI soak from 4/4 to 1/4 finished games, which
looked at first like an infinite loop introduced by the new zone. It was
neither new nor a loop, and the diagnosis is worth keeping:

1. **Bisect before theorising.** Skipping the sideboard object creation
   entirely still failed — so the new zone was innocent and the *deck
   composition change* was the trigger. Different shuffles, different games.
2. **`None` from `play_game` meant three different things** (action cap,
   loop detected, late legality miss) and the test could not tell them apart.
   One `eprintln!` per exit path found it in a single run.
3. The real cause: `compute_legal` offered Force of Will as castable with an
   empty stack. The cast wizard then aborted at the targeting step, the
   engine recovered cleanly — and the agent, facing an unchanged state, chose
   the same illegal cast again. `play_game` abandoned the game on the first
   miss, which is why the count moved so sharply for so small a data change.

Both halves were wrong and both are fixed: the harness no longer gives up on
a miss the engine has already recovered from, and `compute_legal` no longer
offers a spell whose mandatory target has no legal choice (CR 601.2c). The
second is the one that matters beyond the soak — a human client was being
shown a cast button that could only ever produce an error.

Worth remembering: an over-approximated "legal actions" list is not a
harmless convenience. Anything that consumes it — a UI, an agent, a test
harness — treats it as truth.
