# LLM Learnings — baylee

Running log of what works when delegating card implementations. It began
with local models (MacBook M1 Max 64 GB, at most one active at a time) and
the two lanes in use now are remote and cheap, which changes the constraint
from "one at a time" to "what is each one actually good at". Maintained by
the orchestrator; entries dated, newest first.

## Process rules (baseline)

1. First batch per model is verified card-by-card; afterwards only special
   cases (layers, copy, replacement, multi-choice cards) are spot-checked.
2. Fixes are applied by the orchestrator, never by the LLM — but every fix
   is analyzed for a prompt improvement and recorded below.
3. Each card task gets: the stub header (oracle text), the card-script reference
   script, one similar already-implemented exemplar, and the DSL cookbook
   excerpt for its mechanic class. Nothing else (token budget).

## Model scoreboard

Measured over the four-deck batches, 2026-09-16/17. "Written" counts cards
that survived the header check and compiled; an honest refusal naming the
missing DSL variant is a **correct** outcome and is counted separately, not
as a failure.

| Lane | Cards asked | Written | Honest refusals | Input per 30 cards | What that costs |
|---|---|---|---|---|---|
| DeepSeek V4.1 Flash (`deepseek-flash[1m]`, API, non-agentic) | 91 | 42 (46%) | 48 | ~0.1–0.4M tokens (241 KB prefix, cached) | money, and very little of it |
| Gemini 3.8 Flash high (`agy`, agentic, one session per batch) | 93 | 19 (20%) | 73 | ~110–190M tokens (measured off the transcript) | nothing — a subscription, bounded by a **time quota that refreshes** |

Round three is what those two totals fell on: 32 and 33 cards over the last
65 open stubs of the four decks, of which 6 and 3 were written. That is not
a regression, it is the tail — what is left after two rounds is the cards
whose printed sentence the DSL genuinely cannot say, and both lanes said so
rather than half-building them.

| Lane | Tests asked | Written | Passed the first run |
|---|---|---|---|
| DeepSeek V4.1 Flash | 55 | 55 (2 needed a bigger `max_tokens`) | 44 (80%) |
| Gemini 3.8 Flash high | 5 | 4 + 1 correct refusal | 3 of 4 |

**Read that last column before the one beside it.** The token difference is
two to three orders of magnitude and it is structural rather than a tuning
mistake: an agentic session re-reads the repository for itself and pays the
growing transcript on every step (369 steps for 30 cards), while the API
lane sends one cacheable 241 KB context and a stub. Both were *measured* —
`lane.transcript` reads the prefix sum out of the session's own sqlite
transcript — because an earlier unmeasured fan-out spent an estimated 2.2
billion input tokens before anybody noticed.

But those tokens are **not a bill**. The Gemini lane runs under a
subscription and costs no money at all; what it spends is a time quota that
refreshes. So the two lanes are scarce in different units, and planning has
to use each one's own: DeepSeek in cards per batch, Gemini in **minutes**.
The token figure stays worth reading there as the reason a session is slow,
and as the thing batch size actually buys back — five tests in one session
cost 46 steps each, the same work split into a single-card session cost 88,
and nearly all of that gap is orientation paid once.

**They are good at different things, and the numbers say which.** DeepSeek
writes more than twice as many cards per attempt and every generated engine
test so far. Gemini refuses more — and its refusals are the better product:
"`EnterModifier::WithCounters` takes a fixed `u16` and not `Amount::X`, so a
0/0 Walking Ballista would die to state-based actions on arrival" is a
precise engine ticket, and a batch of them is a ranked worklist. That one is
**shipped** (the variant carries an `Amount`, and the refusal was right twice
over: the projection the state-based action reads was stale as well, so the
Ballista died on arrival for a second reason nobody had named). Exactly one
card of the 61 that landed was substantively **wrong** (Mikaeus, DeepSeek,
two keywords no rule reads), and it was caught by a lint plus a played test,
not by reading.

### The cross-lane rule (from 2026-09-17)

A card and its test written by the same model share that model's misreading.
That is not a worry, it is the measured Mikaeus failure: the card granted a
keyword the engine ignores and its generated test asserted the keyword
worked, so the pair was internally consistent and wrong. **So the lanes
swap**: whoever wrote the card does not write its test. A test that goes red
is then evidence about one of the two readings rather than about neither,
and the interesting cell — a cross-written test failing *because the card is
wrong* — is the one worth counting as trust is built and the spot-checks
thin out.

**The first cross round paid for itself on the refusal, not on a test.**
Eight cards, eight tests, written the other way round. Seven landed; three
of DeepSeek's three and three of Gemini's four passed on the first run, and
the one that did not had asserted that a `RemoveCounterSelf` cost is taken
as the ability is announced — targets are chosen first (CR 601.2c) and the
cost is the last step of an activation (CR 601.2h), so the assertion moved
past the target question rather than the engine moving.

The eighth is the result. Asked for Teferi's Protection's test, Gemini
returned `SKIP` and named the reason: `finalize_spell` puts the resolving
instant into the graveyard *after* `Effect::ExileSource` has already exiled
it, so the one clause the card can express is undone one step later. That
was true, it is now fixed with its own rules test, and it had been sitting
under Spirit Water Revival — a `Coverage::Implemented` card with a test
module of its own — the whole time. A model writing its own test would
have written around it; a model reading somebody else's card refused to.

**What that says about where each lane belongs.** DeepSeek is the volume
lane: cheap, parallel, twice the write rate, and its tests now compile and
pass first time. Gemini is the *reading* lane — slow per card in the unit
that lane is actually scarce in, and worth those minutes where the answer is
a judgement about whether something can be said at all, because its refusals
are precise engine tickets and, as of this round, one of them was a defect
rather than a gap. Spot-checks should thin out on DeepSeek's volume first; a
Gemini refusal stays worth reading in full.

Both lanes now live in `scripts/llm/`, one script per (model, job) pair over
a shared `lane.py`, with the four prompt contracts beside them in
`prompts/`. They used to be scratchpad files, which made the trust being
built here last exactly as long as one session.

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
  `TargetSpec::AbilityOnStack`, `TargetSourceLosesAbilities` (Tishana). The
  rider does NOT read `res.targets` — by the time it runs, the ability that
  was targeted has ceased to exist (CR 701.6a) and its `ObjectId` resolves to
  nothing. It reads `Resolution::countered_source`, which the counter writes
  down before removing the object, so the rider has to follow the counter in
  the same effect list.
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
  is rules-correct, CR 508.2).
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
  `Effect::CreateEmblem` + `obj.own_abilities` + command-zone trigger
  scan (emblem triggers route through a DEDICATED push path —
  `push_ability_to_stack` requires card-backed sources; resolution falls
  back to `own_abilities` before the card lookup), `PlayerHexproof`
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
  abilities are `AbilityDef::SagaChapter { chapter, effects, targets }` —
  written `chapter!(1, effects)`, which supplies the `targets: None` a
  chapter has unless the printed chapter says "target" —
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
  `Condition::OpponentGraveyardCountAtLeast`). Lesson: verify
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
  `target/card-batch/<slug>/` (STUB + SCRIPT + SCRYFALL + EXEMPLAR + PROMPT).
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
`Zone::OutsideGame` — not a zone in the rules (CR 400.11 says those cards are
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

## 2026-08-31 (later still) — the riders were the symptom, not the disease

Five card files still carried `// PARTIAL` / `// NOT SUPPORTED` riders naming
mechanics that had landed weeks earlier (this very file records four of them
as done). Nothing checked the riders, so nothing noticed. `baylee-cards` now
has a test that a file naming a missing mechanic may not also claim
`Coverage::Implemented`, and `baylee-engine::engine::card_rider_tests` drives
one real game per rider — Double Major's non-legendary copy, Tazri's
five-colour pump, Doubling Season on a walker's starting loyalty, Force of
Negation's counter-to-exile, and the Lattice paying a green pip off Islands.

Writing those five tests is what turned up three engine bugs that no card
file mentioned, all of them in the mana path:

1. **The Lattice was half-wired.** `pay_wild` existed and was called at
   payment time, but every *affordability* check (`can_cast`, `can_afford`,
   and all four cast-wizard mode probes) used plain `can_pay`. The spell the
   Lattice made payable was therefore never offered as castable, and the
   activated ability it made affordable never appeared in `legal.abilities`.
   A capability reachable only through a code path nobody can enter is
   indistinguishable from a missing one — which is exactly what the card's
   header said.
2. **A dual land could only make one of its colours.** `intrinsic_mana` maps
   basic land subtypes to a colour with an `if/else` chain, so it silently
   returned the first match: Badlands always black, Godless Shrine always
   white, Temple Garden always green. CR 305.6 grants one ability *per* basic
   type. The shortcut cannot ask, so it now declines multi-type lands and
   leaves them to the printed `AddManaChoice` ability — which meant giving
   the four shocklands (Godless Shrine, Hallowed Fountain, Temple Garden,
   Watery Grave) the ability their siblings already had. Their headers had
   said "intrinsic mana via subtypes"; the intrinsic path could not deliver
   what they were promising.
3. **A colour choice untapped the source.** A mana ability resolves off the
   stack, but a colour choice suspends it like any resolution — and the
   resume path ended in `finish_resolution`, whose `on_stack` for a mana
   ability *is the land*. `finalize_spell` duly "resolved the permanent":
   removed `TAPPED` and moved it to the battlefield it was already on. Every
   choice-mana source in the pool — Badlands, City of Brass, Command Tower,
   Chromatic Lantern, Harabaz Druid — untapped itself and made unbounded
   mana, and handed priority to the opponent on the way. `Resolution` now
   carries a `mana_ability` flag and `finish_resolution` returns early.

The pattern worth keeping: **a stale "not supported" note is a load-bearing
lie.** Nobody writes a test for a mechanic the card says is missing, so the
half-built path underneath it never gets exercised. Bug 3 had been live in
every game containing a dual land; it took writing the test that the header
said was pointless to find it.

## 2026-09-05 — salvaging `cards/llm-batch`, and what the transcoder caught

An abandoned branch held 26 hand-written cards. Ten of them had already
reached `main` by other routes (landgen, the fetchland work), so the salvage
was the remaining sixteen — the Commander staples: An Offer You Can't Refuse,
Arcane Signet, Birds of Paradise, Commander's Sphere, Dark Ritual, Farseek,
Fellwar Stone, Lightning Greaves, Llanowar Elves, Mind Stone, Nature's Lore,
Negate, Rampant Growth, Swiftfoot Boots, Thought Vessel, Three Visits.

**The branch was not merged, and could not have been.** Three reasons, each
sufficient:

1. Its `CardIndex` values (194–214) name other cards now. The ledger is
   append-only, and it has grown from 214 entries to 1343 since the branch
   was cut — merging would have put sixteen cards at sixteen occupied seats.
2. The files predate `card!`/`face!` and the prelude, so every one of them
   restates the defaults the macros exist to supply.
3. The DSL had moved underneath them. Seven `AddMana*` variants had become
   one `Effect::AddMana { source: ManaSource, .. }`, and `SearchLibrary` had
   replaced `dest`/`tapped`/`shuffle` with a `finds: &[Find]` slice. The
   branch names variants and fields that no longer exist.

So the salvage was the normal path: sixteen names into `data/card-pool.txt`,
then `xtask codegen`. **The transcoder read eight of the sixteen in full** —
Birds of Paradise, Dark Ritual, Farseek, Llanowar Elves, Mind Stone, Nature's
Lore, Negate, Three Visits — and the other eight came out as honest stubs.

The entry worth keeping is Farseek. It searches for "a Plains, Island, Swamp,
or Mountain card"; the branch had written that as `BASIC_LAND`. That filter
is not a narrower version of the right one, it is wrong in both directions at
once: it admits a basic Forest, which the card excludes, and it refuses Blood
Crypt, which the card allows — and Farseek fetching a shockland is most of
why the card is played. The transcoder, reading the same sentence off the
reference script, produced the four-subtype `Or`. Nothing in the gate could have
told the two apart: both compile, both pass `validate`, both are
`Coverage::Implemented`.

**So where the transcoder produces a card, prefer it to a hand-written one.**
Not because a machine reads better than a person, but because the refusal
rule means it read *every* clause — an LLM that misreads one clause produces
exactly the same artefact as one that reads them all.

### The test debt was real, but half of it was a measuring error

The sixteen cards are the first or second user of four DSL features, so the
first question was which of them the engine had never played. Grepping for
`AddManaCommanderIdentity` and `AddManaLandColor` returned zero cards and
zero tests — which was fiction: those are the *old* variant names, and the
features are alive and well covered as `ManaSource::CommanderIdentity`
(Command Tower) and `ManaSource::LandColor` (Reflecting Pool). Grep the
spelling the code uses now, or the coverage number is made up.

What survived the correction was genuine, and is now three tests in
`card_tests`:

- **Equipment had two cards and no engine test at all.** Sword of Hearth and
  Home and Helm of the Host have been in the pool since M2; nothing had ever
  equipped anything. `lightning_greaves_grants_both_keywords_to_what_it_is_attached_to`
  is the first, and it checks the half that matters — the keywords land on
  the creature, not on the Equipment.
- **`LandColor { mine: false }` had a card (Exotic Orchard) and no test.**
  The two sides of that effect differ by one comparison, so a sign error
  would have produced a Fellwar Stone that reads your own lands and passed
  every test in the suite.
- **`CreateTokenForTargetController` had one card (Crib Swap) and no test.**
  An Offer You Can't Refuse resolves it *after* countering the spell it
  points at, so "its controller" has to be found through an object that is
  already a card in a graveyard.

### "The card is already on main" is not the same as "the card is finished"

The plan for the branch was: for each card, does main have a file? If yes,
nothing to salvage. That test was `[ -f ]`, and it is the wrong test. Nine
lands answered "yes" and three of them — Evolving Wilds, Terramorphic Expanse
and Rogue's Passage — were still `// GENERATED STUB`, because both readers
had refused them: landgen knows thirteen sentence shapes and none of them is
a library search, and the scripts scripts carry clauses it does not claim.
Deleting the branch on the strength of the file existing would have thrown
away the only finished version of three cards.

The check that answers the question is `grep -l "GENERATED STUB"`, and by
extension: before deleting anything, compare what the two sides *say*, not
whether both have something to say.

All three are expressible today and are now written in the current DSL rather
than copied. Two of them are one effect (`Filter::BASIC_LAND` into
`Find::BATTLEFIELD_TAPPED`, which `search_tests` already covers through
Cultivate), but the third was not covered at all: Rogue's Passage is the
first card in the pool to *grant* `KeywordSet::UNBLOCKABLE`, a keyword
`combat::can_block` has always read and no card had ever produced. Its test
attacks with two creatures and points the Passage at one, because an empty
blocker offer proves nothing on its own — a creature that could not block
anyway produces the same empty list.

Two smaller corrections came out of the same pass. Rampant Growth had
restated `Filter::BASIC_LAND` as a local `static`, which is the thing
`filters.rs` exists to stop; and Fellwar Stone's comment blamed
`granted_mana` for the client planner counting it as zero, when the seam is
`simple_mana` in `baylee-cards-dsl/src/manaread.rs` refusing a `LandColor`
source outright. `granted_mana` is about abilities a card does not print;
Fellwar Stone prints its own. A comment naming the wrong seam sends the next
reader to the wrong file, which is the same defect class as the fetchland
comments that said "tapped" where the code says untapped.

### A test that was never run is a test that was not written

The Lightning Greaves test was written in one sitting and first *ran* in the
next. It went red immediately, and not on anything about the card: equipping
worked, `attached_to` was set, and the creature had neither haste nor shroud.

`Filter::AttachedToBySource` is an input to the layer projection, and the
projection is cached behind a single generation counter. Every other write
that feeds it — a counter, a token, a zone change, a static registering —
bumps that counter. `Effect::AttachSelf` wrote `attached_to` and bumped
nothing, and so did the SBA that unattaches an Equipment whose host is gone.

The failure mode is worse than "it does not work", which is why nothing had
caught it: the grant is not lost, it is *late*. It appears the moment
anything else invalidates the cache — a creature entering, a counter, damage
— so a Sword of Hearth and Home has been giving out its +2/+2 since M2, just
not until something unrelated happened. The test found it only because it
equipped and then asked immediately.

Two lessons, and the second is the one worth keeping:

- When a new filter reads a field, ask what invalidates the projection when
  that field changes. "Nothing" is a legal answer for a field nothing reads;
  it stops being legal the moment a filter reads it.
- Equipment had two cards in the pool for months, a `validate` that passed
  and a `codegen --check` that passed, and it did not work. Card data being
  well-formed says nothing about the rules behind it running. The mechanic
  has to be *played* once in an engine test, and the test has to be run.

## 2026-09-16 — two cheap models, 27 cards, and the eight tests that failed

The four Commander decks' open stubs, split into two disjoint batches and
handed to **Gemini 3.8 Flash** (through `agy`) and **DeepSeek V4.1 Flash**
(through its Anthropic-shaped API, non-agentic: one cacheable 241 KB prefix
holding `docs/card-dsl.md`, the whole DSL vocabulary, `filters.rs`,
`tokens.rs` and four finished cards). Both lanes wrote files and neither ran
`cargo`; the build, the tests and the commit stayed here. 27 cards landed —
24 `Coverage::Partial`, 3 `Implemented` — taking the pool from 718 to 745
finished.

Then the same DeepSeek lane was asked for **one engine test per card**, with
the testkit, the shared helpers, the choice taxonomy and one whole finished
test file as its context. Twenty-seven came back and **nineteen passed on
the first build**. That ratio is the entry: a model that cannot run the code
writes a plausible test, and plausible is not the same as right.

### Seven of the eight failures were the *harness*, not the card

Each is now a comment in the test where the next reader will need it, and
each belongs in the next batch's prompt:

1. **`tap_all_mana` is the intrinsic list.** It taps what CR 305.6 gives a
   basic land type, so it taps a Forest and never Mana Vault, whose `{T}` is
   a printed ability activated by index. Three of the 27 assumed it meant
   "tap everything that makes mana".
2. **Affordability is read off the mana *pool*, not off what could still be
   tapped.** This one cost three tests in three different disguises:
   Endurance's printed `{1}{G}{G}` was not an option beside its evoke cost,
   so the wizard had one mode and never asked; Malevolent Hermit's `{U}`
   ability was missing from the offer entirely; and Mystic Remora's `{4}`
   was never asked at all, because `Effect::PlayerMayPayOr` fires its
   fallback outright at a seat whose pool cannot cover the tax. The last of
   those is an engine simplification worth knowing: a player may in the
   rules tap lands *during* resolution, and here they may not.
3. **"You control a commander" is a battlefield sentence.**
   `casting::controls_a_commander` asks the zone, so a commander waiting in
   the command zone does not turn on Deflecting Swat's free cast.
4. **A target requirement that is only a player arrives as
   `Pending::ChoosePlayer`.** The cast wizard branches on
   `TargetSpec::AnyPlayer` *before* the object half of targeting, so there
   is no `ChooseTargets` with an empty object list to inspect.
5. **A gap-strike needs a control that is not confounded.** Archon of
   Emeria's missing "nonbasic lands your opponents control enter tapped" was
   struck by playing Irrigated Farmland, which enters tapped by its own
   text: the assertion would have passed for a reason with nothing to do
   with the Archon.

### The eighth was a wrong card, and a gate caught it

Mikaeus, the Unhallowed claimed intimidate on his face and granted undying
through a static. **No engine rule reads either bit** — `ENFORCED` in
`keyword_tests` is the list — and the two halves were treated completely
differently: the face failed the lint on the first compile, and the grant
was invisible, because `CardDef::all_keywords` reads faces. Worse, the test
DeepSeek wrote *agreed with the wrong card*: it asserted that the Elf came
back. A model writes the test its own card deserves, so a test passing is
evidence about the pair and not about the card.

Both bits came off the card, and the lint now reads every `KeywordSet` in
the card's whole `Debug` rendering instead of a list of places to look
(`no_card_mentions_a_keyword_the_engine_ignores`).

### Prompt rules this batch earns

- Never claim a keyword outside `keyword_tests::ENFORCED`, on a face **or**
  through `Modifier::AddKeyword`, a pump's `keywords`, or a `CopyMod`.
- Before asserting that an ability is offered or a spell castable, put the
  mana in the pool: `tap_all_mana` first, and add lands until the pool
  covers the cost twice over if a tax is to be *asked* rather than skipped.
- `tap_all_mana` does not tap a nonland permanent's printed mana ability.
- Read the question shape off `choice.rs` rather than assuming
  `ChooseTargets`: a player-only requirement is `ChoosePlayer`, a cost
  sacrifice is `ChooseCards { prompt: CostSacrifice }`, an alternative cost
  is `ChooseCastMode` — and only when more than one mode is affordable.
- When striking a `Coverage::Partial` gap, pick a control that cannot
  produce the asserted outcome for any other reason.

## Rounds E and F, 20.09.2026 — 70 lands, and the refusals started clustering

Two rounds back to back over the land stubs, ranked by oracle length
descending with the refusal shelf subtracted first. E: 40 picked, 38 written
(9 `Implemented`, 29 `Partial`), 2 refused. F: 40 picked, 32 written (all
`Partial`), 8 refused. Stubs 550 → 480.

### The budget was refusing cards, and it looked like the model

Five of round E's forty came back with no text at all, and round C had lost
six the same way. They were `max_tokens`, and nobody could see it: the lane
returned an empty answer as an empty answer. `lane.py` now raises with
`stop_reason` and the output token count in the message, and every one of the
eleven was written **unchanged** on a retry with a bigger budget — no prompt
change, no different model.

The number itself was a sibling asymmetry of the kind this repo keeps finding:
`tests_deepseek.py` had asked for 32000 since it was written and
`cards_deepseek.py` for 16000, with nothing deciding the difference. The cap
is not a price — DeepSeek bills the tokens generated — so the smaller number
bought nothing. One card (Great Hall of the Biblioplex) needed 60000 even
after the raise, so the retry-with-more-budget step stays.

### The lane writes Rust that compiles and that rustfmt disagrees with

A `Coverage::Partial` reason too long for its line is the usual shape.
`cargo check` accepts it without a word, so round E reached a commit with
seven unformatted files and the *gate's* fmt step found them. `cargo fmt
--all` now runs **before** `cargo check`, as the fourth rule in
`scripts/llm/README.md`.

### A refusal cluster is worth more than a refusal

Round F's eight refusals were not eight problems. **Five** were banding —
the entire printed text of those cards — and **two** were hideaway. Each
became one ticket naming every card behind it (#164, #163), which is the
output a batch should produce when the DSL runs out: not "eight cards
failed" but "two sentences the DSL cannot say, worth twelve cards between
them".

Reading them together also showed the lane is inconsistent about the choice
it has: three hideaway lands were written as an honest `Partial` carrying
the mana ability, two were refused outright. Both are honest and the
`Partial` is strictly more useful, since only `Implemented` cards reach the
deckbuilder.

### A generated constant is not a guessable one

One card arrived naming `generated_tokens::ROBOT_2_2_COLORLESS`. The real
constant is `ROBOT_ARTIFACT_2_2`, and the model had described the token
correctly and then spelled its handle the way the oracle text reads. A
generated table's names come from the generator, not from the card.

### What the test lane taught, which was about the kit

Both of round E's two test failures were the harness rather than a card, and
the second outlived its test. `tap_mana_except` iterates
`legal.mana_abilities`, which is documented as **only** the CR 305.6
shortcut, so a nonbasic printing `{T}: Add {C}` is never tapped and a test
floats fewer mana than it thinks (#159). That surfaced only because the test
expected a success; one expecting a *refusal* would have passed on "not
enough mana" instead of the rule it names.

### Prompt rules these batches earn

- Never invent a `generated_tokens::` constant. Grep
  `crates/baylee-cards/src/generated_tokens.rs` for the shape you want and
  copy the name; the fields are `&TokenDef`, so it is borrowed.
- `activate(&mut engine, seat, card, index)` finds the **first** object with
  that card index. A test that seats two copies, or whose preset already
  places one, must address the object it means directly with
  `PlayerAction::ActivateAbility`.
- `tap_mana_except` and `tap_all_mana` reach the basic lands and nothing
  else. Tap a nonbasic's printed mana ability by hand.
- `Duel::start` deals **no** opening hand. A test that needs one says so.
- Prefer an honest `Partial` carrying whatever the card *can* do over a
  refusal, when one clause is unreadable and the rest is a plain mana
  ability.
