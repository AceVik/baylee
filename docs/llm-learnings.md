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
- No-lose suppression: `Modifier::PlayersCantLose` is read by one predicate,
  `sba::players_cant_lose`, which both the SBA loss check and
  `sba::lose_by_effect` (an effect saying a player loses, e.g. an unpaid
  pact; #237) consult. A new "you lose the game" effect goes through
  `lose_by_effect`, never straight to `eliminate_player`, which does not
  check (its other callers are concession, which "can't lose" does not
  stop, and the SBA loop, which checks first). `CantLoseLife { who }` is
  checked by the life door `GameState::change_life` for every loss and by
  `GameState::can_pay_life` for every payment (CR 119.8). It used to be
  checked only in the `LoseLife` resolve op, keyed on the wrong player, so
  damage and payments went past it (#244). Never add the check at one loss
  site.

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
  waterbend is its own `waterbend = true` (#229). It was written as
  convoke, which let its taps pay the printed `{1}` and asked for them after
  the waterbend was declined.
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

1. **`tap_all_mana` was the intrinsic list.** It tapped what CR 305.6 gives
   a basic land type, so it tapped a Forest and never Mana Vault, whose
   `{T}` is a printed ability activated by index. Three of the 27 assumed it
   meant "tap everything that makes mana" — and on 20.09.2026 #159 decided
   the three were right and the kit was wrong. It takes both lists now; the
   prompt rule below is the current one.
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
  *(Superseded 20.09.2026: the second half was a workaround for a defect.
  `Effect::PlayerMayPayOr` puts the question whether or not the mana is
  floating — CR 605.3a lets the player make it inside the question — and
  `resolve/mod.rs` says so where it asks. The extra lands now test nothing.
  Its sibling `PlayerMayPayCostOr` does skip a question nobody can answer,
  which is where this sentence is still true.)*
- `tap_all_mana` taps every mana ability whose whole price is `{T}`, on a
  land or not — so a Sol Ring and a Llanowar Elves go with the Forests. If
  the test needs one of them left standing, name it:
  `tap_all_mana_but(&mut engine, seat, Some(sol_ring()))` keeps a printing
  back and `tap_mana_except(&mut engine, seat, object)` keeps one object,
  and `cast_with_floating` then casts off what is floating.
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
the second outlived its test. `tap_mana_except` **iterated**
`legal.mana_abilities`, which is documented as **only** the CR 305.6
shortcut, so a nonbasic printing `{T}: Add {C}` was never tapped and a test
floated fewer mana than it thought (#159). That surfaced only because the test
expected a success; one expecting a *refusal* would have passed on "not
enough mana" instead of the rule it names.

Closed the same day. One primitive taps for the whole crate now, it reads
both lists, and it refuses to return while a land it should have tapped is
still standing — so the next time either list stops carrying something, the
kit goes red instead of quietly floating less. What it deliberately does not
press is a mana ability whose price is something other than its own tap:
Wall of Roots would be a smaller creature afterwards and Ashnod's Altar
would have eaten one.

### Prompt rules these batches earn

- Never invent a `generated_tokens::` constant. Grep
  `crates/baylee-cards/src/generated_tokens.rs` for the shape you want and
  copy the name; the fields are `&TokenDef`, so it is borrowed.
- `activate(&mut engine, seat, card, index)` finds the **first** object with
  that card index. A test that seats two copies, or whose preset already
  places one, must address the object it means directly with
  `PlayerAction::ActivateAbility`.
- `tap_mana_except` and `tap_all_mana` reach **every** source whose mana
  ability costs exactly `{T}` — basics, nonbasics, artifacts, mana
  creatures. A test that needs one of them untapped afterwards names it, and
  says in a comment why.
- `Duel::start` deals **no** opening hand. A test that needs one says so.
- Prefer an honest `Partial` carrying whatever the card *can* do over a
  refusal, when one clause is unreadable and the rest is a plain mana
  ability.

## Round H — eighty lands, and what both lanes got wrong

80 picked, 71 written (25 `Implemented`, 46 `Partial`), 9 honest refusals;
stubs 432 → 361. DeepSeek 28 of 30 in 5.5 minutes, Gemini 43 of 50 in 19.5
minutes at 683M input tokens. The shelf — names sent in an earlier round
that are still stubs — was subtracted before the ranking, which is what kept
thirty known refusals out of the batch.

### The Gemini lane had no door, and three cards walked through it

`cards_deepseek.py` refuses an answer that rewrote the `//!` block or one of
`index` / `oracle_id` / `scryfall_id`. The Gemini lane made no such check,
because `agy` writes the tree itself and there is no answer left to refuse —
so Skyline Cascade, Iron Hills and Shefet Dunes each arrived with a
different `oracle_id` and `scryfall_id` while writing the *right* card's
text. The pool lint `every_card_answers_to_the_oracle_id_it_is_filed_under`
caught **one** of the three: it panics on the first card it disagrees with,
so the other two were invisible until that one was fixed and the suite run
again.

The predicate now lives in `lane.keeps_the_header` and both lanes ask it —
DeepSeek before it writes, Gemini against `HEAD` afterwards, where the
answer is `lane.restore_generated` rather than a refusal. A lane that names
all three at once beats a lint that names one.

### The cross rule paid for itself twice

`Effect::Exile` read `res.targets.first()`, so Pit of Offerings took three
targets, exiled one and left two — `Coverage::Implemented` and silent. A
generated test found it because the test was written from the printed
sentence by a model that had not written the card.

And eleven of the seventy-one tests were wrong, which is the same rule
working from the other side. Five clusters, now rules 11–16 in both test
contracts:

- **A mana creature counts.** Five tests asserted `mana_pool.total()`
  against the number of *lands* while two Llanowar Elves stood beside them,
  which `tap_all_mana_but` taps like anything else. A creature tapped for
  mana also cannot attack, which is how the sixth of those tests failed.
- **A land that entered tapped gives nothing that turn**, and an ability
  costing `{T}` is not even offered. Mishra's Factory is the sharp case:
  it animates *itself*, and from that moment CR 302.6 reaches a creature
  that was never summoned, so its own pump is unpayable until its next turn.
- **"Spend this mana only …" is `RestrictedMana`.** `ManaPool::available`
  reads the plain pool and finds none of it — a test asserting zero there is
  green and describes a land that produces nothing.
- **`tap_all_mana_but` returns `()`.** Two tests bound its result and
  compared it to a number.
- **Boards have to support the claim.** One test expected combat damage from
  a 0/2, another pointed "target **Dwarf** you control" at an Elf.

### The token budget was the refusal, again

`tests_deepseek.py` asked for 32000 and 7 of 43 came back empty — six
`stop_reason='max_tokens'`, one truncated fence. All seven were written on a
retry at 64000 with nothing else changed, which is the measurement
`cards_deepseek.py` had already made for cards. Both lanes ask for 64000 now.

### The nine refusals, grouped

- **Rooms** (2): Jidoor, Aristocratic Capital // Overture and Midgar, City
  of Mako // Reactor Raid — two halves on one card with doors to unlock, and
  `FaceDef` has no shape for it.
- **Reflexive triggers, CR 603.12** (2): Obscura Storefront and Brokers
  Hideout — "when you do, …" after an optional cost.
- Island of Wak-Wak: `Modifier::SetPT(i16, i16)` sets both halves together,
  so "has base power 0" (layer 7b alone) cannot be said.
- Grove of the Guardian: `CostPart::TapOther(&Filter)` taps exactly one
  permanent and carries no count.
- Echoing Deeps: copying a land card in a graveyard as an enter replacement.
- Dakmor Salvage: dredge.
- Scorched Ruins: "sacrifice it unless you sacrifice two untapped lands".

## Round J — the first non-land rounds, and where the tail flattens

The lands were finished in round I, so round J is the first batch drawn from
the rest: 199 fresh stubs after the shelf comes off, ranked by printed
length — 60 creatures, 46 instants, 33 enchantments, 30 sorceries, 25
artifacts, 3 planeswalkers. Three batches of 75/75/49 went to DeepSeek.

**Batch 1: 51 of 75, 26 `Implemented` against 25 `Partial`.
Batch 2: 42 of 75, 4 against 38.** Same lane, same contract, same prompt —
what changed is only how deep in the ranking the batch sat. Stubs 241 → 148.

### The residue stops clustering, and that is the finding

Round F's 8 refusals were two tickets and twelve cards, because five of them
were banding. Grouping batch 2's 38 `Partial` reasons finds **no cluster at
all**: a copied activated ability, an intervening `if` on an upkeep
transform, an `Amount` that adds a constant to a count, "if {C} was spent to
cast it", split second, a fight with a delayed return, cast-a-permanent-of-
each-type-from-your-graveyard. Every one its own missing sentence.

So the arithmetic that made a blocker worth taking has changed. Past the
first couple of hundred cards the next rule bought is worth one or two cards
rather than a dozen, and the instrument to rank with is
`transcode-report --stubs` rather than another length-sorted batch. Length
was the right proxy while the shallow end was full; it is a worse one now.

One cluster did survive the grouping and became **#201**: `ReplacementRule`
has four variants and all four are continuous static abilities, so nothing
can say the *one-shot* shape — "the next time this would be destroyed this
turn". That is regenerate (7 unfinished cards) and "if it would die, exile
it instead" (5 more, three of them the same cards) as one rule.

### `Partial` reasons are claims, and four of them were checked

"No `Effect::Regenerate` exists", "no `Filter` reads a counter on a
permanent", "none compares a power", "exalted is neither a keyword bit nor a
trigger". All four true today, all four checked against the DSL rather than
believed. A stale "cannot be expressed" is how a card that *could* be
written stays a stub, and `docs/card-dsl.md` is a contract the lanes obey
literally.

### A layer is part of the claim, not an implementation detail

**Ashaya, Soul of the Wild** came back `Coverage::Implemented` with its `*/*`
modelled as a printed 0/0 plus `Modifier::ModifyPTPerCount`. That is a
sensible spelling and it is not the card: a characteristic-defining ability
applies in layer 7a and `ModifyPTPerCount` derives to 7c, so the two agree
on every board until a layer-7b effect sets power and toughness and disagree
after one. `validate`'s printed-`*` check caught it — eight other pool cards
spell a CDA this way and every one of them already said `Partial`.

### `castable_from_hand` defaults right for one layout and wrong for the other

`FaceDef::castable_from_hand` defaults to `true`, which is correct for a
modal double-faced card's back face and wrong for a transforming one: CR
712.8a gives a card in hand only its **front** face's characteristics. **The
Everflowing Well // The Myriad Pools** — a transform DFC whose back is an
artifact *land* — left the default in place, and the engine offered and
accepted it as a land drop out of hand.

Nothing in the pool would have said so. It was found by a sweep written for
round J's thirteen modal double-faced cards
(`every_modal_back_face_land_is_played_as_the_land_it_prints`) that plays
*every* spell-fronted land back in the pool — 58 of them — rather than only
the thirteen. The thirteen are pinned by name beside it so a shrinking
population is a failure and not a quieter pass.

Two things that sweep had to learn to be worth running:

- **A back face that shares a type with its front** makes "the front face's
  type is absent" false about a correct card. The Myriad Pools is an artifact
  land behind an artifact. The claim that was meant is equality with the back
  face's own type line.
- **Fifteen back faces ask a question on arrival** — "you may pay 3 life. If
  you don't, it enters tapped" — which is asked before the land is on the
  battlefield, so the harness' `play_land_face` returns a refusal for a
  perfectly good card. They are named in a pinned list rather than filtered
  out silently, and Fell Mire, the one of round J's thirteen among them, got
  a test of its own. A sweep that skips a card and leaves it untested has
  only moved the gap.

### The quota that looked like a wall

Round I's Gemini card lane wrote **0 of 51** and its test lane 10 of 60, with
`RESOURCE_EXHAUSTED (code 429): Individual quota reached … Resets in 3h` in
its own transcript. That reads like "wait three hours" and is not: the quota
is **per model family, not per account**. The same CLI, the same second,
refused `gemini-3.7-flash-high` identically and answered
`gpt-oss-120b-medium`. `BAYLEE_LLM_AGY_MODEL` now overrides the constant and
every report line names the model — without that, a report saying "Gemini"
while another model wrote the batch makes the cross rule unauditable
afterwards, and the cross rule is the one thing about these lanes a later
reader cannot re-derive from the files.

Two measurements worth keeping: the reset is a **fixed wall-clock time** and
probing does not push it out (two readings thirteen minutes apart named the
same instant), and a one-word probe does **not** predict capacity for a
batch — `gpt-oss-120b-medium` answered a probe and then returned
`UNAVAILABLE (code 503)` for a ten-card session, twice. `claude-sonnet-4-6`
took it and wrote 10 of 10.

### What the substituted test lane still got wrong

Three of those ten failed, and all three are rules the contract now carries:

- **`tap_all_mana` presses the card under test.** Creeping Tar Pit's
  `{T}: Add {U} or {B}` is a mana ability whose whole cost is its tap symbol,
  so the sweep tapped it and the activation that followed was refused for a
  land already tapped. `tap_all_mana_but(…, Some(slug()))` is the door.
- **One producible colour is not a choice.** Horizon of Progress adds "any
  type a land you control could produce"; over two Forests the engine has one
  answer and asks nothing, so a test expecting `Pending::ChooseColor` is
  describing a board it did not build. A Forest and an Island make the
  question real.
- **Floating mana is counted too.** Lake of the Dead's line adds `{B}{B}{B}{B}`
  and the test read five, because `tap_mana_except` had left the fodder Swamp
  and an Island in the pool first. The line's whole cost is `{T}, Sacrifice a
  Swamp`, so an empty pool is what makes "four and nothing else" exact.

None of the three is a card defect and all three read like one, which is the
argument for the coordinator compiling before believing a red test — and for
the cross rule, since a lane checking its own card would have "fixed" the
card instead.

## Round K — the lane hits its floor, and what a floor looks like

Seventy fresh stubs to DeepSeek in two batches of thirty-five. **Seven cards
came back and six were kept**, all `Coverage::Partial`; the other sixty-three
are honest refusals. Round I ran at roughly two in three written, round J at four in
five over its first batches. Two in twenty is not a worse lane — it is the
same lane meeting a residue that no longer contains anything a reader can
write, and that is the number this round is for.

The refusals are the evidence, not the yield. Grouped by what they name,
sixty-three refusals over two batches produce **no entry above thirteen**,
and the two largest are a filter and a search destination rather than a
mechanic: `Effect::SearchLibrary` (13 mentions) and `Filter::CmcAtMost`
carrying a fixed `u32` where the card prints X (6). Everything else is a
singleton — Rooms and their doors, devotion, a delayed triggered ability, a
counted sacrifice, a draw limit, dilemmas. `transcode-report --stubs` says
the same thing from the other side: its top entry over the pool's own stubs
is `AlternateMode:` at eleven, which CLAUDE.md already measured as the worst
buy on the list, and the rest of the top twenty is ones and twos.

### A lane cannot see a pin

The seventh card was Crop Rotation, and it is the one worth reading. The
lane wrote it as a `Partial` whose unwritten clause is its printed
additional cost, "sacrifice a land" — and a dropped *cost* is not a weaker
card than the printing, it is a cheaper one. An earlier session had already
found exactly that, reverted the card to a stub, and left the reason behind
as a test: `crop_rotation_is_a_stub_until_a_spell_can_charge_more_than_mana`
pins `Coverage::Unimplemented` until a spell cost list can charge something
that is not mana. That test went red on the lane's card, and the card is a
stub again.

No lane runs cargo, so **no lane can see a pin**, and the refusal shelf that
`pick.py` reconstructs is built from `feat(cards)` commit bodies — which is
a record of what a lane refused, not of what a person decided. The guard is
the only thing between "a previous session decided this" and the card
quietly coming back on the next length-sorted pick. Keep such decisions in a
test, never in a comment on the stub: codegen rewrites every file carrying
the stub marker, so a note left in one does not survive the next run.

So the lane is finished as a volume instrument, and the two gaps it named
loudest were worth closing by hand the same afternoon.

**`Amount::BasicLandTypesAmong`** — domain. Three stubs printed "for each
basic land type among lands you control" and none of the amounts already
there could say it: `CountOf` counts objects, so two Forests answer 2 where
the card wants 1 and a Tundra 1 where it wants 2, and a land is colourless,
so `DistinctColorsAmong` answers 0 for any board of them. Domain is an
ability word with no rules meaning of its own (CR 207.2c), so the count sits
on the card and the variant takes a filter rather than hiding "lands you
control" in the engine. `SubtypeSet::BASIC_LANDS` was already CR 305.6's
five, which is the reason no sixth list of Plains/Island/Swamp/Mountain/
Forest was written — the pool has four of those and they are a mapping to
mana, not this question.

**`Filter::CmcAtMostX`** — "a creature card with mana value X or less". The
bound is read off the ability's **source**, because `eval::matches` is
handed `this` and no announced number beside it, and `cast_wizard` already
writes the announcement to `x_value` there. Threading `x` through the
matcher would have touched every call site in four crates to reach the same
value. Three readers had to learn the variant and two of them answer *no*:
`baylee-ai` and the client's `targeting.rs` both refuse it, because a
`PlayerView` carries no announced X and answering `true` would let a planner
count on a tutor finding a card the search may not legally find.

Five cards came out of the two: Gaea's Might, Evasive Action, Power Armor,
Chord of Calling, and Green Sun's Zenith as a `Partial` — nothing moves a
resolving spell anywhere but the graveyard, so its own shuffle-back clause
is still unsayable.

### The test lane has a session size, and it is eleven

Round I's tests were handed over in worklists of thirty-four. The first such
session wrote **1 of 34** in 10.3 minutes over 101 steps: it read the
framework, read the cards, announced it had everything it needed, wrote one
file and ended. The ten-card pilot before it wrote 10 of 10, and the
thirty-four-card session before that wrote 10. Eleven is the size that
finishes; thirty-four is a session that spends its steps orienting and then
runs out while writing.

That is also the last measurement of the day, because both `agy` model
families answered `RESOURCE_EXHAUSTED (code 429)` within three minutes of
each other — `claude-sonnet-4-6` first and `gemini-3.8-flash-high` on the
retry, with a shared reset about four and a half hours out. The cross rule
has no way around that: DeepSeek wrote these cards and may not test them, so
the debt waits for the quota rather than moving to the other lane.

## Round L, 22.09.2026 — the contract was lying, and ten cards off a corrected one

Round K ended with the DeepSeek card lane at its floor: 70 fresh stubs in, 7
written. The obvious next move was to grow the DSL from
`transcode-report --stubs`, and the ranking said the tail was flat — 81 stubs,
top cause 7, then 5, 3, 2, 2, 2. Nothing there was worth a night.

What was worth a night was **reading what the lane is told**. `scripts/llm/`
packs `docs/card-dsl.md` into every card prompt as the authoring contract, and
that file ends in a hand-kept list of mechanics "not supported yet (M3+)".
Fourteen entries, each naming an example card.

Thirteen of the fourteen named a card that is `Coverage::Implemented` today.

Mox Opal carries the metalcraft activation condition the entry says cannot be
written. Bleachbone Verge carries the same rule under the other name — and is
one of the five example cards the DeepSeek prompt ships, so the lane was
reading a finished use of a mechanic on one page and a sentence saying it is
impossible on another. Urza's Saga has its chapters. Mirrorhall Mimic has its
disturb. Chromatic Lantern has its land grant. Venser's emblem has its
trigger. City of Brass has its becomes-tapped trigger. Opposition Agent has
the real search takeover the entry describes as "approximated as a lock". Path
of Ancestry, Padeem, Reflections of Littjara, Wizard Class, Everybody Lives!
and the daybound villagers are the rest, most of them with no
`// NOT SUPPORTED:` line at all. Only battles survived, on the strength of
Invasion of Ikoria still being a stub.

**A `Coverage::Partial` on a card the engine can play is the expensive kind of
wrong**, which is why this is worth a section rather than a line. It is not a
stub, so no residue ranking lists it. It reads as finished, so no sweep asks
about it. The deckbuilder offers it as second-class and nothing anywhere says
why. How many of them this list bought is not measurable after the fact — what
is measurable is that it stopped being able to buy more.

The check costs nothing and is the shape to repeat: an entry names a card, so
open the card and read its `coverage` and its `// NOT SUPPORTED:` lines. Four
greps and nine file reads settled all fourteen.

### The residue after that is honest

All 82 remaining stubs went back to the lane in three batches against the
corrected contract. **Ten came back**, all `Partial` — 2, 4 and 4. That is
roughly round K's rate and not a jump, which is the answer to "how much was
the contract costing": on *this* residue, not much, because a stub is by
construction a card no reader could write. The refusals now name a missing
`Amount`, a missing `Effect`, a missing `Modifier`, one at a time, and the
banding cluster (five cards wanting "bands with other legendary creatures")
is the largest single thing left.

Two of the ten are the afternoon's `Filter::CmcAtMostX` paying for itself
without being aimed at anything: Reshape and Finale of Devastation both write
it, and Whir of Invention is the third of the family. Nobody put those cards
in front of the lane — they were in the batch because every stub was. **That
is the argument for growing the vocabulary over picking cards**: the lane
reaches a new word on its own, in a batch nobody aimed at it.

### A blocker entry is a sentence, and `cmcLEX` is two of them

`transcode-report --stubs` ranked `filter atom cmcLEX` at 3 and `cmcEQX` at 2,
which reads like one rule worth five cards. Reading the five scripts says
otherwise. `X` is an `SVar` and the five define it two different ways:
`Count$xPaid` is the announced number, which `Filter::CmcAtMostX` already
says, and `Sacrificed$CardManaCost/Plus.N` is the mana value of the permanent
sacrificed as a cost, plus a constant — a different reading with no DSL shape
at all. Two of the five are the first kind and three are the second, and
`cmcEQX` is *entirely* the second.

This is the fourth time the same lesson has been paid for (`Pump`, `Mana`,
`Sacrifice`, now this), and it has a compact form: **a refused value is named
by what it resolves through, never by its own spelling.** The report already
does this for `TokenAmount$ X`; the filter atoms do not yet.

### The quota is sometimes per model family and sometimes not

`lane.py` recorded, measured on 22.09 in the afternoon, that a session
refused by two `gemini-*` models was answered by `gpt-oss-120b-medium`
through the same CLI in the same second — so a lane held by the quota has a
way out that is not waiting. At 02:22 the next morning `gpt-oss-120b-medium`
refused with the same `RESOURCE_EXHAUSTED (code 429)` and the gemini family
was still reporting `Resets in 1h58m`. Both readings are true and the note
now says so: try another family, it costs one probe, and be ready for the
clock.

The sharper half is how a refusal *looks*. `agy` retries an exhausted model
for the whole `--print-timeout` and then exits **0** with partial output, so
a two-minute probe that returns nothing is indistinguishable from a slow
session. The reason is written in exactly one place: the session's own sqlite
transcript under `~/.gemini/antigravity-cli/conversations`, one
`API error (attempt N)` row per retry. A lane that is merely being refused
looks exactly like a lane that is working, and forty minutes of a batch can
go into finding that out the slow way.

### Nineteen shelf cards and ten round-L cards were tested by hand

The cross rule is not negotiable — a card and its test from one model share
that model's misreading — so with both `agy` families out, the coordinator
wrote all twenty-nine tests. That is slower per card than a lane and it
produced tests a lane does not write, because a person can see what a
`Coverage::Partial` is *for*: eight of the twenty-nine assert that a gap is
still there. Land Cap untaps with a depletion counter on it, which the
printing forbids. Legion's Landing adds exactly one permanent, there being no
Vampire token to make. Collector Ouphe watches an artifact's ability resolve.
Grove of the Guardian offers one ability with two untapped creatures standing
ready to pay for the other.

Every one of those is **meant to fail** one day, and the commit that closes
the gap is the commit that deletes the assertion. A pin is cheaper than a
`// TODO` because it cannot rot in silence.


## 22.09.2026 — the pin that reads the pool, and a lock the DSL could not say

Two findings, and the first one is about **every** test in this repo that
says a card does not offer something.

### `legal.abilities` is filtered by `can_afford`, which reads the pool

`abilities.rs` pushes an entry only `if self.can_afford(player, id, cost)`,
and `can_afford` reads the **mana pool** — not the untapped lands beside it.
So a missing ability with any mana cost at all is absent from the offer
whatever the board looks like, and a test asserting

```rust
assert_eq!(legal.abilities.iter().filter(|(id, _)| *id == land).count(), 1);
```

over an untapped board would keep passing the day the ability was written.
It proves nothing, and it looks exactly like a pin that works.

Measured rather than reasoned about. An `activated!` injected into Dungeon
Descent came back **absent** from the offer at `cost!("{4}", TapSelf)`, at
`cost!("{1}", TapSelf)` and at `cost!("{4}")` alike, with six untapped
permanents standing there — and **present** at `cost!(TapSelf)` and at
`Cost::FREE`. The first repair was to put more lands on the battlefield,
which is the same vacuous pin with a longer board; the second was
`tap_mana_except(&mut engine, seat, land)`, keeping the land itself untapped
because that is what both the mana ability and the missing one tap.

Three pins in `lands.rs` were standing on it (Grove of the Guardian, Dungeon
Descent, Howltooth Hollow) and each now goes red under an injected ability
at the card's own printed price. The pool is read as a **delta** afterwards,
because a seat with mana floating already has some of the colour the mana
ability makes.

The same trap has a second door: `!legal.castable.contains(&x)`. A
Counterspell with an empty stack is refused for having no target, so a
Silence test built on one passed against an engine with the rule removed.
Both halves are now in `prompts/tests-gemini.md` and
`prompts/tests-deepseek.md` as rule 3a and rule 9, because the lanes were
writing this shape by themselves — six of the twenty-two tests that came
back in the first two chunks of the test-debt run assert on `legal.abilities`
with nothing floating.

### `Modifier::OpponentsCantCast`, which three cards had already named

Ranger-Captain of Eos carried the gap in its own `// NOT SUPPORTED:` line:
"no Modifier says the effect's opponents can't cast this kind of spell".
That is a permission rather than a timing rule, so it is its own variant
beside `OpponentsCastAsSorcery` and its own `CastError::Forbidden` — a
player told "sorcery-speed timing not met" on their own main phase with an
empty stack goes looking for a rule that is not there.

The filter is what turns one bit into a sentence: `Filter::Any` is Silence,
`Filter::NONCREATURE` is the Ranger-Captain, and Drannith Magistrate needed
no new rule at all — "from anywhere other than their hands" is
`Filter::Not(&Filter::InZone(ZoneRef::Hand))`, which makes a commander, a
flashback card and an adventure in exile one sentence. Stubs 72 → 70, and
the Ranger-Captain left `Coverage::Partial`.

The blocker-entry lesson holds a fifth time: this was not a missing
subsystem. `timing_allows` already read two player-scoped modifiers,
`eval::matches` already answered `InZone`, and what was missing was one
variant that says "and not at all".

### The number CR 602.2b announces, which nobody was asking for

The pin written for Lair of the Hydra was the finding. `{X}{G}: this land
becomes an X/X` was paid as `{G}`: `abilities.rs` reached
`Pending::ChooseNumber` only through `counter_x_part`, which reads the
storage lands' "remove X storage counters" and nothing about mana, so X was
0, the land became a 0/0 and a state-based action buried it before anybody
could attack. Three more cards made the same silent zero — Treasure Vault
sacrificed itself for no Treasures, Kessig Wolf Run pumped by nothing, Blast
Zone added no charge counters — and all four read as correct from every
other side: right header, right effect, right price, `Coverage::Implemented`
on three of them.

What made it invisible is that a pump of zero and a pump that never happened
are the same board. The existing Kessig Wolf Run test asserted the trample it
also grants and said "with X=0" in its own doc comment, which is a test
documenting the bug it was standing on.

Two halves, and the second is where an activation differs from a cast.
`pay_cost` now pays `cost.mana.with_x(x)` — without it the variable pip is
worth nothing, which is a no-op on the cast path, whose wizard substitutes
before it gets there, and the whole of the cost on an activated one. And the
question is **bounded by what the pool can pay** rather than offered up to
`X_CEILING`: a cast that cannot pay unwinds back to the player, an
activation has nothing to unwind to, so the bound is the legality.

One answer means one question, so a cost carrying both kinds of X would take
the counter bound and pay the mana with it. No cost in the pool does, and
`lints::no_cost_announces_two_different_xs` is the guard that fails with the
card's name the day one prints both — the sibling of the lint that already
holds the announcement's *order* against the storage lands.

## Round M, 22.09.2026 — sixteen stubs at the cheap lane, and two came back

The Gemini quota was spent for three hours, so the test debt went to the
coordinator by hand and the DeepSeek lane was pointed at the *stub residue*
instead: sixteen of the 68 cards the transcoder could not write, picked by
reading their printed text for sentences the DSL looks able to say.

**Two came back as cards and fourteen as named refusals**, which is the
expected shape for this population and the reason the batch was worth
sending anyway: a refusal here is a sentence about the DSL, and fourteen of
them cost a few cents. Time Sieve is written in full — "{T}, Sacrifice five
artifacts" is five `CostPart::Sacrifice` parts, one permanent and one
question each. The refusals name real gaps, and three of them are one rule
apiece: an `Amount` for the greatest mana value among a filter (Accelerated
Mutation), an `Amount` reading the *target's* toughness with a branch on it
(Blood Lust), and a `Modifier` that reduces activation costs with a floor of
one mana (Training Grounds).

### Dropping a **cost** is not a `Partial`

Crop Rotation came back `Coverage::Partial`, with the printed additional
cost — "sacrifice a land" — refused by name in a comment and the search
written in full. That was rejected and the file put back to its stub.

The rule the lane had no way to know, and that the authoring contract now
has to say: `Partial` is for a clause whose absence makes the card **weaker**
— a missing ability is an ability the player does not get. A missing *cost*
makes the card **stronger** than the printing, and a one-mana instant that
tutors a land onto the battlefield for free is a different and better card
than the one Wizards printed. There is no honest partial version of that, so
the coverage is `Unimplemented` and the card stays a stub.

This card is the one the repo had already learnt it on:
`instants::crop_rotation_is_a_stub_until_a_spell_can_charge_more_than_mana`
is a pinned limitation that says so, and it is what caught the regression —
a played test would have passed, because the card *does* find a land.

## 23.09.2026 — eight test batches, 359 tests, and the same six misreadings

Eight lane batches of engine tests over machine-owned cards (DeepSeek
55 + 35 + 59 + 60 + 60, Gemini 30 + 30 + 30) came back to one reviewer who
ran them. 27 of the 359 were red — 4, 12 and 11 over the three assemblies —
and 26 of the reds were the test misreading the harness or the rules. The
27th was both, and is the one worth reading first (below). What makes the
other 26 worth a section is that they were the *same* misreadings, batch
after batch, and that each is one sentence the prompt had not said:

- **CR 302.6.** A creature cast this turn cannot pay `{T}`, so a test that
  casts Joven and then asks for his `{R}{R}{R}, {T}` finds nothing offered.
  Seven tests over three batches. The creature starts on the battlefield,
  or the test walks a turn cycle.
- **`.battlefield(seat, …)` replaces.** A second call for the same seat
  drops the first list; Zephid's Islands vanished that way.
- **A player-only target is `ChoosePlayer`.** Soul Feast, Natural Spring,
  Last Caress.
- **"… unless you sacrifice …" asks `(0, 1)` and no yes-or-no first.**
  Lithophage and Thing from the Deep.
- **A spell being cast still counts in the hand** while its target
  question is open.
- **CR 113.6.** Briarknit Kami does not see its own cast.

All ten such rules are now numbered in both `prompts/tests-*.md`. The
general point is the one "fix the reader, never the card" makes for the
transcoder: a prompt that produced the same wrong test three times is the
defect, and correcting the fourth test by hand only buys a fifth.

**Last Caress was a card defect hiding behind a test defect.** "Target
player loses 1 life and you gain 1 life. Draw a card." The lane's test
answered a `ChoosePlayer` as if it were an object target, and it aimed the
spell at its own caster — the one aim under which "you gain" and "the
target gains" are the same player. Re-aimed at the opponent it went red for
the card: the target lost the life, gained it back and drew, and the caster
got nothing. The transcoder read an absent `Defined$` as "the target"
whenever the *chain* targeted a player, and a reference sub-ability inherits
no target — it says `Defined$ Targeted` when it means one. The rule is now
"this line's own target" (`scriptgen::player_rel_of`), with a regression
test beside Piranha Marsh's; one card in the pool changed. The lesson for a
reviewer: a self-targeting test of a drain spell cannot fail on the
question the spell exists to answer, so an aim that makes two readings
coincide is a finding in itself.

One mistake was the reviewer's rather than the lane's, and is recorded
because it is easy to repeat: a regex meant to demote one orphaned `///`
block rewrote every doc comment followed by a blank line — including four
in the committed half of a 70 000-line file — and a second regex meant to
undo it promoted `//` comments that had been `//` on purpose. The repair was
to take the committed region back from `HEAD` whole. A rewrite over a
generated-size file is checked against `git diff` hunk by hunk before
anything else runs. The same pass also lost
two lane doc blocks written as `//!` — an inner doc, which cannot sit
mid-file and was dropped rather than turned into `///` — and demoted three
test descriptions to plain `//` above a card's handle function instead of
its test; both were recovered from the lane's own files.

A four-card test batch for the damage-to-each cluster (24.09.2026) came back
green on the first compile and the first run. That had not happened before
with DeepSeek, and it earned one addition: Dragonback Assault's test played
"each creature" on both sides of the table and never put a planeswalker
anywhere. "And each planeswalker" was half of the printed sentence and had
no witness. A clause with nothing on the board to read it passes whatever
the rule does, so a reviewer checks every noun the sentence names against
the board, not only the ones the test asserts about. Karn, the Great Creator
was added, at loyalty 5 so that 3 damage leaves him standing to be read.

The next four (24.09.2026, the four lands that exile from a graveyard as a
cost) came back green on the first run too, which makes two batches in a
row. The finding was the Dragonback one again in a smaller form. Hostile
Desert's test seeded a graveyard off a library of Forests and asserted that
the menu was "the one land card in this seat's graveyard". On that board
the menu is the whole graveyard, so the assertion held under
`Filter::Any` as well, and the word "land" had no witness. A Llanowar
Elves was put into the same graveyard, and the test now fails when the
card's filter is widened; that was injected and seen red. A menu
assertion only tests a filter if the board also holds something the
filter must refuse.

The third batch that day (Grove of the Guardian, The Gold Saucer and
Westvale Abbey, three tests) was the first to go red. Grove's test asserted
that the first creature was tapped as soon as it was named. It is not,
because CR 601.2h pays the whole cost at once: every question is answered
first and nothing moves until the last one. The engine was right and the
test had the order of the rule wrong. The same test explained the second
menu's shorter length as "a tapped creature is no longer untapped", when
the reason is that each answer is taken off the menus after it. Both
explanations are plausible, which is why the lane wrote them, and a model
that cannot run the test has no way to tell which one is true. The Gold
Saucer test had the Dragonback gap again: "sacrifice two artifacts" was
played with no artifact across the table, so "your" had no witness. One
was added.

The lane run of 24.09.2026 for step 7 (three tests: Bretagard Stronghold,
Abstergo Entertainment, Yawgmoth) wasted a third of its work. Yawgmoth
already had a test in `card_tests/creatures.rs`, and the lane wrote a
second one for the same printed sentences, which was not adopted. The
lane is not told which cards have a test, and it cannot find out without
reading 17 000 lines of card tests. So before a card goes to the lane, grep
`crates/baylee-engine/src/engine/*_tests.rs` (recursively) for its
`oracle_id`. A card that already has a test gets its existing test extended
by hand, or nothing at all.
