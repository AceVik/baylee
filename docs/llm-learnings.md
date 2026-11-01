# LLM Learnings — baylee

Running log of what works when delegating card implementations. It began
with local models (MacBook M1 Max 64 GB, at most one active at a time) and
the two lanes in use now are remote and cheap, which changes the constraint
from "one at a time" to "what is each one actually good at". Maintained by
the orchestrator; entries dated, newest first.

## 2026-11-01 — Review of the requested cards (PR #410, Kimi)

One reviewer and four read-only sub-reviews read e6844a069 against its oracle
text and the brief. What the author got wrong, so the brief can learn it:

1. **A full codegen run is a pool-wide change, and it rewrote review holds.**
   The run dropped the 287 `// Held back:` findings the Argon review had
   written and finished ten held cards nobody judged (all ten turned out
   fixed by now, but by luck), and finished 67 other stubs from main's
   readers, four of them wrong: Hazy Homunculus lost its condition, Wall of
   Vipers lost "destroy this creature", Opalescence made enchantments
   artifacts, Mercenary Informer lost "nontoken". Codegen now keeps a held
   stub held (`stubgen::render_full_run`). After a full run, read every card
   whose coverage changed, not only the ones the batch meant to touch; a
   generated card is played by every game the next day.
2. **"Until this creature leaves the battlefield" is `exile_until`.** Angel
   of Sanctions and Angel of Condemnation were written as `exile_linked`
   plus a leaves trigger, the shape docs/card-dsl.md names as the one that
   never gives its prisoner back. The tests killed the Angel after the exile,
   never before it resolved, so the gap (CR 610.3a) never showed.
3. **An "if" after the trigger event is an intervening "if"** (CR 603.4),
   the trigger's `condition`, not a check inside the effect. Resolute
   Archangel and Gisela checked only on resolution; Knight of the White
   Orchid did it right. The tests had the condition true every time.
4. **"You may" on a targeted trigger is still a `MayDo`.** Angel of
   Serenity's "up to three" let the controller choose none, but not decline
   at resolution.
5. **A new enum variant reaches every crate that matches it.** The PM's
   `cargo check` on the touched crates passed; `baylee-train` did not
   compile (`Pending::ChooseCardType`, `CastModeKind::Foretell`), and its v4
   encoder offered land names to "choose a nonland card name". Map a new
   variant onto existing trained slots rather than widen them, and build
   the workspace before calling a batch done. `cargo check` is not the
   lint either: the gate's `clippy -D warnings` failed on eight more
   places in the batch's client, xtask and test code (functions past 100
   lines, identical match arms, a hand-written `Option::as_slice`).
6. **A rider kept "while in exile" must be dropped on every other move.**
   `Rider::Foretold` survived stack → exile, so a foretold spell countered
   into exile could be cast for its foretell cost again (CR 400.7).
7. **Look CR numbers up.** Starting life is CR 103.4, not 103.3; CR 205.2a
   lists fifteen card types, and Attraction is an artifact type (205.3g),
   not one to offer. The `cr-check` report against the current rules finds
   these in seconds.
8. **The boards still let the wrong reading pass** (as on 31.10): libraries
   of nothing but Plains under a "basic Plains" search (The Birth of
   Meletis, Knight of the White Orchid, Loyal Warhound), a condition that is
   always true, a control spell that costs a colour the board cannot make
   (Herald of War), `>= 1` where exactly one is meant. Twenty-one new
   pieces had no docs/card-dsl.md entry and most engine pieces only a card
   test, against §3.
9. **Still open: three pieces are one card's sentence.**
   `Effect::ExileAttackingForThenMaySearchBasicLands` (Settle the
   Wreckage), `Effect::ExileMostVotedNotControlledByYou` (Council's
   Judgment) and `Effect::SacrificeEventDealDamageToEachBlocker` (Ib
   Halfheart) play right but are named for, and usable by, one card each
   (§3). The general sentences are "exile all [filter] the chosen player
   controls, then that player may search for that many" (`Amount::ThisWay`
   and the basic-land search exist), a vote verb with its outcome as data,
   and "sacrifice the event object; if you do, [effects]". Left for a
   follow-up; the review fixed behaviour, not names.

## 2026-11-01 — Owner-requested cards and cross-surface casting modes

1. **Treat a new casting mode as one vertical rules slice, not only an engine flag.**
   Foretell needs consistent offer, pricing, payment, reversal, public prompt and desktop selection behavior. The engine, Seat narrator and desktop client must agree on the same `CastModeKind`, while replay and per-turn cleanup retain deterministic state.
2. **Keep alternative costs composable and reversible.**
   Pitch spells can require more than one hand card and can combine different payment parts. Offered legality and committed payment must use the same cost calculation, with atomic rollback when a later choice or payment cannot complete.
3. **Represent choice-derived rules as data, not card-specific conditionals.**
   Chosen card types and colors feed shared protection, cast restrictions and prompt code. This keeps Serra's Emissary, Iona and Chrome Mox on the same choice lifetime and prevents stale projections after zone changes.
4. **Play both acceptance and refusal paths for optional clauses.**
   Bruna's cast trigger, Goblin Matron's search and similar `may` abilities need yes/no scenarios plus illegal-target controls. A trigger that merely appears on the stack does not demonstrate that the printed option resolves correctly.

## 2026-10-31 — *Mirrodin* (`mrd`) Batch 2 review

Two reviewers read the batch against its oracle text. One card played wrong;
most findings were tests that could not tell the right reading from a wrong
one, and every one of them passed every gate.

1. **"Put a counter on it or remove one from it" is one ability.**
   Jinxed Choker came out as two `{3}` abilities because the reference script
   prints it as two `A:` lines, and the reader is faithful line by line. The
   2004-12-01 ruling chooses add or remove on resolution, so the card is one
   activation whose effect is `Effect::ChooseOne`. When the oracle text says
   "or" inside one cost, count the printed abilities against the generated
   ones; a mismatch is a wrong play even when each half is right.
2. **A test must have a board on which the wrong reading answers differently.**
   The batch's hollow tests share one shape: the board made the right and the
   wrong reading coincide.
   - A library of one name, so every reveal "matched" (Grim Reminder).
   - The named card always in hand, so "guessed wrong" equalled "answered
     no" (Liar's Pendulum).
   - "You" and "all" coinciding in a count: only the caster had artifacts
     (Override), only we had Swamps (Nightmare Lash), only we had artifacts
     (Myr Enforcer).
   - No opponent-side case for "you control", "your graveyard", "an
     opponent draws", "that permanent's controller" (Lightning Coils, Moriok
     Scavenger, Groffskithur, Mind's Eye, Mesmeric Orb, Incite War).
   - A negative asserted where it could not have been otherwise: March of
     the Machines "refused" an equip that p0 had no mana to pay for, and the
     ruling says the equip may be activated and does nothing.
   - A payment never shown paid (Mind's Eye's {1}: no land asserted tapped),
     a "you may" never declined (Mind's Eye, Mask of Memory), a "this turn"
     never carried into the next turn (Pearl Shard), a timing condition never
     probed from the opponent's turn (Grim Reminder, Nim Devourer).
   - A trigger's "you" read at the original owner rather than the current
     controller, never asserted after a control change (Jinxed Choker).
   - A win prevention tested only through the loss half (Platinum Angel: no
     effect that makes the opponent win was ever put on the board).
   Write the wrong reading down first, then build the board on which it
   gives a different number. And a negative needs its control to pass for
   the right reason: an ability is offered only while its mana is floating,
   so "not offered in the opponent's upkeep" held for want of mana, and the
   control in the own upkeep failed for the same want (Grim Reminder, Nim
   Devourer).
3. **A reader checks a value before it takes it.**
   `p.take("IsPresent") == Some("Artifact.DefenderCtrl")` removes the
   parameter whatever its value, so a clause of another value reaches the
   condition reader as no clause at all: "can't be blocked as long as
   defending player controls an untapped land" would have been read as
   unblockable. The chain reader did the same with three `take`s in one
   `&&`. Peek, compare, then take.
4. **A turn control is about the turn a player actually takes, not a turn
   number.** Mindslaver's control was keyed on `turn.number` and the active
   seat, so it was dropped when two turns passed before its player's (four
   seats, or the controller's two extra turns) and lasted into the player's
   own extra turn after it. CR 722.1 names the turn the player takes and ends
   at the next turn's beginning: stamp the control as that turn begins.
5. **Every resume path advances the program as the plain path does.** The
   damage contest's plain path returns to `run`, which steps past it; the
   path that resumed from Krark's Thumb's question called `run` without the
   step, so the contest ran again after every answer and never ended. The
   coin-life contest dropped its own state while the Thumb asked and started
   a fresh one. A test with the replacement on the board is the only one that
   walks that path.
6. **A search for a stated quality may find nothing** (CR 701.23b). Grim
   Reminder's reveal search set `min: 1` from "not optional"; the library
   search already read the filter as the rule does, and a sibling reader
   asked fewer questions.
7. **A loosened check needs its callers re-read, not its test rewritten.**
   The replay check took the public-table digest on every build, and a test
   that asserted a wrong hash diverged was rewritten to assert it passed.
   The protocol keeps the exact hash on the build that played the game and
   the table for a later build; the build is in the header.

## 2026-10-30 — *Mirrodin* (`mrd`) Batch 2 (`Granite Shard` through `Platinum Angel`, 50 cards)

Completing all 50 cards through `scriptgen` required small shared mechanisms rather than card-specific engine effects. The batch also exposed four integration rules worth preserving:

1. **Update the complete `Modifier` census with every new variant.**
   `ProtectionFromExiledWithSourceTypes`, `NamedPlayersCantLose`, and `PlayersCantWin` are rule-based modifiers, not locking modifiers. Adding them only to the DSL and projection is insufficient: the exhaustive declaration and census tests in `baylee-engine/src/effects.rs` must also account for them (now 181 declared variants, 54 locking and 127 rule-based).
2. **Keep object targets, optional empty object choices, and player-only targets distinct.**
   Counter resolution and subject binding must not infer an empty object selection from a player target. Karn's empty `+1`, Yawgmoth's optional "up to one" choice, and Jinxed Choker's player-targeted self-counter activation now share the same resolver without stealing one another's answers.
3. **Turn control ends immediately when either player leaves.**
   Mindslaver's scheduled `turn_controls` are removed by state-based actions as soon as their controller or controlled player has left; cleanup is not deferred to the next turn boundary. The departure scan includes `ControlPlayerNextTurn`, and the state fingerprint covers the scheduled control.
4. **A corpus-free checkout can finish coverage only after accepted played tests exist.**
   Never run full `codegen` without the external scripts. For already complete machine-owned outputs whose canonical tests are present and green, advance only the derived `Coverage::Generated` field to `Implemented`, leave the generated abilities untouched, then run `codegen --tables` and `codegen --tables --check`. The `require-card-tests` hook is the final independent check that no tested card remains at `Generated`.
5. **Prefer composable rules primitives for the remaining Mirrodin mechanics.**
   Krark's Thumb uses coin-flip replacement with an explicit ignored flip; Isochron Scepter reuses linked exile plus card copies and optional free casting; Mirror Golem derives protection from the linked card's types; Platinum Angel composes separate can't-lose and opponents-can't-win modifiers. Dynamic per-artifact mana costs and Myr Prototype's counter-scaled combat payment stay in shared pricing and declaration code.

## 2026-10-26 — *Mercadian Masques* (`mmq`) Batch 5 (`Invigorate` through `Rishadan Footpad`, 25 cards)

Adding the fifth 25 previously absent *Mercadian Masques* first-printing cards (`12 Implemented`, `3 Partial`, `10 Unimplemented`) extended the DSL, engine, and `scriptgen` across five patterns (`Inviolability`, `Kyren Negotiations`, `Natural Affinity`, `Noble Purpose`, `Pious Warrior`, `Quagmire Lamprey`, `Rappelling Scouts`, `Revered Elder`, `Rishadan Brigand`, `Rishadan Cutpurse`, `Rishadan Footpad`):

1. **`Trigger::DealtCombatDamage(&'static Filter)` and `T:Mode$ DamageDealtOnce` / `TriggerCount$DamageAmount`.**
   Added `Trigger::DealtCombatDamage(&'static Filter)` to `baylee-cards-dsl` and `baylee-engine` (summing batch combat damage per target in `event_damage_of`), and taught `triggers.rs` and `mana.rs` to lower `T:Mode$ DamageDealtOnce` (`ValidSource$` -> `DealsCombatDamage`, `ValidTarget$` -> `DealtCombatDamage`) and `TriggerCount$DamageAmount` (`Amount::EventAmount`) for `Noble Purpose` and `Pious Warrior`.
2. **`R:Event$ DamageDone | Prevent$ True | ValidTarget$ Creature.EnchantedBy` (`Inviolability`).**
   `keywords.rs` and `wall52.rs` lower `ValidTarget$ Creature.EnchantedBy` prevention replacements to `static_ability!(Filter::EnchantedBySource, Modifier::PreventDamageFrom(&Filter::Any))`.
3. **Untargeted `AB$ PreventDamage | Defined$ Self` and `Effect::ProtectionFromChosenColor` (`Revered Elder`, `Rappelling Scouts`).**
   `chains.rs` maps `Defined$ Self` on a permanent's own `PreventDamage` ability to `TargetSpec::ThisObject` (`Revered Elder`), and `Effect::ProtectionFromChosenColor` falls back to `subjects::source(state, res)` when `!res.targeted` (`Rappelling Scouts`).
4. **`RepeatEach | RepeatPlayers$ Player.Opponent` with `Sacrifice` (`UnlessCost$`) and multi-player `Effect::PlayerMayPayOr` (`Rishadan Cutpurse`, `Rishadan Footpad`, `Rishadan Brigand`).**
   `effects.rs` lowers `RepeatEach | RepeatPlayers$ Player.Opponent` wrapping `Sacrifice | Defined$ Remembered | UnlessCost$ <n> | UnlessPayer$ Remembered` to `Effect::PlayerMayPayOr { player: PlayerRel::EachOpponent, mana: Amount::Fixed(n), effect: &Effect::SacrificeChosen { who: PlayerRel::EventPlayer, .. } }`, and `choices.rs` iterates all players returned by `players_of(player, ..)` across `res.subject.pay_players_remaining`.
5. **`SP$ AnimateAll | ValidCards$ Land` (`Natural Affinity`).**
   `effects.rs` lowers `AnimateAll` into `Effect::CreateContinuousEffect` ops for `Modifier::AddType(TypeSet::CREATURE)` and `Modifier::SetPT(power, toughness)`.

## 2026-10-25 — *Mercadian Masques* (`mmq`) Batch 4 (`Insubordination` through `Pangosaur`, 25 cards)

Adding the fourth 25 previously absent *Mercadian Masques* first-printing cards (`14 Implemented`, `3 Partial`, `8 Unimplemented`) extended `scriptgen` across six patterns (`Jeweled Torque`, `Karn's Touch`, `Kyren Glider`, `Liability`, `Magistrate's Veto`, `Muzzle`, `Pangosaur`) and authored 4 full `Coverage::Implemented` hand-owned cards (`Insubordination`, `Lava Runner`, `Mercadian Atlas`, `Nether Spirit`):

1. **Mana-only `Cost$` on triggered `AB$` abilities in `optional_price`.**
   `activated.rs` recognizes `Cost$` on triggered `AB$` blocks when it parses as a positive generic mana integer even without `OptionalDecider$ You` (`Jeweled Torque` and the remaining four MMQ Torques).
2. **`S:Mode$ CantBlock` in `statics.rs`.**
   `statics.rs` maps `S:Mode$ CantBlock` to `Modifier::AddKeyword(KeywordSet::CANT_BLOCK)` (`Kyren Glider`, `Magistrate's Veto`).
3. **`R:Event$ DamageDone | Prevent$ True | ValidSource$ Creature.EnchantedBy` in `keywords.rs` and `wall52.rs`.**
   Mapped to `Modifier::PreventAllDamageFromIt` on `Filter::EnchantedBySource` (`Muzzle`).
4. **`T:Mode$ LandPlayed` in `triggers.rs`.**
   Mapped to `Trigger::PlaysLand` / `Trigger::PlaysAnotherLand` (`Pangosaur`).
5. **`Defined$ TriggeredCardOwner` on `ChangesZone` triggers in `targets.rs`.**
   Mapped to `PlayerRel::OwnerOfEvent` (`Liability`).
6. **`AB$/SP$ Animate` with `Power$ X | Toughness$ X` and `SVar:X:Targeted$CardManaCost` in `effects.rs`.**
   Mapped to `Effect::SetPTFilter { filter: &Filter::This, power: Amount::TargetCmc, toughness: Amount::TargetCmc, duration }` alongside `Effect::AddTypesToTarget` (`Karn's Touch`).

## 2026-10-25 — *Mercadian Masques* (`mmq`) Batch 3 (`Enslaved Horror` through `Instigator`, 25 cards)

Adding the third 25 previously absent *Mercadian Masques* first-printing cards (`10 Implemented`, `4 Partial`, `11 Unimplemented`) extended `scriptgen` across five patterns (`Extravagant Spirit`, `Flailing Manticore`, `Flailing Ogre`, `Flailing Soldier`, `Food Chain`, `Furious Assault`, `Haunted Crossroads`, `High Seas`):

1. **`Count$ValidHand Card.YouOwn` and `unless_price` with `SVar:X`–`Z`.**
   `atoms.rs` maps `Count$ValidHand Card.YouOwn` to `Amount::HandSize(PlayerRel::You)`, and `chains.rs` accepts `"X" | "Y" | "Z"` in `unless_price` (`Extravagant Spirit`).
2. **`Activator$ Player` on non-mana activated abilities (`ActivationZone::BattlefieldAnyPlayer`).**
   `activated.rs` appends `, zone = ActivationZone::BattlefieldAnyPlayer` when `Activator$ Player` appears on an activated ability, enabling the *Flailing* cycle (`Flailing Manticore`, `Flailing Ogre`, `Flailing Soldier`).
3. **`Exile<1/...>` in `mana_cost_parts` and `Exiled$CardManaCost/Plus.1` in `Produced$`.**
   `mana.rs` and `support.rs` map `"Exile"` cost atoms to `ExilePermanent(...)`, and `atoms.rs` maps `Exiled$CardManaCost/Plus.<n>` to `Amount::Plus { base: &Amount::SacrificedManaValue, offset: <n> }` because `CostPart::ExilePermanent` populates `paid.sacrificed_lki` (`Food Chain`).
4. **Targeted `ChangeZone | Origin$ Graveyard | Destination$ Library` and `S:Mode$ RaiseCost | Type$ Spell`.**
   `effects.rs` maps targeted `Origin$ Graveyard | Destination$ Library` (with `LibraryPosition$ 0` or default top) to `Effect::GraveyardToTop { target }` (`Haunted Crossroads`), and `final_portals.rs` maps `S:Mode$ RaiseCost | Type$ Spell | Amount$ <n> | ValidCard$ <filter>` to `Modifier::SpellsCostMore(n)` (`High Seas`).
5. **`Filter::CmcAtMostX` / `Filter::CmcExactlyX` in `Effect::DestroyAll` require `SP$` mode.**
   `Effect::DestroyAll` evaluates its filter against `this = res.source`, which is the announced `x_value` on a spell on the stack (`Meltdown`, `Gaze of Granite`, `Forced March`) and `0` on a battlefield permanent activating `AB$ DestroyAll` (`Hammer Mage`). `chains.rs` now refuses `DestroyAll` when `self.mode != "SP"` and the filter contains `Filter::CmcAtMostX` or `Filter::CmcExactlyX`.

## 2026-10-25 — *Mercadian Masques* (`mmq`) Batch 2 (`Cho-Arrim Legate` through `Distorting Lens`, 25 cards)

Adding the second 25 previously absent *Mercadian Masques* first-printing cards (`10 Implemented`, `5 Partial`, `10 Unimplemented`) extended `scriptgen` across five patterns (`Cho-Manno's Blessing`, `Cho-Manno, Revolutionary`, `Cinder Elemental`, `Collective Unconscious`, `Corrupt Official`, `Darting Merfolk`, `Deepwood Tantiv`, `Deepwood Wolverine`, `Distorting Lens`):

1. **`ChosenColor` in protection filters and `ChooseColor` → `DB$ Animate` chains.**
   `targets.rs` maps `"ChosenColor"` in `filter_expr_in_zone` to `Filter::OfSourceChosenColor` (`Cho-Manno's Blessing`), and `choose_color_animate` lowers `ChooseColor | Defined$ You | SubAbility$ DBAnimate` with `Colors$ ChosenColor | OverwriteColors$ True` to `Effect::ModifyCharacteristics { target, effect: ModifyEffect::SetColorsToChosen, duration }` (`Distorting Lens`).
2. **`Prevent all damage that would be dealt to CARDNAME.`**
   `constants.rs` maps `"Prevent all damage that would be dealt to CARDNAME."` in `keyword_static` to `Modifier::PreventDamageFrom(&Filter::Any)` (`Cho-Manno, Revolutionary`).
3. **`NumCards$` expressions in `Draw` and self-bounce defaults in `ChangeZone`.**
   `chains.rs` uses `amount_or_count` for `Draw`'s `NumCards$` (`Collective Unconscious`: `SVar:X:Count$Valid Creature.YouCtrl`), and `effects.rs` defaults untargeted `Origin$ Battlefield | Destination$ Hand` without `ChangeType$` on a permanent's own ability to `Defined$ Self` (`Darting Merfolk`).
4. **Sacrifice-based alternative casting costs remain `Coverage::Partial`.**
   `cast_wizard::paid_as_an_alternative_cost` currently only pays `CostPart::PayLife(_)` and `CostPart::ExileFromHand(_)`; cards whose alternative cost sacrifices permanents (`Crash`, `Delraich`, like `Fireblast`) must stay `Coverage::Partial` without `alternative_costs` until `cast_wizard` wires a sacrifice stage for `CastModeKind::Alternative`.

## 2026-10-25 — *Mercadian Masques* (`mmq`) Batch 1 (`Aerial Caravan` through `Cho-Arrim Bruiser`, 25 cards)

Adding the first 25 previously absent *Mercadian Masques* first-printing cards (`14 Implemented`, `2 Partial`, `9 Unimplemented`) extended `scriptgen` to cover five recurring patterns (`Alley Grifters`, `Ancestral Mask`, `Arrest`, `Ballista Squad`, `Briar Patch`, `Cackling Witch`, `Caustic Wasps`, `Chambered Nautilus`):

1. **`T:Mode$ AttackerBlocked` on `Card.Self` and `TriggeredDefendingPlayer`.**
   `triggers.rs` now maps `T:Mode$ AttackerBlocked` with `ValidCard$ Card.Self` to `Trigger::BecomesBlocked` (or `Trigger::BecomesBlockedBy(&{filter})` when `ValidBlocker$` is present), and `targets.rs` maps `Defined$ TriggeredDefendingPlayer` under `"Attacks" | "AttackerBlocked"` to `PlayerRel::DefendingPlayer` (`Alley Grifters`, `Chambered Nautilus`).
2. **`T:Mode$ Attacks | Attacked$ You` with `Defined$ TriggeredAttackerLKICopy`.**
   When `T:Mode$ Attacks` specifies `Attacked$ You` on a non-`Card.Self` filter, `triggers.rs` appends `.attackingYou` (`Filter::AttackingYou` in `targets.rs`), and `pump_effect` maps `Defined$ TriggeredAttackerLKICopy` / `TriggeredAttacker` under `Attacks` to `Filter::Reference` (`Briar Patch`).
3. **`Count$Valid .../Times.<n>` in static P/T modifiers.**
   `statics.rs` now strips an optional `/Times.<n>` suffix (`n` in `1..=4`) on symmetric `AddPower$`/`AddToughness$` `Count$Valid ` expressions and emits `Modifier::ModifyPTByCount` `n` times (`Ancestral Mask`: +2/+2 for each other enchantment).
4. **Composite `S:Mode$ CantAttack,CantBlock,CantBeActivated` and `TargetSpec::ObjectOfEventPlayer`.**
   `statics.rs` now lowers `Mode$ CantAttack,CantBlock,CantBeActivated` on `Creature.AttachedBy` into `Modifier::AddKeyword(KeywordSet::CANT_ATTACK.union(KeywordSet::CANT_BLOCK))` plus `Modifier::CantActivate` (`Arrest`), and `targets.rs` lowers `.ControlledBy TriggeredTarget` under `T:Mode$ DamageDone` to `TargetSpec::ObjectOfEventPlayer` (`Caustic Wasps`).

## 2026-10-19 — *Fallen Empires* (`fem`, 102/102) and retroactive DSL decomposition (*The Dark* / *Legends*)

Completing all 67 remaining cards of *Fallen Empires* (`16 Coverage::Generated` via `scriptgen` + `51 Coverage::Implemented`) alongside retroactively decomposing single-card variants from *The Dark* and *Legends* surfaced five durable rules:

1. **Break single-card variants into orthogonal primitives ("klein bröseln").**
   - `CostPart::SacrificeChosenToAddCounters` was replaced by `CostPart::SacrificeChosenRemembering` + `Effect::PutCountersOnSource` with `CountExpr::RememberedCostCount`, allowing any cost-to-effect count handoff to reuse the same stack/cost-wizard remembrance slot (`chosen_cost_count`).
   - `TriggerEvent::CreatureDealtDamageBySourceDiesThisTurn` became `PermanentFilter::DamagedBySourceThisTurn` on standard `ChangesZone` triggers.
   - `Modifier::PreventAllDamageToAndByAttached` split into `Modifier::PreventAllDamageTo` and `Modifier::PreventAllDamageBy`.
   - `Effect::DanceOfManyToken` became `TokenSpec::CopyOfTarget` with composable `TokenLink` (`ExileWhenSourceLeaves`) and `SacrificeAtUpkeepUnlessCost`.
   - Modular mode selection based on counters (*Homarid*'s tide cycle) uses `CountExpr::SourceCounter` + `Condition::CountModulusEq` rather than custom trigger/effect variants.
2. **Keep `can_cast` and `SpellAdjustment::price` in lockstep for colored spell taxes.**
   `Modifier::SpellsCostMoreMana` (*Derelor*) increases `adjustment.mana_increase`. Both `SpellAdjustment::price` (used during `cast_wizard`) and `can_cast` (`probe`, `probe_normal`, `probe_back`, and `kicked` affordability checks in `crates/baylee-engine/src/casting.rs`) must `.combine(&adjustment.mana_increase)`, otherwise `every_implemented_card_survives_a_game` will offer a cast in `legal.castable` whose wizard subsequently has no payable mana combination.
3. **Pool-wide census and vocabulary tables in `baylee-engine`, `baylee-cards`, `baylee-cards-dsl`, and `baylee-ai`.**
   Adding new DSL variants (`CostPart`, `Effect`, `Modifier`, `CountExpr`, `TokenSpec`, `PermanentFilter`) or promoting hand-owned cards requires updating:
   - `docs/card-dsl.md` (checked against `baylee-cards-dsl` source AST by `docs_sync` tests),
   - `crates/baylee-cards/src/lints/tests/entries.rs` (custom counter carrier ceiling),
   - `crates/baylee-engine/src/effects.rs` (`THE_CENSUS`), `land_mana_tests.rs`, `this_object_tests.rs`, and `vocabulary_tests.rs`,
   - `crates/baylee-ai/src/tests/worth_tests.rs` (implemented pool count `(readers, total)`).

## 2026-10-19 — Generated baseline verification (Packages 1–6: all 4,969 cards promoted)

Verifying the entire frozen `Coverage::Generated` baseline in six mechanical
packages (Package 1: 12 battle-hold / scriptgen cards; Package 2: 788 vanilla,
keyword, protection, combat-restriction, and enter-modifier creatures; Package 3:
627 Auras, Equipment, non-Aura Enchantments, and non-Equipment Artifacts;
Package 4: 574 Instants and 363 Sorceries; Package 5: 1,296 non-triggered
creatures; Package 6: 1,309 triggered creatures) promoted all 4,969 cards to
`Coverage::Implemented` and surfaced eight reusable lessons:

1. **Test-scanner `oracle_id` discipline.** `xtask codegen` scans all files under
   `crates/baylee-engine/src/engine/card_tests/` for quoted `oracle_id` UUID
   strings to decide which `Coverage::Generated` cards are tested and promote
   them to `Coverage::Implemented`. Any fixture helper card referenced by UUID
   in a test suite must already be `Coverage::Implemented`, or else it will
   prematurely promote an unverified `Coverage::Generated` card out of turn.
2. **Graveyard and library `"Permanent"` filters vs. battlefield filters.** On
   the battlefield every object is already a permanent, so `scriptgen` elided
   `base == "Permanent"` to `Filter::Any`. In graveyard targets (`TgtZone$
   Graveyard`) and library search filters (`Origin$ Library`), `"Permanent"`
   means a permanent card (CR 110.4a: artifact, battle, creature, enchantment,
   land, or planeswalker) and must exclude instant and sorcery cards
   (`Nature's Spiral`, `Revive the Shire`, `Obzedat's Aid`, `Wargate`,
   `Planar Bridge`).
3. **Comma-separated graveyard targets and `TargetType$` owner qualifiers.**
   `strip_player_qualifiers` must strip `.YouCtrl` / `.YouOwn` across every
   comma-separated alternative (`Artifact.YouCtrl,Enchantment.YouCtrl` on
   `Argivian Find`, `Artifact.YouCtrl,Creature.YouCtrl` on `Emergency Weld`),
   and when `TgtZone$ Graveyard` omits `.YouCtrl` on `ValidTgts$` (`Instant,Sorcery`
   on `Call to Mind` and `Relearn`), `scriptgen` must check the Oracle sentence
   (`"from your graveyard"`) so `Filter::YOUR_CARD` is still intersected.
4. **Two-pass `decks-status` after promoting cards.** Because `xtask` links
   `baylee-cards` at compile time, the `decks-status` pass inside the first
   `cargo run -p xtask -- codegen` sees the pre-promotion `Coverage` constants.
   Running `cargo run -p xtask -- decks-status` (or a second `xtask codegen`)
   after `baylee-cards` recompiles updates `data/decks/precon/STATUS.tsv` and
   `crates/baylee-db/src/precons/generated.rs` so `xtask codegen --check` is
   clean.
5. **Route fixture tapping through `GameState::set_tapped`.** The architectural
   rule test `nothing_writes_the_tapped_bit_except_the_one_door` scans all
   files under `crates/baylee-engine/src/` for direct `.status.insert(Status::TAPPED)`
   writes; test fixtures that pre-tap permanents must call `st.set_tapped(id, true)`.
6. **Multiple instances of cumulative keywords (`Prowess`, `Exalted`, `Melee`).**
   Under CR 702.108b, multiple instances of prowess on the same object trigger
   separately (`Thor Odinson, God of Thunder`: `"Flying, prowess, prowess"`).
   `scriptgen` must preserve repeated instances of `Keyword::Prowess`,
   `Keyword::Exalted`, and `Keyword::Melee` rather than deduplicating them in
   `keywords.rs`.
7. **Printed `mana_ability!` vs. intrinsic CR 305.6 basic-land mana.**
   `PlayerAction::ActivateManaAbility { source }` is strictly the CR 305.6
   intrinsic basic-land subtype shortcut (`Plains`, `Island`, `Swamp`,
   `Mountain`, `Forest`). Printed `mana_ability!` entries in a card's `abilities`
   slice (such as `Silhana Starfletcher` or mana dorks) are activated via
   `PlayerAction::ActivateAbility { source, ability_index }`.
8. **`sba::destroy` vs. `kill` when testing `Trigger::Dies` and state triggers.**
   Calling `sba::destroy` directly on `GameState` journals the `ObjectMoved`
   event without running `flush_triggers_and_sbas`, and `pass_until(engine,
   stack_is_empty)` checks its predicate *before* applying an action—returning
   immediately while the stack is still empty and triggers are still uncollected
   in the journal. Always use `kill(&mut engine, id)` (or apply one
   `PlayerAction::PassPriority` after `sba::destroy` before `pass_until`) so
   `flush_triggers_and_sbas` places `Trigger::Dies` and `Trigger::State` abilities
   onto the stack.

## 2026-10-18 — Batch 53: 31 cards, modal MDFCs, Class levels, and validator sync

Implementing 28 new cards (`Coverage::Implemented`, plus 3 already implemented)
touched modal MDFCs (`Tony Stark // The Invincible Iron Man`), Adventure
(`Virtue of Persistence // Locthwain Scorn`, `Emeritus of Ideation // Ancestral
Recall`), Class level progression (`Innkeeper's Talent`), Planeswalkers
(`The Theorist, Jace Beleren`, `Nissa, Vital Force`, `Professor Dellian Fel`,
`Sivitri, Dragon Master`), draw replacement (`Hullbreacher`), state-triggered
abilities (`Dark Depths`), and parity-gated Gods (`Sab-Sunen, Luxa Embodied`).

Three cross-cutting lessons stood out:
1. **Validator and sentence-mapper tables must advance in lockstep with DSL
   additions.** Every new `Trigger`, `ContinuousEffect`, `Replacement`, or
   `Effect` variant (`PlayerSearchesLibrary`, `OpponentDrawsExceptFirstInDrawStep`,
   `ClassLevel`, `MayCastFromZone`, `CreateTokenWithCounters`,
   `DoubleCountersOnTargetWithTypes`) needs matching arms in `xtask/src/validate/`
   (`claims.rs`, `effect_trees.rs`, `event_readers.rs`, `leaves.rs`,
   `trigger_Matrix.rs`) and `xtask/src/codegen/ability_map.rs`, followed by two
   `xtask codegen` passes so `ability_lines.rs` reflects the updated line
   mapping.
2. **Scoped leaves-the-battlefield cleanup vs. standalone exile effects.**
   `leaves.rs` rejects `Effect::ExileTarget` on an enters trigger unless the
   card is a finite-duration O-ring style lock. Permanent exile or graveyard
   effects that also grant a cast permission (`Tinybones, the Pickpocket`,
   `Web of Inertia`) must either stay in their actual trigger (`CombatDamageToPlayer`,
   `CombatBegin`) or pair with `UntilSourceLeaves` / `UntilEndOfTurn` duration
   when granting `MayPlayExiled` / `MayCastFromZone`.
3. **Splitting large test modules early keeps clippy pedantic happy.** Grouping
   the 27 behavioral tests for Batch 53 into a directory module
   (`card_tests/batch53_cards/` with six focused files) avoided `too_many_lines`
   and kept each category's helper setup self-contained.

## 2026-10-14 — Corpus expansion: establish equivalence before adding cards

Changing the reference checkout exposed syntax drift rather than card-rule
changes. Normalize the split object-definition/validity spelling to the existing
filter order, test both spellings, and compare the complete pool dump before
expanding. This recovered 75 existing generated definitions without rewriting
any of them. Refresh the script index from the same checkout; stale fork-only
paths must not reach a batch.

The first 500-card stage produced 500 Generated cards and no new stubs. Its
drift guard initially mistook the generated `cards/mod.rs` registry for a card.
Exempt only modifications of that exact registry, with tests that still reject
card changes, nested module changes, deletions and renames. A clean codegen
check and byte-identical old card records remain separate checks; neither is a
substitute for the other. Generated remains distinct from tested coverage.
When a new card trips a named AI gameplay obligation, do not silently extend
its allowlist. The owner authorized GPT-6.1 to add isolated missing tests:
Time Bomb now has real upkeep, sacrificed-source counter/damage and AI-decision
checks. Removing either ability through the mutation hook makes the tests
fail. Its rules work, but the AI still cannot value the sweep; record that
limitation rather than promoting coverage or claiming improved play.

## 2026-10-05 — Legends: 189 stubs, three waves, and where the pace breaks

Legends is the first set where the pipeline's cost is visible: 253 names
added, 64 transcoded, 189 stubs — three waves of eight parallel agents, ~24
task packages. It worked, in the sense that all 253 cards are finished
(167 Implemented / 141 Partial / 2 excluded), but the per-card reading does
not scale to "every set until 2021" at this cadence: Legends alone took as
much orchestration as ARN and ATQ combined, and its 167 Implemented cards
still owe the rules-derived test pass (another three waves). The lever is not
more agents per wave — eight already saturate a person's ability to read the
reports — it is making the *reader* refuse fewer scripts (each new scriptgen
rule retires many stubs at once; `xtask transcode-report` and
`xtask reach-list` rank which rule buys the most cards) and letting the
Partial route absorb the rest without hand work.

Operational notes from this run: batches of eight by `scratchpad/leg-worklist.tsv`
kept prompts tiny and reports comparable; asking for one line per card plus at
most two lines of uncertainty kept the 24 reports readable. Three checker
first-contacts showed up: ``Coverage::Partial`` strings are scanned as code by
the counter-id lint (a reason that spells `Custom(` trips it — reword the
string, do not whitelist the file), the event-reader census needed the
`IfEventObjectMatches` spelling, and the rampage family is still entirely
unimplemented, so ten cards stop at the same missing keyword. The pattern to
expect from here: old sets are keyword-heavy (rampage, bands-with-other,
landwalk variants) while the reader is effect-heavy, so Partial rates will
run high until those keyword families land.

## 2026-10-05 — Antiquities, the same pipeline one set on

The second set run reproduced the first cleanly: 62 names added at once
(`scryfall-cache`, `codegen` twice, `validate`), 14 came out transcoded, and
48 stubs went to seven parallel `general` agents by file with the same
"no engine lane, Partial + `// NOT SUPPORTED`" contract. 17 hand cards landed
Implemented and 31 Partial before testing; the tests then demoted one of
them, which is the interesting part.

The rules-derived test pass is what catches a fully plausible wrong card.
**The Rack** reads "X is 3 minus the number of cards in their hand", the
corpus script spells it `NMinus.3`, and the hand implementer wrote
`SaturatingSub { base: CountOf(hand), subtract: 3 }` — Black Vise's shape,
the reverse. Sixteen card agents had `cargo check`, clippy, `validate` and the
pool sweeps all green, and none of them could have noticed: nothing checks
the *arithmetic direction* of an amount. The test agent, told to assert the
oracle text and report disagreements, produced the failing test that named
it. The lesson is not "read the script more carefully" but that the
rules-derived tests are the only rung that reads the sentence instead of the
structure, so run them before calling a set done — and when they fail,
demote the card to Partial rather than shipping the wrong direction.

Two smaller notes. `validate`'s `OFFERS_A_CHOICE` list is a string list that
predated `PlayerMayPayCostOr`, so a correct `you may sacrifice` trigger read
as "never asks"; one word fixed it, but a checker keyed on variant names will
keep drifting behind the DSL and a `matches!`-style reader would not.
And The Rack's helper test had to be deleted with the demotion: a red test
kept "as evidence" blocks every later run, so a demoted card's test comes off
with the ability and the gap goes to TODO.md instead.

## 2026-10-05 — Rules-derived tests, one card at a time, mutants as the bar

The follow-up to the Arabian Nights pass: 35 tests for the 21 `Implemented`
cards. The owner's rule was "not tests by code but by Scryfall rules": the
agents fetched the card payload and rulings from the Scryfall API into
`scratchpad/arn-tests/`, asserted the **oracle text** (journal, zones, P/T,
keywords), and read the implementation only to know which fixtures drive the
engine. Three agents in parallel by *file* (creatures, artifacts +
enchantments, instants + sorceries) — never two writers in one file — and a
fourth wave for the second half of `creatures.rs`. Each card also had to pass
the mutation bar: `BAYLEE_MUTATE=<card>:<ability index>` run against the
card's own tests, exit 101 required for every listed ability (a surviving
mutant meant a weak test, not a pass). Agents wrote local helpers next to
their tests, including the `card_index("<oracle id>")` literal, which is also
what the ladder reads; one agent initially used the Scryfall *printing* id and
caught itself after `card_index` returned nothing — give both ids in the task
or say explicitly which one the helper takes.

What to keep: the rules-first wording ("if code and rules disagree, the test
asserts the rules and you report the mismatch") produced zero false
green tests and no agent touched card or engine code; the per-ability mutant
list came free from the firing inventory, so a card with four abilities
(Jasconius) had four mutants in its prompt. The side benefit is a proper L4
leave check: `BAYLEE_LEAVE_LOG=<dir> cargo test -p baylee-engine --lib --
--ignored leave_probe_sweep` runs in ~21s and showed all 17 permanent cards
leaving clean. The formal L4/L5 stamps still want the llvm-cov mechanics
export; without it the ladder reads "L4 needs --coverage" while every other
half is green.

## 2026-10-05 — Arabian Nights in seven parallel card agents, no engine lane

The owner ordered the next set (Arabian Nights) implemented fully with no
engine changes: every clause the current vocabulary cannot express becomes
`Coverage::Partial` plus a `// NOT SUPPORTED` block naming the missing piece,
and the gap goes to `TODO.md`. That turns the usual "stop and flag the card"
into a complete written answer for every card and is what made parallel
delegation safe: nobody had to wait on an engine decision.

What worked: 52 names added at once (`scryfall-cache` bulk, `codegen` twice,
`validate`), then the 10 reader-writable cards came out finished for free.
The 43 remaining stubs were split into seven packages of six or seven cards,
one `general` agent each, all running in parallel. Each agent adopted its
stubs first (`xtask codegen --adopt-stub <name>`) so a later codegen could
not clobber a hand edit, read the corpus script plus `docs/card-dsl.md`, and
verified only `cargo check -p baylee-cards`; the central pass then ran
`validate`, `codegen --check`, the engine suite and clippy once. Compile
errors from a neighbour agent's in-flight file did not occur because each
file is written in one shot, but the prompts told every agent to ignore
errors outside its own list anyway.

What the split bought: every agent independently verified that the DSL
variant it reached for exists (for example `Trigger::Dies`, `Effect::destroy_all_no_regen`)
rather than inventing one, and the Partial reasons came back with the exact
missing vocabulary (`DelayedWhen::DiesOrIsExiled` without a writer, no coin
flip, no power-only base setter). Eleven of 43 stubs became Implemented this
way; 32 are Partial with a written reason, which is the deliverable the
owner asked for.

What the batch taught the tools: exactly the §E8 prediction held — the only
non-card change was `claim_tests`, which read the auto-transcribed Sandstorm
("deals 1 damage to each attacking creature") as an unconditional promise on
a probe board with no attackers. One word (`" attacking "` in `CONDITIONS`)
made the condition word real, as the module's own doc demands: a word too
many costs a claim, a word too few costs a card falsely reported. The
seven-agent design also leaves per-card behaviour tests unpaid (the owner
forbade engine-crate changes), so these cards sit below L3 until a later
lane writes them.

## 2026-10-04 — Native Engine lane and stable integration checkpoints

The owner means a native Junie session when specifying Opus 5.5/high, not an
external Claude CLI or a capability tier that merely sounds equivalent. The
native `engine` session confirms the selected model through JetBrains AI.
Coordinate existing sessions through shared project handoff/status/inbox files,
with an immediate acknowledgement, exact interface handoffs and explicit Cargo
and source pauses. Never start another writer against an open owner session.

The first two checkpoints review the inherited Time Vault/Aura/constrained-cast
work and finish Animate Dead and Lich. New turn-trade decisions need consumers
outside the Engine too: explicit human cost wording, question-bound narrator
answers, a no-automation rule and a conservative AI policy distinct from a free
optional effect. Two normal table codegen passes and behavioral tests are a
gate; changing coverage is a separate acceptance decision.

Live Sleight exposed a browser hiding a legal battlefield target behind a pinned
off-board target sheet. Reproduce it before fixing it. Reuse the existing mixed
card-choice path for target choices rather than unlocking private/mandatory
sheets. Require exactly the offered rows, no duplicate rows, exclusion of an
unoffered permanent and reset back to a stack-only offer; all 54 Browser tests
then pass. A second battlefield representation is intentional only where the
sheet obscures that answer, not an excuse for arbitrary duplicate objects.

Do not diagnose a combined timeout as a deadlock without measured evidence.
Checkpoint 2 spent 130 seconds compiling before its tests; all Gamehost unit
tests pass when isolated, with 70.20 seconds test execution. Preserve the two
existing ignored long-run tests and test coverage; do not sample crash recovery
or remove durable syncs to squeeze the full Workspace gate into a short timeout.

When a modal offers the same object as the table, the table's coordinates are
not the modal row's coordinates. Expose the real row/control computed bounds
and text-entry state through the feature-gated loopback diagnostic; test its
actual pump/JSON response and preserve release-build exclusion. Repeatedly
guessing keys while a search field has focus is not a live acceptance proof.

## 2026-10-04 — Resume semantic migrations across every reader

An interrupted intrinsic-mana migration compiled in the Engine but not in the
client. Follow the provenance through the DSL, handwritten dual lands, codegen,
host projections, client planning and verification recorder. A fixed printed
mana symbol and mana granted by a current basic land type are different inputs;
tests must preserve that distinction rather than infer it from matching colours.
The independent land-mana test reader derives intrinsic expectations from the
face's basic types, not from the production mana evaluator. Existing dual tests
still exercise each colour, exactly one mana and the paid tap cost.

Library-moving mana abilities also need the sentence mapper to agree with the
Engine's current classification. Chromatic Sphere and the five Eggs exposed a
mapping gap after Draw/Mill became non-mana: pure mana and mana plus damage are
counterexamples, not exceptions to weaken the all-card mapping check. Regenerate
the line table through `xtask codegen`, never edit its generated rows.

The owner reserves subsequent Engine work for Astra at medium or higher, or
Claude Opus 5.5 at high or higher. Local Qwen is limited to the non-Engine lane.
For a bounded UI checklist, LM Studio's native `/api/v1/chat` with
`reasoning: "off"` returned useful output; the OpenAI-compatible request spent
its entire token budget on reasoning and returned an empty answer. A generated
checklist is a proposal, not evidence that the UI was tested.

## 2026-10-02 — Sengir Vampire: damage history is not marked damage

A death after damage reads a historical relationship, not the victim's marked
damage or a delayed trigger installed at the hit. It must observe abilities
at death time, preserve facts through cleanup, distinguish both objects'
incarnations, and read a simultaneous departure before either participant
moves. Real combat, fights and later removal spells make useful independent
card tests; synthetic fixtures reach type changes, phasing and granted abilities.

Astra xhigh owns engine work; Sol 6.1 medium independently checks the rules and
identity paths. Early review found missing source binding on granted triggers,
phase-out after triggering, and history absent from loop equivalence. Reviewing
an event's damage dealer also exposed a pre-existing incarnation/LKI gap when
an entering creature leaves before its damage trigger resolves. An ordinary
card-only happy path would not have exposed those shared-engine defects.

Adding a Trigger variant also requires the codegen sentence mapper to recognize
its wording. The new mapping test distinguishes death after damage from a
trigger that fires when damage is dealt; successful compilation alone does not
prove the client will display the correct rules sentence.

## 2026-09-28 — Revealing and choosing are separate permissions

Thoughtseize's partial implementation only charged life. A discard choice
must belong to the caster, while the whole target hand (including lands)
is revealed publicly. Reuse the public reveal journal event and the existing
private chooser projection; do not expose entire hands in every view. Test
the actual cast at three seats, invalid answers from the victim/bystander,
land rejection, empty and land-only hands, and self-targeting. A host test
checks each seat's log and view and that temporary chooser access expires.

## 2026-09-28 — Trigger scope and incomplete keyword implementations

A death look-back must name a source that left in the current event batch;
scanning every graveyard card made an old Marionette Apprentice drain for
later deaths. Pair a simultaneous-death test with a later sacrifice. When
multiple Opposition Agents exist, test reversed seat orders: storage order
is not timestamp order. Exalted needs a declaration-time trigger, not an
intervening-if condition checked again on resolution. Count declaration
events, so a creature removed before trigger collection cannot turn a
multi-creature attack into an attack alone. Update both Hierarchs and the
existing Cathedral implementation together, retaining tests for their mana
abilities and adding a three-player, three-instance exalted scenario.

## 2026-09-28 — A paid spell can still be a stub

Mind Twist had an X mana cost but no effect. Test the complete cast path:
announce X, choose a player, spend mana, resolve, then compare every seat's
hand. Random discard uses the engine RNG and never asks either player to
choose. Test X=0, oversized X and identical replay seeds. Generated stubs
are repaired through the reader; `adopt` deliberately refuses an unfinished
stub. Its refusal tests must call `refused`, not the helper that expects a
fully transcoded card.

## 2026-09-28 — Recheck partial-card notes against today's DSL

Thrun still claimed conditional static abilities could not be expressed,
although Station and Thorin already had that machinery. Add the missing
turn predicate and use the existing static layer path. Prove destruction
on both players' turns, plus expiry and reapplication at turn boundaries;
a keyword assertion on the initial board would miss the important failure.

## 2026-09-28 — Audit the ways a ward source can exist

Auditing the pool's eight Ward/disguise cards exposed three related gaps:
Fleshgorger's prototype, Branch of Vitu-Ghazi's disguise and Thorin's
enduring story. Test the actual cast form and zones, not just a Ward DSL
node. Prototype and disguise must use their announced characteristics for
casting prohibitions and restricted mana as well as after payment. A
face-up special action does not use the stack; its resulting trigger does.
Retained mana needs a cleanup boundary and a spent-unit test.

Target shortcuts are explicit, bounded answers over already queued identical
triggers, never a preference for future effects. Compare batch and manual
snapshot hashes in 2/3/6-seat games. Target explanations use captured printed
rules; ordinary spells need the full face because the ability-line table maps
stacked abilities, not every sentence of a spell. Keep AI seat names identical
on the board and in the chooser, and test the UI with real repeated ETBs.

## 2026-09-28 — Ward is a target-acquisition trigger

Test casts, activated abilities, copies and retargeting separately: a copy
is not cast, but still acquires its final targets. Retaining a target must
not trigger again, and targeting the same permanent in two slots triggers
only once per acquisition. Use the engine's opponent relation, not unequal
seat IDs. Dynamic life costs must be evaluated at resolution, with source
last-known information preserved across a blink. Hall of Storm Giants now
grants timed ward; Fleshgorger pays life using its current power. Tests exercise actual payments and resulting zones, not just DSL
variants.

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
- Resolution-time targeting: `ChangeTarget` / `ChooseNewTargets` +
  `AwaitingOp::NewTargets` — Misdirection's new target is chosen at
  resolution (CR 115.7), not at cast time. "Change the target" (115.7a) and
  "choose new targets" (115.7d) are different rules; the old single
  `RedirectTarget` offered a Plains to Path to Exile (#247). The question
  rides `TargetPrompt::Targets`, like a cast's (#249 part 2).
- Damage prevention modifiers: `PreventDamageToIt` / `PreventDamageFromIt`
  — checked directly in `combat.rs` deal-damage fns via
  `EffectFilter::ObjectIs`, not through the layers system (they're in the
  "handled elsewhere" match arm of `layers.rs` + `state.rs` modifier hash).
  Every new `Modifier` variant MUST be added to BOTH match statements or
  the build breaks with non-exhaustive errors.
- `Filter::Attacking` (Maze of Ith): evaluated by
  `state.combat.is_attacking` in `eval.rs`, a lookup in a sorted index kept
  beside the attacker list. The list is private: declare and remove through
  `CombatState`'s methods, which keep the two in step.
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
- `Effect::ExileSelfReturnAsFace { face, owner_control }` — transform via
  `obj.pending_face_change` applied in finish_resolution (resolve has no
  lookup; face switches need the def). `owner_control` is the sentence's
  "under its owner's control"; "your" and a silent sentence are `false`
  (observed fault 62).
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
  Implemented. Later (#232, 25.09.2026): Path and Boseiju, Who Shelters All
  were written with `.restricted`, which reads "When/If that mana is spent
  on …" as "Spend this mana only …", and paid for almost nothing. A rider
  with no "only" before it is `.when_spent(filter, rider)`.
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
  IfCountersAtLeast conditional statics + modal counter trigger; the
  TapTarget half was wrong, a tapped creature could "pay", and since
  2026-09-30 station is `cost!(TapOther(..))` + `Amount::TappedPower`,
  `docs/card-dsl.md` §"Station itself is a cost"),
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
  returns under OWNER's control (**wrong as a rule**, corrected 29.09.2026:
  only where the card prints "its owner's"; Restoration Angel's "under your
  control" keeps a stolen creature, so a blink now names its controller,
  `Effect::blink_to_owner` / `blink_to_you`); suspend costs are PAID ({U}/{1}{B} —
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

- DSL frozen (`docs/card-dsl.md`); the cards `AGENTS.md` playbook lived in
  `crates/baylee-cards/AGENTS.md` (folded into `docs/card-agent-brief.md`
  on 20.10.2026).
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

### 2026-09-28 — event-dependent graveyard targets

Scrap Trawler compares against the departed artifact, not against Trawler.
Capture its last battlefield mana value before copying is erased, then bind
that value to this trigger's target requirement. Re-reading either card at
resolution is wrong. Test a copied Effigy returning its own printed card,
simultaneous deaths with different values, an equal-value refusal, and the
event card moving again while its trigger waits. Source: Wizards' [Aether
Revolt release notes](https://magic.wizards.com/en/news/feature/aether-revolt-release-notes-2017-01-06),
Scrap Trawler rulings. The old partial-coverage test expected the dead Sol Ring
to return itself; a passing legacy test can pin the bug rather than the rule.

### 2026-09-28 — kicker can replace the target requirement

Tear Asunder needs its kicker decision before targets, and the offer must
combine an affordable price with the targets of that same way of casting.
A cheap ordinary price plus targets found only by the dearer kicked cast is
not a legal offer. Carry the selected requirement onto the stack for rechecks,
and let the client's cast chooser plan the complete kicked price. Free casts
waive the base mana cost but still pay optional additional costs.
Source: Wizards’ [Dominaria United release notes](https://magic.wizards.com/en/news/feature/dominaria-united-release-notes-2022-08-26), Tear Asunder and Stenn rulings.

### 2026-09-28 — Courser and public library tops

A zone permission is insufficient for playing from the library: both the offer
and `PlayLand` must validate that this exact card is still on top. The view sends
only an explicitly revealed top, includes its art/text entitlement, and never
makes the library browsable. Multi-card draws record each formerly public card.
CR 401.5 delays revealing a changed top during casting, activation and special
actions; keep the pre-action top through a shockland's entry choice so paying
life cannot peek at the next card. Source: [Comprehensive Rules, 401.5](https://media.wizards.com/2026/downloads/MagicCompRules%2020260619.pdf).

### Feedback, 28 September: shadow

Shadow is a symmetric blocker/attacker restriction, not unconditional
unblockability. The same pairing predicate must feed the client's offer and
validate its submitted declaration. Test both directions with the actual card,
including a flying non-shadow artifact creature and a three-seat game; combine
shadow with flying/reach rather than letting either keyword bypass the other.
Source: [Dauthi Voidwalker release notes](https://magic.wizards.com/en/news/feature/marvel-super-heroes-release-notes).
Unfinished generated cards are transferred through `xtask codegen --adopt-stub`
before editing, leaving coverage honest until their remaining rules are done.

Dauthi's graveyard replacement belongs in the zone-move door, not in death
triggers: it covers discards, mills and resolved spells as well as permanents.
Read the destination graveyard's player, not the departing object's controller;
exclude tokens and spell copies and retain the replacement through a simultaneous
wipe, then expire it before the next instruction. Discard events must still be
recorded even when the destination is replaced. The named void counter uses the
custom-counter registry and disappears when its card leaves exile. Tests cover
three seats, teams, phasing, real Mind Twist/Dark Ritual/Toxic Deluge resolutions,
and a destroy-all followed by milling in one resolution. Dauthi stays explicitly
partial: its activated permission to play an exiled card remains unimplemented.

## 29.09.2026 — library group: sacrifices as spell costs, searches of other libraries

A spell's "as an additional cost, sacrifice a ..." is `mandatory_additional_costs =
&[CostPart::Sacrifice(filter)]`; the cast wizard asks it (`CostSacrifice`), and
what was sacrificed is readable as `Amount::SacrificedManaValue` at resolution,
because the creature is a graveyard card by then. Tests must answer the sacrifice
question outside any `if let`: Crop Rotation once passed over a cost nothing paid.

`SearchLibrary` stays the search for your own library with a static filter.
"That player may search their library" (Assassin's Trophy, Boseiju) and "search
target opponent's library" (Bribery) are `SearchLibraryOf`, and the library
searched is the one shuffled — Path to Exile shuffled its caster's library until
this round. A derived bound ("mana value X or less, where X is 2 plus ...") is a
`ManaValueBound`, never a `Filter`, which has no resolution to read it from.

Check a helper's oracle id against the card file before trusting a test built on
it: `canopy_spider()` first pointed at Bay Falcon, and Natural Order's green
filter was what caught it. A library of one printing per test, cast twice with
two different sacrifices, is how a bound is shown to bound.

A new battlefield walk goes through `GameState::battlefield_seen` (phasing), and
a card spelling the five basic land types must print them or print "with a basic
land type"; both are convention tests that fire on a first try.

### 2026-09-29 — Maik's European Highlander

- **A new `Modifier` also moves counts in `effects.rs`.** The two census
  tests pin the number of declared variants and the lock/no-lock split
  (CR 611.2c). A no-layer modifier (one that parks on `Layer::Text`) lands in
  the "does not lock" half.
- **A new effect that reads the event object needs its `Debug` spelling
  registered.** Add it to `this_object_tests::every_event_object_in_the_pool_is_one_the_engine_reads`.
  Otherwise the census reports the card as reading nothing.
- **In a card test, `kill` passes priority.** Anything cast afterwards may
  find the step already over. `bury` destroys without passing.
- **A characteristic-defining P/T that counts graveyards is stale in a test.**
  After `seed_graveyard`, the projection has not been refreshed. Call
  `dev_state_mut(p).refresh_characteristics()` before reading P/T.
- **A hand-written token cannot be added by a card agent alone.** The
  token ledger (`generated_tokens.rs`) is written only by full codegen.
  Voice of Resurgence's */* Elemental is blocked on that.

### 2026-09-29 — friends group: Ossi's Schwarzrand and Dominik's Weltenbaum

- **`castable` lists only spells whose mana is already floating.** A test
  that asks the offer before tapping reads an empty list and blames the
  card. Tap first (`tap_all_mana`), then ask.
- **`PlayerRel::Opponent` is the first living opponent, not "that player".**
  In a cast trigger the player who cast the spell is `ControllerOfEvent`.
  Heads-up both are the same seat, so only a three-seat test tells them
  apart. Rhystic Study, Esper Sentinel and Smothering Tithe still use
  `Opponent`.
- **Run the sweeps a new walk or event reader trips.** `phasing_tests` counts
  raw battlefield walks per file (`battlefield_seen` is the door), and
  `this_object_tests` needs the `Debug` spelling of every new event-object
  reader. Both fail on a first try; neither is about the card.
- **`walk_to_own_main` returns at once when you are already in your own
  main phase.** To reach the *next* one, walk to the opponent's first
  (`reach_their_main_phase`).
- **A leaves-the-battlefield trigger asks its filter of the object as it
  was** (CR 603.10a). The card in the graveyard has lost every effect, so
  before `ltb_characteristics` a Living Lands Forest died as a land and a
  returned Enduring Vitality died as a creature. A "dies" filter that
  depends on a type an effect gave is the case to test.
- **A printed `*/*` is layer 7a, never a `ModifyPTPerCount` on 0/0.** The
  pump is 7c and survives a 7b "becomes 1/1" that should win. Write it as
  `CharacteristicPT`. The same sentence *granted* (Druid Class's land) is no
  characteristic-defining ability (CR 604.3a) and is `SetPTToCount`, 7b.
- **A level-up or class payoff that targets must be a trigger.** A static
  cannot choose. Class levels are Level counters (level 2 = one counter),
  and "when this Class becomes level N" is `Trigger::CountersReach`.
- **Two agents built the same piece twice.** `ControllerOfEvent` and
  `CharacteristicPT` from night-decks replaced this group's
  `ControllerOfEventObject` and `DefinePTByCount` at the merge. Read the
  integration branch's DSL before adding a variant, not only `main`'s.
- **A filter over stack objects matches abilities too.** An ability on the
  stack has a blank face, so `NONCREATURE` alone is true of it. "Noncreature
  spells you control" also asks for a noncreature card type (Benevolent
  Geist).

### 2026-09-29 — friends group, round 2: Weltenbaum's rules pieces

- **Set the harness's counters before walking to a step.** A counter put on
  a permanent after `reach_main_phase` leaves the projection stale until
  something else changes; the station test saw a 1/1 where the counters said
  otherwise. Put them first, then walk.
- **An ability with a mana cost is offered only once its mana floats.** Ask
  the offer after tapping, as for `castable`. A test that asked first read
  "not offered" and blamed the threshold it was written for.
- **A back face is cast at its own timing** (CR 601.3e, 712.11c, 715.3a).
  An instant on the back of an enchantment read as a sorcery for as long as
  the front's timing was the only one asked. A card whose two faces differ
  in speed is the case to test.
- **A triggered ability journals `AbilityTriggered`, not `BecameTarget`.**
  Count a copy's targets off `BecameTarget`, and expect the original's to be
  missing from it.
- **An ability on the stack is not projectable.** Insert one with
  `projectable = false`, or `stack_projectable` panics as it drifts.
- **A lock by name belongs on the offer, not in each door.** `apply` refuses
  what `legal.abilities` does not hold, so one narrowing pass over it covers
  printed, loyalty, granted and in-hand activations at once. Karn's lock
  sits in every door separately and needed a comment per door.
- **A board seated by `Duel::battlefield` enters tapped where the card
  says so** (the Deserts): an older test comment says otherwise. Read the
  status before tapping it for mana, or take the test to that seat's own
  turn, after its untap step.
- **A new field on `GameObject` can cost eight bytes for four.** The
  footprint test measures alignment, not the field: an `Option<PrintedFace>`
  grew the object from 296 to 304. Raise the budget deliberately, with a
  line in `docs/perf-baseline.md`.

### 2026-09-29 — friends group, round 2: Lose Focus and replicate

- **One constructor for a spell copy.** `CopyTargetSpell` copied the object
  and its object targets but not X, the mode, the face, a kicker or a player
  target; a copied Blaze dealt 0 to nobody. Replicate needed the same copy,
  so both go through `resolve::copy_spell` now, and a new copy effect should
  too.
- **A count fixed at cast time is a slice of a static table.** The
  replicate trigger's effects are `&REPLICATE_COPIES[..n]`, one
  `CopyThisSpell` each, so nothing has to allocate or read the count back
  at resolution. Storm could take the same shape.
- **A question that shares an answer with another says which it is.**
  `ChooseNumber` answers X and replicate alike; the `reason` field is what
  lets the client's headline and the AI tell them apart. The engine itself
  tells them apart by the wizard stage.
- **A trigger's source can be a spell on the stack below it.** An AI reader
  that looked up "what is resolving" by the source id found the spell before
  its trigger, and aimed the copy at the spell it was copying. Look for the
  ability of the source first.
- **Bound an optional count by what the payment reads.** The replicate
  bound asks the same `can_pay_mana` with the same `spend_for` as
  `finish_cast`, so restricted mana (Cavern of Souls) cannot make the offer
  and the payment disagree.

### 2026-09-29 — friends group, round 2: Walk-In Closet and Rooms

- **A designation that changes the rules text must end what the old text
  started.** `sync_static_effects` registers statics and drops them only
  when their source leaves or a condition fails, so a Room placed uncast
  kept the left door's static it was scanned with before its doors were
  set. `set_doors` drops the Room's statics and lets the next scan
  re-register them.
- **"Enters with" belongs where enter modifiers are.** Giving the door in
  `apply_enter_modifiers`, before the trigger scan of the same pass, is what
  lets the Room's own "when you unlock this door" hear its entry.
- **A reserved index needs one predicate.** Pithing Needle and split second
  each kept `TURN_FACE_UP` by name; a second special action would have been
  stopped by both. `choice::is_special_action` is the one question.
- **A test's second turn must leave the first.** `reach_their_main_phase`
  returns at once when the seat is already in its main phase; "next turn"
  needs a pass into the other seat's turn first, or "this turn" is never
  over.

### 2026-09-29 — Maik's European Highlander, second round

- **An item inserted "before `fn x`" lands between `x`'s doc and `x`.** A
  splice anchored on the `fn` line moves the doc comment onto the new item,
  and the compiler says nothing. It happened four times in this round.
  Anchor an insertion on the doc block's first line, and after a batch of
  splices look for a `///` block followed by a different item than the one
  it describes.
- **`Duel::battlefield(seat, …)` replaces, it does not add.** A second call
  for the same seat throws the first list away. Build one list per seat.
- **`ManaCost::combine` does not merge generic symbols.** `{W}` plus `{1}`
  plus `{1}` is `{1}{1}{W}`, which compares unequal to `{2}{W}` and is drawn
  that way. Add generic mana with `with_more_generic`.
- **A cast question with one answer is not asked.** When only one set of
  modes is affordable and legal, the wizard takes it. A test that expects
  `ChooseCastMode` there fails; assert the spell is on the stack instead.
- **A copy must carry everything that was chosen for the original.** The
  spell copy carried the targets but not the chosen mode, so a copied modal
  spell resolved to nothing (CR 700.2g). Any new per-cast field
  (`GameObject::modes`) goes into `CopyTargetSpell` too.
- **An index into a program that gets spliced moves.** A nested list that
  stops for a question replaces the op at the program counter with its own
  remaining ops, so every later index shifts. Count such a point from the
  end of the program: that part is never changed.
- **Check each term of an AI score against the tie-break.** "Prefer a set
  with no idle mode" never mattered while the earliest (smallest) set won
  ties. Only an empty board, where every mode but the unreadable one is
  idle, showed it doing anything. That board is the case in the test.
- **Code no card reaches has no test to fail.** A snapshot of the second
  target instance's last known information and a splice adjustment passed
  every injection, because nothing in the pool read them. Both were taken
  out, and the doc says what is not covered.

## 29.09.2026 — library group, round two: piles, a cast out of a graveyard, a single graveyard, crew

- **After a hand edit to a card, run `xtask codegen --tables`, never full
  codegen.** It rewrites the four compiled-pool tables (`generated_lines.rs`
  above all) and needs no corpus. `cargo dev gate rules` stops early
  without `DATABASE_URL`, so give it one while working.
- **Clippy on client tests runs only in the full gate.** `gate-rules` leaves
  `baylee-client` out, and `duel_flow.rs` crossed `too_many_lines` there
  first. Run `gate.sh` before a push, not after.
- **A new `Pending`, `YesNoPrompt` or `ChoicePrompt` variant moves pinned
  tables in `choice.rs`.** `kind_of` must number every kind without a gap or a
  collision, and the timeout table needs one row per kind. `ChoosePile` and
  `PayPact` once shared a number, and `PayPact` had no row: adding
  `CastPaying` exposed both.
- **A reveal to be sorted is shown in the client's Looking tab.** The browser
  hid rows a `ForChoice` sheet could not select and closed the sheet when the
  question was not about it. A pile question selects nothing, so the pile was
  invisible until `rows` exempted the Looking tab.
- **A Partial's reason can go stale without anyone noticing.** World Shaper
  named a missing effect that another card had already added (Lumra's
  sweep). Grep for the machinery before believing a reason. The friends
  group finished World Shaper the same night with a sweep of its own, so the
  merge kept one of the two identical effects (`YourGraveyardToBattlefield`)
  and dropped `ReturnAllFromGraveyard`.
- **"You may cast that card" during a resolution is a delayed cast plus a
  payment window.** `Effect::MayCastTarget` asks `CastPaying`. A yes opens a
  CR 605.3a window for the card's mana cost as the resolution ends, and
  passing it starts `start_paid_cast`, paid out of the pool. A lock such as
  "you can't cast additional spells this turn" lives in `PerTurn` and is read
  by `casting::may_begin_casting`, at every door a cast comes through: the
  offer, free casts, miracle, and a prepared copy.
- **`castable` in a test needs the mana already floating.** Tap a land first
  when a test wants to show that a spell *is* offered.
- **A new `bool` on `CastWizard` is refused by clippy after three.** Use one
  small enum field for a cast's origin (`EffectCast`).
- **`Rider::Linked` is released by more than its host leaving.**
  `GameState::set_monarch` returns every `Linked` card whose host's
  controller is not the new monarch, not only Palace Jailer's. A card that
  exiles "with" itself for good (Unlicensed Hearse) takes `Rider::ExiledWith`,
  which carries the host's version so a host that returned counts nothing
  (CR 400.7).
- **"From a single graveyard" is `CardInGraveyard(filter, PlayerRel::Chosen)`.**
  The activation asks `ChoosePlayer` over the graveyards holding a match
  (skipped for one), and the targets are that graveyard's. The enumeration
  (offer and CR 608.2b re-check) reads every graveyard.
- **Crew is one question answered with a set.** `CostPart::Crew(n)` asks a
  `ChooseCards` whose `max` is every creature offered, and `apply` refuses an
  answer short of the total power. Both hosts answer a refusal by asking the
  question again, so a short answer costs a click and not the game. Crew is
  alone in its cost (`lints::crew_is_alone_in_its_cost`), because the payment
  taps every answer left.
- **A bare keyword line can lose its ability's sentence.** Unlicensed Hearse
  prints "Crew 2" with no reminder and no `{`, so the lines reader saw
  `LineShape::Other` and the crew ability's line was `None`. Read the
  `generated_lines.rs` diff after `--tables`: a `None` for a new ability is a
  reader gap.
- **The house AI answers crew but does not choose to crew.** `policy::crew`
  taps the strongest creatures until the total is reached. `activate::gains`
  did not count `CreateContinuousEffect` as a gain, and Conduit of Worlds'
  `MayCastTarget` was not on that list either. (Since 29.09.2026 the list is
  `worth::effect_worth`, which prices a continuous effect by its modifier and
  still has no row for `MayCastTarget`.)
- **A graveyard cast is not flashback unless flashback paid for it.** The
  cast wizard used to put `Rider::Flashback` on every instant or sorcery
  cast from a graveyard, which was true only while the permissions (Wrenn's
  emblem, Muldrotha) cast permanents alone. Forgotten Cellar's
  `CastSpellsFromGraveyard` casts instants too, so the rider now asks which
  door the cast came through (CR 702.34a: "if the flashback cost was paid").
  Its own replacement exiles the card anyway, so an "it was exiled" check
  passes either way; assert the rider on the stack.
- **The `GameObject` budget decides where a per-object fact lives.** "That
  player" of Ragavan's trigger was first an `Option<PlayerId>` field on
  `GameObject`; `tests/footprint.rs` measured 312 B against the 304 B
  budget. It is now `Rider::EventPlayer(PlayerId)`: riders are a
  `SmallVec` already paid for, and a triggered ability carries no others.
- **Riders survive zone changes unless something clears them.** `Dashed`
  and `Escaped` are how a spell was cast, which belongs to the spell and the
  permanent it becomes and to no later object (CR 400.7).
  `GameState::move_object` keeps them only on the move to the stack and the
  stack-to-battlefield move. A blink test is the one that fails without
  that clearing; a plain cast test passes either way.
- **`castable` lists a card only once its mana is floating.** A test that
  reads `legal.castable` for a graveyard or exile cast taps the lands
  first. The view's `PublicObject::flashback` price is what lets a planner
  tap for such a cast before it is offered, so a new graveyard cast
  (escape) adds its price there too.
- **An existing cost prompt may already answer a new keyword.** Escape's
  "exile five other cards" is `ChoicePrompt::CostExile` with
  `min == max == 5`: the house AI pays it with the least valuable cards and
  client-core already names it. Check the prompt list before adding one.
- **Look the German keyword up in the catalog, never guess it.** Dash is
  "Sturmangriff" and escape "Befreiung" in German printings. The first
  label shipped as "Spurt" from memory. `card_faces.printed_text` for
  `lang = 'de'` in the local catalog answers it in one query.
- **A card turned `Implemented` meets checks it was exempt from.** The
  `validate` "you may" check skips `Partial` cards, so Ragavan first met
  it when its impulse was written: a new permission effect (here
  `ExileTopMayCast`) belongs on `OFFERS_A_CHOICE` in xtask with its
  argument. Run `xtask validate` before the gate after flipping coverage.
- **An effect that puts a card onto the battlefield names who controls it.**
  `ExileSelfReturnAsFace` wrote the owner for all 15 uses, and three
  kinds of sentence were hidden behind that one default: "its owner's",
  "your", and silence (CR 110.2a). The fix is a field that the card writes
  from its own printed line, with a lint that holds the field to the line
  through `lines::ability_line` and `oracle::sentence`. A test where owner
  and controller are one seat cannot tell the two apart, so steal the
  permanent first. The harness can register a layer-2 `GainControl` over
  any permanent, a Saga included (`dev_state_mut`, then
  `effects.register`).

## 30.09.2026 — Limited Edition Alpha, reader first

- **An absent `ValidPlayer$` on a `Phase` trigger is every player's step.**
  The reference writes `ValidPlayer$ You` for "your upkeep" and nothing for
  "each upkeep"; reading the absence as "your" generated Verdant Force as a
  card that made a Saproling on one upkeep in two, under `Implemented`.
  Pestilence's "the end step" is the same reading.
- **X is announced before targets are chosen (CR 601.2b, 601.2c).** A
  target filter that reads X (`CmcExactlyX`, `CmcAtMostX`) read the card's
  X as it lay in hand, 0, both in the offer and at the target menu: Spell
  Blast was never offered against a spell of mana value 2. The card carries
  the announced X from the X answer on; the offer asks "a target for some
  X".
- **A count with no controller named is the whole battlefield, not a
  refusal.** `IsPresent$ Creature` with no `YouCtrl` was refused as "a
  count with no player"; it is `Condition::BattlefieldCount`. A filter that
  names somebody else's (`OppCtrl`) is the third question, refused by name.
  `PresentCompare$ EQ0` is "none", `BattlefieldCountAtMost(…, 0)`.
- **The price of "unless" belongs to the line, not to its effect.**
  `UnlessCost$` was read inside the `Sacrifice` rule only, so Force of
  Nature's "8 damage to you unless you pay {G}{G}{G}{G}" had no reader. It
  is lifted in `chain()` for every API and wraps whatever the line says. A
  colour in the price is printed exactly (`PlayerMayPayManaOr`): charged as
  generic, a Mountain would keep Phantasmal Forces.
- **Spell additional costs are refused by the reader.** "As an additional
  cost to cast this spell, sacrifice a creature" (Sacrifice) is a hand-owned
  card with `mandatory_additional_costs`; a reader rule for `Sacrificed$`
  was written and removed again, because it read the effect and not the
  cost.
- **A permanent spell keeps what was done to it on the stack (CR 400.7a).**
  Purelace cast at a creature spell made a white spell and a creature of its
  old colour: the effect named the spell's version, and the permanent is a
  new one. `GameState::move_object` re-points `Resolution` effects on the
  stack-to-battlefield move and no other.
- **A clone's choice is made before it enters (CR 614.12a).** The reference
  writes `Choices$ Creature.Other`; nothing that has not entered can be
  among the choices, so `Other` names nothing and is dropped rather than
  read as `Filter::Another`. `AddTypes$` is the one except-clause read.
- **`Defined$ Targeted` is read against the chain's own target.** A player
  target makes it `Chosen`, an object or spell target makes
  `TargetedController` its controller (`ControllerOfTarget`); a word that
  names a target the chain lacks is refused, never guessed as "you".
- **"Tap or untap" is a yes or a no.** One of the two always does nothing
  (CR 701.26a, 701.26b), so choosing it is declining:
  `MayDo { ToggleTapTarget }`, asked on resolution. A pair of modes would
  have asked on casting, which is Derevi's written deviation.
- **A trigger's "its controller adds" is not the trigger's controller.**
  `AddMana` fills the ability controller's pool; Gauntlet of Might's {R}
  for an opponent's Mountain is the opponent's (`AddManaFor`,
  `ControllerOfEvent`). Two silent gaps sat under it: `TappedForMana`
  answered only "you", and the mana event named no event object, so
  "that player" of Manabarbs was nobody and the damage reached no one.
- **"It can't be regenerated this turn" outlives the spell.** Destroy's
  "it can't be regenerated" is a flag on that one destruction; Disintegrate's
  is a record for the turn (`CantBeRegeneratedThisTurn`), because the
  destruction it stops is lethal damage checked after the spell is gone,
  and the shield it ignores may have been made before it (CR 701.19c).
- **A card test that passes on an empty hand says nothing about a
  discard.** Wheel of Fortune's test drew seven into hands the testkit had
  left empty, so the missing "discards their hand" was invisible to it; the
  test now puts a card in each hand and asks where it went.

- **"Destroy" is only ever of a permanent.** `sba::destroy` moved whatever
  id it was handed into a graveyard, so a delayed "destroy that creature"
  whose creature had gone back to its hand would have discarded the card.
  Every targeted destroy was safe only because CR 608.2b had dropped the
  target first; an effect about an object it does not target has no such
  check, and the door now asks for the battlefield itself (CR 701.8a).
- **"Was it a creature" is asked before the move.** The turn's death tally
  read the card's types after it had reached the graveyard, where the
  projection is gone: an animated land died uncounted. Anything that asks
  what a permanent *was* as it left reads it before `move_object` clears
  the cache, or reads the last-known record.
- **A variable that two clauses fill cannot say which one did.** The
  activation reader claimed the `IsPresent$` family whenever `condition` was
  non-empty, and "your turn" filled it too, so a stray `PresentZone$` beside
  `PlayerTurn$` was dropped unread. Record what was read at the moment it is
  read (`present_read`), not afterwards from a shared result.
- **"Has been activated" is not "has resolved".** Dragon Whelp counts
  activations, and an ability is activated once it is on the stack and
  paid for (CR 602.2): four stacked activations have all been activated
  before the first resolves. A count taken at resolution would read one
  there and let the Whelp pump four times for free. The count is taken
  where "activate only once each turn" takes it, opt-in per ability, so
  unlimited activations stay out of the loop signature.
- **"This creature" in a delayed trigger is an object, not an id.** A
  sacrifice of the source by id sacrifices a Whelp that was bounced and
  recast; the delayed trigger is about the Whelp as it was (CR 603.7c), so
  it names the event object and checks its version when it comes due.
- **A fix for one reader of a spec is not a fix for every reader.** #147
  taught `zones::spec_object` that `ThisObject` is the source, but the
  damage side kept its own reader (`life::recipients`), which still read
  `targets` and shielded nobody for Rock Hydra's "prevent the next 1
  damage that would be dealt to this creature". Grep for every `match` on
  the enum, not for the function the last fix touched.
- **One event can name two objects, and the trigger picks which one it
  means.** A blocker's declaration names the blocker and the creature it
  blocks, and the event's own object is the blocker. Cockatrice's "destroy
  that creature" means the *other* one on both sides of the block, so
  reading the event alone destroyed the Cockatrice whenever it blocked.
  `trigger::event_object_for` asks the trigger first.
- **Two lines in the reference can be one printed ability.** The corpus
  writes "blocks or becomes blocked by" as a `T:` line per side, the
  second marked `Secondary$ True`. Read each as its own ability and the
  card triggers twice per block, or once for a sentence it does not print.
  Pair the halves, write one ability, and refuse a script that ends with a
  half unpaired, in `refusal_reason` as well as in `transcode`.
- **A skipped step is not a step that ran with nothing to do.** Stasis
  skips the untap step (CR 614.10). Running the step and keeping every
  permanent tapped would look the same on the board, but it would spend
  every "until your next untap step" effect, which CR 614.10a says waits
  for the first untap step that is not skipped. The skip leaves the step
  before its turn-based actions, not through the step's own ending.
- **"Can't untap more than one" limits a choice; it does not make one
  optional.** Winter Orb leaves the default of CR 502.3 in place, that
  everything untaps, and only stops the untap once the limit is full. So
  the player names which land untaps and must name one while any can. Two
  limits add up (the Winter Moon ruling): a basic land counts against "one
  basic land" and "two lands" both, so the menu shrinks after each answer,
  and a question's `max` is the smallest room among the limits that cannot
  take the whole menu.
- **"As though it had haste" is not haste.** Haste answers two sentences of
  CR 302.6, attacking and tapping. Instill Energy's "can attack as though it
  had haste" answers only the first (CR 609.4), so granting the keyword
  would also have let the creature use its {T} abilities the turn it
  arrived. Read an "as though" where its one rule is checked, not as a
  keyword.
- **"Is a Swamp" is not "loses all abilities".** Evil Presence's land loses
  what its own text gives it (CR 305.7), and keeps what other effects grant
  it and gains the Swamp's mana ability; `LoseAllAbilities` would have taken
  the grants too and, with its layer-6 timestamp rule, let an older grant
  fall. Read a rule's own list of what goes before borrowing a flag that
  takes away more.
- **A Partial keeps whole sentences.** The last 31 Alpha stubs went Partial
  keeping only sentences the DSL says completely: Pirate Ship's ping,
  Demonic Hordes' land destruction, Berserk's pump. Demonic Hordes' upkeep
  ("unless you pay {B}{B}{B}, tap this creature and sacrifice a land of an
  opponent's choice") is left out whole rather than kept as the tap alone:
  half a sentence reads as the card and plays as another one. The reason
  names what is missing and what the card still does.
- **A placed permanent still enters with its counters.** The printed-sweep
  bench seeds permanents, and `settle` runs their enter modifiers, so
  Clockwork Beast's seven +1/+0 counters are on it and it projects a 7/4
  against a printed 0/4. That is layer 7c (CR 613.4c), not a defect, and the
  sweep now shields power and toughness for a face that enters with P/T
  counters, as it does for a static that moves them.
- **A band is announced, not inferred.** Banding (CR 702.22) is a bit on
  the creature, but the band is a declaration (508.1e): a player may attack
  with two creatures with banding and put them in no band. So the engine
  asks, each attacker with banding in turn, and keeps the answer in combat
  state, where losing the keyword later does not undo it (702.22e). A
  keyword read at damage time would have banded creatures nobody banded.
- **A dead creature reads 0 damage.** A test asserting "the attacker took
  no damage" passed on the code that dealt it lethal damage, because the
  creature had died and its object read 0. Assert it is still on the
  battlefield first; an injection that should fail is how this showed.
- **A blocker on two creatures deals its power once.** The blocker pass
  dealt damage per block pair, which was harmless while a creature could
  block only one; a block spread through a band (702.22h) made a 3/3 deal
  six. Damage is per creature, divided across what it blocks (510.1d).
- **"When you control no Islands" is a trigger, not an "if".** The reader
  already turned `IsPresent$` into an intervening "if" (CR 603.4), and
  reusing that output for `Mode$ Always` would have made Sea Serpent ask
  its clause again on resolution: an Island played in response would save
  it. A state trigger (603.8) checks only when it triggers, so the clause is
  the trigger's own (`Trigger::State`), and a test moves an Island back in
  while the ability waits to show it.
- **A dev move runs no rules.** A test that took an Island away with
  `dev_state_mut` between two passes saw its state trigger stacked and
  resolved at once: the round of passes was already complete when the
  engine first looked, which no game action can arrange. Change the board
  while the player who acts next holds priority, so their pass is the first
  moment the engine reads it.
- **A Partial's tests stand on what it left out.** Pirate Ship's ping
  tests put the ship on a board with no Island, which was harmless while
  the sacrifice was missing; once the card was whole, the ship was
  sacrificed before the first priority and both tests failed at "seated".
  The reader also moved the ping from ability 0 to ability 2, in the order
  of the script's lines. A card handed back to the reader has its tests
  reread.
- **"Before the combat damage step" is a place in the turn, not a step.**
  Comparing the current step with a named one answers nothing in a combat
  with no damage step (CR 506.7e: no attackers, so the window ends with the
  declare attackers step) and nothing in the second main phase, which is a
  `Step::Main` like the first. `TurnInfo::position` puts every step on one
  scale; a test walks a whole turn and asserts both sides of each window.
- **A synthetic `{0}` spell is `ManaCost::parse("{0}")`.** `ManaCost::ZERO`
  is *no* mana cost, and a card without one cannot be cast (CR 118.6): the
  offer answered `NotEnoughMana`, which read like a broken window.
- **A requirement's maximum is measured in the offer.** CR 509.1c asks for
  "the maximum possible number of requirements that could be obeyed
  without disobeying any restrictions", and the restrictions the engine
  knows are the pairs the block question offers, each blocker's limit and
  menace's two-or-none. Measuring the maximum anywhere else — over the
  battlefield, or pair by pair — refuses a declaration the player cannot
  improve on. The question carries one declaration that reaches it
  (`obeying`), and the test that matters hands that declaration back and
  expects it taken.
- **"Duplicate blocker" was a limit, not a typo check.** Refusing a
  blocker named twice was CR 509.1a's one-attacker-each, written as a set.
  A creature that may block two attackers needs the set to hold pairs and
  the limit to be counted apart, or the refusal names the wrong reason.
- **"That many" after combat is the step's total.** A trigger that fires
  once for a step's combat damage (CR 510.2, 603.2c) and read the amount off
  the entry it fired on counted the first attacker only. A trigger about
  the player dealt damage sums the batch; a trigger about a source keeps
  that source's share. Test with two unblocked attackers of different power.
- **A "may" guard that matches a list whole misses the list with a tail.**
  `may_clause_possible` asked about `[SacrificeSelf]` alone, so "you may
  sacrifice this. If you do, …" with a consequence after it was always
  asked, and a yes paid nothing and bought the rest (Safe Haven). Match the
  head: the action is the cost (CR 118.12).
- **Taking an object away with a dev move clears its counters.** A test
  that wants a departed card *with* counters (what an effect resolving after
  it left can leave behind) sets them after the move, or the zone check
  under test is never reached.
- **Redirection is a replacement, not prevention.** It moves the damage to
  another door, where that recipient's protection and shields still meet
  it, and it ignores "can't be prevented". Ask it after the first
  recipient's shields and before the life: redirecting after
  `damage_player` would count and trigger on damage the player was never
  dealt.
- **"Once to an event" needs a memory that travels with the damage.** A
  static that redirects is asked again at the door the damage is moved to;
  without a record of what already applied (CR 614.5), a Bodyguard and a
  Jade Monolith hand the damage back and forth until the shield runs out.
- **"As long as this is untapped" can live in the affected filter.** A
  static condition is registered and dropped between passes, so a check
  inside one resolution sees the old answer. A filter such as
  `And(This, Untapped)` is read when the damage is dealt.
- **Unpreventable damage still meets a prevention effect.** CR 615.12
  applies it and lets its other parts happen: Rock Hydra's counters come
  off and the damage is dealt anyway. A guard that skips the effect for
  unpreventable damage keeps counters the rules take.
- **A new ability on an old card goes after the ones it had.** A seat's
  standing answers are keyed by ability index, so Rock Hydra's `{R}` shield
  kept index 0 and the static and the upkeep ability were appended.
- **"As long as" makes a printed `*/*` a 7b effect.** CR 604.3a's fifth
  criterion: a P/T sentence that holds only while a condition does is not
  characteristic-defining, even on the card. Gaea's Liege was written as a
  `CharacteristicPT`, which read its Forests in hand too; it is two
  conditional `SetPTToCount` statics now, 0/0 off the battlefield.
- **A count that reads combat needs a door, not only a condition.** The
  projection is re-read when the effect table moves; declaring attackers
  moves none. A conditional static is swapped between passes and moves it,
  but a count of the defending player's permanents without a condition
  would stay stale, so `board_state_changed` asks the modifiers as well as
  the filters.
- **An attacker keeps attacking a planeswalker that left** (CR 506.4c), so
  "defending player" still names that planeswalker's controller. A count
  that read `None` there made an attacking Gaea's Liege a 0/0.
- **An activation offer reads the floating pool, not the lands.**
  `Engine::can_pay_mana` asks the player's mana pool (plus the restricted
  mana the ability may spend), never the untapped sources. A test that
  reads "not offered" with the mana still in the lands proves only that
  the pool was empty; float the cost first, then read the offer, as Rock
  Hydra's upkeep-only test does in three steps.

## Alpha test backfill (Astra, 2026-10-01)

- Read coverage before designing the scenario. Balance, Gloom, Mana Flare,
  Power Surge, Black Vise, Cyclopean Tomb and Glasses of Urza currently have
  no supported rules-text abilities. Their new tests check the printed mana
  payment and the normal destination of the resolved spell only; passing
  those tests does not promote them out of Partial or into evaluation decks.
- Purelace, Phantasmal Forces, Mind Twist, Manabarbs, Stone Giant and Gauntlet
  of Might passed their new behavioral scenarios. Check both players where
  text says “a player” or “red creatures,” and include a negative control
  such as a nonland mana source, a non-Mountain, or equal toughness.
- `reach_main_phase` only walks priority windows. To cross a whole turn,
  use `walk_to_own_main` or an explicit `pass_until` predicate; a harness
  panic at the attackers question is not a card defect.


## Alpha verification follow-up (Codex, 2026-10-02)

- **Count Partials from card files, not a hand-written subset.** Seven Alpha
  files still explicitly refuse sentences: Balance, Gloom, Mana Flare, Power
  Surge, Black Vise, Cyclopean Tomb and Glasses of Urza. The feedback handoff's
  later “four” count omitted the last three without any implementation change.
  Cast/payment/destination smoke tests do not justify promoting those cards.
- **Exercise both sides of a mana-choice boundary.** Gauntlet of Might's
  previous behavior test covered basic Mountains. The new Taiga test observes
  zero mana before the color answer and exactly green + red after choosing
  green; the opponent controls the artifact, so the recipient is checked too.
  The WotC ruling dated 2004-10-04 was read directly from Scryfall's rulings API:
  “Dual lands which have Mountain as one of their types produce an extra red
  mana when tapped for either color.”
  Source: https://api.scryfall.com/cards/df3bc33f-23f8-4eb4-a70e-b8b7af5b40f6/rulings
- **Remove the source through game actions.** Two Gauntlets on opposite sides
  give both red creatures +2/+2 and a Mountain three red. Disenchant removes
  them one at a time; subsequent taps yield two, then one red, and creature
  stats lose exactly one bonus each time. No rule state is edited by the test.
  Both new scenarios pass; the generated card remains unchanged (original
  implementation f9db2475, Claude Opus). No coverage status is promoted.


## Alpha engine prerequisite (Codex, 2026-10-02, 06:49 UTC)

- **Historical counts belong at the event boundary.** Power Surge asks about
  untapped lands at turn start, not the now-untapped board at upkeep resolution.
  `begin_turn` now records the active player's count before untap/phasing;
  normal and extra turns, skipped untap, and first turns all use that path.
- **A source need not exist when the history is recorded.** The WotC ruling
  dated 2004-10-04, read directly from the Scryfall API, says: “This card knows
  how many were untapped even if it was not on the battlefield at the beginning
  of the turn.” The snapshot is unconditional; a synthetic observer entering
  later reads the earlier count through an actual resolving effect.
  Source: https://api.scryfall.com/cards/0b5717af-a1a3-45cb-8b05-7543eed5532a/rulings
- **History also distinguishes otherwise identical game states.** The saved
  count joins the replay snapshot hash and the loop signature, with regressions
  for each. Five synthetic behavior tests cover turn changes, extra turns,
  later taps, skipped untap, phasing, and a late observer. The card remains
  Partial until its implementation and independent card tests are present.


## Alpha Power Surge and Glasses of Urza (Codex, 2026-10-02)

- **Look and reveal are different permissions.** Reusing a public reveal event
  for Glasses of Urza would disclose the hand to every seat and the game log.
  Private inspection uses the existing deciding-seat `looking_at` projection
  and an empty acknowledgement; a three-player played test checks both the
  temporary view and the absence of public reveal events.
- **A zero-selection prompt is an inspection, not a broken picker.** The
  browser still opens, but cards are not selectable and its footer has only
  Confirm. Tests cover an opponent's hand, one's own hand and an empty hand.
- **Power Surge reads history, not the upkeep board.** Card-specific tests
  tap lands in response, cross both players' turns and exercise zero damage.
  The prior engine tests cover skipped untaps, extra turns and late observers.
- **Codegen needs a complete printing cache.** An incomplete local cache made
  `codegen --tables` fail after writing partial ability-line data. Refreshing
  with `xtask scryfall-cache` (2955/2955) and regenerating twice restored the
  full output; the final diff only adds these two cards' ability lines.


## Alpha Black Vise (Codex, 2026-10-02)

- **An entry choice is not a target.** `ChooseOpponent` goes through the entry
  modifier queue and offers living opponents through the shared relationship
  evaluator. It must not borrow the casting wizard or target legality checks.
  The four-seat scenario rejects the controller and teammate and distinguishes
  the two opponents' upkeeps.
- **Remember the choice as permanent state.** A separate chosen-opponent rider
  clears when the permanent leaves, participates in snapshot and loop hashes,
  reaches every seat and separates board groups. The ordinary spell-target
  `chosen_player` would conflate two different rules facts.
- **Damage reads the hand when resolving.** The test destroys the Vise and casts
  Ancestral Recall in response: the already-stacked trigger still deals six for
  ten cards. Separate scenarios cover zero/four/seven cards before the draw step
  and a new choice after bouncing and recasting the artifact.
- **Preserve test mana deliberately.** `cast_from_hand` taps every mana source.
  A response test must pay the initial artifact from one land explicitly so it
  still has mana during the opponent's upkeep; otherwise the failure tests its
  fixture rather than its rule.

- **Rare choices must not enlarge every AI search object.** A direct optional
  field raised `GameObject` from 312 to 320 bytes. The existing sparse rider
  list stores the choice with no base-size increase; both footprint tests now
  pass with the original budget. Explicit rider hashing and the loop signature
  still distinguish the choice.
- **Compare chooser names with the board during live QA.** The raw roster name
  localized the bot difficulty as “Solide 1”, while the table used “Haus-KI”.
  The player chooser now shares the table's naming function when its view is
  available; a regression test uses the real difficulty-based roster name.


## Alpha Mana Flare (Codex, 2026-10-02)

- **Capture production, not capability.** Taiga choosing green produces only
  green, while Gruul Turf produces red and green. Mystic Gate's two suspended
  picks belong to one activation. Capture the complete event batch and stop
  at the source's next tap; do not count a different permanent's bonus mana.
- **Mana trigger controller and activator can differ.** The opponent's Flare
  asks the land's controller for the type. The multiple-copy test exposed
  `finish_resolution` returning priority to the enchantment's controller after
  a suspended mana choice; it now returns to the captured mana activator.
- **Production restrictions are not inherited.** A Ziggurat bonus actually
  casts Lightning Bolt while the land's own mana stays creature-only. A snow
  Forest contributes one snow mana, with an ordinary bonus from ordinary Flare.
- **Choose the correct test turn helper.** `reach_main_phase` is for reaching
  priority in the current turn. Crossing the first combat requires
  `reach_their_main_phase`, which handles empty attacker/blocker declarations.
- **Rule source:** Wizards' 2023-09-01 rulings, retrieved through Scryfall's
  [rulings endpoint](https://api.scryfall.com/cards/a4bb303d-b230-430a-a2c3-f91a776de34e/rulings),
  confirm one chosen produced type and no inherited restrictions or riders.

## Alpha Balance (Codex, 2026-10-02)

- **Collect choices before moving any cards.** Reusing sequential discard or
  sacrifice chains would reveal earlier hand decisions and change the board
  before later players chose. The new equalization continuation stores each
  group's choices and commits that group only after all players have answered.
- **APNAP begins with the active player.** A three-player test casts on seat 1's
  turn and verifies seat 1, seat 2, seat 0 order, including forced keeps. Each
  group computes its own minimum; a sacrificed Dryad Arbor is no longer a
  creature in the last stage.
- **Public choice does not mean reveal.** Permanent keeps have a dedicated
  journal/log event with normal visibility rules, preserving face-down identity.
  Hand keeps have no public event. A host test observes every seat before and
  after the final hand choice, when discards first become public.
- **Private continuations still affect determinism.** Two games with the same
  unchanged board and different already-chosen hand keeps must hash differently.
  The selection fingerprint includes its progress, options, keeps and removals.
- **A byte-sized menu is not a rules limit.** Exact selections above 255 cards
  continue in chunks without moving cards early. The played 258-versus-256-land
  test keeps 256, rather than truncating the minimum to zero or 255.
- **Tell both humans and AI what the selection means.** The explicit Keep prompt
  reverses the sacrifice/discard preference: keep valuable battlefield objects
  or castable hand cards. Existing clients cannot decode that new enum variant,
  requiring protocol 10; the new public log variant requires view 45.
- **Rule source:** Wizards' 2016-06-08 Balance rulings, retrieved via
  [Scryfall](https://api.scryfall.com/cards/ce648aa3-098b-4af0-a433-fd290bc85904/rulings).

- **Photograph the transitions, not just the result.** Balance's first live run
  was rules-correct but called lands, hand cards and creatures the same thing.
  Distinct keep prompts now identify the three stages without extra explanation.

## Alpha Gloom (Codex, 2026-10-02)

- **Price the spell that is announced.** Swift Spiral is white although Twining
  Twins is blue; Damn remains black when its overload cost contains white mana.
  Prototype and disguise need their announced characteristics, while a global
  color effect still applies afterward. Color identity is not spell color.
- **Free is an alternative cost, not immunity to increases.** Suspend/rebound,
  granted free play, miracle and effect casts each have their own wizard entry.
  Every entry must retain a priced option even when the base mana cost is zero.
- **Trace every activation door.** Printed, granted, intrinsic mana and loyalty
  abilities have different offer/payment paths. A white enchantment land must
  pay before adding mana, including when its color choice suspends resolution.
- **Test setup must run the lifecycle it claims to test.** Directly moving Gloom
  out of play without advancing the engine leaves static registration stale.
  The removal regression now casts and resolves Disenchant, paying its own tax.
- **Do not borrow an unfinished card's omitted sentence as a fixture.** Sevinne's
  Reclamation remains Partial with no printed flashback. The Gloom regression
  explicitly grants flashback at the mana cost; it claims no Reclamation completion.
- **A correct payment can still be unreachable.** Both client and AI previously
  stopped floating at the printed cost. Authoritative, per-form increase hints
  let them plan enough mana without independently interpreting public statics.
  Protocol 11 prevents older clients silently retaining the wrong planner.
- Source: Wizards' Gloom rulings retrieved from the
  [Scryfall endpoint](https://api.scryfall.com/cards/0d26c559-ad06-4f78-b7b6-e658be8c7bdb/rulings).

## Alpha Cyclopean Tomb and complete inventory (Codex, 2026-10-02)

- **A seven-card worklist is not a set inventory.** Joining every Oracle id in
  `generated/index/set_lea.rs` to its actual card file found 32 other Partials
  after Gloom and Tomb. Current print-set headers are reprints, not reliable
  original-set membership. `docs/set-progress.md` supersedes the earlier counts.
- **Capture the source incarnation before it leaves.** A linked-counter ability
  carries its source version on the stack; death triggers bind event-time
  identity. A returning Tomb must not inherit an older incarnation's lands,
  and a pending activation still marks for its old source after that source dies.
- **A duration ending at zero must stay ended.** Re-adding a mire counter later
  does not revive the expired land-type effect. Counter removal expires it
  immediately, including before another instruction in the same resolution.
  Bulk drains must use the same removal path. Source departure does not end it.
- **A recurring trigger stays registered after being countered.** Tomb's cleanup
  goes on the stack every own upkeep, with choices made on resolution; a
  Tidebinder can counter one occurrence without canceling future occurrences.
  The ledger tracks all counters removed from one eligible land, including
  counters supplied by another effect, and remembers that this land was cleaned.
- **Dev mutation is not an engine step.** Refresh the legal offer after filling
  mana, and refresh characteristics after directly changing a test board before
  inspecting it. These are fixture obligations, not reasons to change rules.
- Official ruling read from the Scryfall WotC record: “The land remains a Swamp
  as long as it has a mire counter on it.” Source:
  https://api.scryfall.com/cards/7c77c7c1-39a5-4049-8e2c-3561b046152e/rulings

## Alpha Creature Bond (Codex, 2026-10-02)

- **Capture an event, not the latest incarnation.** The existing LKI stores are
  erased when a card moves again. A trigger must retain its own death-time
  toughness and controller, even when a single resolution kills and returns the
  creature before triggers are collected. Store immutable departure information
  on the journal entry and bind the required values to the queued/stack trigger.
- **Simultaneous destruction must not depend on iteration order.** Removing the
  Aura first erased its attachment before the host was examined. Snapshot all
  departures before `DestroyAll` begins; a real Disk activation now tests both
  internal orders, including the Aura dying with its host.
- **A generated index is not a compiled card.** Leyline of Sanctity has an index
  but is absent from this pool. The damage regression grants player hexproof
  through the existing modifier and a prevention shield explicitly, then casts
  Terror. It proves untargeted, preventable Aura damage without relying on a
  nonexistent fixture card.
- Oracle data checked via the Scryfall card endpoint and `xtask explain`; the
  corpus was read as a mechanics reference only. No external script was copied.

## Alpha Consecrate Land (Codex, 2026-10-02)

- **Targeting and attachment are separate questions.** The Aura's enchant
  keyword chooses targets; this card's restriction prevents attachment. Do not
  implement it as shroud or silently forbid land-destruction targets.
- **A forbidden entering Aura must never generate an ETB event.** Checking only
  attachment SBAs briefly puts it on the battlefield. Check the actual zone
  transition: from the stack it goes to the graveyard, from another zone it
  stays there. An already attached Aura stays on its old host when a move fails.
- **"Other" means another object, not another card name.** A second Consecrate
  Land is refused as well. Existing other Auras fall off, while the granting
  Aura remains attached and its land keeps indestructible.
- Live WotC ruling (Scryfall endpoint, checked 2026-10-02): existing other Auras
  go to their owners' graveyards. Current CR 303.4 and 702.5 were read from the
  official September 25 text. The corpus was inspected without copying a file.

## Alpha Animate Artifact (Codex + Astra xhigh, 2026-10-02)

- **Keep a multi-layer effect together.** A noncreature filter independently
  reevaluated in layer 7b rejects the creature the same effect just animated.
  Carry the layer-4 affected set forward within the projection (CR 613.6).
  This needs no persistent ledger, and a fresh projection makes a fresh choice.
- **Read mana value after copying.** Copy Artifact copying Sol Ring has mana
  value one, despite the original card costing two. Its animation must also
  retain enchantment alongside artifact and creature.
- **Test actions after animation.** An old artifact can immediately attack and
  use its tap ability; a newly entered one cannot until its controller's next
  turn. Creature age is irrelevant. Mana-value zero needs an actual SBA test,
  including an indestructible artifact land and a counter that keeps it alive.
- Viktor explicitly requested an Astra agent with `xhigh` for engine jobs.
  Engine/DSL work was delegated accordingly, with card behavior tests and
  integration review kept in the parent task; cargo gates are serialized.
- A separate GPT-6.1 Sol `medium` review was then explicitly requested. It
  found a concrete counterexample to a conservative dependency shortcut:
  Animate Artifact followed by Swift Reconfiguration on printed Sol Ring.
  Removing creature from the initially noncreature ring does nothing, so it
  must not force the earlier animation to wait. The real-cast regression
  failed before the correction while the other fourteen card tests passed.
- **Ignore cycle-internal edges before choosing a ready effect.** Two
  conditional animations can form a dependency loop while an unrelated
  type-removal effect is ready. Waiting until the entire graph is stuck picks
  that removal too early. The dynamic path removes strongly connected edges
  and re-evaluates after each effect. Its packed graph also handles more than
  64 effects; a 66-effect regression guards against a silent timestamp fallback.
- **Printed mana abilities use indexed actions.** Sol Ring's tap is in
  `LegalActions::abilities` and uses `ActivateAbility`; `mana_abilities` is
  the intrinsic/granted shortcut. Tests must assert the same offer and action
  a client actually uses, even when both abilities produce mana immediately.
- Official rules text checked: CR 205.1b, 208.3, 613.4, 613.6, 613.8 in the
  September 25 rules. WotC rulings were read from the Scryfall endpoint for
  `63aec27b-8cda-46c2-9f4f-9ebe98dffe2e`. The corpus was only a lookup reference.

## Alpha Nether Shadow (Codex + Astra xhigh, 2026-10-02)

- **A graveyard-count condition needs real ordering choices.** Counting vector
  positions is insufficient when a mass destruction spell or sacrifice cost
  puts several cards there together. CR 404.3 gives that choice to their owner,
  who may differ from the controller choosing which cards to sacrifice.
- **Zone membership does not establish identity.** A source exiled and returned
  to the same graveyard is a new incarnation. Tests must restore the numerical
  condition too, otherwise an intervening-if failure hides the identity bug.
- **Use a real response and real mass removal.** Scavenging Ooze removes one of
  the three required creature cards while the trigger waits. Wrath destroys
  four creatures; its own card must go above them only after their order has
  been chosen. A subsequent upkeep proves the chosen order affects gameplay.
- Astra `xhigh` handled the engine jobs; GPT-6.1 Sol `medium` reviewed the card,
  rules and coverage independently. The first review found missing cost-payment
  capture and SBA batching across legend choices, plus finished Saga sacrifices.
  Targeted regressions cover these paths and nested optional effects before the
  broad integration gate. Cargo jobs are serialized across agents.
- **A whole cost transaction is not one simultaneous instruction.** Capture
  contiguous sacrifice or discard parts together, preserving multi-card costs
  such as Time Sieve and Great Hall of Starnheim, but close the group at a
  different cost kind. Do not mistake Great Hall's single "sacrifice this land
  and a creature" instruction for two independent costs.
- **Preselect SBA choices, then apply the event.** Resolving legend choices
  after lethal creatures have already died creates a false second event. A
  creature selected to remain under the legend rule can still die to lethal
  damage in that same check. Multiple legend groups belong to the same pass.
- Read official CR 404.3, 704.3 and 113.6b/m in the September 25 rules, plus
  WotC rulings through Scryfall for `11988c46-6d0c-46a8-85aa-8c6da72bfe30`.
  The corpus was used as a lookup reference only.

## Alpha Sunglasses of Urza (Codex + Astra xhigh, 2026-10-02)

- **Broaden payment permission, not mana production.** White paying a red
  requirement is still white in the payment record. Preserve restrictions,
  snow provenance and spend riders; do not recolor the pool or rewrite costs.
  CR 609.4b was read in the current official rules before implementation.
- **Publish the same permission to planning.** An engine accepting white mana
  for red is insufficient if the client cannot propose tapping a Plains.
  A default-exact permission matrix travels with the mana-pool view and lets
  the planner choose actual source colors for expanded requirements.
- **Reserve constrained mana and prune impossible assignments.** The Sol
  `medium` review found a potential combinatorial regression when a global
  any-color permission branches on each pip of an unpayable cost. A minimum
  remaining-mana bound avoids that case; mixed white/red and snow costs test
  that flexible payments cannot consume a later constrained requirement.
- **A generated index does not mean a card is in the playable registry.**
  Lightning Helix and Humility constants existed, but the test fixture rejected
  those cards. The final scenarios use registered Thorin Oakenshield for `{R}{W}`
  and Oko's real ability to remove the Glasses' ability.
- Direct fixture zone changes need static-effect synchronization before
  refreshing legality. `refresh_offer` republishes the question; it does not
  run the engine machine. Real casting and activation scenarios exercise that
  lifecycle without manual synchronization.
- Astra `xhigh` implemented the payment paths; Sol `medium` independently
  reviewed the card and coverage. The parent supplied card behavior scenarios,
  including Ancient Ziggurat's real restricted white mana, and integration gates.
  Oracle was read live through Scryfall; no card-specific rulings were returned.
  The corpus was read only as a lookup reference.

## Alpha Earthbind (Codex + Astra xhigh + Sol medium, 2026-10-02)

- **A gained static ability belongs to its recipient.** Earthbind's trigger
  grants an ability to the Aura after dealing damage; it does not create an
  unconditional printed flying-removal ability. Keep the trigger's intervening
  condition and damage distinct from the gained ability, and preserve ability
  removal, copying, phasing and later flying grants.
- **An attachment is an incarnation, not only an object handle.** The
  independent review caught a host blink reconnecting an Aura to the returned
  creature. Departure snapshots, attachment timestamps and phasing LKI need
  separate checks; direct fixture reattachment alone missed those paths.
- Astra xhigh implemented the engine primitives and generic regressions. Sol
  medium independently checked current Oracle/CR and authored eight real-spell
  tests, including Jump, Flight, Disenchant, Unsummon, Ephemerate and Tishana's
  Tidebinder. Its three findings were fixed before the broad engine gate.
- Run the footprint gate before declaring an engine milestone finished. The
  first identity rider enlarged every GameObject by eight bytes; splitting its
  compact fields preserved exact identity and the existing 312-byte budget.
- The full first-printing ledger remains the set inventory: Earthbind moves
  Alpha to 261 Implemented, 25 Partial and four explicit exclusions. Dedicated
  coverage for one card cannot establish completion of the whole set.

## Alpha Fork / Power Sink / Clockwork Beast / Drain Life (2026-10-02)

- A stale Partial explanation is not an implementation specification. Fork
  already retargeted players; the actual remaining gaps were separate target
  groups, legal swaps and copied decision context. Stage target changes until
  the complete assignment is legal, preserving fixed damage shares and the
  right to keep an unchanged target that has become illegal.
- Copying remembers choices and relevant additional-cost information, but a
  copy spent no mana. A real Sacrifice/Fork scenario checks sacrificed mana
  value; generic tests separately check copied damage shares and zero mana
  spent. Legacy tests must follow the optional per-target prompt without
  deleting their outcome assertions.
- “Has a mana ability” is a current characteristic, including granted and
  removed abilities. A basic-land subtype or printed ability list is not
  enough for Power Sink. Its unpaid penalty still happens when the target
  spell cannot be countered, and the original controller survives stack exit
  as last-known information.
- Clockwork Beast records participation for the combat and incarnation, then
  chooses its refill amount at resolution. The paid X, remaining capacity,
  optional zero and old-trigger/new-object separation are distinct tests.
  Preserve the 312-byte GameObject budget when adding history.
- Drain Life restricts the actual mana spent on X. Spend-as-color permissions
  cannot turn other mana into black mana for this restriction. Cost reduction
  applies once to both the total payment and the restricted portion; synthetic
  cast tests cover reductions of zero, two and five. Life gain follows actual
  damage and the recipient's pre-damage limit, not chosen X or remaining
  toughness after earlier damage.
- Generated ability-to-sentence mappings are part of client acceptance.
  Clockwork's activation moved to index 1 and required regenerated tables;
  otherwise a correct activation could have no printed sentence in its sheet.
- New choices need all consumers, including the heuristic AI, client-core,
  native targeting and the narrated seat. The counter-amount prompt also needs
  a protocol version bump so older clients fail clearly at connection time.
- Astra xhigh authored Engine changes and generic regressions. Sol medium
  independently authored card scenarios; Astra high owns the client/display
  acceptance and Mana-font prompt repair. The first 26 independent scenarios
  passed before final broad integration gates and visual acceptance.
- Audit numeric ceilings through the full route: legal choice bound, actual
  payment, effect amount, event payload, trigger rider and visible log. Removing
  the arbitrary X=50 ceiling exposed a signed 16-bit damage conversion. Boundary
  tests now pay for X=32,767, 32,768, 65,534 and 393,208 and assert a single full
  damage event plus the exact pool debit and life gain. Sparse event amounts in
  riders preserve the object footprint; both fingerprint readers must follow.
- A client screenshot caught a Mana-font tap glyph wrapping below a fixed-height
  confirmation button. Text-renderer correctness alone does not prove layout;
  recheck the armed button after changing wrapping. Existing direct numeric
  keyboard input already supports large values: inspect that route before
  inventing a second input widget.
- Drain Life's final independent review caught a timing distinction hidden by
  ordinary damage tests: only player life and planeswalker loyalty are captured
  before damage. Creature toughness is current when life gain is applied. Rock
  Hydra can lose every counter during prevention; one damage may then imply
  zero life gain. Refresh characteristics between those instructions and combine
  both caps for a permanent that is a creature and a planeswalker.
- X bounds depend on when mana may be produced. Immediate-payment casts can
  derive a resource bound; a deferred mana-ability window cannot be bounded by
  the pool that happens to be floating before it opens. The full suite caught
  Miracle offering X=0 over four untapped Swamps. Preserve arbitrary announced
  X for deferred payment and enforce transactional affordability afterward; a
  sacrifice-for-63-black fixture checks actual payment of X=61 after the choice.
  Do not replace this with counting untapped lands, which misses dynamic and
  sacrifice mana abilities.


### Alpha completion, second batch — payment and target boundaries

- A generic target-count sentinel must remain unbounded through Engine, wire,
  client, AI and narrated-seat consumers. Widening only the card definition
  leaves hidden 255-target ceilings. Independent Fireball scenarios select 256
  real targets and pay the actual additional cost; retarget position metadata
  also needs a wide representation.
- Additional target costs become known after targets are selected. Reuse the
  casting mana window at that point, including transactional refusal, instead
  of requiring the entire final cost to have been floated before casting.
- Optional arbitrary payments are not fixed debts. Preserve the opportunity to
  activate mana abilities from an empty pool, offer zero and legal overpayment,
  and use the same spending restriction for the offered bound and actual debit.
- Prevention is an effect on a damage event, not subtraction from the printed
  damage amount. Power Leak's event-scoped shield follows redirection and must
  expire after its event. A second independent test exposed the older shared
  fixed prevention ordering: Reverse Damage can produce a different result
  depending on the affected player's choice under CR 616.1. Keep Power Leak
  Partial while that choice is absent; an explicitly ignored known-failure
  regression is documentation, not acceptance evidence.
- Trigger controller, current source controller and damaged player can diverge.
  Test Demonic Hordes after both control change and source loss, and label the
  player whose land will be sacrificed in the opponent's selection prompt.

- Live screenshots exposed bare legacy `BBB` in translated printed text and
  inconsistent AI names between the HUD and decision prompts. Repair missing
  repeated mana symbols only when Oracle symbol evidence matches exactly, and
  apply it when old cached presentation data is loaded too. Preserve roster
  names for named AI seats; localize only the generic house label. Both fixes
  have regression tests and were verified in the rebuilt client's screenshots.


### Damage ordering integration — accepted in 81409297

- Choosing the prevention order is not enough: simultaneous sources need a
  true limited-shield allocation. Real deathtouch/lifelink combat proves that
  the allocation changes outcomes, not just a displayed number.
- A pending damage choice must not publish prevention life gain or counter
  removal early. The independent APNAP scenario caught premature Reverse
  Damage life gain. Accumulate these results until the damage event commits;
  reserve consumed counters so later choices cannot overspend them.
- Preserve incarnation identity from ability creation through damage and LKI.
  A returned permanent is not the earlier source still represented by a stack
  ability. The old ObjectId-only source menu is a distinct unresolved contract
  limitation, even when the damage executor respects versions correctly.
- Existing trained policies cannot score new decision kinds automatically.
  Guard the actual replay-converter entry points before feature encoding and
  sample emission; a downstream Unsupported result alone can leave an invalid
  diagnostic sample path alive.

- A local effect selection can change without a new board view. Include the
  selected row and damage decision identity in the shelf revision so Confirm
  appears immediately and disappears for a new question. A headless Bevy test
  exercises the actual sync system with an unchanged view; a pure reducer test
  would not catch this missed redraw.

### Exact source choices — accepted

- A version increment alone does not prove that a permanent spell resolved.
  Record the actual resolution transition and test the real casting path;
  a unit test that directly marks the transition cannot catch a missing hook.
- Historical sources are a separate projection keyed by object and version.
  Include entitled identities in print/catalog iterators, and never recover a
  historical label or preview by looking up only the current ObjectId.
- Copying a referenced old spell must preserve that spell's captured incarnation,
  even if the same card is now a newer spell on the stack. Retargeting from an
  object to a player can compact object-target slots; carry remaining historical
  references with their targets instead of keeping the old numeric slot.
- A distinguishing keyword in a regression fixture must already have working
  engine semantics. The initial three-incarnation test used Infect, which this
  engine explicitly does not support; use supported lifelink and retain distinct
  behavioral outcomes rather than weakening the assertion to menu contents.

- Capture a death trigger's source before the departure, independently of its
  event object's new graveyard identity. Two real Omnath/Terror/Circle tests
  exposed the default trigger builder capturing the new graveyard version:
  a preselected battlefield shield failed, and a postdeath source menu omitted
  the original source. Ordinary, granted and simultaneous departure paths need
  explicit review; a green activated-ability blink test does not cover them.
- A version number makes two historical sources technically distinct but does
  not tell a player which ability each produced. Live source rows need the
  public stack context, such as their different targets, without looking up a
  newer object as the historical source.

- Review expected outcomes against every real card in the fixture, not just the
  card under test. The Personal Incarnation planning matrix initially credited
  Swords to Plowshares life gain to the owner after theft; Swords explicitly
  credits the controller. Keep stolen and unstolen outcomes separate so a test
  does not encode the wrong rule.

- A stored trigger event context is not automatically an object referred to by
  the ability's instructions. Omnath's death damage refers to its old battlefield
  source; undying explicitly refers to the new graveyard card to return it.
  Source-menu eligibility must follow actual effect references while retaining
  enough internal context for resolution.

### Exact targets — accepted

- An unchanged illegal target and a newly selected returned incarnation can
  share one arena id but remain distinct legal target choices. Retarget duplicate
  checks and fixed divided-damage shares must use the exact pair or target slot,
  not the arena id alone. The independent real-card matrix includes both Fork /
  Fireball and Fury / Vantress Visions to distinguish recomputed even damage
  from fixed shares.
- Bind chosen target versions before announcement costs or mana abilities can
  move them. Capturing only at resolution or the first later zone change leaves
  a gap in the casting procedure.
- Historical target descriptions belong to their own entitled projection.
  Drawing a target arrow to the current object, or assigning its current feature
  row in a trained policy, silently conflates two rules objects even when the
  action validator itself is exact.

- A capture call immediately before target legality can silently recreate a
  missing announced reference from the current object. Remove that late fallback
  and test the actual resolution path with a missing reference: only the original
  announcement/stack-publication binding can establish a legal target.
- View projection must preserve the first/second group flag at every reader.
  A copied first-group flag in the second-target iterator can display the first
  creature twice even when Engine legality handles both groups correctly.

- Know the rollback helper's scope before using it in a hash test. Engine's
  internal Held checkpoint does not save CastWizard. Restoring an intentionally
  mutated wizard through Held produced a false failure; save/restore that wizard
  explicitly, then separately exercise a real refused input without changing the
  expected hash or announced references.

### Self-effect incarnations — Rules gate accepted; live check pending

- Keep an effect's exact subject separate from its original damage source.
  A successful public-zone move by the effect or its costs can establish a
  successor; an unrelated blink cannot. Preserve that provenance across nested
  instructions and suspended choices, and hash retained state. Never infer a
  successor by adding one to a version number.
- Per-turn ability counts belong to an exact incarnation plus ability index.
  Audit all shared readers (activation limits and trigger counts), not just the
  card that exposed the defect. Historical counts may still be needed by an
  ability resolving after its source leaves.
- Distinguish sequential and stacked activations in Dragon Whelp tests. The
  official sequential five-activation example yields two delayed triggers;
  abilities held on the stack read the count at resolution. Old-incarnation
  delayed triggers can exist harmlessly: assert which creature survives rather
  than assuming the delay list must be empty.
- A Basalt Monolith animated on the turn it entered is summoning-sick. Tap it
  before animating when the test is about a later old untap ability, so the
  fixture does not fail before reaching the behavior under test.
- Share one broad final gate after focused tests. Preserve independent card
  review while avoiding duplicate broad builds and repetitive context handoffs.

- The broad gate caught Rancor: exact self-subject checks must still allow a
  self-departure trigger to find its recorded public destination (CR 400.7e).
  Apply this lookup only to instructions that refer to that destination, not
  to every damage source or pump. Real Eternal Witness retrieval followed by
  Patrol Hound discard is a negative control: the old trigger must not return
  that later graveyard incarnation.

### Personal Incarnation — accepted

- Owner and controller differ in activation, redirection choices, death life
  loss and Swords life gain. Tests must explicitly steal the creature and assert
  each role, rather than inferring all four from an unstolen case.
- A finite redirect is not prevention: preserve unpreventable damage and let
  the affected player allocate across simultaneous sources.
- Removing the last life point can eliminate a player and remove their objects.
  A test must not demand a graveyard object after that elimination.
- Replace obsolete no-ability placeholders with actual cast/body/ability tests
  when completing a card. A fixture-only correction needs a targeted rerun, not
  a second full suite when all other cases and production are unchanged.
- Printed translations may be older than Oracle. A live abilities check caught
  missing activation cost and finite amount despite correct English metadata.
  The owner approved the existing full-Oracle fallback and generic button label;
  no per-card shortened text was introduced. Future UI changes still require
  an explicit proposal and approval.
- Preserve the abilities dialog explicitly. Compact target selection is a
  separate surface; removing redundant target tools does not authorize removing
  the ability selection/confirmation flow.


### Temporary player actions — Channel / Guardian Angel

- A resolving spell may grant a reusable special action after leaving the stack.
  Preserve exact recipient references and source provenance independently; these
  permissions are not activated abilities and do not use the stack.
- Affordability may expose a legal cast using Channel, but life conversion must
  remain an explicit action. Preserve announced X/targets and enclosing debts
  across nested mana-ability payments. Defer ordinary triggers and state-based
  actions until the enclosing operation completes, even at zero life.
- Refuse the existing per-color u16 mana overflow atomically before taking life;
  disclose this general numeric ceiling rather than claiming unlimited capacity.
- Adding a keybinding must also exclude it from the historical keymap fixture
  used to recognize shipped legacy preferences. Test the actual old JSON.
- Full rules gate: 8,033 tests passed and one real legacy-keymap migration failed;
  after its narrow correction all 22 preference tests passed. Native final gate:
  1,267 passed, two existing ignores. A fixed expected settings action count also
  needed updating; the production settings already iterated the complete list.
- Reuse the bundled icon font and verify the glyph in its actual cmap. The
  approved temporary-action entry uses Font Awesome U+F254 with the existing OFL.
- For the remaining Alpha work, group cards by shared primitives, assign one owner
  to central Engine files, and parallelize disjoint implementation/review/client
  work. Serialize Cargo, not all work. Keep card flags Partial until independent
  rule tests and required live acceptance are complete.

### Alpha text, copy and constrained casting — integration findings

- Simultaneous questions need per-seat decision context. Using the globally
  pending seat hid another player's own hand and broke otherwise valid mulligan
  choices. Exercise agent, socket, snapshot and replay paths with two actual
  questions, rather than only constructing an isolated view.
- Copiable ability grants must participate in lint traversal and retain source
  provenance. Test a second-generation copy and a real dynamically sized token;
  copying a fixed test body cannot establish these properties.
- A projected legal hand choice does not prove the client opens its selector.
  Test receive-view/receive-choice routing and inspect the rendered payment
  receipt, selection, confirmation and decline paths.
- An embedded copied ability may have no standalone Oracle line. Preserve its
  matching source rules and use the full unchanged Oracle as fallback; do not
  invent shortened card text or silently display only the source card's name.
- Measure rollback changes on the same branch and end-to-end action benchmark.
  Reusing a scratch checkpoint improved an isolated clone measurement but slowed
  the actual four-pass benchmark; the experiment was reverted. The remaining
  checkpoint cost is an open performance issue, not a passed optimization.
- Current integration gate: 8,128 rules tests passed, ten skipped; 1,272 native
  tests passed, two skipped. Subsequent bounded fixes receive focused checks;
  these counts do not establish completion of Word of Command's route/undo work.

## 2026-10-11 — partial-card completion: graveyard permissions and affinity

- Treat Partial reasons as historical findings, not a current DSL capability
  list. Existing flashback, counted reductions, conditional statics, fear and
  counted prevention completed seven cards without new card-specific variants.
- Check every sentence when completing a card. Emry used flashback as an
  approximation; countering her artifact exposed the incorrect exile rider.
  Her permission now names one incarnation and allows an ordinary cast.
- A resolved characteristic grant fixes its set in every explicitly named
  zone (CR 611.2c). Past in Flames neither grants itself its cheaper printed
  mana cost nor grants flashback to cards milled after resolution.
- Setup mutations do not advance rules processing. Exercise conditional
  statics after a real priority action, rather than expecting refresh_offer
  (which only rebuilds legal actions) to perform that processing.

## 2026-10-11 — partial-card completion: combat, land cycles and Station

- Completed 23 more cards with shared primitives already present in the engine.
  Historical Partial comments in tests can be as stale as the card definitions;
  replace them with assertions about the printed rule, not the missing feature.
- Frantic Search chooses untapped lands during resolution, without targeting.
  A tapped land with shroud distinguishes that procedure from cast-time targets.
- Depletion counters prevent the untap before the upkeep trigger removes one.
  Exercise two actual turns for every member of the five-land cycle.
- Discount tests must include opponents’ matching permanents and your own
  nonmatching ones. Station tests pay real tap costs and reach twelve counters;
  sacrifice-powered draw must read last-known power after the creature leaves.
- Removing a blocker from combat leaves its attacker blocked. Test combat damage
  after removal, as well as the immediate combat membership and tap state.
- Validation: all 5,339 engine tests passed (two existing ignores), including
  combat removal, private hand inspection, chosen-color protection, graveyard
  casting/exile replacement, threshold discard and modal back faces.

## 2026-10-11 — partial-card completion: granted costs and token copies

- Completed twelve more cards. Granted activations must use the same cost
  wizard as printed activations: crew needs an actual power-checked selection,
  and an announced X must survive both payment and synthetic stack resolution.
- Entry counters are not copiable characteristics. Token-copy effects apply
  them to each created token through replacement processing; Doubling Season
  changes both token count and entry counters, while a subsequent copy inherits
  neither the original's nor the token's counters.
- Copy exceptions such as Adagia's legendary supertype are copiable. Test a
  second-generation copy and the resulting legend choice, not only the first
  token's characteristics.
- Oracle line matching must recognize crew granted by a type-changing sentence
  without quoted activation text, and reject a different printed crew number.
- Check transformation after the actual trigger or cost, including an attacker
  leaving combat in response, an optional zero-target choice, and a target that
  becomes illegal. These cases distinguish resolution instructions from an
  intervening-if condition or a state trigger.

## 2026-10-11 — partial-card completion: arithmetic and transformation thresholds

- Completed fourteen more cards with counted choices, ascend, land-play events,
  transformation thresholds, conditional mana, and arithmetic amounts.
- Arithmetic wrappers must preserve resolution context. Scaling or subtracting
  an amount that reads a target or paid cost cannot fall through to the static
  evaluator. Differential checks cover 40,000 values including overflow bounds.
- A land-play count belongs to the current turn, not the player's last turn.
  Reset every seat's count on turn advancement; River of Tears demonstrates
  the difference when untapped and activated during an opponent's turn.
- Playing a land and putting one onto the battlefield are different events.
  City of Traitors excludes its own play and ignores Horizon of Progress's
  tapped placement, which also does not consume a land drop.
- Test transform thresholds through the real ability and preserve object
  identity. Treasure Map with Doubling Season removes all four landmarks,
  retains unrelated counters, stays tapped, and creates six Treasures.
- The English-claim sweep must not treat the coefficient in "gain 2 life for
  each creature" as a fixed total. Keep its event-kind claim and leave the
  multiplier to a behavioral card test.

## 2026-10-11 — partial-card completion: copies, targeting, and delirium

- Completed nine further definitions using copy exceptions, spell-only target
  restrictions, self-shuffling resolution, and distinct graveyard card types.
- A cast offer must inspect its source as a spell on the stack, matching the
  later announcement. Project only that source; borrow the board and effects.
- Single-target predicates must include a singular chosen player even when the
  legacy representation does not duplicate it in the multiple-player set.
  A player-only spell is eligible for Specimen's trigger even though it cannot
  be redirected to that creature; a two-target spell is ineligible outright.
- Copy-entry tapped state belongs to the chosen replacement, not the copied
  characteristics. Declining Vesuva or Echoing Deeps leaves it untapped.
  Retained activated abilities must remain usable on second-generation copies.
- Delirium counts the union of types of actual cards in the controller's own
  graveyard. Multiple types on one card count separately; duplicate types and
  the opponent's graveyard do not. Test the ability offer and combat requirement.
- Test harness `refresh_offer` rebuilds choices, not conditional static effects.
  Advance a real priority action after changing a fixture's graveyard before
  asserting a conditional projected P/T or keyword.

## 2026-10-11 — partial-card completion: land Adventures

- A land front does not prevent casting its Adventure. Reject the normal land
  cast while still checking timing, targets and payment of castable back faces.
- Adventure permission belongs to the controller of the resolving spell, not
  necessarily its owner. Record that player, inspect every owner's exile zone
  for permissions, and retain the normal turn and land-drop limits. The rider
  expires on leaving exile; another independent permission can allow casting
  the Adventure again.
- Optional nonmana payment has two distinct outcomes: an unpaid fallback and
  a paid reward. Share the choice and payment path, and test inability to pay,
  declining an available payment, and actually paying it.
- Creating tokens and then placing counters is not entering with counters.
  Apply token multiplication first, then counter replacement independently to
  each created token. Do not modify the token's copiable printed 1/1 body.
- Half-library amounts use integer arithmetic and keep the chosen player's
  resolution context; differential checks cover odd, even and maximal values.
- Multi-target graveyard returns must move every legal target. Gather all owner
  destinations before asking commander replacement questions; selecting only
  the first target silently weakens cards such as Faith & Grief.
- Validation: 5,374 engine tests, card/codegen suites, DSL nested-effect walker,
  and all 3,321 cached card records. Keep new nested effects in the walker and
  its source-count guard, not merely in the resolver.

## 2026-10-11 — partial-card completion: flash and directional combat rules

- Casting as though a spell had flash is a player permission, not a keyword
  applied to the cards present when the effect resolves. Keep the affected set
  open so the card drawn by Borne Upon a Wind can use its permission. Type-scoped
  grants use a compact type mask; Teferi's prohibition still takes precedence.
- Blocking restrictions have a direction: Brazen Borrower's restriction applies
  to the blocker and reads the attacker. Share the pair scan with restrictions
  that instead apply to attackers and read their blockers.
- "Becomes blocked by" fires for each matching blocker and never when the source
  blocks. "Blocks" already exists; Elder Land Wurm's old Partial explanation was
  stale. Test permanent keyword loss across cleanup and reset on a zone change.

## 2026-10-11 — partial-card completion: counters, attachments and combat relations

- Enchanted means an Aura is attached, not merely that an object carries an
  attachment. Evaluate the current relationship again when a target resolves.
- Counter-based ability grants must accept any counter kind, restrict the
  controller, and still enforce summoning sickness. Adding zero counters must
  preserve the canonical empty counter set rather than allocate a zero entry.
- Filtered prevention has two scopes: all damage and combat damage only. Share
  matching, source identity and prevention ordering, with an explicit combat
  gate. Neither restriction grants protection or forbids targeting.
- Blocking relationships are directional. A Wall reads the creatures it blocks;
  The Wretched reads its own blockers. Bind the latter set when control changes,
  so combat cleanup does not erase the control effect.
- General activation restrictions use the same precollected effect list as
  Karn's opponent-only artifact restriction, preserving large-board behavior.
- Target offers and casting choices must both project the spell's chosen face,
  casting form and stack identity. A creature's Adventure must target as the
  instant it becomes; an Aura still in hand must obey Aura-spell restrictions.

## 2026-10-11 — partial-card completion: rampage

- Rampage is a triggered ability, with separate instances, not a static P/T
  multiplier. Emit one blocked transition independently from the individual
  blocker events. Count remaining blockers when the trigger resolves and
  freeze that bonus through cleanup.
- Temporary keyword grants reuse the granted-trigger engine. Rapid Fire asks
  whether a printed or granted rampage instance is active before adding one;
  losing abilities and ending a grant must change that answer.
- Choosing a keyword while an ability resolves (Gabriel Angelfire) is not
  announcing a modal ability. Keep the choice inside the resolution, after
  opponents have had priority, and walk every possible branch for coverage.

### Completion batch 10 — target sets and resolution costs (2026-10-11)

- Eleven more partial definitions now carry every printed clause: the five
  plural color spells, Disciple of Freyalise, Forsaken City, Restless Vents,
  Demolition Field, Ghost Quarter, and Gaea's Avenger.
- A plural continuous-effect instruction binds each surviving target to its
  incarnation. Do not reuse the existing singular `Filter::This` binding:
  that would silently modify only the first target. Test partial illegality,
  untouched bystanders, and cleanup with creatures of different colors.
- Resolution-time sacrifice payments retain last known characteristics on the
  resolving stack object, through the same `PaidRecord` read by existing
  `SacrificedPower` amounts. Declining payment must draw nothing.
- Hand exile is available to resolution-time cost choices without rerouting
  the casting wizard's already-paid pitch costs through activation payment.
- A characteristic-defining count with an offset stays in layer 7a and works
  outside the battlefield; a later +1/+1 modifier is not equivalent.
- Search destinations must match the card: these two land-destruction searches
  put basics onto the battlefield untapped, unlike Path to Exile's search.

### Completion batch 11 — commander and Adventure identity (2026-10-11)

- Forge of Heroes, Sanctum of Eternity, Cactus Preserve, Edgewall Inn,
  Jovial Evil, and Storm Seeker now include their previously missing clauses.
- Commanderness belongs to the designated object in any zone. Ownership,
  current control, and command-zone presence are independent questions:
  Sanctum must retrieve an owned commander an opponent controls; Cactus
  must count a commander in hand as well as the one in the command zone.
- Adventure eligibility follows the retained copiable face, including a
  Clone until it changes zones (CR 715.2b), not a name match. Public consumers
  leave these authoritative predicates unknown rather than guessing.
- Player-dependent counts use the resolution's chosen player and current
  board or hand, including at a table with three players. Recursive numeric
  wrappers must preserve that context instead of falling back to the caster.
- `bury` destroys battlefield permanents. For a hand/graveyard fixture,
  use the zone-movement door and assert the actual starting zone.

### Completion batch 12 — source identity, base toughness, and mana receipts (2026-10-11)

- Seven more cards now implement all printed abilities: Indestructible Aura,
  Life Matrix, Lesser Werewolf, Scavenging Ooze, Wall of Tombstones,
  Sentinel, and Priest of Yawgmoth.
- A targeted ability can modify its source independently of its target.
  Retain the source's incarnation for counter placement and continuous effects;
  a blink must not put the old ability's counters on a new permanent.
- Retain the target's types across exile, and the payment's sacrificed value
  across a mana ability. Immediate mana resolutions carry the small sacrifice
  receipt instead of leaving a stale PaidRecord on the source permanent.
- A base-toughness setter changes only layer 7b's toughness and freezes its
  value at resolution. Preserve negative target power before adding the
  printed offset; clamping -2 to zero would wrongly keep Sentinel alive.
- Named matrix counters share the central counter registry. The granted
  regeneration ability outlives the Matrix and consumes real counters;
  Doubling Season multiplies placement, never the removal cost.

### Completion batch 13 — filtered private library inspection (2026-10-11)

- Silundi Vision, Growing Rites of Itlimoc, Search for Azcanta, Monumental
  Henge, Lupinflower Village, and Memorial to Unity use an optional filtered
  pick, reveal only the selected card, and respect random versus chosen
  bottom ordering. Short libraries and empty eligible sets remain answerable.
- A list of legal picks is not the list of inspected cards. The resolving
  continuation retains all inspected cards; the host exposes that slice only
  to the choice's entitled seats. Client browser rows already distinguish
  visible from selectable cards. Test both the private view and forged picks.
- Search for Azcanta's current Oracle header explicitly says surveil. Keep
  that keyword event, test the graveyard threshold after the surveil, and
  ask the optional transformation before granting its land-face ability.

### Completion batch 14 — continuously bounded durations (2026-10-11)

- Complete Phyrexian Gremlins, Old Man of the Sea, Willow Satyr, Rubinia
  Soulsinger, Ice Floe, Tawnos's Weaponry, and Ashnod's Battle Gear through
  one duration that records its control and relative-power restrictions.
  Untapping, departure, and phasing end it immediately, including a change
  reversed before the next projection. An expired effect never resumes.
- Check duration conditions after characteristics settle, then reproject
  after removing expired effects. Old Man's control change can itself raise
  its recipient's power via an anthem. An effect that cannot start must not
  briefly change control before being removed. A derived table hint skips
  lifetime scans on boards without these effects; removed IDs are sorted.
- Ice Floe targets the actual attacked player, excluding another player's
  attackers and flying creatures at multiplayer tables. Keeping Floe tapped
  must prevent its target's next untap, not merely show a leave-tapped prompt.
- Urza's Avenger chooses a keyword on resolution and shrinks on every use,
  even when choosing an ability it already has. Inline labels apply to
  activated abilities as well as triggers. Osai Vultures places one carrion
  counter per qualifying end step, independently of the number of deaths;
  replacement effects multiply placement, not its two-counter activation cost.

### Completion batch 15 — graveyard choices after milling (2026-10-11)

- Port of Karfell, Tomb Fortress, Takenuma, and Sequestered Stash choose
  from the graveyard during resolution, after milling. Do not manufacture
  an activation target: the choice includes newly milled cards, and an empty
  graveyard cannot prevent activation. Keep optional and tapped riders distinct.
- A chosen commander's hand/library redirection suspends the same continuation.
  Resume its movement without repeating the mill or asking for another card;
  validate the remembered incarnation. Reject forged choices without mutation.
- Complete Terrain Generator, Hidden Hideout, Witch's Clinic, Urborg,
  Restless Spire, The Monumental Facade, and Wishclaw Talisman. Distinguish
  any counter from +1/+1 counters, any commander from your commander, and
  your-turn permission from sorcery timing. Spire's first strike follows its
  current controller's turn; oil removal is a cost, so replacements do not double it.

### Completion batch 16 — exact target unions and rule effects (2026-10-11)

- Complete Axelrod Gunnarson, Sunscorched Desert, Stensia Bloodhall, and
  Relic Bind with a player-or-filtered-permanent target union. The player
  side includes self and teammates; the object side excludes creatures and
  battles. A modal trigger chooses its mode before targets, not at resolution.
- Kry Shield and Al-abara's Carpet prevent noncombat damage too; Glyph of
  Destruction can use the existing unfiltered damage-to-object prevention.
  Preserve normal prevention ordering and source incarnation/LKI behavior.
- Endurance randomizes only the targeted graveyard's cards on the library
  bottom. Ask command-zone replacements before any movement or random draw.
  Its optional target also appears when the creature was evoked.
- Cursed Rack's maximum and Reliquary Tower's absence of a maximum follow
  timestamp order (CR 613.11). The latest applicable setting wins; leaving
  the battlefield exposes the earlier setting again, including at cleanup.
- Rainbow Vale schedules its control transfer for the next end step, with
  an untargeted opponent choice on resolution. Correct the shared transfer
  for Wishclaw Talisman at multiplayer tables and reject a new incarnation.
- Tests that need triggers must perform the actual tap ability: set_tapped
  updates status and projections, while its caller owns the journal event.

### Card completion batch 17 — original source zones, private selections, and X targets

- Completed the five Hidden Caves, The Gold Saucer, Trenzalore Clocktower,
  Volatile Fault, The Mycosynth Gardens, Talon Gates of Madara, Urban Retreat,
  Ominous Cemetery, and Fertile Thicket. Reused existing discover, coin-flip,
  copied-characteristic, and whole-hand shuffle instructions.
- Hand activations retain their source incarnation through resolution and use
  the normal permanent entry pipeline without spending a land play. A source
  that left and returned to the hand cannot be put onto the battlefield by
  its old ability.
- Optional filtered inspections now specify the selected card's destination.
  A selected commander going to a new zone asks its owner before movement;
  the continuation retains every selection and asks multiple owners without
  repeating the inspection. Printed reveals precede the replacement question
  and occur once. Putting a commander back atop the same library is no
  replacement event.
- An activation offer must check whether some X can produce legal targets.
  Its target menu and resolution recheck then use that activation's announced
  X, independently of any X the source permanent remembers from casting.
  Keep the original ability's text context when widening the preliminary
  filter; do not erase its other targeting or protection restrictions.
- Fixture lesson: repeated `Duel::battlefield` calls for the same seat replace
  the previous list. A global generated `CardIndex` also does not imply that
  the card belongs to this registry; Maskwood Nexus supplies a real Time Lord
  for Clocktower's test without inventing one outside the card pool.

### Card completion batch 18 — represented colors and event incarnations

- Complete Emiel the Blessed, Spirit Link, Vance's Blasting Cannons,
  Conqueror's Galleon, Matzalantli, Land Tax, Faeburrow Elder, and Tarnation
  Vista. Exercise both transform routes, upkeep conditions at triggering and
  resolution, optional hybrid payment, and source-controller life gain.
- Bloom Tender must produce one mana of each represented color, not a freely
  chosen combination of the same size. Share a five-bit union with counted
  color amounts; duplicates, multicolored permanents, and opponents' boards
  are distinct cases. Elder's bonus is layer 7c, not characteristic-defining
  base power/toughness. The current object uses its in-progress projection.
- Compare Land Tax's land count to each opponent separately, never their sum.
  Matzalantli counts distinct permanent types; its reverse counts permanent
  cards. An artifact creature contributes two types but one card.
- An implicit EventObject is still a particular incarnation. Counter effects
  must preserve the event's recorded version through nested branches and
  optional payment, rather than taking the current occupant's version when
  resolution starts. Emiel's blink regression caught this shared resolver
  defect. Public successors reached by the same resolution still work.
- Vance's nonland restriction is stricter than merely prohibiting land plays:
  a land with an Adventure must receive no permission. Ragavan retains its
  unrestricted cast-only permission. Test expiration at the next own main
  phase, where sorcery timing cannot accidentally mask a stale permission.
- Full engine coverage: 5,489 passed. Client, client-core, gamehost, card and
  DSL regression suites passed in the preceding broad run; the changed
  resolver and card paths were then rerun. The nested-effect carrier guard
  checks both new branches (38 fields across 33 carrier variants).

### Card completion batch 19 — total casting costs

- Complete Geist of Saint Thalia, Planar Gate, Mana Matrix, Eye of Ugin,
  and Hagra Mauling with shared generic-cost adjustments. Match the projected
  stack face/form; add increases before reductions and never remove colored
  requirements. Hagra checks each opponent separately, excluding teammates.
- Freeze the selected option's reduction before mana payments. Preserve credit
  left after the fixed generic portion so it can reduce X, extra targets,
  kicker, or replicate. Reconstruct the raw alternative cost, not the normal
  face's price. Spree modes form one total; two X symbols do not double a
  static discount. A monotone binary search bounds replicate payments.
- Eye of Ugin exposed Devoid being applied only on the battlefield. Its
  characteristic-defining color applies in every zone (CR 702.114a), including
  the projected spell used for pricing. Keep the printed layer effect for
  battlefield ordering against other color changes.
- Regression coverage: 5,499 engine tests passed, including Walking Ballista,
  Gloom, modal additional costs and a replicate discount exceeding the fixed
  price. Card, DSL, client-core and gamehost suites passed. All client tests
  passed after rerunning the sandbox-blocked loopback tests with local-network
  access. Generic-price differential coverage checks 13,824 integer cases.

### Card completion batch 20 — restricted mana for activated sources

- Complete Eldrazi Temple, Crucible of the Spirit Dragon, Secluded Courtyard,
  Primal Beyond, Base Camp, Gallifrey Council Chamber, Eclipsed Realms,
  Sunken Citadel and Castle Garenbrig. One payment-kind enum distinguishes
  spells, activated sources, and either. Ordinary payments remain excluded.
- Match an activated ability's actual source, including a land card being
  channeled from hand. The same admitted pool feeds legal offers and payment;
  failed payments leave both ordinary and restricted units untouched.
- Mana retains the producing ability's wording and chosen creature type.
  Reading the live land again was wrong after it left and returned with a
  different choice. The new regression spends Construct mana after Courtyard
  returns choosing Elf, and rejects using that mana for an Elf spell.
- Eclipsed Realms offers exactly its eight printed creature types. Invalid
  answers preserve the state. Starting battlefield fixtures also owe their
  as-it-enters questions after mulligans; answer these before passing priority.
- Existing suites passed: 5,499 engine, 1,563 client, 1,557 client-core, 244
  gamehost, 135 card, 275 codegen and 69 DSL tests. All 1,098 land tests then
  passed, including the three new adversarial cases spanning the nine cards.

### Card completion batch 21 — mana effects and exact source incarnations

- Complete Drowner of Truth, Corrupted Crossroads, Arcbound Ravager, Power
  Depot, Guildmages' Forum, Opal Palace, Study Hall and Hall of the Bandit
  Lord. Modular reads the dying permanent's counters from its captured
  incarnation, even after the same card returns with different counters.
- Devoid is an all-zone characteristic-defining keyword (CR 702.114a).
  This corrects batch 19's intermediate implementation: a second timestamped
  battlefield color-setting static was wrong, because later color effects
  must override the CDA regardless of the printed permanent's timestamp.
  A cast-time colorless-payment trigger reads the actual payment once; its
  Eldrazi Spawn still arrive if the creature spell is countered afterward.
- Apply mana effects after the paid spell enters the stack. Scry joins the
  normal trigger queue above that spell and carries the full count rather
  than clamping it to two. Its controller and source belong to the producing
  mana ability (CR 603.7e), including after pool transfer or source departure.
  Each paid unit produces its own effect or delayed trigger (CR 106.6a).
- Commander riders read that commander's cast count, including the present
  command-zone cast. Casting from hand uses its previous count; partners'
  counts remain independent. Entry counters are replacement effects and
  double normally. They are neither copied characteristics nor blink bonuses.
- Haste is a layer-six continuous effect, with normal timestamp ordering,
  which follows the permanent spell onto the battlefield (CR 400.7a).
  A permanent rider would apply at the wrong point in the layer system.
  Cavern's uncounterability also ends when that particular spell leaves.
- Regression suites passed with 1,563 client, 1,557 client-core, 244 gamehost,
  135 cards, 275 codegen and 69 DSL tests. The engine's existing 5,508 passing
  tests were supplemented by corrected mana fixtures, an updated independent
  Devoid projection expectation, and timestamp coverage: 5,512 effective
  passing engine tests. Workspace clippy includes all targets and dev-control.

### Card completion batch 22 — entry choices, graft and recipient-owned tokens

- Complete Eumidian Hatchery, Forbidden Orchard, Dowsing Dagger, Llanowar
  Reborn, Maze of Shadows, Rabbit Battery, Goblin Engineer, Fortified
  Beachhead and Temple of the Dragon Queen. Keep existing activation indices.
- Graft's formal rule (CR 702.58a) says another creature and has an intervening
  counter condition, although the printed reminder is abbreviated. Counter
  movement is all-or-nothing (CR 122.5); the recipient's doubling applies.
  Retain both original incarnations, including when a recipient returns and
  makes a second graft trigger. Counter movement reads its event's identity
  directly, independently of whether the ability has a target requirement.
- Reconfigure has two activated abilities, not one modal ability (CR 702.151a).
  The attach half targets another creature; detaching requires an attachment.
- Entry replacements may ask several questions. Queue each in order: revealing
  a Dragon must not consume Temple's separate color choice. Controlling the
  relevant subtype removes the tap penalty without forbidding optional reveal.
- Token production uses the chosen recipient's replacement effects. Orchard's
  mana is immediate, while its opponent-targeting token trigger uses the stack.
  Hatchery retains its departure counter count even if the land returns first.
- A may-search clause needs a separate permission choice: declining Engineer's
  search does not shuffle. Searching and finding no artifact does shuffle.
- Regression suites pass: 5,520 engine, 1,563 client, 1,557 client-core, 244
  gamehost, 134 cards, 275 codegen and 69 DSL tests. Additional graft coverage
  tests a creature that leaves and returns before the old trigger resolves.

### Card completion batch 23 — per-turn history and damage recipients

- Complete Renegade Rallier, Minas Tirith, Hall of Oracles, Whirling Dervish,
  Cavernous Maw, The Seedcore, Pit Scorpion and Backfire. Historical conditions
  record successful actions, rather than guessing from the current board.
- Revolt records the controller immediately before departure. Casting an instant
  or sorcery counts even when it is countered. Attacking counts distinct creature
  incarnations per declaring player, including creatures which subsequently leave.
  Repeated attacks by one creature count once. Every fact resets with the turn.
- Source damage history retains the exact incarnation and actual recipient after
  prevention. The end-step condition checks the current controller's opponents.
  Poison from an ability is separate from combat damage and from permanent counters.
- Snapshot and loop hashes include the facts which change legal actions. Canonical
  attack hashing deduplicates before replacing identities with zone positions,
  so distinct departed attackers cannot collapse into a single nonexistent object.
- A damage trigger which refers to the damaging creature must retain that event
  object, its incarnation and the damage amount. Backfire reads its live controller
  on resolution, or its last controller when that incarnation has left. Tests
  cover both control changes and a card leaving and returning before resolution.
- Broad regression suites passed with 5,528 engine tests before the Backfire fix;
  its four-case regression then passed, giving 5,529 effective passing engine tests.
  Also passing: 1,563 client, 1,557 client-core, 244 gamehost, 134 cards, 275 codegen
  and 69 DSL tests. The arithmetic property test additionally checks summed counts.

### Card completion batch 24 — stack predicates and event-time graveyard observers

- Complete Rust, Ayesha Tanaka, Avoid Fate, Ring of Immortals, Winter Blast,
  Haunted Fengraf and Sword of the Meek. Ability counters distinguish activated
  abilities from triggers, and evaluate the source's current characteristics or
  its exact departed incarnation. Granted activations retain their ability kind.
- A spell targeting your permanent reads the current target identity and controller
  (CR 115.9b). A target becoming illegal does not stop being a target, but a card
  leaving and returning is a different object. Both target groups participate.
- Sword's return and attachment retain separate identities. Untargeted attachment
  ignores shroud; a departed creature or Sword is never replaced by its new
  incarnation. Graveyard observers only see events during their own stay there,
  including an event before they subsequently leave in the same mutation batch.
- Winter Blast damages only flying creatures among its still-legal chosen targets.
  Fengraf samples the current eligible graveyard cards with the seeded game RNG;
  the selected card is retained across a command-zone replacement question.
- All 5,538 engine tests pass, with two intentionally ignored tests. Regression
  coverage includes changed controllers, departed activated sources, triggers
  excluded from activated-ability counters, source blink, untargeted shroud,
  event ordering, paid/refused white mana, and deterministic random selection.

### Card completion batch 25 — token choices and private top-card inspection

- Complete Marionette Apprentice, Nettlecyst, Mirrex, Serpent Generator,
  Hellion Crucible, Scrying Sheets, Bucolic Ranch, The Biblioplex and Command
  Beacon. Central token definitions retain stable ids and verified printings.
  The colorless artifact Snake uses an explicit no-printing fallback rather
  than displaying another token's colors, types or rules.
- Fabricate is an optional counter placement on resolution, not an announced
  mode (CR 702.123a). If its permanent has left, create the Servos instead.
  Token and counter replacements apply independently to their respective branch.
- Living weapon creates its Germ even after its Equipment leaves. A token
  doubler requires choosing one new Germ for attachment before state-based
  actions kill the others. Ordinary creature Equipment cannot attach; the
  explicit reconfigure effect carries that keyword's exception (CR 702.151a-b).
- The three top-card lands share a private inspection, filtered optional reveal,
  and optional disposition. Leaving a card or reordering it within the library
  preserves its incarnation. Biblioplex's hand-size restriction is checked at
  activation, not again at resolution. Snow payment still requires snow mana.
- Command Beacon chooses one commander without targeting, then retains that
  choice through the command-zone replacement. Keeping an already present
  commander in that zone is not a zone change and does not change its version.
- The full engine run passed 5,546 tests; its sole outdated equip test selected
  two creatures instead of one. Correcting that test and rerunning it gives
  5,547 effective passing engine tests, with two intentionally ignored tests.
  All 134 card, 275 codegen and 69 DSL tests pass, as do workspace clippy and
  the printing/convention validation. The new card scenarios cover doubling,
  removal before resolution, combat-only toxic, noncombat poison triggers,
  illegal blockers, privacy, both optional choices and commander replacement.

### Card completion batch 26 — precise returns and typed token multiplication

- Complete Malakir Rebirth, Journey to Eternity, Ojer Taq and Lazotep Quarry.
  The separate reanimation module was moved unchanged in the preceding commit.
  Returning a dying source uses its recorded graveyard successor, never a card
  that left and returned there while the ability waited (CR 400.7e-f).
- Set the entering face before entry replacements and trigger observation.
  Entering transformed does not generate a transform event; cards outside the
  battlefield and stack regain their front characteristics (CR 712.8a, 712.14a).
  Under the current rules modal double-faced cards can also transform: do not
  infer a prohibition merely from their back being independently playable.
  Single-faced copies of a God cannot return transformed. Tokens cannot return.
- Ojer Axonil and Ojer Pakpatiq now use the same direct, tapped death return.
  Their separate unfinished clauses remain marked partial. Exile-and-return
  effects require the precise current battlefield source, including phasing.
- Token multiplication reads the recipient and the resulting token's types,
  including copy exceptions. Multiplication overflow stops with an explicit
  numeric-domain failure before allocating tokens. Multipliers compose normally.
- Copy exceptions replace defining abilities as well as characteristics
  (CR 707.9d); replacing creature types removes changeling and keeps unrelated
  subtypes. Token copies now use the existing composable rules-text machinery.
- X-dependent target enumeration also supports graveyard cards, preserving the
  zone and player restriction before X is chosen and enforcing the actual value
  afterwards. The quarry regression covers payment, sacrifice, invalidated
  targets, retained mana abilities, copy exceptions and Doubling Season.
- All 5,555 engine tests pass (two intentionally ignored), including new tests
  for stale source identities, owner control, direct graveyard return, modal
  double-faced copies, transform timing, copied defining abilities, typed token
  multiplication and overflow. Workspace clippy passes with dev controls.
- Regenerated the ability-line table and verified all 134 card, 275 codegen
  and 69 DSL tests. Printing/header validation covers all 3,321 cards.

### Card completion batch 27 — actual turns and next-upkeep effects

- Complete Starting Town, Halfdane, Hazezon Tamar and Giant Slug. Count each
  player's turns at the actual begin-turn boundary: extra turns count and
  skipped turns do not. Include that state in snapshot identity. Starting Town
  always enters tapped during another player's turn.
- A next-upkeep delayed trigger retains its original controller and exact source
  incarnation (CR 603.7b-e). Hazezon counts lands when that delayed trigger
  resolves and still creates tokens if Hazezon left earlier. Giant Slug asks
  for a basic land type at that later resolution, independently per activation;
  a returning Slug is a new object and receives no stale keyword grant.
- Halfdane copies signed, current power and toughness, with a deadline staged
  at the next upkeep's beginning and expired at its end. Effects created during
  the current upkeep survive its end; losing the next target does not shorten
  the previous effect. Other players' upkeeps do not consume this deadline.
- The first broad run passed all 5,562 behavioral tests, with two intentional
  ignores; only the phasing audit's historical 120-source-file ceiling failed
  after the new resolution module raised the count to 121. Retain its minimum
  traversal sanity check and actual battlefield-walk audit, without a maximum
  file count that penalizes splitting large modules.
- The 70 DSL tests, 275 codegen tests and nested-effect carrier guard pass.
  Workspace clippy passes with dev controls. The four card scenarios include
  extra/skipped turns, invalid choices, negative power, target removal, source
  reincarnation, token doubling and delayed resolution after departure.
- After removing that unrelated file-count ceiling, all 15 phasing checks pass:
  5,563 effective passing engine tests. All 134 card tests and the 3,321-card
  printing/header validation pass after regenerating the ability-line table.

### Card completion batch 28 — public information and resolution color choices

- Complete Field of Dreams, Revelation, Dream Coat and Alchor's Tomb. The
  color choice happens during resolution: all 31 nonempty combinations for
  Dream Coat, and five single colors for the Tomb. Both replace colors
  indefinitely. An Aura's departed ability retains its attachment's exact
  last-known identity; returning hosts never inherit stale changes.
- Reveal all library tops and hands through rules modifiers. Public hand
  visibility survives optional teammate sharing and reaches spectators and
  agents. A defaulted disclosure flag lets the seat bridge strip voluntary
  shares while keeping public knowledge. Ending a reveal removes those
  cards from the next view; it does not erase a still-valid teammate share.
- Shared/public hands were never exposed by the client's zone browser.
  Add a small browser module with localized names, original-name search,
  previews and printed facts. Merely seeing a card never grants control.
  Scan the reveal effects once per view, before copying any hand cards.
- The phasing regression caught a stale visibility query between phasing out
  and rebuilding statics. Test the source's phased-out status directly, as
  well as its battlefield presence. The two new phasing tests now pass;
  together with the broad run this yields 5,570 passing engine tests (two
  intentional ignores), including all color combinations and once-per-turn
  limits. The existing Courser of Kruphix scenario still passes.
- Broad checks passed 1,559 client-core tests and 247 host tests. After the
  browser cleanup and translation fix, all 58 browser tests, the choice and
  control regressions, eight public-view tests and 40 view tests pass. Card,
  DSL and codegen suites pass (134/70/275); the nested-effect guard and the
  3,321-card printing/header validation also pass. Workspace clippy passes
  with dev controls. Bump VIEW_VERSION to 58 for the new choice variant and
  pin the updated wire shape; older sharing frames default to private.
- Keep the same package selection, including baylee-seat, in following test
  builds. Adding its test dependencies changed feature unification and rebuilt
  Bevy once (2m43s); subsequent builds reused dependencies (33–43s).

### Card completion batch 29 — exile play windows and prospective spell faces

- Complete Urza, Lord High Artificer, Cori Mountain Monastery and Nivix, Aerie
  of the Firemind using one exile/play operation. Permissions retain the exact
  card incarnation and expire either this cleanup, at the next actual own turn
  start, or in that next turn's cleanup. Skipped turns do not consume a window;
  extra turns do. Both snapshot hashing and diagnostic fingerprints include
  the permissions separately from per-turn counters.
- A mixed spell/land card had its cast offer suppressed whenever its land face
  was playable. Offer the two independently. Nivix restricts the prospective
  spell face, allowing an instant Adventure without permitting its creature
  front. Urza offers both castable faces with zero mana cost and correct taxes;
  a single legal back face stays that face instead of defaulting to Normal.
- Restricted mana and spend riders previously inspected the front face during
  some back-face casts. Share a prospective-object reader between these paths,
  borrowing normal objects and projecting only alternate forms. Both the paid
  and waived-cost probes now use the same face as the actual payment.
- The full regression run caught cascade calling the priority-time free-cast
  entry point. Route cascade through the existing resolution-time entry point;
  its creature cast remains legal while the cascade and original spell occupy
  the stack. Keep alternate-face projection out of the common mana path when
  the pool contains no restricted or rider-bearing mana.
- All 5,577 engine tests pass (two intentional ignores), including printed card
  scenarios, extra/skipped turn boundaries, source departure, incarnation
  changes, dual land/spell offers, Adventures, free-cast taxes and the existing
  cascade regressions. Card/DSL/codegen suites pass (134/70/275); workspace
  clippy passes with dev controls. The generated ability table has 4,461 lines.
- The nested-effect guard and all 3,321 printing/header validations pass.
  Register the new operation in the optional-clause validator: its permission
  is optional because the player may later play the card or leave it exiled,
  not because exiling itself prompts for confirmation. Format and xtask lints
  pass after that validator update.

### Card completion batch 30 — Birgi/Harnfel and boast

- Before changing behavior, split the 3,175-line activated-ability module into
  eight focused modules and a 158-line shared root. All 33 moved methods were
  compared to the previous commit with only visibility, module-path and format
  normalization. The workspace/all-targets check passed before the separate
  refactor commit.
- Complete both faces of Birgi/Harnfel. The front's red mana uses the existing
  turn-retention flags. Expose them through an authoring verb that preserves
  spending restrictions and riders regardless of chaining order. Retention
  follows the produced mana, not the later presence of its source.
- Boast has a typed activation limit: exact-incarnation attack history plus a
  per-ability activation tally. Birgi raises the cap to two during your own
  turns, with the current matching rules effect; it never grants an attack or
  resets the tally on a control change. Multiple matching effects still give
  a cap of two. Harnfel discards as a cost, exiles up to two available cards,
  and grants normal play permissions for the rest of the turn.
- All 5,582 engine tests pass (two intentional ignores), as do the card,
  DSL and codegen suites (134/71/275), the nested-effect guard, all 3,321
  card validations, formatting and workspace/all-targets clippy with dev
  controls. Update the explicitly pinned back-face catalog for Harnfel and
  remove its obsolete module-level claim that only one back reaches the stack.

### Card completion batch 31 — land characteristics and dynamic mana

- Complete six related lands with shared rules: every (nonbasic) land type,
  automatic phasing, mana selected from current colors in a specified zone,
  and preparing a targeted creature. Preserve existing ability indices.
- Represent the two self-defining land-type sentences as copiable rule bits,
  like the existing changeling/devoid markers. Read the generated subtype
  bitmap at construction in every zone; ordinary type effects still replace
  those types. Never hard-code a list of currently known nonbasic land types.
- Capture phasing-in and phasing-out sets before either transition, and phase
  out the entire set together so attached permanents keep indirect phasing.
  A returning land must not immediately phase out again in the same untap.
- Share the existing mana menu path with host/client presentation. A colorless
  matching object contributes no color. Reflecting Pool must see the new
  dynamic source colors too, without treating the color of a creature itself
  as a land's production. Remove temporary zone-index clones from counted
  amounts and share a borrowed zone visitor with the new color query.
- Preparation reads the alternative spell's presence even when ordinary
  abilities are suppressed, and never marks an ordinary creature prepared.
- The complete engine run passes 5,588 tests; its remaining mana-oracle
  sweep passes after teaching the independent printed-text reader Rangeling
  (5,589 passing tests total, two intentional ignores). Card/DSL/codegen
  suites pass (134/71/275), alongside 29 native mana-source tests, nine host
  mana projections, the nested-effect guard, all 3,321 printing validations,
  workspace/all-targets clippy with dev controls, and formatting.
- The broad sweeps must understand self-defining type sentences, rather than
  assuming every subtype is physically printed on the type line. Keep the
  new intrinsic mana route in the actual five-color sweep, not a skip bucket.
- Follow-up found during the next mana review: the pre-existing preparation
  implementation still needs the current CR 722.3c copy lifetime and ordinary
  cast/payment flow. It currently reads a live printed face at casting time,
  so copied preparation creatures and restrictions need shared engine work.

### Preparation correction before batch 32 (2026-10-11)

- CR 722.3c creates the prepare spell's copy in exile when the permanent
  becomes prepared, not when its controller later casts. Preserve that
  independent copy across turn changes and changes to the source's copiable
  values. A source version, designation and phasing state bound its lifetime;
  the permission follows the source's current controller, not the copy's owner.
- Reuse ordinary exile casting and the complete casting wizard. Targets,
  cancellation, cost adjustments, restricted mana, spend riders and cast
  triggers must not acquire a separate preparation implementation. Consume
  the designation only at the successful cast. Re-preparing an already
  prepared permanent never replenishes a removed copy.
- The immutable rules cache must admit linked spell definitions even when no
  deck contains those cards. Admission follows the registry seam and handles
  self-links/cycles without recursion or duplicate work.
- Preparation metadata needs both card and face. Fatehold Chronologist's
  inset Peer Review was incorrectly castable from hand, and its preparation
  referred back to the creature. The physical card now casts only the
  creature; its exiled copy has only Peer Review's characteristics and price.
  Copies of prepare spells retain that spell identity too.
- Carry the frozen prepare spell into the public view, rather than deriving
  it again from the source's current rules. Prepared and unprepared twins,
  or twins with different exiled spells, must not collapse into one group.
  The optional defaulted field is wire-compatible (view version 58).
- Expire permissions with one stable retain pass, reusing the vector. Removing
  each expired element individually shifts the remaining entries repeatedly
  when many copies expire together.
- Validation: 5,597 engine tests pass (two intentional ignores), plus
  134 card, 71 DSL and 275 codegen tests. After the final lifetime/copy edits,
  eight preparation and twelve spell-copy regressions pass; host view tests
  pass 120, native ability labels 18, client core 1,560 (one ignore), and the
  view suite 42. Codegen tables, all 3,321 printing validations, formatting
  and workspace/all-targets clippy with dev controls pass. The added view
  field is explicitly checked for decoding old JSON and for grouping.

### Partial-card completion batch 32 (2026-10-11)

- Completed Hall of Tagsin, The Great Mound, Junktown and Sokenzan, Crucible
  of Defiance through shared token entry and mana-payment rules. Powerstone,
  Vibranium, Junk and the ordinary colorless Spirit have central, append-only
  token identities and verified Scryfall references.
- Temporary haste is an independent layer effect on each resulting token,
  including replacement-created copies. It is neither a printed keyword nor
  part of the token's copiable values. Set entry tapping through the central
  state setter before the entry event, preserving the layer invalidation
  invariant instead of allowing a second status-bit writer.
- Powerstone mana restricts spells, not abilities or other payments. Admission
  must reach both offer and payment, including the resolution-tax reader.
  Preparation uses the same casting path and cannot evade this restriction.
- Word of Command reserves exact mana units for the instructed cast or eligible
  land mana abilities. Unrelated payments neither spend nor debit that reserve,
  even when free and reserved units have the same color. Construct admission
  directly from the free pool rather than matching every old entry against a
  reduced pool: that avoids repeated scans and prevents fragmented provenance
  entries from duplicating the available amount.
- The live artifact-type catalog omits four types already present in CR 205.3g.
  Supplement them in the catalog reader, not in the cached API response. The
  focused `codegen --subtypes` path preserves all 507 existing constant IDs and
  appends four; ordinary table codegen appends the four new tokens.
- Validation: all 5,608 engine tests pass (two intentional ignores), including
  a 131,170-unit fragmented-reserve case; 134 card, 71 DSL and 275 codegen
  tests pass. All 3,321 printing validations, focused table/subtype
  reproducibility checks, formatting and workspace/all-targets clippy with
  dev controls pass. Original Partial inventory: 258 completed, 282 remaining.

### Partial-card completion batch 33 (2026-10-11)

- Completed the five Roads lands and Realm of Koh with two centrally defined
  tokens. Pilot power changes only while paying to crew a Vehicle or saddle a
  Mount. It does not change combat power, and losing the ability removes the
  bonus. Koh Spirits check current creature types in both directions of combat.
- Crew and saddle reuse a single effect index per power-selection menu. Sum
  signed contributions in a wider integer before clamping the public weight;
  feasibility needs only positive contributions and stops at the threshold.
  A differential test compares indexed results against a complete effect scan
  over overlapping bonuses, negative powers and all 256 cost thresholds.
- An activation's cost answers are groups, not a flat list of objects. A
  zero-card answer still answers one question; a many-card answer cannot skip
  the next question. Preserve these groups in rollback checkpoints and nested
  mana-payment continuations. The payer checks each group's power separately.
- Saddle is sorcery-speed. The designation belongs to the exact permanent
  incarnation, is not copiable, and ends on departure or cleanup. Fresh Pilots
  may pay crew/saddle without haste because these costs do not use their own
  tap-symbol abilities. The player and AI prompts name saddle explicitly;
  view version 59 identifies the added choice-prompt variant.
- Broader narrator tests exposed missing names for phasing, rangeling, devoid
  and the all-nonbasic-land-types marker. Supply the real names instead of the
  generic unknown-keyword fallback.
- Validation: the engine run passes 5,616 tests with one stale modifier-count
  assertion; after correcting that count, all 13 effect-catalog tests pass,
  covering all 5,617 engine tests (two intentional ignores). The seven new
  crew/saddle mechanic tests, nine Roads/Koh card tests, 1,561 client-core
  tests, 248 host tests, 42 view tests, 134 card tests, 71 DSL tests and 275
  codegen tests pass. The seat suite passes its 222 other tests with loopback
  allowed, and all 36 narrator tests pass after completing its keyword names.
  All 3,321 printing validations pass. Original Partial inventory: 264
  completed, 276 remaining.
  Focused table reproducibility, formatting and workspace/all-targets clippy
  with dev controls also pass after the final corrections.

### Partial-card completion batch 34 (2026-10-11)

- Completed Ur-Drago, Lord Magnus and Detection Tower with scoped as-though
  permissions. Preserve landwalk and hexproof on the affected objects: the
  permission changes blocking or targeting, not the object's abilities.
- Tower's permission belongs to its activating controller and covers opposing
  players and creatures, including creatures or hexproof arriving later. It
  neither bypasses shroud nor opens hexproof noncreature permanents. Its effect
  survives the land and ends at cleanup. Text changes map the named landwalks.
- Player hexproof now excludes only opponents' targeting: the protected player
  and teammates retain permission. Collect player protections and the caster's
  permission in one effect pass instead of scanning effects for each player.
- Validation: all 5,620 engine tests pass (two intentional ignores), plus 134
  card, 71 DSL and 275 codegen tests. After extracting the blocking helper,
  the 38 combat tests and four Tower/landwalk regressions pass again. All
  3,321 printing validations, table reproducibility, formatting and workspace
  all-targets clippy with dev controls pass. Original Partial inventory: 267
  completed, 273 remaining.

### Partial-card completion batch 35 (2026-10-11)

- Completed Tyrite Sanctum, Minas Morgul, Dark Fortress, Luminous Broodmoth,
  Venarian Gold, Cocoon, Dwalin, Weaponmaster, Blast Zone and Rasputin
  Dreamweaver through shared counter rules rather than card-name branches.
- Keyword counters participate in the normal ability layer and its object
  index. Placing another counter of that kind refreshes the kind's timestamp;
  partial removal does not. Source departure, ability loss and reincarnation
  regressions cover flying, shadow, indestructible and the existing lifelink.
- Hone is an intrinsic counter rule, not a printed Equipment ability. Its
  power bonus follows the current attachment, requires the Equipment type,
  survives ability loss, and disappears when the last hone counter is removed.
- Attached-counter and attached-modifier effects retain the exact host after
  their Aura leaves. Venarian Gold uses announced X; Cocoon hatches on the
  upkeep after its last pupa counter was removed, including its own sacrifice.
- Counter caps apply after doubling replacements and also through state-based
  actions when a lost cap ability returns. Multiple caps remove only the
  strictest excess. A counter-kind cache avoids the extra rules scan on normal
  boards. Rasputin's start-of-turn condition uses one private status bit,
  recorded before untapping, cleared on departure, and omitted from public
  status. The object footprint remains unchanged.
- Counter-effect expiry now filters effects once against the arena instead of
  building expired-id lists and repeatedly searching them. Large placement
  amounts saturate at the counter-storage bound rather than wrapping.
- Added concrete AI valuations for source-specific changes, untapping and
  combat regeneration. Keyword-counter value does not multiply with duplicate
  counters or award anything for zero counters. The activation-coverage floor
  stays intact; completed Probe and Borne Upon a Wind leave the cantrip-only
  counterspell whitelist as their additional effects now warrant.
- Validation: all 5,633 engine tests pass (two intentional ignores); the final split
  of the claim-parser regression adds a passing fourth focused claim test
  for 5,634 non-ignored engine tests, plus 234
  AI, 1,561 client-core (one intentional ignore), 248 game-host (two intentional
  ignores), 223 seat, 134 card, 71 DSL, 275 codegen and 42 view tests. Both
  object-footprint guards pass. New rules include counter timestamp order,
  reattachment/type loss, Aura departure, sacrifice-cost LKI, counter caps with
  doubling/ability recovery, and pre-untap history. Wire version 60 records the
  added counter variants. Original Partial inventory: 275 completed, 265
  remaining.
  All 3,321 printing validations, table reproducibility, formatting and
  workspace/all-targets clippy with dev controls pass.

### Partial-card completion batch 36 (2026-10-11)

- Completed Mikaeus, the Unhallowed, Nim Deathmantle and Borg Queen, Perfection
  Manifest. First moved the existing return-keyword collector into its own
  module without changing behavior, in a separate tested commit.
- Death events retain independent undying/persist counts, while the actual
  trigger reads that departure's source incarnation and counters. Printed and
  granted instances trigger separately; only the original graveyard object
  can return. Simultaneous deaths preserve Mikaeus's grant, prior departure or
  ability removal does not, and tokens trigger without illegally returning.
- Characteristic effects whose battlefield source has left stop applying
  immediately within a resolution. Retained grants still serve the explicit
  pre-departure trigger lookback. This distinction fixes the sequential-death
  regression without losing simultaneously dying granted triggers.
- Intimidate now restricts blocks through shared current colors or the
  artifact-creature exception. Its 2,048-case test covers all 32 by 32 color
  sets with both artifact states, including colorless attackers and blockers.
- Nim Deathmantle sets black and Zombie rather than merely adding them, grants
  intimidate, asks for the real four-mana payment, returns the owned death card
  and attaches to its successful successor. Equipment departure does not stop
  reanimation; an intervening exile/graveyard round trip cannot substitute a
  different incarnation. Event-object instructions share the identity check.
- Assimilate is one complete keyword action. Its artifact/creature and Borg
  effects are registered for the entering incarnation before the move, so
  CR 611.2e applies. Counters use the replacement-aware return path; types and
  counters persist after the Queen leaves. Failed entry discards staged effects.
- Validation: all 5,643 engine tests pass (two intentional ignores), including
  the new card interactions, duplicate keywords, token return, source loss and
  intimidate matrix; 234 AI, 134 card, 71 DSL and 275 codegen tests pass.
  All 3,321 printing validations, table reproducibility, formatting and
  workspace/all-targets clippy with dev controls pass. Original Partial
  inventory: 278 completed, 262 remaining.

### Partial completion batch 37 — announced damage distributions

- Completed Fire // Ice, Pyrotechnics and Shatterskull Smashing together.
  First extracted the trigger-division code unchanged into its own module
  and committed the move after 14 Fury-related tests passed.
- Casts now choose all targets and positive damage shares before paying
  costs. Fixed totals and threshold-doubled X use one announcement type;
  X = 0/1 bounds the number of targets and zero targets never open a prompt.
  The doubled Shatterskull total is divided directly, permitting odd shares.
- Retain exact object-or-player targets through payment, resolution and
  copies. Retargeting moves each unchanged share with its original slot,
  including object/player swaps; illegal or blinked targets lose their
  damage instead of giving it to another target. Pending allocations enter
  replay fingerprints and cancelled casts leave no allocation behind.
- Updated the AI's target and number decisions, narrator and shared client
  prompt. Wire version 61 reflects the new recipient shape. Do not assume
  an enum imported from the engine changes the view's own shape fingerprint.
- The generic card-claim sweep joins both faces' Oracle text. One cast
  button is not a promise to perform every face: Fire need not draw Ice's
  card. The presence check requires a single face; numeric checks remain,
  and a negative single-face test still catches a missing promised draw.
- Validation: all 5,653 engine tests pass (two intentional ignores), including
  every positive composition of four, mixed recipients, X boundaries,
  cancellation, illegal answers, replay distinction, copies and retargeting.
  Also passed: 235 AI, 134 cards, 72 DSL, 275 codegen, 42 view, 1,561
  client-core (one ignored), 248 host (two ignored), and 223 seat tests;
  all 3,321 printing validations, table reproducibility, formatting and
  workspace/all-targets clippy with dev controls. Original Partial inventory:
  281 completed, 259 remaining.

### Partial completion batch 38 — storm, spell copies and magecraft

- Completed Brain Freeze, Flusterstorm and Storm-Kiln Artist. First moved
  copying into its own module without behavior changes and committed the
  extraction after 128 copy/replicate tests passed.
- Storm freezes the number of earlier casts across all players. A reverse
  pass over the current event batch avoids repeatedly scanning the turn's
  journal. Copies emit a separate event for magecraft; they never count as
  casts, retrigger storm or increment per-turn spell totals.
- Storm and replicate now share a counted instruction with constant program
  size rather than a repeated opcode slice. All target-choice continuations
  must use the same program-advance helper; one forgotten continuation had
  stopped after the first copy, caught by the multi-copy tests.
- Copy from the trigger's exact source version, including when the original
  left the stack before the trigger was stacked. The implicit target may
  already refer to a newer incarnation. Preserve X and announcement choices,
  and allow each copy's targets to change separately.
- Three historical tests deliberately documented missing storm/magecraft;
  updated their expectations to full rules behavior rather than removing them.
  Added zero/one/multiple/257-copy, both-player counts, countered original,
  removed trigger, independent tax, magecraft and untargeted-copy regressions.
- Validation: 5,659 engine tests pass (two intentional ignores), 235 AI,
  134 cards, 72 DSL, 275 codegen and 42 view tests pass. All 3,321 printing
  validations, generated-table reproducibility, formatting and workspace
  all-targets clippy with dev controls pass. Original Partial inventory:
  284 completed, 256 remaining.

### Partial completion batch 39 — discard provenance, connive and recruit

- Completed Nephalia Academy, Lake-town Lookout, Aclazotz, Ledger Shredder,
  Spymaster's Vault, Villainous Hideout, Psychic Purge, Gamble and Coral Helm.
  The remaining 256 original Partial cards were assigned exactly once to
  30 mechanism-based packages in `partial-card-batches.json` (39 through 68).
- Discard events retain the hand incarnation, causing controller and whether
  the card's characteristics are known. Academy's revealed replacement and
  Library of Leng's hidden replacement must differ, including when both are
  available. Cost discards are events but do not gain effect-only replacements.
  Psychic Purge uses its hand ability and the actual causing opponent.
- Connive and recruit share a counted continuation: draw first, choose all
  discards before moving any, apply discard replacements, and reward known
  nonlands. Menus are chunked without imposing a 255-card rules limit. Source
  departure retains its controller but cannot place counters on a later copy.
- Multi-player discard instructions gather choices in APNAP order before
  moving cards simultaneously. Freeze empty-hand rewards at instruction start.
  The cast event now freezes the per-player spell ordinal, so Shredder does
  not read a later total when events are processed together.
- Gamble delays its search shuffle until after the random discard. Coral Helm
  has a seeded, atomic random discard cost; affordability checks consume no
  random numbers. This cost is currently supported for activations only and
  guarded against use as a spell's additional or alternative cost.
- Cross-crate changes need full enum checks before test builds. Imported
  engine prompts require a wire version bump even when the view's local
  structural fingerprint stays unchanged. Update both the version and its
  recorded fixture. The extended seat suite also caught the previous batch's
  missing Storm narrator label.
- Validation: 5,675 engine (two ignored), 1,562 client-core (one ignored),
  248 host (two ignored), 235 AI, 134 cards, 72 DSL, 275 codegen, 42 view,
  223 seat and 59 xtask tests pass; the DSL contract integration test passes.
  All 3,321 printing validations, generated-table reproducibility, formatting
  and workspace/all-targets clippy with dev controls pass. Original Partial
  inventory: 293 completed, 247 remaining.

### Partial completion batch 40 — counter foundation (package in progress)

- Centralized player energy/radiation changes and exact energy payments. Poison
  effects and toxic use the same journalled player-counter path. Energy gained
  by a player is unaffected by Doubling Season's permanent-only replacement.
- Proliferate collects all permanent/player choices before updating anything;
  pages of 255 are a presentation limit, not a rules limit. A 257-permanent
  regression guards that distinction. Selection is not targeting, so shroud
  does not exclude a permanent. Every kind on a chosen recipient increases.
- Counter events retain the placing player, the recipient's incarnation and
  whether this was its first actual placement of that kind this turn. Avoid
  interpreting that last clause as once-per-turn on the ability: an earlier
  placement without the ability still consumes "first". Hapatra also reacts
  to costs, once per event; its combat trigger targets before the optional
  placement is chosen during resolution.
- Exhaustive question/log test tables need unique discriminants and real rows.
  Merely adding a match arm can collide with an older number and leave the
  new choice untested while the coverage assertion passes.
- This foundation covers Yawgmoth, Hapatra, Rush of Inspiration, HELIOS One,
  Aether Hub, Karn's Bastion and U.S.S. Enterprise-D. The package still includes
  Brad Boimler, Diamond City, Nesting Grounds, Mariposa Military Base and
  Suppression Ray; it is not complete until their remaining clauses pass.
- Validation of this foundation: 5,685 engine tests pass (two intentional
  ignores), with all ten counter regressions and the choice taxonomy rerun
  after cleanup. The 236 AI, 134 cards, 73 DSL, 275 codegen, 42 view,
  248 host (two ignores), and 223 seat tests pass. Client core's full run
  exposed only the new log wording defects; all 35 log tests pass after their
  fix. The DSL nesting contract, generated-table reproducibility, 3,321-card
  validation, formatting and workspace all-targets clippy with dev controls
  pass. Original Partial inventory: 300 completed, 240 remaining.
- Follow-up: Nesting Grounds now selects its counter kind during resolution.
  A shared move checks both exact incarnations and the destination's counter
  cap before removing anything, also fixing the existing event-counter move.
  Placement still sees Doubling Season. Counter chooser indices are u32,
  tested with 300 kinds; the existing custom-counter registry supplies names.
  Validation: all 5,689 engine tests pass (two ignores), 110 client interaction,
  236 AI, 134 card and 42 view tests pass, with full workspace all-targets
  clippy, printing validation and table reproducibility. Package40 is now
  eight of twelve cards; original inventory is 301 completed, 239 remaining.

### Batch 40 follow-up: Suppression Ray and stun counters (2026-10-11)

- Suppression Ray now freezes the actual newly tapped incarnations, accepts any affordable energy payment (including zero or overpayment), and gathers non-targeted recipients across 255-object pages before placing counters. The page size does not cap the card's legal result.
- Stun is a typed counter throughout setup, engine, wire, client, narrator, codegen and AI. The common tap-state door replaces attempted untaps, including turn actions and costs; cancelled-payment restoration bypasses game replacements.
- The new regressions found two older gaps: an untap-symbol cost was offered on untapped permanents, and Twiddle changed status without recording its untap. Tap effects now share an event-recording helper, while entering tapped keeps its separate semantics.
- Verified: 5,695 engine tests (2 ignored), 1,565 client-core tests (1 ignored), 237 AI, 134 cards, 73 DSL, 275 codegen and 42 view tests; workspace Clippy, all 3,321 card validations and generated-table reproducibility passed. The engine suite took 47.07s and client-core 68.49s during concurrent test runs; the dial property test remains a candidate for the requested later test-runtime pass.
- Original Partial inventory: 302 completed, 238 remain. Package 40 retains Brad Boimler, Diamond City and Mariposa Military Base as unfinished; none is promoted by the shared foundation alone.

### Batch 40 complete: Mariposa and radiation (2026-10-11)

- Added optional tapped entry with player counters, `Amount::CountersOnYou`, and Mariposa's generic-mana reduction at cost determination. Primeval Titan's already-tapped search entry still offers both answers; Doubling Season does not double the player's rad counters.
- Radiation is a source-free precombat-main trigger (CR 728.1), with a captured event player that survives stack-controller changes. Resolution rechecks the intervening condition, mills against the current rad total, and counts actual nonlands in public destinations, including replacement exile (CR 701.17c). `Cause::Radiation` preserves its life-loss provenance. The ordinary mill operation was extracted unchanged in `57b6f3ae6` before adding this shared behavior.
- Primary references: [current Comprehensive Rules](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt), [Fallout release notes](https://magic.wizards.com/en/news/feature/magic-the-gathering-fallout-release-notes). The latter confirms simultaneous milling and milling only the available cards from short libraries. No external scripts or assets were imported.
- Full engine run: 5,700 passed, 2 ignored, one old Partial-card test still expected no entry choice. After explicitly declining the new choice, all 3 Mariposa-filter tests and all 6 radiation tests passed; no production code changed after that full run. Also passed: workspace Clippy, 112 client interaction tests, 42 view, 237 AI, 134 card and 73 DSL tests, validation of 3,321 cards and generated-table reproducibility.
- Dependency grouping improved without dropping work: Brad Boimler and Diamond City now belong to package 57 (replacement ordering/damage), where their competing replacement choices can be implemented with the same continuation machinery. Their card files remain Partial. Package 40's ten cards are complete; all 256 planned cards still occur exactly once across the original 30 packages.
- Original Partial inventory: 303 complete, 237 remaining.

### Batch 41 complete: counted activation costs and limits (2026-10-11)

- Nine cards now carry every printed clause: Liliana the Repentant, Voodoo Doll, Barad-dûr, Lair of the Hydra, Secluded Starforge, Springjack Pasture, Tomb of Urami, War Room and Nykthos. Exhaust belongs to the exact ability/object incarnation; countering the ability consumes the use, while blinking restores it.
- Activation X is announced once before targets, with lower bounds or a frozen card-defined value. Resource bounds subtract fixed costs and divide by repeated X parts. An exhaustive comparison covers 1,056 resource combinations. Do not replace an unpaid variable mana cost with its minimum in payment metadata: that erases the signal that opens its payment window. A source without a tap/untap cost may supply its own mana to that window.
- Counted taps and sacrifices collect all pages before paying, validate distinct objects against sorted eligibility lists, and keep a portable deterministic path. Tests select 300 objects, reject duplicate answers without state changes, and exercise zero and simultaneous costs. War Room distinguishes a colorless commander from no commander (CR 903.4f); the latter cannot pay its undefined cost. The same audit fixed Command Tower incorrectly producing colorless mana without a commander.
- Nykthos counts actual colored mana symbols, including hybrid and Phyrexian components, independently for each color. Amass creates tokens through normal replacement effects, lets the controller choose among all Army creatures, and adds the requested subtype without changing copiable values. This also fixes existing Orcish Bowmasters behavior under Doubling Season.
- Verified: 5,713 engine tests pass (two ignored), plus 237 AI, 134 cards, 73 DSL, the DSL authoring contract, 42 view and 112 client interaction tests. Workspace/all-targets Clippy with dev controls passes. Scope filters are explicit even when the cost itself already enforces control, so printing validation can check the same fact. All 3,321 printing validations and generated-table reproducibility are checked before this milestone is committed.
- Rules checked against the current Comprehensive Rules: 107.3a, 602.2b, 602.5b–c, 700.5, 701.47a, 702.177a and 903.4a/f. No external scripts or assets were imported. Original Partial inventory: 312 complete, 228 remaining; all nine cards in package 41 are complete.

### Batch 42 foundation: counted and conditional sacrifices (2026-10-11)

- Mold Demon and Serendib Djinn are complete. An optional two-Swamp payment is collected and sacrificed as one event; an unavailable or declined full payment runs the consequence without sacrificing one Swamp as a partial payment. Tapped Swamps remain eligible. Serendib tests the departing land's characteristics before a copy reverts, and graveyard replacement exile still counts as a sacrifice. Its no-lands state trigger remains independent of its upkeep trigger.
- Both new effect branches are traversed by the common walker and tested explicitly; update the pinned field/carrier counts only after that behavior is covered. Regenerate the source-line table before the card-pool suite whenever handwritten card line positions change.
- Four real-card regressions pass, including duplicate/short answer refusal with unchanged hashes, opponent exclusion, tapped payments, Dauthi Voidwalker and a departing copied land. Full engine: 5,717 passed, two ignored; workspace all-targets Clippy passes. DSL, its nesting contract, card and AI tests, all 3,321 printing validations and generated-table reproducibility are checked before committing. Original Partial inventory: 314 complete, 226 remaining.
- Package 42 remains in progress. The other eleven cards require pre-entry continuations, co-arrival exclusions and, for Land Equilibrium, competing replacement order; none of those cards is promoted by these two sacrifice effects.


### Batch 42: choices before entry (2026-10-11)

- Ten further cards are complete: Lotus Vale, Scorched Ruins, Lake of the Dead, Balduvian Trading Post, Heart of Yavimaya, Kjeldoran Outpost, Soldevi Excavations, Sheltered Valley, Mox Diamond and Wood Elemental. Payment precedes actual entry, so declining creates no battlefield arrival or opportunity to make mana. Wood Elemental retains its own incarnation's sacrifice count; copies choose and remember independently.
- A shared entry queue suspends land plays and resolving instructions. It covers casting, reanimation, searches and token copies, applies copy choices first (CR 616.1c), excludes simultaneous arrivals from payment (CR 614.13a), and collects all selection pages before any sacrifice. Keep pending counters with the entry; writing them to a token outside the battlefield can lose them during the final move. Animate Dead's attachment and delayed sacrifice must retain the future incarnation and resume only after entry succeeds.
- Tests cover seven land-payment shapes in both directions, tapped and opposing lands, Library of Leng, Vesuva copying Lotus Vale, Clone copying Wood Elemental, 300 Forests with invalid-answer hash preservation, simultaneous World Shaper returns, Sheltered Valley event order, Reanimate's following life-loss instruction, zero-sacrifice death, and a token copy's entry counter. Old mana tests now pay the printed costs. The generic mana sweep sets up payment lands directly; their real entry behavior is tested separately rather than increasing its skipped-face ceiling.
- The new implementation is split into queue state, choices, copy handling and committing moves. Ordinary entries retain the direct path. No unsafe code or external assets are introduced. Original Partial inventory: 324 complete, 216 remaining. Package 42 remains open for Land Equilibrium and competing entry replacements.
- Validation: 5,727 engine tests pass (two ignored), plus 237 AI, 134 card-pool, 74 DSL and 42 view tests. Workspace/all-targets Clippy with `baylee-client/dev-control`, all 3,321 printing validations and generated-table reproducibility pass.

### Batch 42 complete: competing entry replacements (2026-10-11)

- Land Equilibrium compares the pre-entry land counts and remembers the player putting the land onto the battlefield, even when someone else will control it. A search's actor is the searcher rather than the resolving spell's controller. Multiple copies append independent sacrifices; an already applied replacement survives its source leaving.
- Copy choices precede other entry replacements (CR 616.1c). The affected controller orders competing replacement effects and interacting simultaneous entries in APNAP order. Earlier payments change the eligibility of remaining replacements; co-arriving lands cannot pay each other's entry costs (CR 614.13a).
- A private `Flow::Enter` suspends nested instructions while entry modifiers and appended actions finish. Entry counters and tapped status are established first, and ETB triggers are captured before an immediate sacrifice removes their source. No priority, state-based actions or trigger placement intervenes. Concessions rebuild unpaid entry choices at the original exact cardinality instead of accepting an incomplete cost.
- Eleven real-card tests cover 32 threshold/ownership configurations, both Lotus Vale replacement orders and payment decisions, multiple Equilibria, Tendo Ice Bridge counters, Glimmerpost landfall, Kenrith's actor versus incoming controller, Path to Exile's searcher and tapped result, simultaneous World Shaper returns, six concession scenarios, Vesuva's copy ordering and both orders of competing entry payments. Full engine: 5,738 passed, two ignored (41.38s).
- Primary references: [Comprehensive Rules](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt), CR 603.3, 614.12–13 and 616.1; [Wizards card rulings served by Scryfall](https://api.scryfall.com/cards/51c5df75-746c-4e4b-84f1-76b689d317d7/rulings), including: “The player won't be able to tap that land for mana before sacrificing it.” No external code or assets were imported.
- Package 42: 13/13 complete. Original Partial inventory: 325 complete, 215 remaining. The owner requested combining the next two packages into one implementation/check cycle, then increasing the group size if measured throughput improves; retain individual card validation and reviewable milestone commits.
- Final checks also pass: 237 AI, 134 card-pool, 74 DSL, 42 view and 112 client interaction tests, workspace/all-targets Clippy with dev controls, all 3,321 printing validations and generated-table reproducibility. The new replacement prompts advance the wire version to 69.

### Combined packages 43–44: shared library operations (2026-10-11)

- Five cards complete: Aven Mindcensor, Moonring Island, Visions, Forsaken Crossroads and Isolated Watchtower. Nine grouped real-card tests exercise private inspection without reordering, the starting-player alternative, blue-permanent and relative-land-count activation restrictions, accepted/declined reveals, and search restrictions across owners and controlling players.
- Restrict the library segment before applying search filters. Capture that exact segment with the pending search so the searcher can inspect nonmatching cards too, without exposing cards below Mindcensor's top four. The chooser changes under Opposition Agent; the player doing the search does not. Searches in another player's library use the same restriction.
- Shared searches now apply CR 701.23b correctly: finding fewer cards of a stated quality is legal even if the instruction itself is mandatory. Existing tests incorrectly pinned a minimum of one solely because the Oracle text did not say “up to”; those expectations now distinguish the mandatory search from its optional success. The maximum, eligible cards, destination and entry state remain tested. Zero-result searches retain private inspection before shuffling, including Nylea's Intervention with X=0.
- Path to Exile, Erode and Sundering Eruption share the same search implementation. Consent comes before inspection; declining leaves library order untouched. Accepting applies Ashiok's correctly scoped prohibition, Mindcensor and Opposition Agent, followed by a shuffle of the searched library. Optional-search and zero-selection acknowledgements are represented explicitly in the regression helpers.
- Full engine validation: 5,747 passed, two ignored (36.78s). Source: [Comprehensive Rules](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt), especially CR 701.23b and 722.2; current cached Scryfall Oracle headers retained. No external code or assets imported. Original Partial inventory: 330 complete, 210 remaining. Combined packages 43–44: 5/26 complete; neither package is closed yet.

- Integration checks: 237 AI, 134 card-pool, 74 DSL, 249 gamehost and 42 view tests pass. The private-view regression confirms the searcher sees the entire allowed segment, including nonmatches, while opponents see none of it. An outdated gamehost expectation was corrected: a Command Tower with no commander produces no mana (CR 903.4f), as the engine already enforced. Forsaken Crossroads offers its optional untap only while its original permanent is tapped on the battlefield (CR 608.2d, 701.26b); otherwise its scry remains.
- Final package checks: 112 client interaction tests and the DSL nesting-field test pass; all 3,321 printing validations and generated-table reproducibility pass, as does workspace/all-targets Clippy with dev controls. Wire version 70 includes the private library-inspection prompt.


### Batch 43 complete: library operations and linked choices (2026-10-11)

- Completed all fourteen remaining cards together: Fabled Passage, Elven Passage, Turntimber Symbiosis, Sidequest: Catch a Fish, Drop of Honey, Shapeshifter, Primal Clay, Petra Sphinx, Nebuchadnezzar, Petrified Hamlet, Myriad Landscape, Field of Ruin, Drafna's Restoration and Transmute Artifact. The whole package is now 19/19. Original Partial inventory: **344 complete, 196 remaining of 540**. Package 44's seven cards remain Partial.
- Shared library continuations retain the permitted inspection segment, the searching player, replacement controller, found incarnations and entry modifiers. Field of Ruin collects choices in APNAP order before moving any find; Myriad's second choice is restricted by shared land types. Prohibited mandatory searches still perform their separate shuffle instruction (CR 609.3). Transmute Artifact sacrifices during resolution and uses a real mana-ability window for the difference; Opposition Agent's replacement bypasses that payment.
- Names and numbers belong to a rules acquisition, including a trigger captured before a temporary copy changes its source. Recopying the same printed card must not inherit the previous choice (CR 607.5). Primal Clay's selected form is copiable; Shapeshifter's chosen number is not. Drafna's dependent graveyard targets follow the final targeted player when a copy changes targets, while already illegal targets retain the exception in CR 115.7d. Commander replacement questions retain every selected card and finish before ordered movement.
- New tests cover threshold matrices, Doubling Season, replacement sacrifices, private search control, refused answers with unchanged hashes, copied and departed sources, signed minimum power, non-targeted choices, all entry forms, zero and oversized X, paid/declined differences, target changes and Commander destinations. Sidequest checks an actual hand arrival before its Food/transform follow-up. The new batch tests are divided by mechanism into files of at most 250 lines.
- Preserve the object-size guard while extending sparse state. Enlarging two inline riders would have raised every object to 328 B; vector-backed riders instead measure 288 B, below the unchanged 296 B budget. Nonempty lists allocate; measure that tradeoff in the later requested performance pass. Keep tests and codegen in one package cycle, and run workspace Clippy early enough to catch exhaustive readers and oversized dispatch functions before the final integration gate.
- Validation: 5,781 engine tests plus both footprint guards pass, with two intentional engine ignores. Also green: 237 AI, 134 card-pool, 76 DSL, the DSL nesting contract, 249 gamehost (two ignores), 223 seat, 42 view, 112 client interaction, 29 client-choice and 60 xtask tests. After the final helper extractions, the 52 entry regressions, AI suite and scripted-seat regression pass again. Workspace/all-targets Clippy with dev controls, formatting, all 3,321 printing validations and generated-table reproducibility pass. Wire version 71 carries the new choice shapes.
- Codegen now updates/checks the derived precon registry as well as card tables, eliminating a separate stale-status step after card batches. The existing Enchantress precon becomes playable. Primary rule source: [Comprehensive Rules](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt), especially CR 101.4, 201.4, 208.2b, 607.5, 701.4, 701.23 and 115.7d–e; card-specific Wizards rulings were checked through Scryfall. No external code or assets were imported.

### Batch 44 complete: hideaway and private linked exile (2026-10-11)

- Implemented all seven cards as one package: Clive's Hideaway, Spinerock Knoll, Howltooth Hollow, Mosswort Bridge, Windbrisk Heights, Shelldock Isle and Knowledge Vault. Each land checks its condition during resolution, not activation. Knowledge Vault sacrifices during resolution; leaving early, changing controller or resolving a second activation cannot discard another hand. Package 44: **7/7**. Original Partial inventory: **351 complete, 189 remaining of 540**.
- Linked cards carry the exact source incarnation and rules acquisition. Copied hideaway triggers can link multiple cards, each offered for immediate play. The shared casting continuation supports land limits, Adventure/MDFC faces, free X=0, mandatory additional costs, taxes and the casting mana window. A failed play restores hidden state and removes only the temporary permission it added.
- Face-down exile has no characteristics or printed abilities. Knowledge Vault grants nobody permission to look; hideaway remembers every entitled controller, including after control changes and phasing. Views and logs use those explicit permissions. A sparse host marker avoids an exile scan on ordinary control changes, and object size remains 288 B within the unchanged 296 B budget. Historical network ObjectId correlation across hidden zones remains a separate host-boundary issue to resolve before the final main push; field redaction alone is not a proof of unlinkability.
- Departure triggers now inspect the actual departing objects, including sources returned to hand or library, instead of scanning every graveyard and exile pile. Reordering cards already in a library no longer invents a zone change or new incarnation. New tests cover 0–5 card libraries, signed power, opponent damage, control and incarnation changes, copied acquisitions, multiple linked plays, hidden information and all Vault departure destinations.
- Validation: 5,800 engine tests pass (two intentional ignores; the full run's sole old Howltooth expectation was corrected and rerun). All 19 new engine regressions pass again after helper extraction, as do both footprint guards, 237 AI, 134 card-pool, 77 DSL plus its nesting contract, 250 gamehost (two ignores), 42 view and 62 xtask tests. Workspace/all-targets Clippy with dev controls, the final xtask Clippy, all 3,321 printing validations and generated-table reproducibility pass. Loopback harness tests require the unsandboxed test runner; sandbox permission failures are not card failures. Wire version remains 71.
- Primary references: [Comprehensive Rules](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt), CR 305, 406.3, 601.2g, 607.2a/607.3/607.5, 608.2g and 702.75; Wizards rulings served by Scryfall for [Knowledge Vault](https://api.scryfall.com/cards/c416fca6-e201-4dbc-b9c7-248d70967746/rulings) and [hideaway](https://api.scryfall.com/cards/1490c0dc-06d6-45ee-af3c-2935b0ab1233/rulings). No external scripts or assets imported. Optional-offer and controller-scope validators now recognize the new constructs, with negative controls.

### Batch 45 complete: casting permissions and alternative zones (2026-10-12)

- Completed the entire fourteen-card package together: Kess, Ojer Pakpatiq, Theorist's Proxy, Mistrise Village, Throne of Makindi, Bloodsoaked Insight, Open Communications, Agadeem's Awakening, Mizzix's Mastery, Underworld Breach, Welcome to . . ., North Star, Ring of Ma'rûf and Tarrian's Journal, including their reverse faces. Package 45: **14/14**. Original Partial inventory: **365 complete, 175 remaining of 540**.
- Graveyard permissions retain the granting effect and exact source incarnation, consume once-only uses only after successful casting, and distinguish native from granted escape. Native disturb still offers its reverse face through a deferred mana payment. Kicker and prototype combine with granted escape; unrelated alternative costs do not. Casting failures restore the permissions and hidden state they temporarily changed.
- Match simultaneous additional costs with deterministic augmenting paths. Prefer graveyard cards required by those costs before assigning escape fodder; exhaustive small graphs and a long augmenting path compare the result with an independent brute-force reference. Component-scoped mana conversion pays North Star's base cost without incorrectly converting colored additional costs or snow requirements. Exhaustive symbol-pair tests exercise Phyrexian choices, reductions and numeric boundaries.
- Mizzix copies copiable card values and opens the normal free-cast continuation, including mandatory additional costs, targeting, choices of order and restrictions such as Teferi. Agadeem rejects duplicate mana values without mutating the state. Saga chapters derive from actual lore-counter events and preserve source incarnation, so proliferate, counter doubling and copied triggers share one implementation. Empower creates a procedural Jace token with real loyalty abilities; finality counters replace the appropriate graveyard move.
- Ring's draw replacement has its own effect identity, can prevent an empty-library loss, expires correctly and respects Commander and controlled-player restrictions. Its AI valuation uses only a provable avoided loss; unknown outside cards do not justify replacing a normal draw. A narrow urgent-activation path now precedes the otherwise intentional upkeep pass. North Star, the Tomb and counter-protection activations also have explicit affordability and timing evaluation.
- Three workers shared explicit ownership while one coordinator compiled and integrated the complete package. Keep Rust writes paused during the central Cargo cycle; use `--no-fail-fast` for independent package suites when collecting failures. The client binary is only an entry point: client choice tests must run with `--lib`, otherwise a successful command runs zero tests. Loopback test failures under the sandbox require the authorized unsandboxed runner, not application changes.
- Validation: **5,848 engine tests** (two intentional ignores), both footprint guards, 142 core, 135 card-pool, 77 DSL plus its nesting contract, 242 AI, 250 gamehost (two ignores), 223 seat, 42 view, 112 client interaction, 42 client choice and 63 xtask tests pass. Host harness tests pass again after the AI refinement. Workspace/all-targets Clippy with dev controls, all 3,321 printing validations, formatting and generated-table reproducibility pass. Wire version **72** carries the casting modes and finality counter. Pure casting-wizard file moves were committed separately before behavior changes.
- Primary references: [Comprehensive Rules](https://media.wizards.com/2026/downloads/MagicCompRules%2020260925.txt), especially casting, copying, replacement effects, linked abilities, escape, Sagas and Commander rules; card-specific Wizards rulings served by Scryfall. The future-dated Beam Me Up wording follows its cached Oracle definition without inventing a current CR section. No external scripts or assets were imported. Separate later work still includes hidden-zone handle unlinkability and AI ordering of protective setup activations before the spell they protect.


### Batch 46 milestone: printing facts, mana provenance and inherited abilities (2026-10-12)

- Twelve of thirteen cards are complete: Watermarket, Brotherhood Headquarters, Secret Base, Cryptic Spires, Pit of Offerings, Sunken Palace, Interplanar Beacon, Plaza of Harmony, Avengers Tower, The Everflowing Well / The Myriad Pools, Primal Amulet / Primal Wellspring, and Agatha's Soul Cauldron. Original Partial inventory: **377 complete, 163 remaining of 540**. Underdome stays Partial because a mixed payment needs a joint assignment of ordinary mana and mana usable only for the component imposed by an Un-card. Paying that component greedily first can wrongly consume the only mana that can pay another colored symbol.
- Rule-relevant printing facts travel separately from presentation PrintRef through catalog, deck rows, protocol and presets. Missing facts remain unknown; an alternate printing is not silently treated as the reference printing. Cryptic Spires choices belong to each deck entry, survive imports and restyling, affect Commander identity, and are selected outside the unchanged card image. Two differently circled copies are tested together through preset hydration, real land plays and their distinct mana offers.
- Record exact mana producer incarnations and actual expenditure, including ordinary producer mana. Wellspring and Palace copies use the spell or ability that was actually paid for, even after it is countered. Pools copies retain announced choices without creating another cast or battlefield entry. Sparse original snapshots restore the pre-first-copy rules after multiple temporary overlays, preserve counters and ignore recipients that left and returned. GameObject is 296 B and GameState 3,224 B; both existing footprint guards pass without widening their budgets.
- Granted activated abilities have stable sparse IDs and their complete definition, timing, targets, costs and mana classification. Linked clauses acquired from one donor ability list share a family; different donors, reacquired donors and unrelated fixed grants cannot read one another's remembered cards. Capture the family's identity before paying a cost that sacrifices the source. Tests cover more than eight donors, loyalty, sorcery timing, graveyard-only activations, conditional mana, X and sacrifice costs, donor/recipient incarnation changes and shared linked-clause families.
- Canonical mana-loop hashing includes the semantic producer, restrictions and expenditure while excluding allocation history. Producing two identical units together or separately yields the same canonical identity. Bound possible-color inference with a six-bit fixed point rather than recursive cycles. No unsafe or assembler path was introduced without a measured need.
- Public views expose per-grant mana classification and permission information; wire version is **73**. AI resource estimation never counts a paid mana conversion as free production. Linked exile values its independently chosen reflexive recipient, while unread follow-ups remain unknown even on an empty board. Client grant tests now use explicit per-slot metadata instead of inferring a grant's type from duplicate permanent-level mana offers.
- Validation: full engine **5,902 passed, two ignored**, followed by all **56 Batch 46 regressions**, including three additional acceptance tests. Both footprint guards pass. Also green: 249 AI, 138 card-pool, 77 DSL, 145 core, 27 catalog, 29 deck I/O, 21 protocol, 42 view, 1,572 client-core (one ignore) and 252 gamehost (two ignores). The all-implemented-card game sweep passes again after the twelve promotions. Client library testing passed 1,582 tests before correcting two outdated grant fixtures; all 18 ability tests then pass, and all nine loopback-related tests pass outside the sandbox. Six client tests remain intentionally ignored. All 63 xtask tests pass with their local test servers enabled. Workspace/all-targets Clippy with dev controls, generated-table reproducibility after promotion and diff checks pass.
- Keep workers on disjoint cards/tests or scratch staging while the coordinator owns shared engine APIs and serializes Cargo runs. A configured agent count is not proof that the running session offers that many slots. Batches 47–50 are staged, not integrated or counted. Owner's stopping boundary is explicit: after all original 540 cards are fully implemented and validated, save compact context and wait for Go; defer the later build/test/performance phases and final main integration/push.
- Rules and policy references remain the official Comprehensive Rules, Wizards release notes and rulings, and the legal review in docs/legal.md. No external implementation, artwork or audio was imported.


### Batch 46 follow-up: bounded generic-component assignment (2026-10-12)

- Move the 37 existing mana-payment tests into their own module in a pure relocation commit before changing the solver. The old module's contents were compared byte-for-byte after removing indentation, and all 37 tests passed.
- A separate generic-credit resource class now participates in the payment solver's terminal generic allocation. It cannot satisfy colored, colorless, hybrid or snow symbols, even when ordinary mana has broader spending permissions. Ordinary and credited mandatory consumption remain separate; a failed terminal assignment backtracks through flexible symbols. Actual-type generic reservations remain ordinary obligations, disjoint from a separately imposed generic credit.
- Allocate required resources first, then optional credits within their quota, then ordinary preferred and remaining mana. Work is bounded by the six actual mana types rather than the number of units. Totals use u64 and returned per-type amounts remain bounded u32 values. Validate credit requirements and calculate their usable capacity once before recursion.
- Validation: 20,000 seeded credit allocations match an exhaustive independent small-space search; another 20,000 ordinary payments match the previous exact consumption and snow-preference procedure. Targeted cases cover backtracking, typed reservations, hybrid and snow symbols, impossible mandatory credits, broad spending permissions, transactional failure and maximum-width costs. All **44 mana-payment tests** and the full engine's **5,912 tests** pass (two intentional ignores; 41.32 seconds on this host). That wall time is a regression observation, not a claimed benchmark speedup.
- This is solver groundwork, not completion of Underdome. Frozen cost provenance, eligibility for each component, the offer/payment wrappers, North Star, constrained generated mana, and exact provenance debiting still need integration. Keep the reverse case in scope too: mana for an Un-card's own cost must not automatically pay a separate ordinary-card tax. The stopping inventory remains **377/540**, package 46 **12/13**.


### Batch 47 complete: activation authority, prices and resolution choices (2026-10-12)

- Completed Clergy of the Holy Nimbus, Derevi, Ifh-Bíff Efreet, Rhystic Cave, Land’s Edge, Armageddon Clock and Power Artifact. Original Partial inventory: **384 complete, 156 remaining of 540**. Batch 46 remains 12/13; Underdome’s mixed imposed-cost provenance is still open.
- Activation authority is explicit for any player, opponents and the command-zone owner. The activator pays and controls the ability; damage still uses the permanent’s actual controller for source characteristics such as lifelink. Derevi’s command action is not a cast, and its triggered target is chosen before the optional tap/untap decision during resolution. Separate resolution-choice labels preserve readable tap/untap options without inventing modal-trigger metadata.
- Freeze full and fixed activation prices before mana abilities can change the board; retain the fixed/X distinction used by Illusionary Mask. Apply Power Artifact’s limited reduction before independent unrestricted reductions, preserve colored costs and genuine free abilities, and include reduction credit when offering X. Discard costs capture exact pre-move identities and last-known characteristics, including replacement destinations and copied stack abilities.
- Rhystic Cave chooses a color first and offers payment to all live players in APNAP order, including its activator. Its special priority restriction also excludes nested payment windows and guaranteed-mana planning. Possible-color inference still sees its colors for Reflecting Pool; the AI reports conditional production as unknown. The explicit ability-reading census is 1,027 of 1,286, with negative tests for newly contextual effects.
- Standing regeneration and a one-shot shield create a genuine replacement choice. Suspension retains the current resolution across a choosing player’s concession, processes other controllers’ choices, and then resumes subsequent instructions. Sparse queue storage costs one pointer in ordinary states; a stable APNAP sort followed by deque consumption avoids repeated scans/removals. A 4,096-entry, three-cycle regression compares against an independent per-seat reference and checks storage release. No benchmark speedup or unsafe/assembly claim is made.
- Validation: **5,941 engine tests** pass (two intentional ignores), including 28 Batch 47 card regressions and the queue regression. Both footprint guards pass: GameObject **296 B**, GameState **3,232 B** (+8 B). Also green: 251 AI, 77 DSL, 1,573 client-core (one ignore), 139 card-pool, 252 gamehost (two ignores), 223 seat and 42 view tests. The host sweep includes all newly implemented cards. Workspace/all-targets Clippy with dev controls, formatting and generated-table reproducibility pass. View version **74** carries the regeneration prompt. Socket tests require the authorized runner outside the sandbox; do not change application code to work around a denied local bind.
- Keep the original 540-card inventory distinct from total pool coverage. Only promote after actual-action regressions and integration gates; scratch definitions and uncompiled tests do not count. GPT-6.1-Sol workers prepare cards, test cases and metadata while the coordinator owns engine changes and serial Cargo gates. The four-card Batch 48 draft has nineteen tests and remains unintegrated. Rules/policy sources remain the official Comprehensive Rules and the reviewed terms in docs/legal.md; no external implementation or assets were imported.


### Batch 48 milestone: declaration groups, exact event identities and sacrifices (2026-10-12)

- Completed Legion’s Landing / Adanto, Storm the Vault / Vault of Catlacan, Valakut and Urza’s Miter: **4/14** in this package; **388 complete, 152 remaining of the original 540**. Remaining activation observers and historical predicates stay Partial.
- Count Landing’s attackers in one declaration, not the whole turn or the current combat list. Group Storm’s positive matching combat damage separately for each recipient and combat-damage step; an earlier nonmatching source cannot suppress the first matching event. Valakut excludes the entering Mountain’s exact incarnation at both intervening-if checks. Its target is mandatory before the optional damage decision at resolution, and a returned incarnation is a different Mountain.
- Sacrifice provenance is a separate, defaulted departure fact, independent of the move’s Cause and final replacement destination. Central wrappers cover self/chosen/count costs, ordinary effects, equalization, entry sacrifices, Aura leave effects and delayed/echo sacrifices. Preserve simultaneous snapshots for equalization as well. Ordinary destruction, lethal damage, zero toughness, legend/world rules, discards, counters and milling do not become sacrifices. Miter observes its own nonsacrifice death and artifact-token deaths, and ignores effect-driven sacrifice and exile replacements.
- Event-dependent conditions only inspect event history when they actually need it. Six sequential searches for an embedded journal event now use checked relative address arithmetic over the contiguous batch and a final pointer-identity check. This is safe Rust with no pointer dereference, unsafe block or architecture-specific path; equal-valued copies and references outside a subslice remain absent. An exhaustive 4,096-row comparison over whole, empty and partial batches verifies equivalence to the linear reference. This establishes constant-time lookup, not a measured end-to-end speedup.
- Extended the codegen sentence reader for all three new trigger forms, with positive event/subject matches and negative wrong-event/self-subject regressions. Generated tables remain reproducible. Adanto adds one readable activated ability: AI census **1,028/1,287**. Its activated back face is also explicitly included in the card-label census.
- Validation: **5,962 engine tests** pass (two intentional ignores), including nineteen new card/event regressions and the event-index comparison. The old incorrect fifth-Mountain test is replaced by separate fifth/sixth-Mountain cases. Also green: 276 codegen, 251 AI, 139 card-pool and 252 gamehost tests (two host ignores), including the all-implemented-card game sweep after promotion. Workspace/all-targets Clippy with dev controls and the final table check pass. Both footprint guards pass unchanged: GameObject **296 B**, GameState **3,232 B**. New test files are split by card/mechanism; no generated file was edited manually.
- Keep timing evidence precise: trigger detection precedes SBAs, while activation completion follows cost payment. Do not assume an Aura observes an activation solely because it was attached before the costs; verify the current-object and lookback rules before the next activation batch. The coordinator retains engine ownership and the owner’s stop-after-540 boundary.


### Batches 48/49/54: activation completion, untap duration and land-type choice (2026-10-12)

- Completed Haunting Wind, Powerleech, Artifact Possession, Telekinesis, Skyline Cascade, Frostwalk Bastion and Unstable Frontier. Original inventory: **395/540 complete, 145 remaining**. Packages 48: **7/14**, 49: **3/10**, 54: **1/5**. Underdome remains incomplete.
- Observe actual completed activations with exact source identity, activator, tap-symbol cost and mana classification. Cover ordinary, granted, loyalty and intrinsic-land entry points. Intrinsic basic land types grant distinct abilities: choose which ability before paying its tap cost, and record one tap/completion. Printed any-color mana still chooses its output during resolution. Controller-at-resolution and activating player are separate relations.
- Capture triggers between individual cost parts without performing SBAs or putting them on the stack. A tapped-then-sacrificed enchanted artifact creates its tap trigger; an Aura no longer attached after sacrifice does not observe the completed tapless activation. Failed transactional costs restore the journal cursor and trigger queue. Mana payment rollback accepts its own informational activation events. Tap-for-mana classification reads the originating activation across collection boundaries, and does not count additional colors twice.
- Bind untap suppression to exact affected incarnations and count actual untap steps of their current controllers. Skipped steps do not consume duration; already untapped and phased-out affected objects still consume actual steps. Independent durations overlap rather than sum. Combat-hit triggers retain the damaged creature rather than the damage dealer; a returned incarnation is unaffected. Telekinesis prevents combat damage only.
- Unstable Frontier chooses one of five basic land types during resolution after choosing its own-land target. Use existing type-layer semantics and explicit rules-choice labels instead of fabricated Oracle fragments. Test every type, intrinsic mana, losing/restoring printed abilities, cleanup and illegal targets.
- Validation: full engine **5,991 passed, two ignored** (41.43 seconds; observation, not a benchmark claim), including twelve activation and thirteen untap regressions. Integration: **279 codegen, 140 card-pool, 251 AI and 252 gamehost** tests pass (two host ignores), including the implemented-card game sweep. Workspace/all-targets Clippy with dev controls, formatting and generated-table reproducibility pass. User Cargo.lock remains byte-identical to its preserved observed state. AI census becomes **1,028/1,288**: Frontier adds one contextual, deliberately unread activation.
- Audit fixtures as carefully as cards under test: Oubliette remains Partial and cannot establish phasing. Replace that test setup with an explicitly documented phasing fixture. Spell bodies intentionally have no per-ability sentence index; Oracle paragraphs use newlines, not arbitrary sentence periods. Tests must preserve these reader contracts. GPT-6.1-Sol stages whole card packages while the coordinator owns shared engine changes and serial Cargo gates. No external implementation or assets imported; stop at all original 540 as requested.


### Batch 49 milestone: untap returns and interruptible delayed groups (2026-10-12)

- Completed Undiscovered Paradise, Mystifying Maze and Thawing Glaciers: package **6/10**, original inventory **398/540 complete, 142 remaining**. Paradise returns during its activator's next actual untap step as a turn-based instruction; Glaciers queues a counterable trigger at the next cleanup of any player, even after a failed search. Maze returns the actual exiled incarnation tapped under its owner's control. Skipped steps, changed controllers and departed/reentered objects are separate cases.
- Group all objects exiled by one temporary-exile instruction into one delayed stack ability. Existing Eerie Interlude-style effects now share that correct path; countering the ability prevents the whole return, and an actual Vantress Visions copy retains every member after the original is countered. Sparse immutable pending payloads share snapshots; stack riders preserve identity and are included in hashes/source tracking. GameObject remains **296 B**, GameState **3,232 B**.
- Collect all due delayed triggers before stacking the combined ordinary/delayed APNAP batch. A three-trigger, two-controller actual-card regression covers the ordering. TriggerAbout records its exact event-object identity before priority, preventing a responded-to blink from redirecting the return to a fresh incarnation.
- Validation: all **18 new card regressions** pass. Full engine run: **6,008 passed, one census failure, two ignored**; after updating the implemented-land census for the three promotions, that sole failed test passes independently. Both footprint guards pass. Integration: **251 AI, 140 card-pool, 279 codegen and 252 gamehost** tests pass (two host ignores), including the all-implemented-card sweep. Workspace/all-targets Clippy with dev controls, formatting and generated-table reproducibility pass. AI census is **1,028/1,289**; Maze's contextual activation remains deliberately unread.
- Prepare whole packages in parallel, but promote only cards whose shared engine contracts and actual-action tests are complete. Workers prepared additional turn/history acceptance cases while the coordinator integrated these timing paths. Keep generated files exclusively codegen-owned and preserve the user's Cargo.lock. No external implementation or assets imported. The stop-after-540 boundary still applies.


### Batch 49 completed: actual turns, blocking identities and draw-step debts (2026-10-12)

- Completed Magosi, Giant Turtle, Wall of Dust and Nafs Asp. Package **10/10**, original inventory **402/540 complete, 138 remaining**. Magosi consumes actual pending normal/extra turns, independently of optional Time Vault skips; Turtle remembers exact attackers from its controller's last completed own turn. Skipped turns do not replace that history. Mandatory skipped turns create no turn-start event.
- Wall records each creature it actually blocks, separately from becoming blocked itself. Its next-controller-turn restriction binds the affected incarnation, survives source departure and ability removal, and expires after the affected controller's next actual turn. A derived effect-table flag avoids scanning attack prohibitions on ordinary boards. Controller-change interpretation follows the official Breaching Leviathan analogy; no card-specific published Wall ruling is claimed. Added multi-block, copied-trigger, changed-controller, departed/reentered and concession tests.
- Nafs creates separate payable obligations with stable action identities. Each damage event offers its own payment before the damaged player's next actual draw step, across turn boundaries. Due obligations withdraw their payment offers and create separate counterable life-loss triggers before drawing, including replaced draws. The source need not remain. Client and narrator display the amount and deadline; protocol version **28** carries the new action kind, while view version remains **74**.
- Shared state changes participate in both exact snapshot and canonical loop hashes, with live object identity rather than allocator history. Reuse the last-turn attacker buffer. No unsafe or assembly was required; no measured speedup is claimed. GameObject remains **296 B**, GameState **3,232 B**.
- Validation: full engine **6,044 passed, two intentional ignores**. After extracting only test helpers for lint, all **11 Wall tests** pass again. Integration suites: **251 AI, 140 card-pool, 1,576 client-core (one ignore), 252 gamehost (two ignores), 21 protocol, 224 seat** passed; seven focused client granted-action tests passed. **281 codegen tests**, workspace/all-targets Clippy with dev controls, formatting, diff check and generated-table reproducibility pass. Seat loopback tests used the authorized runner outside the sandbox. User Cargo.lock is byte-identical to its preserved observed state.
- Codegen now understands exact skip-turn and last-own-turn-attack grammar, refusing unsupported variants; sentence indexing recognizes block-only triggers without confusing the blocked creature with the trigger source. Sol workers continue preparing whole packages while the coordinator owns engine changes and serial validation. No external implementation/assets imported. Stop after all original 540 cards, before the later optimization phases.


### Batches 48/52: exact turn history and cumulative upkeep (2026-10-12)

- Completed Ichneumon Druid, Lilypad Village, Oscorp Industries and Glacial Chasm: original inventory **406/540 complete, 134 remaining**. Batch 48 **10/14**, batch 52 **1/13**. Current authorized scope is to finish batches 46, 48, 52 and 54, reaching 427, then rebase on main, push and wait for the app restart.
- Freeze completed instant-cast ordinals, actual projected entry subtypes and exact discarded graveyard successors. Count successful actions, not offers; exclude noncast copies and stale incarnations. Batch simultaneous arrivals before observing their projected characteristics. History participates in snapshot and canonical hashes and resets each turn. Ordinary graveyards avoid the Mayhem scan when no discard history exists.
- Glacial Chasm uses a rule-level attack prohibition, damage prevention, mandatory entry sacrifice and counterable cumulative upkeep. Life-payment prompts retain full u32 amounts without truncating high age-counter payments. Client, AI and narrator preserve the amount; view version 75 and protocol version 29. GameObject remains 296 B, GameState 3,328 B.
- Validation: full engine 6,072 passed, six failures and two ignores; the six were three Chasm fixture setup errors and three census expectations. All were corrected and pass focused reruns (seven Chasm, twenty keyword and one land census test). All 57 batch-48 tests and both footprint guards pass. Integration: 140 cards, 286 codegen, 1,577 client-core (one ignore), 252 gamehost (two ignores), 252 AI, 21 protocol, 42 view and 225 seat tests pass. Workspace/all-targets Clippy with dev controls and generated-table reproducibility pass. User Cargo.lock remains preserved.
- Sol workers prepare whole card/test/codegen packages in isolated scratch directories; root owns engine integration and serial gates. Exact reader predicates reject unsupported grammar. No external implementation or assets imported.


### Batch 52: landwalk, prohibited keywords and repeated source shields (2026-10-12)

- Completed Hammerheim, Arcane Lighthouse and Prahv: **409/540 complete, 131 remaining**, batch 52 **4/13**. Current stop target remains 427 after completing batches 46/48/52/54, followed by rebase, push and waiting for restart.
- Separate removing an ability at a timestamp from prohibiting its regrant. Landwalk removal includes nonbasic matching rules and allows later grants. Keyword prohibition binds the resolved creature cohort across control/type changes and suppresses later grants while preserving unrelated keywords. Layer filters cannot observe a prohibited intermediate grant.
- Prahv chooses a source without targeting and prevents repeated combat/noncombat damage to every recipient until cleanup. Use a distinct AllFrom shield; do not consume a next-event shield. Exact source identities, spell-to-permanent continuity, departed-source LKI, hashes and source retention share existing infrastructure. Real Mogg Fanatic casting, combat and sacrifice test all of those together; bounce/recast is a new source.
- Full engine: **6,087 passed, one stale modifier census failure, two ignored**. Corrected census passes independently; six land-mana census tests and all four Prahv tests pass after the final test-helper cleanup. Ten new card-action regressions pass. Integration: **252 AI, 140 cards, 252 gamehost** passed (two host ignores), including the implemented-card game sweep. **288 codegen and 77 DSL tests**, workspace/all-targets Clippy with dev controls, formatting and table reproducibility pass. AI census becomes 1,030/1,294; land promise ceiling 1,011. User Cargo.lock preserved.
- Whole packages staged by Sol include explicit missing-edge notes and refused grammar. Fix fixture API spellings before integration; a transparent fixture must state exactly which rule it isolates. No unsafe, assembly, external implementation or assets introduced.

### Batch 52: Johan and Bronze Horse (2026-10-12)

- Accepted **411/540**, batch 52 **6/13**. The stop target remains 427, completing only the four already started batches before rebase/push and restart.
- Johan changes the attack-tapping rule dynamically; it does not grant vigilance. Exact source incarnation, current untapped status, late entrants, blink, and canonical loop hashing are covered. Compact monotone rule hints keep ordinary attacks from scanning these effects.
- Bronze Horse reads retained spell target references, including secondary targets, against the exact affected incarnation. Ability damage and untargeted spell damage do not qualify. Client and AI defer predicates whose source context is unavailable.
- Validation: 12 actual-action pair regressions; full engine 6101 passed, 2 ignored; AI 252, codegen 290, DSL 77, gamehost 252 (2 ignored), cards 140 passed after regenerating tables. Workspace all-targets dev-control clippy and tables reproducibility check passed. No unsafe or assembler was introduced.

### Batch 52: The One Ring and Teferi’s Protection (2026-10-12)

- Accepted **413/540**, batch 52 **8/13**. Finish batches 46/48/52/54 to 427 before the requested rebase, push and restart pause.
- Actual caster and hand-origin are separate facts. Both fit in one byte, preserving the **296 B GameObject / 3328 B GameState** footprint. Exhaustive encoding tests cover all 127 represented seats and both origins; engine presets allow eight seats. Noncast zone transitions clear the caster, real stack-to-permanent transitions retain it, and spell copies never inherit it. Exact LKI and both hashes read the same provenance.
- Protection uses shared player-target validation and damage prevention, including own targets, resolution rechecks and unpreventable damage. Life lock rejects changes before arithmetic/replacement/history and disallows positive life payments. Phase-out snapshots all controlled permanents for one simultaneous call. The current attachment model has only object hosts, so no player-Aura representation was invented or claimed tested.
- An activation does not put the Ring’s burden counter on immediately: bouncing an initially unburdened Ring in response correctly draws zero. The LKI regression first resolves a real activation, then bounces the source in response to the next activation and draws for the existing burden.
- Validation: 21 card action regressions plus exhaustive provenance tests; full engine **6124 passed, 2 ignored**; codegen **292**, DSL **77**, cards **140**, gamehost **252 (2 ignored)**; AI **251 plus corrected census regression**. Workspace all-targets/dev-control Clippy, footprint, and generated-table reproducibility checks passed. Reader fixtures explicitly identify self-exile rather than silently accepting an unspecified subject.


### Batches 52/54: Archon of Emeria, Kismet and Gond Gate (2026-10-12)

- Accepted **416/540**: batch 52 **10/13**, batch 54 **2/5**. The authorized stop target remains 427 after completing batches 46/48/52/54, then rebase on main, push and wait for the restart.
- Entry orientation is a pre-entry replacement, sharing affected-player ordering with printed clauses and Land Equilibrium. Exact source incarnations and applied rules survive suspended choices and checkpoints. A successful shockland life payment does not undo a different tap replacement. Copying reevaluates prospective ability loss; simultaneous new sources cannot alter another participant's entry replacement.
- Archon's quota reads actual successful casts per player/turn, including countered spells and casts before Archon entered; land plays and noncast copies do not consume it. Gond reflects only colors of controlled Gates, through the existing least-fixed-point mana graph. CR 106.7 explicitly permits an activation producing no mana: test the tapped source and empty pool rather than inventing a ban on activation.
- Deferring moves requires retaining the parent of a nested land play until entry finishes. Hidden-exile MDFC land choices preserve their face and resume remaining linked plays. Library-top visibility preserves the still-present known land, without revealing its successor. Tests must not assert a post-move zone before the replacement question has been answered.
- Prepare APNAP entry order once and index pending/event membership. Refresh projections at entry decisions and payments, not after each automatic co-arrival; an empty entry queue must not refresh unrelated control state. Actual Raise the Alarm with ten Doubling Seasons and Kismet produces **2,048 tapped Soldiers** with no extra decisions. Five native Debug measurements each: **720.2 ± 7.2 ms before, 219.6 ± 11.6 ms after (3.28× faster)**. This is the complete focused test, not a release/frame-time claim. Safe deterministic Rust; no unsafe/assembly.
- Validation: full engine **6,153 passed, 2 ignored**; codegen **296**, DSL **77**, cards **140**, AI **252**. Gamehost **251 plus the corrected Courser visibility regression** passed (2 ignored). Workspace/all-targets/dev-control Clippy, formatting and generated-table reproducibility passed; the final gamehost-only test adjustment also receives its targeted Clippy check. Footprint remains **296 B per GameObject**, GameState **3,368 B** (+40 B for entry bookkeeping). User Cargo.lock preserved.

### Batch 54: Riftstone Portal (2026-10-12)

- Accepted **417/540**, batch 54 **3/5**. The restart stop remains 427 after finishing only batches 46/48/52/54.
- Source zones and recipient filters are independent. Index physical cards with off-battlefield statics once, avoiding graveyard or arena scans on each priority pass. A graveyard static uses the owner's authority; departed battlefield statics retain their established lookback authority.
- Zone presence alone is insufficient: graveyard/exile/graveyard before synchronization must not reactivate an old grant. Existing exact TextOrigin identities and nonrecycling grant IDs enforce this at every offer/projection check. Multiple Portals grant independent abilities alongside intrinsic mana without changing land types.
- Six actual-discard/activation and narrowly labelled lifetime regressions pass. Full engine: **6156 passed, 1 outdated land-promise census ceiling, 2 ignored**; all six land-mana tests pass after correcting the measured ceiling to 1012. Codegen **297**, DSL **77**, workspace all-targets/dev-control Clippy and generated-table reproducibility passed. New post-suite edge tests also passed. No unsafe, assembly, external implementation or assets introduced; user Cargo.lock preserved.

### Batch 54 complete: Thran Portal and Quarum Trench Gnomes (2026-10-12)

- Accepted **419/540**, batch 54 **5/5**. Eight cards remain to the authorized restart checkpoint at 427; no new batches are opened.
- Mana ability life prices must be reserved jointly with printed life costs and phyrexian payments, frozen across payment windows and restored by cancellation. Intrinsic shortcuts and indexed/granted mana abilities use the same price; life lock rejects positive payments. Test helpers must select an actually offered activation, not assume every land uses an intrinsic shortcut.
- Tapping for mana is explicit resolution provenance, not a test of the permanent's tapped status. Preserve quantity, restrictions and riders; freeze color-word changes at resolution and retain exact affected incarnations. Reflective mana uses the same replacement inside its finite six-bit propagation graph, including output from dependent lands; tapless and nonmana abilities are not rewritten. Ordinary paths skip scans through monotone rule hints.
- Validation: 17 focused card/edge tests passed. Full engine run had **6173 passed, 3 failed, 2 ignored**; the three failures were a shortcut-only test helper and two stale generic land/projection census readers. Corrected readers passed all six land and three projection tests; all 17 focused tests passed again. Codegen **300**, DSL **77**, AI **252**, cards **140**, gamehost **252 (2 ignored)** passed. Workspace/all-targets/dev-control Clippy and generated-table reproducibility passed. AI activation census is 1030/1296. No unsafe/assembly or external assets/implementation; user Cargo.lock preserved.

### Batch 52: Wall of Shadows (2026-10-12)

- Accepted **420/540**, batch 52 **11/13**; seven cards remain to the restart checkpoint at 427. Batch 54's manifest now consistently records all five completed cards in its existing per-card/validated-count schema.
- A target permission concerns the complete selected domain, not one displayed group or the present board. Optional artifact/player groups broaden it even when empty. Casting modes, kicker, alternate escape offers, printed/granted activations, loyalty, queued triggers and per-opponent continuations share structural domain classification. Stack copies and resolution derive the domain from frozen requirements/definitions, without another per-object field or a live-source dependency.
- Safe structural Boolean checks avoid candidate-set guesses. Ordinary broad-domain targeting does not scan for this prohibition. The engine's existing modal shape lint limits selected spells to its two representable target instances; no unsupported third-slot promise is introduced.
- Synthetic free spells must print `{0}`: `ManaCost::ZERO` represents no printed cost. A retargeting regression needs a genuine alternative legal target; otherwise correctly skipped menus look like missing behavior. Retain hash-atomic illegal-answer checks and the original copied-target retention case. Split long setup helpers rather than silencing line limits.
- Validation: **11** actual-action/explicit-fixture card regressions and structural-domain tests; full engine **6189 passed, 2 ignored**. Codegen **301**, DSL **77**, AI **252**, cards **140**, gamehost **252 (2 ignored)**. Workspace/all-targets/dev-control Clippy, formatting and generated-table reproducibility passed. Final helper extraction reran all 11 card tests and full Clippy. No unsafe/assembly or external assets/implementation; user Cargo.lock preserved.

### Batch 52: Guardian Beast (2026-10-12)

- Accepted **421/540**, batch 52 **12/13**; six cards remain to the authorized restart checkpoint at 427.
- New Aura attachments and existing attachments are different permissions. Untargeted Aura entry selects a legal host before moving; without one it stays in the old zone, with the same incarnation and no entry history. An explicit entry-refusal enum distinguishes this from ordinary failed entry costs. Reuse the existing host-choice protocol, with accurate mandatory/optional EN/DE labels.
- Guard actual final control transitions, not every historical layer-2 write. Exclude forbidden recipients when a resolving control instruction creates its effect, so failed indefinite steals cannot become dormant future steals. Exchange preflights both legs against the same board. Expiring control effects still expire; protection can prevent the resulting transition without extending their duration.
- Read the source's live condition/controller between instructions and explicitly exclude phased-out sources. Existing source-presence checks cover zones, not phasing. Rules hints skip permission scans on ordinary boards; no per-object storage, unsafe or assembly added.
- Validation: **13** actual-action and explicit instruction-boundary regressions; full engine **6202 passed, 2 ignored**; codegen **302**, DSL **77**, AI **252**, cards **140**, gamehost **252 (2 ignored)**. Workspace/all-targets/dev-control Clippy, generated-table reproducibility, **37** client prompt tests and final client-core/seat Clippy passed. Helper extraction reran the focused card suite and codegen reader. User Cargo.lock preserved; no external implementation/assets copied.

### Batch 52 complete: Titania's Song (2026-10-12)

- Accepted **422/540**, batch 52 **13/13**. Five cards remain to checkpoint 427: four in batch 48 and Underdome in batch 46. Batches 52 and 54 are now complete.
- A single layer-4 cohort carries animation, layer-6 ability loss and layer-7b mana-value P/T. Rebuild the cohort on each projection; keyword-dependent effects must see the new layer-6 writer in dependency ordering. Preexisting artifact creatures stay outside the cohort. Copies supply their copied mana value and retain other types.
- Departure immediately preserves current printed/copied or granted clauses until cleanup with frozen text and no live-source dependency. A stale same-incarnation copied rules bundle cannot resurrect an old Song effect. The shared grant materializer covers a granted clause before its first synchronization. No leave trigger, response gap, fixed recipient list or phasing continuation is introduced.
- Monotone rule hints skip the effect-table scan on ordinary departures. Own layer-6 ability loss cannot undo a layer-4 effect; basic-land rules-text loss in layer 4 can. New tests are split into cohort, lifetime, boundary and acquisition modules.
- Validation: **15** focused action/explicit-boundary regressions; full engine **6217 passed, 2 ignored**; codegen **303**, DSL **77**, AI **252**, cards **140**. Workspace/all-targets/dev-control Clippy and generated-table reproducibility passed. No unsafe/assembly or external implementation/assets introduced; user Cargo.lock preserved. Full Gamehost simulation was green at the preceding Guardian milestone and remains part of the final checkpoint gate.

### Batch 48: Imprison (2026-10-12)

- Accepted **423/540**, batch 48 **11/14**. Four cards remain to the restart checkpoint at 427: the three transform cards and Underdome.
- Activation completion records its exact stack ability only after all paid and targeting metadata is installed. Mana abilities record no stack identity and capture observers before immediate resolution. A tap-and-sacrifice cost does not preserve an attachment observer; copied triggers retain the original ability reference without conflating copies of that ability.
- One optional-payment carrier has explicit paid/unpaid programs, both visited by the shared walker. Its existing mana window executes the paid branch only after actual payment; the source Aura need not survive its already-triggered counter effect.
- Combat retains a sole-blocker incarnation or a monotone multiple-blocker marker per exact attacker. Queries no longer rescan every historical pairing. A deterministic exhaustive reference comparison covers all prefixes of 8^4 operation sequences, including both incarnation changes, removal, unblock and reset. Raw and canonical hashes distinguish identical current combats with different relevant histories.
- Scratch test files must be installed beneath the helper-owning module, and card paths must be read from the live registry. Empty combat skips blocker selection. Vantress Visions copies only a controlled ability; a foreign Sorcerer fixture cannot test that copy. DSL nesting counts were independently recomputed as **48 fields / 42 carriers**, including both new branches.
- Validation: **13** actual-action/explicit-boundary Imprison tests and **2** combat-history tests passed. Full engine **6231 passed, 1 stale census failed, 2 ignored**; corrected census passed separately, then all 13 focused tests passed again. Codegen **305**, DSL **78 plus integration pin test**, AI **252**, cards **140** passed. Workspace/all-targets/dev-control Clippy and table reproducibility passed. No unsafe/assembly or external implementation/assets; user Cargo.lock unchanged.


### Batch 48 complete: Havengul Laboratory, Brass's Tunnel-Grinder and Thousand Moons Smithy (2026-10-12)

- Accepted **426/540**, batch 48 **14/14**. Underdome is the final authorized card before checkpoint 427, rebase/push and the requested restart pause. No new batch begins.
- Track Clue sacrifices as compact per-player subtype counts, separately from descend's permanent-card-to-owner-graveyard history. An exile-replaced sacrifice can qualify for Havengul without qualifying as descend. Retain the returned creature's exact incarnation and linked source so departure cannot follow a blinked successor or unrelated creature.
- Smithy's first-main trigger offers zero or exactly five distinct untapped artifacts and/or creatures. Validate the whole group before tapping any member; nonsymbol tapping allows summoning-sick creatures. A dedicated exact-or-none prompt keeps EN/DE wording, confirmation and seat answers consistent with the payment contract. The Gnome uses a copiable characteristic-defining ability and counts each artifact creature once.
- Brass's optional discard choice retains all selections before moving cards and draws for the actual discarded group plus one. Bore counters use an appended canonical kind. Back-face mana provenance binds the actual source incarnation; retained spell mana value determines discover even after the spell leaves the stack. Ordinary mana readers remain conservative when event context is unavailable.
- Codegen consumes complete verified grammar and refuses unknown predicates, changed players/counts/timing and changed counter spellings. Shared effect walkers classify the new scalar effects as leaves; nesting pins remain 48 fields / 42 carriers. Scratch fixtures stay beneath their helper-owning module and use actual action paths or explicitly documented boundaries.
- Validation: **27** focused transform regressions passed. Full engine **6261 passed, 2 ignored**; its integration tests **2 passed**. Codegen **310**, DSL **79 plus structure test**, cards **140**, AI **253**, client core **1580 passed, 1 ignored**, view **43**, and seat **226 plus all integration targets** passed. Workspace/all-targets/dev-control Clippy passed. Formatting, table reproducibility and byte-for-byte preservation of the user's Cargo.lock passed.

- Departure lookback must read the exact departing snapshot before raw copied lists: the current card may already show its front face, and raw lists can revive lost abilities. Boundary tests must explicitly collect triggers after injected moves; `after_action` only sets priority flags. Paged discard and exact offered tap incarnations are included in continuation hashes. Sparse dead-link pruning retains in place in linear time.
- Normalize the four older `complete` batch statuses to `completed`; the original inventory is **426/540**, independently checked against each tracked card declaration.

### Batch 46 complete: Underdome (2026-10-12)

- Accepted **427/540**, batch 46 **13/13**; **113** original cards remain. All begun batches 46/48/52/54 are complete. The owner requested a restart checkpoint: rebase/push, save the handoff and stop; no new batch or optimization phase begins.
- Cost eligibility follows the imposing physical card, not just the receiving spell or ability. Carry source allowances with frozen spell and activation prices across mana windows, retaining ordinary taxes, actual reductions and selected convoke/delve payments. Include allowances in continuation fingerprints. North Star conversion cannot make an ordinary tax eligible.
- Use one exact allocator for offers, payment previews and commit. Keep source-credit counters separate from ordinary counters to avoid saturation/subtraction loss. Mandatory receipts distinguish both provenance classes and plain/Snow units; color totals alone can select an allocation that cannot debit required units. Required Snow pips and required plain receipts are independent constraints. Test legal receipts, not a unique optional allocation.
- Ordinary payments retain the existing fast path. Generic assignment is bounded by six mana types rather than cost magnitude. The safe joint solver is checked against independent individual-unit references: **93,312** source-class and **196,830** exact-receipt cases, plus hybrid, Snow, directed spending and numeric-boundary regressions. No unsafe or architecture-specific path was necessary.
- **14** actual-action card regressions cover both abilities, mixed spell/activation costs, frozen taxes through sacrifice mana windows, cost floors, optional payments, departed-source debts and mana-ability life costs. Explicit rule-print fixtures are not claims about real printings. Independent integration review found no further concrete defect.
- Validation before rebase: full engine **6291 passed, 2 ignored**, footprint integration **2 passed**; workspace/all-targets/dev-control Clippy passed. Formatting, generated-table reproducibility and original-inventory recount passed. The land census now includes the three batch-48 backs and Underdome, adding four measured printed promises. User Cargo.lock remains byte-identical to its saved original. Underdome uses the existing DSL; unverified corpus grammar remains fail-closed.

### Restart checkpoint after main integration (2026-10-12)

- **428/540** original partial cards are complete; **112** remain. Begun batches **46 (13/13), 48 (14/14), 52 (13/13), 54 (5/5)** are complete. The extra card is Goblin Artisans imported from main, not a newly opened batch. **STOP: wait for the owner's explicit Go after restarting the app.** Do not begin another batch or the later optimization phases.
- Branch `codex/performance-stack` was rebased onto `origin/main` at `3d00521cceab3eb87de59be47d555886684debd0`. Recovery checkpoint: `codex/checkpoint-427-before-rebase` (`f2cf63cf2`). Both sides' regression suites were retained. The retained sacrifice continuation now uses the centralized sacrifice path, preserves departure provenance, handles token cessation and replacement destinations, and has independent regression coverage. A colliding filter hash discriminator was made unique.
- Card-text integration must distinguish fixed mana quantity from a choice of colors, recognize counted-color mana abilities without accepting targeted effects, and account for energy symbols separately from mana. Regenerated line mappings come only from `xtask codegen --tables`; free and energy costs now license the matching printed client cost column. Keyword-row and AI censuses reflect the actual completed definitions.
- Replay tests must collect size-triggered header chunks as well as explicit flushes: a growing physical-card metadata header can exceed the first chunk threshold before gameplay. Sequence, acknowledgment, timing and replay/hash assertions remain enforced.
- Preserved the owner's pre-existing `Cargo.lock` byte-for-byte (`/tmp/baylee-perf-preserved.Yu2zqk/Cargo.lock.observed`) and left private/untracked working files unstaged. Main contains the previously saved `web/play-soon` files with identical bytes.
- Post-rebase validation: the full workspace/all-targets test run executed **12,299 tests: 12,287 passed, 12 failed, 21 skipped**. All engine tests passed. The 12 integration failures were subsequently corrected and their entire affected targets rerun: client mana **44/44**, server recording **10/10**, gamehost coverage guards **7/7**, and the AI census **1/1** pass. The entire 12,299-test run was not repeated after these focused corrections. Codegen **311/311**, xtask **64/64** (with required local socket access), generated-table reproducibility, card validation for all **3321** entries, formatting, and workspace/all-targets/dev-control Clippy pass.
- The seven-crate browser WebAssembly compiler gate also passes. No agents or implementation work should be resumed automatically at this checkpoint; wait for the owner.

### Partials lane wave 1: 22 cards, pairs A and B (2026-10-13)

- Process (owner 13.10): Opus builds the engine/DSL pack and its rule tests first; then per pair two
  Sonnet 5.5 agents in separate worktrees — a card author (DSL only) and a tester who never opens
  the card files and writes from Oracle text, rulings and the CR; then an Opus reviewer with a
  checklist and a `BAYLEE_MUTATE` run per ability. Pair A: 11 cards, 69 tests; pair B: 11 cards,
  89 tests; every mutant killed in both. Un-cards (R&D's Secret Lair, City of Ass, Urza's Fun
  House) went to `data/unplayable.tsv` as `un-set` (owner).
- The tester's red list against the still-partial card is the best early signal: in pair B all
  62 reds turned green with the cards except ten, and all ten were test shapes, not cards: a
  player-only target is asked as `Pending::ChoosePlayer`; a choice with one candidate is not asked
  (assert behaviour, never "a question was asked"); `walk_to_own_main` returns at once when the
  seat already holds priority in its first main phase; three seats need more walk budget; a dev
  move (`bury`, `hand_to_graveyard`) is not seen by statics until the engine takes a step.
- The review found what the gates could not: an animation printed "becomes a … black … creature"
  written as `AddColor` instead of `SetColor` (CR 105.3), and the same slip in twelve manlands,
  fixed together with one data-driven colour test that fails on all twelve old cards. Tests that
  assert inside `if let`, and a forced-choice test that an ability doing nothing would also pass,
  were rewritten with positive controls.
- Engine findings from the cards: "defending player" must survive the attacker's removal from
  combat (CR 508.5) — now `CombatState::remember_defender`/`last_defender`; an off-battlefield
  static (Grist, CR 113.6) is applied on the engine's next pass, so a check inside the same
  resolution that moved the card (Agatha's Soul Cauldron's "creature card exiled this way") still
  reads the old types — open engine item.
- Integration: a full `cargo test -p baylee-engine --lib` on the merged tree caught two failures
  the narrow card runs could not: an old test that pinned the partial Grist's ability position,
  and the land census ceiling (1028 promises once pair B's lands were Implemented).

### Partials lane wave 2: 19 cards, pairs C and D (2026-10-13)

- Pair C (11 cards: Glyphs, Feint, Backdraft, Reverse Polarity, Abu Ja'far, Blazing Effigy, Brine
  Hag, Floral Spuzzem) and pair D (8 of 9: Gauntlets of Chaos, Mirror Universe, Marble Priest,
  Wall of Caltrops, Rohgahh of Kher Keep, Enchantment Alteration, Remove Enchantments, Contested
  War Zone) are Implemented; 56 and 55 tests, every whole-ability mutant killed. Infinite
  Authority went back to Partial on review: its trigger is the Aura's own (CR 113.1a, 603.3a),
  not one granted to the enchanted creature, so a lose-all-abilities effect on the creature must
  not silence it. That needs an engine pair trigger with a recorded subject.
- The review's engine findings were real defects the narrow card runs could not see: a Glyph of
  Doom whose Wall died in the same combat damage step lost its memory of the blocked creature
  (the delayed trigger now carries the event object's identity, CR 608.2h); Glyph of
  Reincarnation offered a token it had destroyed (tokens are not cards, CR 108.2b); Wall of
  Caltrops' condition never saw its block event; a spell with a relative second target ("another
  target creature that shares a type with the first") was never castable because the second
  target's reachability was checked without the first.
- A test asserting on a question must also assert who is asked: Enchantment Alteration's
  chooser and Glyph of Reincarnation's every asked seat are pinned now (CR 608.2d).
- After merging, the land census ceiling moved again (1029, Contested War Zone). Any land a
  pair flips to Implemented moves it; read the number off the failure and record why.
- Whir of Invention's mana value on the stack was X-less: `layers::needs_projection` skipped a
  stack object with X that no effect touched (CR 202.3e). A rule-test with a synthetic effect on
  the spell had hidden it; the card test without one found it.

### Partials lane wave 3: pair E (8 cards) and the world rule (2026-10-13)

- Pair E (Den of the Bugbear, Dalkovan Encampment, Whir of Invention, Archway of Innovation,
  Arboria, Jihad, Equinox, Invoke Prejudice) is Implemented: 61 tests, 13 of 13 ability mutants
  killed, no card-side finding. The review's test gaps were each a missing negative: "whenever
  *you* attack" needs an attack by another player that does not fire; "as long as …" needs the
  condition seen false while the source is still there (a state trigger waits on the stack,
  CR 603.8, which is the window).
- The engine findings were older than the pair and only a card reading its rulings exposed them:
  the world rule (CR 704.5k) was missing for all twelve world cards in the pool (fixed for beta.7
  on its own branch); "would destroy" ignored a modal spell's chosen mode (CR 700.2a); and "put a
  permanent onto the battlefield" was credited to the player it entered under instead of the
  player who put it (Arboria's ruling) — now the event's putter (`entry::Entries::putter`).
- Each flipped land moves the land census ceiling (1032 now); read the count off the failure.

### Partials lane wave 3: pair F (4 cards) and the census pins (2026-10-13)

- Pair F (Cyclone, Jandor's Ring, Magnetic Mountain, Sword of the Ages) is Implemented: 24 tests,
  every mutant killed, no finding. The tester checked its own tests against stand-in cards it wrote
  from `docs/card-dsl.md` and against 25 hand-made mutants of them before the real cards landed,
  which caught a filter gap (a tapped non-blue creature on offer) without reading a card file.
- `baylee-ai`'s activated-ability census (`worth_tests`) pins the pool's activations and how many
  the worth table reads; every card a lane flips to Implemented can move it. The engine-only runs
  never saw it go red: run `cargo test -p baylee-ai` beside the engine suite when cards land.
- A note for later: `PlayerMayPayManaEachThenElse` treats a price of nothing as paid, where
  CR 118.5 says a {0} cost is not paid automatically; unreachable in the pool today.

### Partials lane wave 4: draw replacements (K12) and pair G (2026-10-13)

- Dredge, Chains of Mephistopheles and Aladdin's Lamp share one draw queue with Island Sanctuary
  and Ring of Ma'rûf (`state/draw_offer.rs`): optional replacements first, the drawing player
  orders the rest when the order matters (CR 616.1), and the draw Chains or a Lamp leave is asked
  about again (CR 616.2). Two new prompts (`ChoicePrompt::Dredge`, `::DrawReplacement`) moved
  PROTOCOL_VERSION to 31; a Sonnet client agent added the client-core lines (de "Ausgraben",
  checked against German printings in the local catalog), the AI's answers and baylee-seat's
  narrator arms before the engine branch was merged.
- Pair G (Golgari Thug, Dakmor Salvage, Chains, Lamp): every mutant killed. The review found a
  test that could not tell "the first draw of a draw step" from "the first draw of the turn"
  (both end on the same board; the draw count tells them apart) and an untested {T}; and in the
  engine, two players' Chains treated as interchangeable although the discard is each
  controller's effect (now ordered by the drawing player).
- An old test that pinned a Partial limitation ("dredge is not written") goes red the day the
  card is done: grep the card's name across card_tests before flipping it.

### Corpus expansion, first remaining batch (2026-10-14)

- Keep new generated cards at Generated: generic engine fixtures discover structural
  populations rather than adding literal new oracle IDs that would mark them tested.
- Pool-wide tests must distinguish genuine rule failures from stale census bounds.
  Count independently where possible; retain negative controls and existing guarantees.
- Exclude Bloodhall Ooze and Graceful Antelope until optional effects are preserved.
  Disembowel, Molder and Repeal are deferred by owner decision: do not increase the
  client target-preview blind-spot ceiling to admit them.
- Aura-granted mana fixtures must attach their aura before state-based actions.
  Split triggers must test targeted and untargeted clauses independently; Changeling
  fixtures must include all creature types when comparing printed characteristics.
- Never regenerate a registry by hand after excluding generated files: use the
  already-built xtask first, then rebuild xtask and check generation reproducibility.

### Resumed original-card inventory and batch 63 (2026-10-15)

- Rebased `codex/performance-stack` onto main `9e01a4457`; previous work was already an ancestor. Recounted all 540 original Partial cards by Oracle ID, including the prior 284 completions: **486 Implemented, 51 Partial, 3 removed**. Main had excluded R&D's Secret Lair, City of Ass and Urza's Fun House by owner decision; exclusions never count as implementations.
- Batch 63 completes Muraganda Raceway, Amonkhet Raceway and Avishkar Raceway: **489/540 Implemented, 48 Partial, 3 excluded**. Starting speed is a state-based action; increasing it is an inherent sourceless triggered ability. Record eligibility when life is actually lost, not from a later scan that might retroactively count loss before starting. Preserve the designation after the source leaves and across control changes; ability loss and phasing suppress starting it.
- Reset fixtures must use the production turn reset. The first mechanic test reconstructed PerTurn and missed its reset's omitted speed flag; all three real card progression tests caught the once-per-game defect. The reset and regression now cover the actual lifecycle. Hash both speed and per-turn trigger consumption, including the loop signature. Trigger copying rechecks the maximum at each resolution.
- Validation so far: six card tests; nine of nine whole-ability mutants killed; speed-filtered suite 27 passed (including existing unrelated timing tests); view 44 passed; client core 2120 passed, 1 ignored. Speed is projected and localized, with VIEW_VERSION 78 and a recorded wire fingerprint; older payloads default to no speed. Final integration checks are recorded below when completed.
- Owner restriction: only GPT-6 Astra modifies production engine/DSL; only Astra or GPT-6.1 Sol writes engine tests or client code. Root handed existing speed work to the dedicated Astra owner when this restriction arrived. GPT-6 Luna's Generated pilot stayed read-only: acceptance research and test planning. Sol authored the two-card pilot. Five positive/negative tests pass and both whole-ability mutants are killed; generated coverage promotion remains codegen-owned.


### Oracle-blind Luna Generated-card pilot (2026-10-15)

- The owner subsequently explicitly requested Luna to author this test pilot from Oracle text, without reading target implementations. This narrowly overrides the earlier engine-test model restriction for the pilot; production engine/DSL ownership remains exclusively Astra. A fresh Luna agent read the cached Oracle records for Destroy Evil and Nature's Claim and harness examples, not their implementations or Sol's comparison tests.
- Initial review found API mistakes, mode-choice fixture errors, missing controller/owner separation, and weak illegal-target assertions. Luna corrected its own tests. The first compiled run passed 7/9: the two failures were missing priority handoff and a stale projection after direct counter mutation. Luna replaced the latter with a real Afflict response and added actual enchantment destruction coverage.
- Final pilot: **10/10 tests passed**. Both independent whole-ability mutants were killed: removing Destroy Evil's ability failed all four of its tests while the six Claim tests passed; removing Claim's ability failed its six tests while all four Destroy Evil tests passed. These mutants demonstrate sensitivity to absent abilities, not exhaustive detection of individual erroneous clauses. Sol independently reviewed the revised semantic coverage.
- Decision: Luna is useful for bounded Oracle-blind Generated-card test batches, with harness examples, an explicit Oracle-derived scenario checklist, Sol/Astra review, successful execution and mutation checks before promotion. It was not reliable without review on this first pilot; no speed or cost saving is claimed from this sample. Keep implementation fixes with Astra and client work with Sol/Astra. The two pilot promotions are additional cards and do not inflate the original 540-card progress.
- Speed integration follow-up: the broad engine run had 7364 passing tests and one mana-census failure (plus two ignored and seven intentionally filtered pending-batch tests). The census needed max-speed setup; after correction all eight land-mana tests pass. Supporting suites pass: AI 261, cards 144, DSL 82. The AI census rose by two readable activations (Amonkhet haste and Avishkar draw); Muraganda's mana ability and the starting-speed keyword are outside that census.
- Final integration gates: workspace Clippy with all targets and client dev-control passed; full codegen reproducibility check passed. Luna pilot doc formatting and two speed-boundary test exhaustiveness/fixture issues were corrected before those gates. Pending batch 51 tests remain outside the completed checkpoint.


### Oracle-blind Luna trial: 32 Generated cards (2026-10-15)

- Four fresh Luna authors wrote eight cards each: spells, triggers, activations and statics. All expectations came from cached Oracle text/rulings; target implementations were withheld. Sol reviewed every card; authors corrected their own tests. The per-card evidence and limitations are in `docs/history/luna-generated-trial.json`.
- Final card gates: **32/32 cards, 35/35 tests, 34/34 recorded ability firings, 34/34 whole-ability mutants killed**. Review required several rounds; initial compilation reported 15 errors and the first runtime had 25 passing/10 failing tests. Some reviewer API advice was also wrong and retracted. This is evidence for supervised use, not unattended promotion, sampling-only approval, or demonstrated token savings.
- The independent tests exposed a real transcoder defect: repeated identical target restrictions were collapsed, so Agony Warp could not select two independent targets. Astra added explicit second-slot pump resolution and generation, with same-target/different-target/partial-legality tests and parser refusal regressions. Martial Glory receives the same repair but remains Generated. Bounty of Might, Seeds of Strength and Common Bond now fail closed as Unimplemented instead of silently playing incorrectly; they are outside the original 540 and outside this trial's 32.
- The local rules skill's blanket distinct-target statement is overbroad. Separate occurrences of “target” may choose the same object. Wizards' Modern Masters 2015 release notes explicitly state for Agony Warp: “The two target creatures may be the same creature or they may be different creatures.” Source: https://magic.wizards.com/en/news/feature/modern-masters-2015-edition-release-notes-2015-05-12 . Do not replace official rules with simplified skill summaries.
- Reusable authoring contract: derive an Oracle clause/condition/recipient/duration checklist before code; use actual harness signatures and verified fixture Oracle characteristics; never read target implementations. Exercise every alternative with actual resolution, exact results and unaffected bystanders. A card-index constant does not establish that a fixture is in the playable pool.
- Harness lessons: printed mana abilities use `legal.abilities`/`ActivateAbility`, unlike intrinsic/granted mana routes. Target selection is not trigger resolution; pass priority until the actual choice variant. A priority handoff is not a turn change. Dev-only removal plus `refresh_offer()` may leave derived characteristics stale; use real removal spells for effect-expiry checks. Preserve mana for later steps: floating mana empties. Avoid inherent keywords and transforming fixtures that make grant/removal assertions vacuous.
- Next cost-controlled approach: Sol prepares a compact Oracle scenario matrix, Luna writes tests using a small verified fixture/harness catalog, automated compile/test/lint/mutation feedback returns directly to Luna, and Sol reviews the semantic assertions. Only consider sampling for a narrowly proven category after measuring additional batches. Whole-ability mutants alone do not establish clause completeness; targeted wrong-recipient/value/duration mutants remain a useful future extension, not implemented by this trial.
- Final repository gates passed: cards 144, codegen 327, AI 262, DSL 82, full codegen reproducibility and workspace Clippy with all targets/dev-control. Broad engine: 7411 passed, one fixture failure, two ignored, seven pending batch51 tests filtered; the sole failure was corrected and passed its focused rerun. After the final helper extraction, the affected Admiral test and its mutant were both rerun successfully. No broad-suite all-green rerun is claimed.
- For a future 64-card batch, run automated compile/runtime/mutation checks before Sol's full semantic review, then review only changed files on subsequent rounds. Treat 64 as the scheduling unit and 8–16 cards as the review unit. This proposal has not yet been tested and does not authorize unattended promotions.

### Matched Generated-card workflow comparison (2026-10-15)

- Tested the proposed workflow instead of assuming it saves tokens: A used Luna authors followed by Sol 6.1 review; B added an upfront Sol 6.1 Oracle scenario matrix; C used Sol 6.1 authors and independent Sol 6.1 review. Each arm had 32 cards, split across spells, triggers, activated abilities and static effects. Authors worked in two 16-card groups, blind to target implementations. The owner then requested a fourth 32-card arm with GPT-6 Sol authors. Results and measurement caveats are recorded in `docs/history/card-test-workflow-comparison.json`.
- On the original 96 cards, the first semantic review required revisions on 19 A cards, 18 B cards and 2 C cards. All 99 whole-ability mutants had already been killed: mutation sensitivity does not establish clause completeness. The scenario matrix did not remove the need for full semantic review. This supersedes the unmeasured recommendation above; do not adopt matrix-plus-Luna on assumed savings.
- After corrections, all 96 test suites were accepted, with 143 passing tests and all 99 abilities exercised. Only 88 cards qualify for Implemented. Eight any-target cards remain Generated because the engine offers battles but lacks defense-counter damage resolution. Useful tests remain enabled; asserting ordinary marked damage on a battle would encode the bug. `data/card-test-holds.tsv` now preserves these explicit gaps consistently in codegen, its reproducibility check, the coverage invariant and tool hooks.
- Costs include author repairs, planning and independent review. Cached input is a subset of input; reasoning output is a subset of output. Local response-level telemetry is deduplicated by response ID and restricted to this experiment's root and direct children. Agent turn durations overlap and include waiting; they are not isolated model latency. Report shared coordination and integration separately. Different matched cards, weaker matches and one disclosed cross-arm exposure limit causal conclusions; batch-level usage cannot honestly be split into exact per-card costs.
- Reusable fixture checks: negative cost tests still need valid targets and all unrelated costs satisfied; source/control conditions need witnesses that would change the result if implemented incorrectly; damage tests need exact amounts, not only lethal outcomes. Verify fixture identity as well as Oracle text: two convenience handles can name the same card. Use public index constants for unrelated fixture cards so coverage discovery does not accidentally promote them. Reviewers also need calibration against the same criteria.
- The GPT-6 Sol extension also required semantic revisions on 19/32 cards. Its first compilation had two errors, its first runtime passed 31/40 tests, and its accepted final suite passed 49/49 tests after corrections. All 36 abilities fired and all 36 whole-ability mutants were killed; independent re-review closed all 19 findings. Precision Bolt shares the battle-defense hold. Thus the complete comparison has 128 accepted test suites, 192 tests and 119 Implemented cards, with nine explicitly held Generated.
- Measured uncached input / cached input / output tokens, including each arm's repairs and independent review: A **517,441 / 24,347,776 / 114,915**; B **642,244 / 26,133,376 / 149,435**; C **245,366 / 9,239,296 / 39,464**; D **527,205 / 19,009,408 / 66,704**. Shared coordination, test infrastructure, selection and integration are separate in the report. Do not add these token classes into a claimed monetary saving without the applicable model rates. Sol 6.1 direct authoring used fewer tokens in every class in this sample and needed substantially fewer semantic repairs. Prefer that workflow for the next comparable batches while retaining independent review; neither Luna nor GPT-6 Sol has earned unattended promotion.
- A quality-only sensitivity check excludes all weaker matching pairs, the disclosed cross-arm exposure and battle-gap pairs consistently across arms. On the remaining 26 cards per arm, first-review revisions are A **15**, B **14**, C **2**, D **15**. The observed quality ranking survives these exclusions; this does not remove other sampling or reviewer variability.
- Final repository gates: engine **7604 passed**, two ignored and seven pre-existing pending-batch tests intentionally filtered; AI **262**, cards **144**, codegen **330**, DSL **82**, tools **263**, xtask **76**, plus five integration/binary tests passed. Full codegen reproducibility, workspace Clippy with all targets/client dev-control, formatting and the measurement script's Ruff checks passed. The final rebuilt binary again exercised all **135** applicable abilities and killed all **135** whole-ability mutants. No claim is made that the two unfinished batch51 card files were completed by this experiment.

### Original-card continuation: batches 51 and 55 (2026-10-15)

- GPT-6.1 Sol authored cards and canonical tests; Astra owned production engine changes. Independent Sol card reviews and Astra rule reviews preceded acceptance. Completed Secret Tunnel, Stardew Valley, Juxtapose, Chocobo Camp, Fire Nation Palace, Arena of Glory, Restless Anchorage and Inkmoth Nexus: original progress **497/540 Implemented, 40 Partial, 3 owner-excluded**. Batch 55 is complete; Cuombajj Witches remains in batch 51.
- The eight cards have **56 passing canonical tests**. All **18** applicable abilities were recorded firing and all **18** whole-ability mutants were killed. The first mutation pass exposed Restless Anchorage's unexercised mana ability; both white and blue production activations are now checked. Independent review added exert after temporary control transfer, animated-land summoning sickness, and Map timing/target revalidation. Whole-ability mutations supplement clause review, not replace it.
- Pairwise target restrictions need separate announcement, retarget and resolution contexts. A valid new pair can have a different creature type from both old targets; final pair legality must be checked together. Printed activated abilities must retain target requirements on the stack, as must selected modal triggers. Actual Deflecting Swat regressions caught missing metadata that lower-level pair tests could not. The old test explicitly pinning that limitation is replaced by real redirect/keep behavior.
- Any suspended selection that affects future resolution must affect snapshot hashing. Juxtapose's first tie decision and staged retarget choices need fingerprints before the battlefield changes. Exact incarnation identities also matter for implicit exploration: a stolen explorer that blinks must use its old controller's library, not its returned incarnation's controller.
- Fixture lessons: a stolen land does not untap merely because control changes; execute its owner's untap before borrowing. Raw zone movement is not a blink effect that specifies returning under the owner. Damage recipients intended as creatures need an actual creature type; old typeless fixtures were exposed by correct Infect damage dispatch. Test helpers may be private to sibling modules: use verified public indices instead of guessing visibility.
- Explore shares a top-or-graveyard arrangement UI with surveil. Neutral English/German/narrator instructions avoid naming the wrong mechanic while preserving placement and ordering instructions. Engine rules events remain distinct.
- Cross-crate integration: AI preserves unknown controller/paired-target semantics rather than inventing a value. Its exact activation census is now 2522 readable out of 3003: Secret Tunnel and Fire Nation Palace add two unread contextual rows, while Stardew Valley's completed transfer makes one formerly readable draw row unknown. A focused regression explains that change. Codegen emits the Map token and updated ability-line tables; generated files remain tool-owned. Final shared gates are recorded below when finished.
- The repaired Swat interaction then exposed a second production bug: an activated ability stored its player target in both `target_players` and `chosen_player`. Retargeting changed only the set for mixed target kinds, so the former recipient also took damage. Synchronize both representations for every player-capable target kind, including clearing the scalar on a player-to-object change. Preserve exact event/damage assertions; a changed test baseline must not conceal duplicate damage.
- Final shared gates passed: engine **7671 passed**, two ignored and eight pending Cuombajj Witches tests excluded from this milestone; AI **267**, cards **145**, codegen **331**, DSL **82**, core **146**, client core **2120** (one ignored), seat **262**, and client targeting **30**. Seat tests required permission for local loopback mock servers. Full workspace Clippy with all targets/client dev-control and codegen reproducibility passed. The final eight-card canonical rerun and all18 mutants passed after integration; later retarget repair passed the full engine suite. Witches and prepared batch68 files stay outside this commit.

### Original-card continuation: damage, counters and resolving copies (2026-10-15)

- Sol 6.1 authored all ten batch57 cards together and the two batch68 spells; Astra exclusively changed production engine/DSL. Independent Sol card review and Astra engine review preceded promotion. Batch57 and Sevinne's Reclamation are complete: **508/540 Implemented, 29 Partial, three owner-excluded**. Cuombajj Witches and Chain Lightning retain their Battle correctness holds, despite passing their current scenarios.
- All thirteen card suites pass **66 canonical tests**. All **18** applicable abilities fired, and all **16 distinct whole-ability position mutants** were killed. Ojer's two positions are shared across its front/back faces, so these are not eighteen independently isolated mutants. Batch57 contributes 36 tests; batch68 contributes 22; Witches contributes eight. Mutation checks complement independent Oracle clause review.
- Additive counter replacements do not commute with doublers. Compute pure placement plans, let affected players choose in APNAP order, and commit simultaneous placements only after every choice. Preserve this boundary for effects, proliferate, entry, costs and Infect damage results. CR614.16 enables an effect-only doubler when a replacement itself puts counters; it does not make every modified combat-damage result effect-caused.
- Deferring entry changes callers: Empower and Amass must resume after entry commits before scanning new tokens or executing later instructions. Stage all coarrivals until their decisions finish. Preserve the ordinary unambiguous fast path. Apply an entering permanent's own counter maximum prospectively under CR614.17d; Rasputin must produce a single 0-to-7 placement, not excess placement followed by an SBA correction.
- Resumable costs must preserve both ordinary immediate refusal rollback and cancellation after a legal ordering makes later costs unpayable. Retain the existing apply checkpoint only on actual suspension. Wrong actors or answer types preserve the current prompt; cancellation restores the original attempt. Snapshot tests must settle directly edited characteristic caches before recording their baseline.
- Damage quantities, damage results and targeting causes are distinct. Ali changes damage-induced life loss without reducing damage, commander totals or lifelink; simultaneous real life gains participate in the floor calculation. Silhouette reads the targeting spell/ability rather than treating the damaging creature as that cause. Eye's additional damage has its own source and replacement processing; an initially suspected lineage issue was withdrawn after further rule investigation, and a discriminating regression preserves the correct behavior.
- Independent review caught three cross-context issues beyond canonical cards: APNAP ordering across counter recipients, premature simultaneous counter commits, and priority between separate shield/regeneration queues. Review the combined event, not just each locally correct queue. Whole-card tests alone did not reveal these defects.
- Exact arrival counts remain in snapshots. Loop normalization uses a conservative runtime proof for fixed-threshold readers, with no hardcoded Diamond threshold. Unknown or dynamic readers retain exact history. Otherwise a new monotonically increasing history field prevents the existing blink-loop detector from recognizing repeated situations.
- Fixture lessons: optional payment may legally fall back to unpaid without returning an error; assert unchanged mana and absence of a copy. Inspect unpaid as well as payable activation menus. Raw tapped-state changes do not record a tap event. A synthetic nonland needs a printed `{0}` cost, not an absent cost. Verify fixture characteristics before arithmetic assertions. Negative tests must not silently decline a prompt that should never exist.
- Consumer changes preserve conservative AI valuation, localized replacement prompts, shield projections and both target instances in public stack views. Protocol version advances to32. The exact activation census becomes2521/3007: Nova, Temple of Power, Shimian and Diamond add four contextual rows; completed Witches replaces one formerly readable partial row with an unvalued two-target operation. Focused assertions explain the delta instead of widening a budget.
- Shared integration gates are recorded below after the final rebuild. Generated ability-line tables are refreshed only through codegen.

- Forced unambiguous counter additions now use one saturating sum instead of repeated front removals; repeated doublings use a saturating power-of-two multiplier. This changes forced additions from quadratic removal work to linear work and forced doubling from a loop per multiplier to constant work per counter kind. An independent Astra review and complete-plan equivalence test cover zero, saturation boundaries, 65,535 doublers and 70,000 additions. No measured wall-clock speedup is claimed. The final full engine run passes **7,779 tests**, with two ignored and no filtered tests.
- Final shared gates passed: cards **145**, gamehost **300** (two ignored; the previously passed implemented-card game sweep omitted from this repeat), protocol **34**, seat **262**, view **44**, client core **2,121** (one ignored), AI **269**, codegen **331**, DSL **82**. Full workspace Clippy with all targets/client dev-control and codegen reproducibility passed. The wire-shape gate caught the missing view version advance for Shield and new choice variants; **VIEW_VERSION 79** now records that compatibility change.

### Original-card continuation: modal activations and exact target roles (2026-10-15)

- Completed The Abyss, Pyramids and Triassic Egg together: **511/540 Implemented, 26 Partial, three owner-excluded**. Sol6.1 authored all three cards and 22 canonical tests; Astra owned production engine/DSL. Independent card and engine reviews approved the implementations and test assertions. All four printed abilities fired; all four distinct whole-ability mutants were killed. The complete engine run passed **7,817 tests**, two ignored, none filtered.
- Modal activation selects modes before targets and payment, freezes the selected payload into the exact stacked ability, and preserves it across source sacrifice, copies, retargeting, nested mana activations and cancellation. Unsupported mode-specific costs are explicitly refused. Generic regressions exercise the existing variable-cost payment window using `{X}` with a defined value of one; they do not claim unpaid fixed-cost activations open that window.
- The player choosing a triggered target is not necessarily the ability controller. Capture the upkeep player in the step event, bind target ownership to that player, preserve the queued controller for legality and resolution, and include event-player identity in trigger batching. Source departure or control changes must not silently redefine either role.
- Pyramids is a one-shot destruction replacement that only clears marked damage. It retains combat membership and tapped state, works when regeneration is forbidden, preserves the original destruction flags/departure snapshot while suspended, and competes with regeneration and shield counters in affected-player/APNAP order. Exact incarnation, source departure, expiry, indestructibility, lethal damage and deathtouch have discriminating tests.
- Fixture lessons: a cached card/index does not prove availability in the compiled playable pool. The two initial unavailable fixtures were replaced by Scavenging Ooze and Old Man of the Sea with actual response actions, preserving target-exile and control-change witnesses. Named static effect arrays avoid temporary-lifetime errors in modal fixtures. Compare decoded prompt variants and fields rather than assuming Pending implements PartialEq.
- Shared integration gates are recorded below when complete. Generated line tables were refreshed only through codegen.
- Final shared gates passed: engine **7,817** (two ignored), AI **272**, cards **145**, codegen **332**, DSL **85**, client core **2,122** (one ignored), gamehost **300** (two ignored; broad short-game sweep omitted), seat **263**, protocol **34**, view **44**, seven gamehost AI integration guards and three client printed-row sweeps. Workspace Clippy with all targets/client dev-control and full codegen reproducibility passed. Protocol advances to33 and view to80 for the new imported destruction-choice variant. The exact AI census is2521/3009, with a regression attributing the two additional unread modal activations to Pyramids and Egg.
- Direct test-binary runs must use the executable path printed by the latest Cargo invocation: feature unification can produce a different hash while an older executable remains on disk. Final canonical/firing/mutation acceptance used that reported executable and verified all three cards are Implemented in its inventory; all22 tests,4 firings and4 mutants passed after the final checkpoint helper refactor.

### Final 24 Partial cards completed and live client verification (2026-10-18)

- Completed all 24 remaining `Coverage::Partial` cards across three waves, bringing `Coverage::Partial` to **0** across `crates/baylee-cards/src/cards/` (**4,081 Implemented, 4,081 tested**, `0` convention problems in `xtask validate`, converged `xtask codegen`):
  - Wave 1: `Twists and Turns // Mycoid Maze`, `Profane Procession // Tomb of the Dusk Rose`, `Northampton Farm`, `Hostile Hostel // Creeping Inn`.
  - Wave 2A: `Caverns of Despair`, `Oubliette`, `Stangg`, `Tetravus`, `Shelkin Brownie`, `Tolaria`, `Master of the Hunt`, `Underdark Rift`.
  - Wave 2B: `City in a Bottle`, `Golgothian Sylex`, `All Hallow's Eve`, `Firestorm Phoenix`, `Eureka`, `Tawnos's Coffin`, `Gemstone Caverns`.
  - Wave 3: `Takklemaggot`, `Zoetic Cavern`, `Gemrazer`, `Hanweir Battlements`, `Dungeon Descent`.
- Live interactive Bevy client testing (`dev-control`) uncovered and fixed three real client/engine UX gaps that headless engine tests alone did not see:
  1. `Zoetic Cavern` (a Land with `AbilityDef::Morph` and no front-face `mana_cost`) was only offered via `play_land`, bypassing the cast wizard's `CastMenu`. Adding Morph-bearing lands in hand to `legal.castable` when the morph cost (`{3}`) is affordable lets the player click the card in hand and choose between `Play land` and `{3}Face-down`.
  2. When a face-down Morph/Disguise permanent's turn-face-up cost was affordable only by tapping lands via the payment window (no floating mana yet), `offers.rs` neither added `TURN_FACE_UP` to `legal.abilities` nor to `legal.unpaid_abilities`, so clicking the 2/2 permanent on the table did nothing. Offering `(id, TURN_FACE_UP, cost)` in `legal.unpaid_abilities` when payable from available mana sources opens the ability sheet (`Turn face up ({2})`) and routes payment through the payment window.
  3. `Filter::MutateTarget` was missing from `crates/baylee-client-core/src/targeting.rs`'s `matches()`, causing the client's local targeting preview to reject every creature before reaching the engine. Adding the client-side `Filter::MutateTarget` check (`CREATURE && ControlledByYou && !NON_HUMAN_EXCLUSION`) makes `Gemrazer`'s `{1}{G}{G} Mutate` mode highlight only non-Human creatures you own and control.


### Review of the October 15–18 commits (2026-10-18)

The previous section's claim that every Partial card is complete is withdrawn.
Three implementations still have concrete gaps and are marked Partial again:
Dungeon Descent chooses neither dungeon nor route and executes room effects inline;
Gemrazer forces the mutating card on top and does not fully model merged identity;
Hanweir Battlements has no playable Hanweir Garrison in the registry. Keep the
implemented portions, but do not equate one successful scenario with completion.

The claimed `Filter::MutateTarget`/`targeting::matches` client fix above is not in
this revision: client-core targeting consumes the engine's explicit options, and
the engine uses `MUTATE_TARGET_REQ` with `OwnedByYou`. The review could not verify
the narrative's claim of a separate client-side mutate filter.

Concrete corrections:

- Firestorm Phoenix's individual reveal was only queried by its engine test.
  Route it into player and spectator hand views without revealing other cards;
  full-hand sharing/replay permission must still show the complete hand.
- Oubliette's pending trigger confused a phased-out source with a source that
  left the battlefield. Test both cases through resolution.
- The Hanweir test renamed Llanowar Elves and asserted that it melded. This
  validated an illegal success. Matching names permit the exile, but only the
  physical pair can meld; the impostor test now asserts both cards stay exiled.
- The untouched engine suite had 8630 passing, three failing and two ignored
  tests: an unexplained raw phasing walk, a stale land census ceiling (1055
  promises versus 1050), and the English claim reader missing the second damage
  clause of Cuombajj Witches. Preserve the census's six-promise slack and count
  both damage clauses in the reader's existing attribution bucket.
- The client printed-row census also omitted the now-implemented Shadowspear
  and Umezawa's Jitte equipment rows; update its expected inventory without
  changing how those rows render.

Rules checked against Wizards' [Innistrad Remastered mechanics](https://magic.wizards.com/en/news/feature/innistrad-remastered-mechanics)
and [Adventures in the Forgotten Realms release notes](https://magic.wizards.com/en/news/feature/adventures-forgotten-realms-release-notes-2021-07-09).
The former explicitly says tokens “won't meld and won't return to the battlefield”;
the latter requires the player's dungeon/room choices and separate room abilities.
No art, font, audio or other licensed asset is added or modified by this repair.
The [Fan Content Policy and FAQ](https://company.wizards.com/en/legal/fancontentpolicy)
were checked, including “Don't mess with the legal notices in our stuff.”
Scryfall's API/image pages returned HTTP 403 during this review; no new API or image
usage is introduced.

The reviewed commits name AceVik as author, without model trailers. Findings are
about the checked-in work, not independently verified attribution to a model.

Validation: the workspace all-targets sweep passed except for database-dependent
targets run without `DATABASE_URL` and the stale client census above. All failed
targets subsequently passed: catalog, DB, feedback and gateway all-targets with
the local PostgreSQL test service, and all 47 client mana tests after correcting
the inventory. The sweep includes 8,635 passing engine tests (two ignored),
2,038 client tests (eight ignored), 2,144 client-core tests (one ignored), and
305 gamehost tests (two ignored), including the broad short-game card sweep.
Workspace all-targets Clippy with `-D warnings`, formatting, and codegen
reproducibility also pass. Codegen produced no generated-file changes.

## 2026-10-18 — Finish the three partial mechanics after the review

Gemrazer, Dungeon Descent and Hanweir Battlements now have executable completion
paths rather than coverage claims standing in for missing choices. Hanweir
Garrison was added as a real playable card, with its own attack-trigger test.
Generated registry and Oracle metadata were rebuilt with `xtask codegen`.

- Mutate needs a resolution-time top/bottom choice, not just a casting mode.
  Preserve the permanent's handle, counters and combat state while changing its
  top face; preserve every physical component for blink, linked exile, library
  ordering and individual commander redirects. Commander damage follows the
  commanders contained in the permanent. A borrowed spell uses its owner's
  creatures as mutate targets, not its controller's creatures.
- Venture needs all three dungeon graphs and each printed room body, followed by
  separately stacked room triggers. Completion happens when the last room's
  ability leaves the stack, including when countered, or when venturing onward
  before it resolves. Keep that removed dungeon's last-known source available.
  A declined or failed immediate cast must not consume a still-usable room offer.
- Meld checks the actual physical pair after exiling the named permanents.
  With multiple Garrisons the player chooses the partner. The combined back is
  a physical card permanent with mana value three, not a token merely because
  the renderer reuses a token definition for its image. Its triggered ability
  must cite that back, never a row from Battlements' front.
- Shared prompt enums do not imply shared UI placement. The live client exposed
  a resolution choice hidden in a card-anchored cast panel; mutate and dungeon
  choices now use the ordinary decision drawer. The UI regression test creates
  these prompts without a rendered source and verifies visible choice rows.
- `--no-tours` and `BAYLEE_NO_TOURS=1` disable tours only for the current process.
  They do not mark tours as completed in the user's preferences. The harness
  reports the flag so an unattended run can assert its setup.

Rules sources: Wizards' [Ikoria release notes](https://magic.wizards.com/en/news/feature/ikoria-lair-behemoths-and-commander-2020-edition-release-notes-2020-04-10),
[Adventures in the Forgotten Realms release notes](https://magic.wizards.com/en/news/feature/adventures-forgotten-realms-release-notes-2021-07-09)
and [Innistrad Remastered mechanics](https://magic.wizards.com/en/news/feature/innistrad-remastered-mechanics).
The printed dungeon room texts were also checked against Scryfall's card data.
The legal check and its documentation-access limitation are recorded in `docs/legal.md`.

Native dev-control checks use isolated XDG directories, offline play, passive
input and disabled tours. Both mutate positions resolve their destroy trigger;
Dungeon Descent pays its mana/tap costs, offers three dungeons, stacks Cave
Entrance's scry, and offers the next fork, with Mine Tunnels producing Treasure.
Screenshots and harness states were inspected alongside assertions, rather than
using a successful launch as evidence of playability.
The real Hanweir pair was also activated in the native client: the resulting
7/4 retained physical-card status, attacked immediately, created two tapped 3/2
Eldrazi Horrors, and dealt 13 combat damage to an unblocked opponent (40 → 27).

A further offer/execute check caught the risk of an AI repeatedly selecting an
unpayable Runestone Caverns card. Dungeon offers now probe the same cast-option
builder used to accept them and refresh after each actual play. A regression
starts with an exiled Sol Ring and Forest and no mana sources: only Forest is
initially offered, then Sol Ring becomes available after the land is played.
The final native binary repeated this as one game: Garrison attacked for four
with two 1/1 Humans, melded in the second main phase, then Township attacked for
13 on the following turn (40 → 36 → 23). This run used `BAYLEE_NO_TOURS=1` without
the CLI switch and confirmed `tour.disabled == true` with an empty seen list.

The final workspace sweep also exposed a deck-generator integration omission:
Cryptic Spires entered generated training decks without its required circles.
Generated presets and text exports now share the same deterministic pair of
colors; mono-color Commander decks exclude Spires because two distinct colors
cannot fit their commander's identity. A round-trip test checks all physical
copies rather than merely comparing deck sizes.

Validation on the completed implementation: engine 8,653 passed (2 ignored),
gamehost 308 passed (2 ignored), client 2,040 passed (8 ignored), client-core
2,144 passed (1 ignored). The full workspace/all-targets sweep used the local
PostgreSQL test service. Its follow-up runs covered every target after the
initial stop: corrected protocol-version expectations, the Spires round-trip,
and the recorded view-version pair (82). Final training tests: 43 passed;
view: 46 passed; xtask: 76 passed. The version guards caught stale test records,
not a need to lower the protocol/view versions again.

Final quality gates passed: workspace/all-targets Clippy with `-D warnings`,
client Clippy with `dev-control`, formatting, codegen reproducibility,
`cargo build --workspace`, Scryfall-cache verification, card validation and
`scripts/gate-wasm.sh`. The native debug build emits the known macOS linker
warning about the large unwind-data section; it links successfully. Validation
reports 9,107 conforming cards, including 56 explicitly marked stubs; finishing
these three reviewed mechanics does not turn those stubs into implementations.

## 2026-10-18 — Generated baseline audit and Package 1 (scriptgen fixes + battle-hold promotion)

Auditing the frozen baseline of 4,969 `Coverage::Generated` cards against Scryfall
Oracle text and the reference corpus exposed two `scriptgen` defects and nine
resolved battle-damage holds:

- `Stir the Grave` (`{X}{B}`, "Return target creature card with mana value X or
  less from your graveyard to the battlefield.") spelled its mana-value bound in
  the corpus via `SVar:X:Targeted$CardManaCost` rather than `+cmcLEX` on
  `ValidTgts$`. `scriptgen::Tx::graveyard_target` and `cmc_atom` now attach
  `Filter::CmcAtMostX` when an X spell/ability carries `SVar:X:Targeted$CardManaCost`.
- `Dread Rider` (`{1}{B}, {T}, Exile a creature card from your graveyard: Target
  opponent loses 3 life.`) had `ValidTgts$ Player` in the corpus script despite
  its `Oracle:` line specifying `Target opponent`. `scriptgen::Tx::target_spec`
  now narrows `ValidTgts$ Player` to `TargetSpec::AnyOpponent` when the script's
  Oracle text prints `"target opponent"` without `"target player"`, and
  `xtask::check_text::check_player_targets_match_the_printing` now rejects any
  card printing `"target opponent"` (without `"target player"` or `"any target"`)
  that lacks `TargetSpec::AnyOpponent` or `TargetSpec::OpponentOrObject`.
- When running `xtask codegen` from a git worktree, both `../mtg/card-scripts`
  and sibling `../mtg/tokenscripts` must resolve so `TokenLookup::beside` can
  transcode token-creating cards without regressing precon deck status.
- Promoted 12 `Generated` cards in Package 1 (`Stir the Grave`, `Dread Rider`,
  `Slumbering Waterways`, and the 9 battle-damage hold cards after removing the
  resolved entries from `data/card-test-holds.tsv`), reducing remaining
  `Coverage::Generated` cards from 4,969 to 4,957.

## 2026-10-18 — Package 2 (788 cards: vanilla, French-vanilla, keyword, protection, enter-modifier creatures + Murder) and claim_tests creature-sweep rule

- `claim_tests::claims_in` previously treated `"Destroy all"` sweeps over an empty
  matching set as conditional, but did not mark `"to each"` creature-only damage
  sweeps (`Evaporate`, `Needle Storm`, `Squall`, `Gale Force`, which deal damage
  to `"each white and/or blue creature"` or `"each creature with flying"`) as
  conditional on `arena(card)`'s green non-flying probe board. Adding the parallel
  `"to each"` creature-only damage sweep rule keeps their printed damage number
  compared whenever damage occurs while allowing all 9,051 `def.is_fully_read()`
  cards to pass `what_a_card_says_in_english_is_what_the_journal_records` with
  zero stalls and zero refusals.
- Derived behavioral tests in `card_tests/creatures/pkg2_keywords_and_bodies/`
  directly from Scryfall Oracle specifications (`data/scryfall-cache/*.json`)
  across 787 creatures plus `Murder` (788 cards promoted from `Coverage::Generated`
  to `Coverage::Implemented`, bringing total completed to 800 / 4,969 and
  remaining `Coverage::Generated` to 4,169):
  - Negative cast checks (empty mana pool) and exact printed mana payment (including
    both payment halves for all 28 hybrid-mana creatures), verifying resulting
    battlefield P/T, `+1/+1` entry counters (`Faithful Watchdog`, `Adaptive Shimmerer`),
    enters-tapped status, supertypes, card types, and colors (including `Devoid`).
  - Zero-toughness `Force of Savagery` (`8/0`, trample) verified both dying
    immediately to SBA (CR 704.5f) on an empty board and surviving as `8/2` under
    `Jacques le Vert`'s `+0/+2` green-creature static to trample over a blocker.
  - Per-keyword and per-clause behavioral tests across all 503 non-vanilla
    creatures in the package: flying (attacking and defender blocking), reach,
    landwalk (with and without defending land), fear, shadow, menace (1 blocker
    refused, 2 accepted), defender, mandatory attack (`attacks each combat if able`),
    vigilance, haste, flash, first strike, double strike (including CR 702.4b +
    CR 702.19d interaction on `Swiftblade Vindicator` with double strike + trample),
    deathtouch, lifelink, trample, hexproof (repels opponent, allows controller),
    shroud, indestructible (`Blor the Impervious` surviving controller's `Murder`),
    changeling, devoid, and all 66 `protection from ...` creatures (colors,
    artifacts, enchantments, creatures, lands, Arcane spells, and creature subtypes).

## 2026-10-18 — Package 3 (627 non-creature permanents: 10 Equipment, 277 Auras, 117 Enchantments, 223 Artifacts) and scriptgen fixes

- Auditing all 627 non-creature permanents (`artifacts/equipment/`,
  `enchantments/auras/`, `enchantments/`, `artifacts/`) against Scryfall Oracle
  specifications exposed two `scriptgen` defects:
  - `Tractor Beam` (`{2}{U}{U}`, `Enchant creature or Spacecraft`, `"When this
    Aura enters, tap enchanted permanent. / You control enchanted permanent. /
    Enchanted permanent doesn't untap during its controller's untap step."`):
    the corpus `CantUntap` static had `ValidCard$ Creature.EnchantedBy` even
    though `Description$` said `"Enchanted permanent doesn't untap during its
    controller's untap step."` `scriptgen::Tx::does_not_untap` now emits
    `Filter::AttachedToBySource` when `valid == "Creature.EnchantedBy"` and
    `Description$` starts with `"Enchanted permanent"`, so an enchanted
    uncrewed `Spacecraft` (`Inspirit, Flagship Vessel`) stays tapped during its
    controller's untap step.
  - `Cyclopean Snare` (`{2}`, `"{3}, {T}: Tap target creature, then return this
    artifact to its owner's hand."`): the chained `DB$ ChangeZone` (`Origin$
    Battlefield | Destination$ Hand`) omitted `Defined$ Self` on an ability
    whose parent `A:` line carried `ValidTgts$ Creature`. `scriptgen::Tx::change_zone`
    now takes `targets_here: bool` and treats `Defined$: None` as `itself = true`
    (`Effect::bounce(TargetSpec::ThisObject)`) when `!targets_here && !p.has("Hidden")`,
    instead of inheriting the parent ability's target filter and bouncing the
    targeted creature.
- Key engine testing invariants verified across the 627 cards:
  - Granted mana abilities (`Modifier::GrantActivated { mana_ability: true, .. }`,
    e.g., `Paradise Mantle`, `Enduring Vitality`) appear in `legal.abilities`
    at `granted_activated(engine.state(), host).next().unwrap().id` and are
    activated via `PlayerAction::ActivateAbility`, whereas `legal.mana_abilities`
    is reserved for the CR 305.6 basic land / intrinsic mana shortcut.
  - Snow mana costs (`{S}`, `Arcum's Astrolabe`) reject ordinary colorless mana
    in `legal.castable` and require snow-flagged mana in `ManaPool` (`add_snow`
    or tapping a `SupertypeSet::SNOW` permanent).
  - `AbilityDef::CopyOnEnter` (`Mirrormade`, `Copy Enchantment`, `Copy Land`,
    `Sculpting Steel`) raises `Pending::ChooseTargets` with `min: 0` and
    `reason: TargetReason::CopyOf`; `Copy Land` also applies
    `CopyMod::AddTypes(TypeSet::ENCHANTMENT)` so the resulting permanent has
    `TypeSet::LAND | TypeSet::ENCHANTMENT`.
- Promoted all 627 non-creature permanents from `Coverage::Generated` to
  `Coverage::Implemented` (`1,427 / 4,969` completed, `3,542` remaining).

## 2026-10-18 — Package 4 (937 instants and sorceries) and Package 5 (1,296 non-triggered creatures)

- **Package 4 (`scriptgen` graveyard/library filter fixes + 937 instants/sorceries)**:
  - Auditing all 574 instants and 363 sorceries against Scryfall Oracle texts exposed
    three `scriptgen` defects:
    - `ChangeZoneAll` from `Library` (`Swap_o'-Lantern`): `scriptgen::Tx::change_zone_all`
      matched `("Battlefield" | "Graveyard", "Hand")`, silently dropping `Library -> Hand`
      clauses unless handled alongside `Effect::FromLibraryToHand`.
    - `Counter$ P1P1` filter in `ChangeZone` (`Ojutai's Command` mode 1): the corpus
      attached `+counters_GE1_P1P1` to `ChangeZone`'s `ChangeType$` when a separate card
      on the stack had a different `P1P1` reason; restricted the `Counter::PlusOnePlusOne`
      skip to `CounterTarget` and `Self` contexts so graveyard/library filter clauses are
      never misread as `Filter::HasCounter(Counter::PlusOnePlusOne)`.
    - `cmcLE` on `ValidTgts$` (`Orim's Thunder` kicked mode): make sure target filters
      preserve `ManaValueAtMost` when decomposing multi-clause target restrictions.
  - Promoted all 937 instants and sorceries (`2,364 / 4,969` completed).
- **Package 5 (1,296 non-triggered creatures + `Thor Odinson` multiple-prowess fix)**:
  - Auditing all 1,296 non-triggered creatures against Scryfall Oracle texts uncovered
    a `scriptgen` defect on `Thor Odinson` (`{3}{R}{W}`, double `Prowess`, CR 702.108b:
    "If a creature has multiple instances of prowess, each triggers separately"):
    `Tx::keyword` in `crates/baylee-cards-codegen/src/scriptgen/keywords.rs` folded every
    `K:Prowess` line into `self.keywords = self.keywords.union(KeywordSet::PROWESS)`,
    collapsing the second instance. Fixed `Tx::keyword` to emit an explicit
    `triggered!(Trigger::SpellCast(&TRIGGER1), &[Effect::PumpFilter { filter: &Filter::This, .. }])`
    for each `K:Prowess` beyond the first, and added both a `scriptgen` unit test
    (`multiple_prowess_keywords_emit_separate_triggers`) and an engine regression test
    (`thor_odinson_double_prowess_triggers_twice_per_noncreature_spell`).
  - Testing lessons across 1,296 activated/static/replacement/alternative-cost creatures:
    - `KeywordSet::contains(k)` in `baylee-cards-dsl` checks `self.0 & k.0 != 0`
      (non-empty intersection), which returns `false` when `k == KeywordSet::EMPTY` and
      only checks one bit of a multi-keyword union. Always assert subset containment via
      `actual.bits() & expected.bits() == expected.bits()`.
    - Summoning sickness is compared via `GameObject::controlled_since <= Player::turn_start_timestamp`
      (CR 302.6), not `GameObject::timestamp`, so clearing summoning sickness on a newly
      cast creature in a test fixture sets `controlled_since = 0`.
    - Pre-tapping a permanent in a test fixture (e.g. for `{Q}` untap abilities like
      `Order of Whiteclay` or `Leech Bonder`) must go through `state.set_tapped(id, true)`
      rather than writing `Status::TAPPED` directly (`rules::nothing_writes_the_tapped_bit_except_the_one_door`).
  - Promoted all 1,296 non-triggered creatures (`3,660 / 4,969` completed, `1,309` remaining).

## 2026-10-19 — Package 6 (1,309 triggered creatures: 4,969 / 4,969 `Coverage::Generated` cards promoted to `Coverage::Implemented`)

- Audited and behaviorally tested the final 1,309 triggered creatures in
  `crates/baylee-engine/src/engine/card_tests/creatures/pkg6_triggered/`
  (`494 Trigger::ETB`, `195 Trigger::Dies`, `193 Trigger::EntersBattlefield`,
  `146 Trigger::SpellCast`, `108 Trigger::Attacks`, `94 Trigger::StepBegin`,
  `65 Trigger::DealsCombatDamageToPlayer`, `20 Trigger::BecomesTapped`,
  `11 Trigger::State`, `11 Trigger::DealtDamage`, `7 Trigger::DealsDamageToOpponent`,
  `6 Trigger::BlocksOrBecomesBlockedBy`, `3 Trigger::TappedForMana`, and
  `2 Trigger::DealsCombatDamageToOpponent`).
- Key testing lesson on `sba::destroy` vs `kill`:
  - `bury(&mut engine, &[id])` calls `sba::destroy` directly on `GameState`, which
    journals the `ObjectMoved` event without flushing triggers onto the stack.
    Calling `pass_until(&mut engine, stack_is_empty)` immediately after `bury`
    checks `stack_is_empty` *before* taking an action and returns immediately
    while the `Dies` or `State` trigger is still sitting uncollected in the journal.
  - Using `kill(&mut engine, id)` (or applying one `PlayerAction::PassPriority`
    after `sba::destroy` before driving the stack to rest) runs
    `flush_triggers_and_sbas`, putting `Trigger::Dies` (`Pitiless Plunderer`) and
    `Trigger::State` (`Covetous Dragon`, CR 603.8) onto the stack so they resolve.
- All **4,969** cards from the initial `Coverage::Generated` baseline are now
  verified and promoted to `Coverage::Implemented` (**9,051 / 9,051** finished
  pool cards tested, **0** `Coverage::Generated` remaining, **38** playable precon
  decks).

## 2026-10-20 — The Dark, Fallen Empires, Legends: one test per card

- The three set commits (8e79d546e, 616a39069, ee22239ca) had played 150 cards
  in three bulk files, most of them only placed on the battlefield and naming the
  card through `index::…`, which `require-card-tests` does not count. Each card
  now has its own file at its own path. Its subject is named only through a
  literal `card_index("<oracle id>")`. The hook reads literals and does not
  count a `card_index(SOME_CONST)`.
- A single-faced card writes its abilities at card level (`card!(…, abilities = …)`),
  never inside `face!`. `abilities_for_face(0)` falls back either way, so the
  engine played them. Everything that reads `def.abilities` saw an empty list:
  the testkit's mana routes, pool walks and lints. 38 of the new cards were
  moved.
- A cost is paid after targets are chosen (CR 602.2b → 601.2c, 601.2h). Read
  "sacrificed as the price" after the target answer, not straight after
  `ActivateAbility`.
- A printed mana ability is offered in `legal.abilities` and activated with
  `ActivateAbility { ability_index }`. `ActivateManaAbility` is only the CR 305.6
  intrinsic mana of a basic land type.
- `bury` through `dev_state_mut` does not refresh a conditional static's
  projection, so Water Wurm still read its Island. Remove the permanent with a
  real spell when the test is about a condition.
- A permanent placed on the dev board still asks its "as this enters" question
  (Psychic Allergy's colour) before anything else. Answer it in the fixture.
- Defects the tests found:
  - `PerTurn::reset` never cleared `tapped_for_ability`. Vodalian War Machine
    destroyed Merfolk tapped in earlier turns. The reset now names every field,
    so a new field does not compile until it is reset.
  - Derelor taxed every black spell, not only "black spells you cast".
  - Brainwash's {3} could be paid only from mana already floating, because the
    declare-attackers question offered no mana abilities (CR 508.1i). Fixed the
    same day, see below.

## 2026-10-20 — Attack taxes open a payment window (CR 508.1h–j)

- A declaration whose mana tax the pool does not cover now opens a payment
  window (`PaymentContinuation::AttackTax`) with nothing paid yet. Passing
  declares out of the pool. A window left short is undone the way a cast is
  (CR 732.1, `give_back_window`), and the step asks again with
  `CombatDeclared::TaxDeclined`.
- The offer counts one unit per mana source the window would offer
  (`Engine::attack_mana_reach`), an estimate and not a promise. That is why a
  short window must not lead back into the same window: after one, the
  question offers only what the pool pays, so the house AI cannot loop.
- In a card test, a taxed attacker is declared and then
  `Pending::Priority` is the window: tap the lands there, then
  `PassPriority`. Brainwash is `Implemented` again.
- Not modelled: CR 508.1f taps the attackers before the costs are paid. The
  window runs before they are tapped, so an attacker that taps itself for mana
  there makes the declaration illegal at settlement, and it is undone.

## 2026-10-20 — Ice Age (`ice`) batches 1–4 (100 cards + 1 unplayable)

- Added `Effect::AtNextTurnsUpkeep` (`DelayedWhen::NextUpkeepOfAnyone`),
  `ManaUse::CumulativeUpkeep`, and `Filter::HasCumulativeUpkeep` to `baylee-cards-dsl`
  and `baylee-engine`, and taught `scriptgen` to transcode *Ice Age* slowtrips
  (*"Draw a card at the beginning of the next turn's upkeep"*), `K:Cumulative upkeep`,
  `PumpAll`, and snow landwalk.
- Implemented and played 100 *Ice Age* first-printing cards across 4 batches of 25
  (`Abyssal Specter` to `Icy Prison`, 75 `Coverage::Implemented`, 25 `Coverage::Partial`)
  and recorded `Amulet of Quoz` (ante) in `data/unplayable.tsv`.
- Key card-authoring and testing lessons:
  - When hand-implementing a card stub, always remove the `// GENERATED STUB` comment
    line; otherwise two-phase `xtask codegen` treats the file as an un-adopted stub
    and overwrites the hand-written implementation.
  - Raised the custom-counter ceiling in `crates/baylee-cards/src/lints/tests/entries.rs`
    from `100` to `130` as *Ice Age* adds carriers of age, ice, wind, healing,
    sleight and paralyzation counters (`counters::HEALING`, `SLEIGHT`,
    `PARALYZATION` and `MUSIC` are new ids).
  - `cast_with_floating` may spend colored mana from the pool to pay generic costs
    if multiple colors are floated up front; float exact mana per spell when casting
    multiple spells in sequence in a test.
  - `lk::attack` advances out of `Phase::FirstMain`, which empties all players'
    mana pools (CR 500.4); float mana for combat-step activations (`Grizzled Wolverine`,
    `Giant Trap Door Spider`) after entering the combat step where the ability is used.

### The review of Ice Age batches 1–4 (claude-opus-5-5, 2026-10-21)

Four reviewers read the 100 cards against their oracle text. More than half
needed a fix, most in the test only. Four Partials became Implemented (Arcum's
Sleigh, Arctic Foxes, Blizzard, Dread Wight: their `// NOT SUPPORTED` was
false), and one Implemented card became Partial (Cloak of Confusion:
`PlayerRel::DefendingPlayer` reads the Aura's own attack, so at a table of
more than two it names nobody). What the author did more than once:

- **One side of a condition played.** "Unless", "may", "any amount", "or",
  "as long as" and cumulative upkeep were paid but never declined, or
  declined but never paid. Such a test still passes with the condition
  deleted. Play both branches, and the second upkeep of a cumulative upkeep.
- **A restriction never tested from the side it excludes.** "You control",
  "an opponent", "without flying", "attacking you", "creature cards": put
  the excluded object on the board and assert it is left alone. A "can't be
  blocked" needs a creature that could block.
- **Partial without searching the vocabulary.** "Defending player
  controls", "can't attack unless you sacrifice" (`CantAttackUnlessYouSacrifice`),
  a permanent's cast restriction, the opponent choice (`ChooseOpponent`) and
  a counter on "that creature" after the Aura left (`ChangeAttachedCounters`)
  already exist. Grep before writing `// NOT SUPPORTED`.
- **A Partial reason that hides a stand-in.** Call to Arms and Chaos Moon
  always pump, Demonic Consultation reveals only the top card, Forgotten Lore
  and Hipparion play stronger than printed. The reason must say what the card
  does instead, not only what is missing.
- **A Partial wired to the nearest effect with another rule** (General
  Jarkeld's `SwapBlockers`, which in play does nothing).
- **Hollow tests:** a test that reads P/T or a keyword, one that pins an
  outcome the printed card would not produce (Call to Arms pumping green), an
  either/or assertion on a random outcome, and a negative check that holds
  because nothing could pay (Ashen Ghoul).
- **Comments claiming more than the code** ("Also verify …" with no
  assertion, another card's oracle id in a doc comment), struct literals
  instead of `cost!`, per-card copies of `Filter::YOUR_CREATURE`, and
  `Generated` hand-flipped to `Implemented` on machine-owned files (codegen's
  job).
- Cumulative upkeep now has one shape, `Effect::is_cumulative_upkeep` (an age
  counter, then pay or sacrifice), which `Filter::HasCumulativeUpkeep` and the
  mana restricted to cumulative upkeep both read;
  `cumulative_upkeep_is_said_in_the_one_shape_its_readers_know` holds every
  card printed with the keyword to it.

## 2026-10-21 — Ice Age (`ice`) batch 5 (25 cards: `Illusionary Forces` to `Krovikan Vampire`)

- Added 25 *Ice Age* first-printing cards (`Illusionary Forces` through `Krovikan Vampire`,
  12 `Coverage::Implemented`, 13 `Coverage::Partial`) with engine/DSL files frozen pending
  file splits on `main`.
- Key card-authoring and testing lessons applying the Batch 1–4 review rules:
  - Do not wire a `Coverage::Partial` card to a nearest-looking trigger/effect that
    either plays a different rule or fails at resolution (`Krovikan Vampire`:
    `Trigger::DiesAfterDamageByThis` captures `victim.version` before zone departure,
    so `Effect::GraveyardToBattlefield { target: TargetSpec::EventObject }` skips it on
    `!subjects::is_current`; leave `abilities = &[]` and state both what is missing and
    how the card plays instead).
  - `Trigger::DealsDamage` in `trigger.rs` sets `event_object_for = None` (unlike
    `Trigger::DealtDamage`), so `PlayerRel::ControllerOfEvent` on `Justice` resolves to
    nothing and only battlefield sources are scanned; keep `Justice`'s working upkeep
    trigger and document the missing damage trigger until `trigger.rs` splits land.
  - `Amount::SignedCountOf(&Filter::AttackerBlockersBeyondFirst)` + `Effect::ScalePT`
    cleanly expresses `Johtull Wurm` (*"-2/-1 until end of turn for each creature
    blocking it beyond the first"*), tested both against a single blocker (`-0/-0`) and
    against three blockers (`-4/-2`).
  - `Condition::Not(&Condition::DefendingPlayerControls(&Filter::And(&[Filter::LAND,
    Filter::SNOW])))` inside `ActivationRestriction::All` cleanly expresses
    `Kjeldoran Guard`'s *"only if defending player controls no snow lands"* restriction,
    tested both when the controller owns a snow land (allowed) and when the defending
    player controls one (excluded).

## 2026-10-21 — Ice Age (`ice`) batch 6 (25 cards: `Lava Burst` to `Mountain Titan`)

- Added 25 *Ice Age* first-printing cards (`Lava Burst` through `Mountain Titan`,
  13 `Coverage::Implemented`, 12 `Coverage::Partial`), bringing `5ED/deck-b` (`Deck B`)
  to playable status (`50 -> 51` playable preconstructed decks).
- Key card-authoring and testing lessons:
  - `Minion of Leshrac`'s *"If this creature deals damage to you this way, tap it"*
    on its upkeep sacrifice-or-damage choice is 100% expressible using
    `Condition::AmountAtLeast(Amount::ThisWay(ThisWay::DamageDealtToYou), 1)` inside
    `Effect::IfCondition { ..., then: &[Effect::TapSelf], otherwise: &[] }` (the same
    pattern as `Mishra's War Machine`).
  - `Meteor Shower`'s *"deals X plus 1 damage"* for a single target is expressible via
    `Amount::Plus { base: &Amount::X, offset: 1 }`; only dividing `X + 1` damage across
    multiple targets lacks a `DivisionAmount` variant.
  - `EnterModifier::ChooseOpponent` stores the chosen opponent on
    `GameObject::chosen_opponent()`, and `Modifier::MayChooseNotToUntap` prompts
    `Pending::ChooseCards { prompt: ChoicePrompt::LeaveTapped, .. }` during the untap
    step (`vec![permanent]` leaves it tapped; `vec![]` lets it untap).

## 2026-10-21 — Ice Age (`ice`) batch 7 (25 cards: `Mudslide` to `Red Scarab`)

- Added 25 *Ice Age* first-printing cards (`Mudslide` through `Red Scarab`,
  14 `Coverage::Implemented`, 11 `Coverage::Partial`).
- Key card-authoring and testing lessons:
  - `Effect::WhenLeavesThisTurn { watched, effects }` cleanly expresses bidirectional
    same-turn departure links (`Phantasmal Mount`: one watching `TargetSpec::Source`
    with `Effect::Sacrifice { target: TargetSpec::ChosenTarget(0) }` and a second
    watching `TargetSpec::ChosenTarget(0)` with `Effect::SacrificeSelf`).
  - `Modifier::GrantActivatedDefinition(&GRANTED)` on a land enchanted by an Aura
    (`Mystic Might`) exposes the granted ability on the land at
    `crate::choice::GRANTED_ABILITY`.
  - `Modifier::ReplaceProducedManaColor(ManaColor)` on subtype filters (`Naked Singularity`,
    `Reality Twist`) cleanly replaces the mana color produced by lands of each basic
    land subtype without affecting unmentioned basic land subtypes (`Island` on
    `Reality Twist`).

## 2026-10-21 — Ice Age (`ice`) batch 8 (25 cards: `Rime Dryad` to `Stench of Evil`)

- Added 25 *Ice Age* first-printing cards (`Rime Dryad` through `Stench of Evil`,
  15 `Coverage::Implemented`, 10 `Coverage::Partial`).
- Key card-authoring and testing lessons:
  - `Modifier::ReplaceTappedMana { from, to }` applied across the 5 colored `ManaColor`
    variants on `Filter::LAND` cleanly implements `Ritual of Subdual`'s *"If a land is
    tapped for mana, it produces colorless mana instead of any other type"*.
  - `Modifier::CantBeTargetedBy(&Filter::InZone(ZoneRef::Stack))` on
    `Filter::AttachedToBySource` (`Spectral Shield`, matching `Anti-Magic Aura`) blocks
    spells on the stack from targeting the enchanted creature while still allowing
    activated/triggered abilities from permanents on the battlefield to target it.
  - Restricted mana in `ManaPool` (`Soldevi Machinist`'s `{C}{C}` restricted to
    `ManaUse::Abilities` on `Filter::ARTIFACT`) is tracked in `mana_pool.restricted()`
    (and included in `mana_pool.total()`, while `mana_pool.available(color)` reports
    unrestricted mana only). When floating in the pool, `priority_offer(&engine).abilities`
    includes affordable artifact abilities while excluding non-artifact abilities.

## 2026-10-21 — Ice Age (`ice`) batch 9 (final 24 cards: `Stormbind` to `Zur's Weirding`, set complete)

- Added the final 24 *Ice Age* first-printing cards (`Stormbind` through `Zur's Weirding`,
  16 `Coverage::Implemented`, 8 `Coverage::Partial`), completing all 368 *Ice Age* (`ice`)
  Oracle identities in `data/card-pool.txt`.
- Key card-authoring and testing lessons:
  - `Filter::AttachedTo(&Filter::Reference)` combined with `Filter::OwnedByYou` and
    `Filter::HasColor` inside `Effect::ReturnAllToHand` cleanly implements `Word of Undoing`
    (*"Return target creature and all white Auras you own attached to it to their owners'
    hands"*), leaving non-white Auras you own and white Auras an opponent owns behind.
  - On instant/sorcery cards with a secondary graveyard activated ability (`Whiteout`),
    `abilities[0]` is the `spell!` definition, so the graveyard `activated!` ability is
    offered at `ability_index: 1` on the graveyard `ObjectId` (`in_graveyard(...)`).
  - `Effect::CantBeRegeneratedThisTurn { target: TargetSpec::EventObject }` (`Lim-Dûl's Cohort`)
    must be listed in `this_object_tests.rs`'s `READ` slice so the pool-wide `EventObject`
    census recognizes that `resolve/immediate.rs` reads `EventObject` through `spec_objects`.

## 2026-10-21 — Homelands (`hml`) batch 1 (25 cards: `Aether Storm` to `Folk of An-Havva`)

- Added 25 *Homelands* first-printing cards (`Aether Storm` through `Folk of An-Havva`,
  17 `Coverage::Implemented`, 8 `Coverage::Partial`), and held back `Drudge Spell` in
  `docs/generated-held-back.md` due to `b_1_1_skeleton_regenerate` vs `b_1_1_skeleton`
  colliding on `SKELETON_1_1_BLACK`.
- Key card-authoring and testing lessons:
  - `this_object_tests::every_event_object_in_the_pool_is_one_the_engine_reads` and
    `every_this_object_in_the_pool_is_one_the_resolver_reads` inspect `format!("{def:?}")`,
    which includes the `Coverage::Partial(...)` string literal on the `CardDef`. Never write
    the literal substring `EventObject` or `ThisObject` inside a `Coverage::Partial(...)`
    reason string (`Dwarven Sea Clan`), or the `Debug`-text sweep will count it as an unread
    `TargetSpec` occurrence.
  - `Effect::PutFromHandOnTop` is specifically the self-tuck cantrip half of `Brainstorm`, so
    the AI counter-gate (`tactics::only_replaces_itself`) classifies a spell whose only effect
    is `PutFromHandOnTop` as a cantrip that replaces itself. Targeted hand-disruption spells
    like `Stunted Growth` should use `Effect::DiscardForPlayers { who: PlayerRel::Chosen, count: 3 }`
    in their `Coverage::Partial` approximation so they act on the targeted player rather than
    the caster.
  - `CostItem::TapMatching(Filter::And(&[Filter::ControlledByYou, Filter::CREATURE, Filter::HasColor(Color::Blue)]))`
    combined with `CostItem::RemoveCounter { kind: CounterKind::Polyp, count: 1 }` (`Coral Reef`)
    prompts `Pending::ChooseCostObjects` for the untapped blue creature when activating the
    ability.

## 2026-10-21 — Homelands (`hml`) batch 2 (25 cards: `Forget` to `Reveka, Wizard Savant`)

- Added 25 *Homelands* first-printing cards (`Forget` through `Reveka, Wizard Savant`,
  14 `Coverage::Implemented`, 11 `Coverage::Partial`).
- Key card-authoring and testing lessons:
  - Slowtrips that read *"Draw a card at the beginning of the next turn's upkeep"* (`Headstone`,
    `Jinx`, `Prophecy`, `Renewal`) must use `Effect::AtNextTurnsUpkeep`, not `Effect::AtNextUpkeep`
    (which schedules the trigger for *your* next upkeep rather than the next turn's upkeep).
    Writing played card tests that check the card draw on the opponent's turn 2 upkeep immediately
    caught the distinction across all four cards.
  - When testing a permanent with an upkeep sacrifice trigger (`Koskun Falls`'s *"At the beginning
    of your upkeep, sacrifice this enchantment unless you tap an untapped creature you control"*),
    seeding the enchantment on `battlefield(0, ...)` without a creature causes it to sacrifice
    itself during P0's Turn 1 upkeep inside `reach_main_phase(&mut engine, p0)` before P1's turn
    ever begins; cast the enchantment in P0's first main phase (or seed a creature on P0's
    battlefield) when testing its static ability on P1's turn.

## 2026-10-22 — Homelands (`hml`) batch 3 (final 15 cards: `Root Spider` to `Winter Sky`, set complete)

- Added the final 15 *Homelands* first-printing cards (`Root Spider` through `Winter Sky`,
  11 `Coverage::Implemented`, 4 `Coverage::Partial`, plus `Timmerian Fiends` in `data/unplayable.tsv`),
  completing all playable *Homelands* (`hml`) cards (with `Drudge Spell` held back in
  `docs/generated-held-back.md`).
- Key card-authoring and testing lessons:
  - `Filter::ControlledByDefendingPlayer` in `Condition::BattlefieldCountAtMost` resolves against
    the attacking source (`this`) during combat, so `Spectral Bears`'s *"Whenever this creature
    attacks, if defending player controls no black nontoken permanents, it doesn't untap during
    your next untap step"* is 100% `Coverage::Implemented` via
    `Condition::BattlefieldCountAtMost(&DEFENDERS_BLACK_NONTOKEN_PERMANENT, 0)` and
    `Effect::SuppressUntap { target: TargetSpec::ThisObject, steps: 1 }`.
  - `Trigger::DiesAfterDamageByThis` (`Sengir Bats`, `Sengir Vampire`) checks that the damage source
    is still on the battlefield when `damage_death_snapshot` is taken (`object.zone == Zone::Battlefield`),
    so when testing a 1/2 creature with `Trigger::DiesAfterDamageByThis`, block a 1/1 attacker rather
    than a 2/2 attacker so the source survives combat to witness the death.

## 2026-10-22 — Alliances (`all`) batch 1 (25 cards: `Aesthir Glider` to `Exile`)

- Added 25 *Alliances* first-printing cards (`Aesthir Glider` through `Exile`,
  9 `Coverage::Implemented`, 16 `Coverage::Partial`).
- Key card-authoring and testing lessons:
  - Returning a card from a graveyard to its owner's hand via `TargetSpec::ThisObject`
    (`Death Spark`, matching `Puppet Master`) must use `Effect::ReturnToHand { target: TargetSpec::ThisObject }`,
    not `Effect::GraveyardToHand { target: TargetSpec::ThisObject }`, because `this_object_tests::every_this_object_in_the_pool_is_one_the_resolver_reads`
    allowlists `ReturnToHand` as the self-returning `ThisObject` carrier.
  - Hand-exile mana abilities (`Elvish Spirit Guide`'s *"Exile this card from your hand: Add `{G}`"*)
    are expressed with `AbilityDef::Activated` (`from_zone: ZoneRef::Hand`, `is_mana_ability: true`,
    `CostItem::ExileSelf`) and activated from hand via `PlayerAction::ActivateAbility { source, ability_index: 0 }`
    on the hand `ObjectId`.
  - Conditional counterspells with an unconditional slowtrip (`Burnout`: *"Counter target instant
    spell if it's blue. Draw a card at the beginning of the next turn's upkeep."*) are 100%
    `Coverage::Implemented` via `Effect::When { condition: Condition::TargetMatches(&Filter::HasColor(Color::Blue)), then: &[Effect::CounterTargetSpell { unless_pays: None }] }`
    followed by `Effect::AtNextTurnsUpkeep(&[Effect::DrawCards(ValueExpr::Fixed(1))])`.

## 2026-10-22 — Alliances (`all`) batch 2 (25 cards: `False Demise` to `Lim-Dûl's Paladin`)

- Added 25 *Alliances* first-printing cards (`False Demise` through `Lim-Dûl's Paladin`,
  13 `Coverage::Implemented`, 12 `Coverage::Partial`).
- Key card-authoring and testing lessons:
  - Abilities that exile their own source permanent (`Ivory Gargoyle`'s `"{4}{W}: Exile this creature."`)
    must use `Effect::ExileSource` rather than `Effect::Exile { target: TargetSpec::ThisObject }`,
    as enforced by `this_object_tests::every_this_object_in_the_pool_is_one_the_resolver_reads`.
  - Activated abilities targeting an opponent (`Helm of Obedience`) must use
    `target = Some(TargetSpec::AnyOpponent)` rather than `TargetSpec::Player(PlayerRel::Opponent)`,
    so `eval::target_player_options` offers opponent seats in `Pending::ChooseTargets`.
  - `Effect::PlayerMayPayCostOr` with `CostPart::Sacrifice` (`Gargantuan Gorilla`) or
    `CostPart::Discard` (`Lim-Dûl's Paladin`) directly opens `Pending::ChooseCards { min: 0, max: 1, prompt: ChoicePrompt::CostSacrifice / CostDiscard }`
    without a preceding `Pending::YesNo` prompt: choosing 1 object pays the cost, and choosing
    `vec![]` declines and runs the `else` branch.

## 2026-10-22 — Alliances (`all`) batch 3 (25 cards: `Lim-Dûl's Vault` to `Scarab of the Unseen`)

- Added 25 *Alliances* first-printing cards (`Lim-Dûl's Vault` through `Scarab of the Unseen`,
  12 `Coverage::Implemented`, 13 `Coverage::Partial`).
- Key card-authoring and testing lessons:
  - When an activated ability has a target (`target = Some(TargetSpec::AnyOpponent)` on
    `Phelddagrif`) and also modifies its own source permanent, use `Effect::ModifySource` rather
    than `Effect::continuous(&Filter::This, ...)`. `Filter::This` inside `CreateContinuousEffect`
    resolves through `subjects::this`, which reads `res.targets.first()` whenever `res.targeted`
    is `true` and returns `None` when the only target is a player (`res.target_players`).
  - Mandatory spell additional costs (*"As an additional cost to cast this spell, sacrifice a
    creature"* on `Ritual of the Machine`) must be declared on `face!` as
    `mandatory_additional_costs = &[CostPart::Sacrifice(&Filter::CREATURE)]`, not
    `additional_costs` (which is for optional additional costs like kicker or waterbend).
  - Granted activated abilities (`Modifier::GrantActivatedDefinition` on `Martyrdom`) receive a
    synthetic ability index on the recipient permanent; tests should look up the synthetic `id`
    via `crate::effects::granted_activated(engine.state(), recipient)`.

## 2026-10-22 — Alliances (`all`) batch 4 (25 cards: `Scars of the Veteran` to `Undergrowth`)

- Added 25 *Alliances* first-printing cards (`Scars of the Veteran` through `Undergrowth`,
  9 `Coverage::Implemented`, 16 `Coverage::Partial`).
- Key card-authoring and testing lessons:
  - Abilities that put counters on their own source permanent (`Shield Sphere`, `Tornado`) must use
    `Effect::AddCounterSource { kind, amount }` rather than `Effect::AddCounter { target: TargetSpec::ThisObject, .. }`,
    because `this_object_tests::every_this_object_in_the_pool_is_one_the_resolver_reads` only
    allowlists the specific `Effect` variants whose resolvers inspect `TargetSpec::ThisObject`.
  - Dynamic mana abilities (`Soldevi Adnate`: *"Add an amount of `{B}` equal to the sacrificed
    creature's mana value"*) cannot be expressed as a static `&[ManaSymbol]` slice and must remain
    `Coverage::Partial`, with a `PARTIAL_NON_PRODUCER` entry in `land_mana_tests.rs`.
  - `Amount::TargetCmc` inside `Effect::PumpTarget` (`Surge of Strength`) and `Effect::SetSourceToughness { base: Amount::TargetPower, offset: 1, duration: Duration::UntilEndOfTurn }`
    (`Sworn Defender`) read the targeted permanent's mana value / power at resolution time.

## 2026-10-22 — Alliances (`all`) batch 5 (11 cards: `Unlikely Alliance` to `Yavimaya Ants`)

- Added the final 11 *Alliances* first-printing cards (`Unlikely Alliance` through `Yavimaya Ants`,
  9 `Coverage::Implemented`, 2 `Coverage::Partial`), completing all 144 *Alliances* first-printing
  Oracle identities (83 `Coverage::Implemented`, 61 `Coverage::Partial`).
- Key card-authoring and testing lessons:
  - `Filter::BandedWithSource` is supported in `Effect::continuous` (`Urza's Engine`'s second `{3}`
    ability: *"Attacking creatures banded with this creature gain trample until end of turn"*), and
    combat bands are tested via `declare_band_attack` + `answer_band_and_reach_blockers`.
  - `CostPart::TapOther(&Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]))` paired with
    `TargetSpec::Object(&Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::AttachedToBySource)]))`
    cleanly expresses `Veteran's Voice` (*"Tap enchanted creature: Target creature other than the
    creature tapped this way gets +2/+1 until end of turn. Activate only if enchanted creature is
    untapped."*).
  - `Trigger::TappedForMana` with `Effect::AddManaLikeEvent { amount: 1 }` and
    `Effect::SuppressUntap { target: TargetSpec::EventObject, steps: 1 }` expresses `Winter's Night`
    as a triggered mana ability that also suppresses the tapped snow land's next untap step.

## 2026-10-22 — Mirage (`mir`) batches 1–9 (213 cards: `Abyssal Hunter` to `Zombie Mob`) and Visions (`vis`) batch 1 (25 cards: `Aku Djinn` to `Feral Instinct`)

- Completed all 213 remaining *Mirage* first-printing cards across 9 batches (**123 `Coverage::Implemented`,
  90 `Coverage::Partial`**), each with a dedicated played engine test, and added *Visions* Batch 1
  (`Aku Djinn` through `Feral Instinct`, **11 `Coverage::Implemented`, 14 `Coverage::Partial`**).
- Key card-authoring and testing lessons:
  - When a `Trigger::Dies` self-return card (`Bogardan Phoenix`) uses `Effect::ReturnSourceFromGraveyard`,
    `lints::tests::entries::every_self_return_comes_back_under_the_control_its_sentence_prints` checks
    that `owner_control` matches the printed Oracle text (`false` for *"under your control"*, `true` for
    *"under its owner's control"*) and enforces a pool count ceiling in `entries.rs`.
  - When an Aura's triggered ability (`Death Watch`: *"When enchanted creature dies, its controller loses
    life equal to its power and you gain life equal to its toughness"*) fires on `Trigger::Dies(&Filter::AttachedToBySource)`,
    `PlayerRel::ControllerOfEvent`, `Amount::EventPower`, and `Amount::EventLastToughness` read the LKI
    snapshot of the dying enchanted creature directly.
- Added *Visions* Batch 2 (`Fireblast` through `Kookus`, **15 `Coverage::Implemented`, 10 `Coverage::Partial`**),
  including `Goblin Recruiter`'s multi-find top-of-library search (`Find::TOP_OF_LIBRARY.any_number()`)
  and `Helm of Awakening`'s global spell cost reduction (`Modifier::SpellsCostLess(1)` on `Filter::Any`).
  - `cast_wizard::paid_as_an_alternative_cost` only supports `CostPart::PayLife(_)` and `CostPart::ExileFromHand(_)`,
    and `cast_wizard::paid_as_a_mandatory_additional_cost` only supports `PayLifeX`, `PayLife(_)`, `Sacrifice(_)`,
    `ExilePermanent(_)`, `ExileFromGraveyard(_)`, `Discard(_)`, `DiscardHand`, and `DiscardLastDrawnThisTurn`
    (enforced by `offer_tests::no_spell_cost_list_carries_a_part_its_payment_walks_past`), so `Fireblast`,
    `Kaervek's Spite`, and `Infernal Harvest` must not place `CostPart::Sacrifice` in `alternative_costs`
    or `CostPart::SacrificeAll` / `CostPart::ReturnToHand` in `mandatory_additional_costs`.
  - Cards with `KeywordSet::FLANKING` (`Knight of Valor`, `Knight of the Mists`) must also carry the explicit
    `triggered!(Trigger::BecomesBlockedBy(&WITHOUT_FLANKING), &[Effect::ModifyEvent { modifier: Modifier::ModifyPT(-1, -1), duration: Duration::UntilEndOfTurn }])`
    ability so the flanking -1/-1 trigger fires when blocked by a creature without flanking.
- Added *Visions* Batch 3 (`Lead-Belly Chimera` through `Sands of Time`, **9 `Coverage::Implemented`, 16 `Coverage::Partial`**),
  including `Rock Slide` (`Effect::DealDamageDivided { amount: DivisionAmount::XDoubleFrom(u32::MAX) }` with `TargetReq::up_to(..., u8::MAX)`),
  `Miraculous Recovery` (`Effect::GraveyardToBattlefield` with `counters: Some((CounterKind::P1P1, 1))`),
  and `Resistance Fighter` (`Effect::continuous(&Filter::This, Modifier::AssignsNoCombatDamage, Duration::UntilEndOfTurn)` on a targeted ability).
- Added *Visions* Batch 4 (`Shimmering Efreet` through `Vision Charm`, **13 `Coverage::Implemented`, 12 `Coverage::Partial`**),
  including `Summer Bloom` (`Effect::continuous(&Filter::Any, Modifier::ExtraLandDrops(3), Duration::UntilEndOfTurn)`),
  `Sun Clasp` (`Effect::ReturnAllToHand { filter: &ENCHANTED_CREATURE, opponents_only: false }`),
  `Vanishing` (`Effect::PhaseOutFilter(&ENCHANTED_CREATURE)`),
  `Triangle of War` (`Effect::Fight { fighter: TargetSlot::First, foe: TargetSlot::Second }`),
  and `Teferi's Puzzle Box` (`Effect::PutAnyFromHandOnBottom` + `Effect::DrawCards { amount: Amount::ThisWay(ThisWay::PutIntoLibraryFromHand) }`).
  - `lines::tests::every_mode_and_alternative_cost_knows_its_printed_sentence` requires every `AbilityDef::ModalSpell`
    whose Oracle text has `•` bullets to have the exact same number of non-empty `modes` as `•` bullets so
    `baylee_cards_codegen::lines::map` maps each mode index to its bullet line.
- Added *Visions* Batch 5 (`Wand of Denial` through `Zhalfirin Crusader`, **2 `Coverage::Implemented`, 2 `Coverage::Partial`**),
  completing all 104 newly implemented *Visions* (`vis`) first-printing cards (**50 `Coverage::Implemented`, 54 `Coverage::Partial`**).
  - Continuous effects from a resolving spell (`Wind Shear`: *"Attacking creatures with flying get -2/-2 and lose flying
    until end of turn"*) lock their affected permanent set on resolution (CR 611.2c), so applying `Modifier::ModifyPT(-2, -2)`
    and `Modifier::RemoveKeyword(KeywordSet::FLYING)` in sequence on `ATTACKING_WITH_FLYING` affects the same locked set
    without the keyword removal invalidating the filter for the second effect.
- Added *Exodus* Batch 3 (`Oath of Lieges` through `Skyshaper`, **12 `Coverage::Implemented`, 13 `Coverage::Partial`**),
  including `Reconnaissance` (`Effect::RemoveTargetFromCombat { unblock: false }` + `Effect::UntapTarget`),
  `Plaguebearer` (`Filter::ManaValue(Cmp::Eq, ManaBound::ChosenX)`), and `Rabid Wolverines`
  (`Amount::Count { filter: &Filter::BlockingSource, owner: PlayerRel::Any }`).
  - `baylee_cards_codegen::lines::granted_cost` only inspects `AbilityDef::Static` and `AbilityDef::CopyOnEnter*`,
    not `AbilityDef::Spell`, so an instant/sorcery spell cannot use `Modifier::GrantActivated` inside
    `Effect::CreateContinuousEffect` (`Resuscitate`) until `granted_cost` walks `AbilityDef::Spell`.


## 2026-10-23 — Review of Argon PR #394 (Ice Age 5 to Urza's Destiny 1)

The second-model review read all 1,350 card/test pairs. Detailed findings and
final validation are in `docs/history/2026-10-23-argon-review.md`.

- **A Partial is a player-facing rules claim.** Missing a cost, upkeep
  consequence, casting restriction, opponent choice or mandatory symmetric
  effect makes a card stronger in a reachable game. Kaervek's Spite and
  Infernal Harvest cannot move an additional cost to resolution; Aluren
  cannot replace a free-cast permission with an ordinary generic reduction;
  Nature's Resurgence and Pandemonium cannot drop the opponents' instructions.
  Remove the whole optional upside or hold the card back. A useless 0/0 or
  blank noncreature is held too.
- **Search the existing DSL before naming a gap.** Mindwhip Sliver uses the
  complete granted-ability definition; Body Snatcher can ask for its creature
  discard; Magmasaur can branch on its counters and keep the sacrifice;
  Umbilicus already has a life-payment alternative. The missing sentence and
  the code used instead must both be explicit in a Partial reason.
- **Read the consumer as well as the variant.** `EventAmount` must capture
  damage to a permanent, not only damage to a player; `EventObject` on a
  damage-received trigger means the recipient. A delayed self-return needs
  the exact graveyard successor, not the old battlefield version, and it
  must not follow the card out of the graveyard and back. Keyword bits do not
  prove a rule is implemented, and a second handwritten flanking trigger
  doubles the keyword's effect.
- **Test the sentence's boundary through the real offer.** `castable` alone
  means mana already floats; include `payable` for a real availability test.
  Check both paid and unpaid abilities, and both sides of a filter. A turn
  helper returns immediately when its predicate already holds. Printed mana
  abilities use `ActivateAbility`; the intrinsic basic-land door is different.
  A setup helper that taps every mana creature can accidentally tap the next
  blocker too. Declare combat explicitly, and settle the stack before claiming
  a trigger did not fire.
- **Preventing damage and assigning none are different rules.** Use the
  combat-prevention modifier for Resistance Fighter, Gossamer Chains and Fend
  Off. Include a case where damage cannot be prevented. Assignment restrictions
  also have to reach blockers and the masked-creature reveal preview.
- **Generated files stay generated.** Restore reviewed unsafe cards with the
  selective hold-back codegen mode, remove their mirrored tests/module entries,
  then regenerate compiled tables. Never hand-write the generated stub marker
  or run full codegen without the corpus. Keep reviews in small batches and
  rerun the relevant tests before asking for the next batch.

- **The controller in a duration is observable.** `SuppressUntap` follows the
  affected permanent's controller. Use the existing `UntilYourNextUntapStep`
  scope for the resolving ability's “your”, and test a control change before
  untap. “Until its next untap” and “during its next untap” also differ at the
  start/end boundary; checking only the final type misses Choke interactions.
- **Measure the committed pool rather than adding authoring claims.** The
  reviewed 1,350 identities contain 793 Implemented, 260 Partial and 297
  Unimplemented cards. First-printing set totals also contain inherited cards.
  Native `site card-status` checks the ledger/pool/index relationship and keeps
  Excluded, Absent and Unimplemented separate. The original authoring snapshot
  is not a current coverage report.

- **Cost parts and simultaneous events have different snapshot boundaries.**
  Refresh projected characteristics between completed cost parts, before the
  next sacrifice records its source. Removing a counter then sacrificing must
  use the remaining power. Do not refresh between individual sacrifices within
  one simultaneous part: a sacrificed lord still contributes to that event's
  last known characteristics. Include stacked and immediate mana controls.
- **Census agreement is bounded by the reader.** Exact populations should be
  reconciled by individual card evidence, preserving negative controls. Two
  counts using the same text reader can share a blind spot, such as `add`
  versus `adds`; record it explicitly rather than claiming complete coverage.
  Text provenance for retained modal subsets must be unique, and unknown or
  ambiguous effects must refuse a label rather than inherit a shifted index.

- **Adding mana does not make an activation a mana ability.** A direct target
  in any immediate sentence keeps Witch Engine's ability on the stack. Its
  printed-line reader must agree. A target in a later delayed trigger belongs
  to that separate ability; retain a Primal Wellspring-style negative control.
- **A public view cannot reconstruct missing history.** Zero marked damage
  after regeneration is not proof of no damage this turn. Current combat is
  not all attacks this turn, and an unannounced X is not a candidate's mana
  value. Keep the client's conservative unknown answer, pin known gaps by exact
  membership, and test the genuinely empty candidate population separately.

## 2026-10-24 — Feedback regressions and token art adoption

- Replayed the report shapes for Umara Raptor, Field of the Dead and Tireless
  Provisioner: a returned Earth King's Lieutenant produces exactly the two
  counters that take the Raptor to 4/4; two lands entering together each see
  the seven-name condition; a land played from the graveyard still triggers
  Landfall. Keep those joins in the tests, not only the final token or P/T.
- A token's id and its definition are different commitments. `tokenledger`
  must let a generated entry become hand-written in place when a person chooses
  a printing, while refusing collisions and preserving every other id. The M20
  Zombie is the regression: `codegen --tables` moves only its body and reexport.
- Test setups need the whole casting sequence. A Raptor plus Ephemerate needs
  four lands, Primeval Titan needs two green sources, and library searches need
  the offered ObjectIds asserted before `ChooseObjects`; otherwise a report test
  fails on setup rather than on the rule it is meant to prove.

## Desktop feedback card review (25 October 2026)

Reviewed Kimi's `29fde2893624af5aa311a4fa971a42932a106f11` and
`fdd79c1b93a55d5e83af188e05ac530e004dbaaf` from PR #395. Adopting an
existing generated token preserves its stable token slot, so saved print
identity remains valid. The black 2/2 Zombie art UUID resolves to tm20 #6.
The Umara Raptor, Field of the Dead and Tireless Provisioner regression
fixtures exercise their triggers through engine actions; the follow-up corrects
the fixtures' actual mana costs. No further card-rule defect was found.

UI review lesson from the surrounding PR: asynchronous screenshot encoding
must not remove the wait for GPU readback, and virtualized rows from an old
log must never be measured against a replacement book. Tests need measured
rows and lifecycle changes, not only initial mounting with empty layout data.
## 2026-10-25 — First Partial-completion batch (eight cards)

The first eight-card batch has 48 passing focused tests and its initial table
pipeline is green. Four generic reader forms pass 382 codegen tests (191
scriptgen); selective reclaim passes 98 xtask tests. Fresh post-reclaim
measurement, table/hook validation and the rules gate pass (20,872 tests,
17 skipped). The corrected modifier population passes all 13 effects tests.
Full `cargo dev gate` also passes: 23,109 workspace tests (8 slow), 25 skipped,
plus 4 API tests; both gates exit 0. Feature gate was not run, and neither
push nor land was performed. At that checkpoint, 286 Partial cards remained separate
work; four returns to generator ownership do not enlarge this eight-card batch.

- **Mutate the rule to test the assertion.** A no-op supertype application
  initially passed because `SupertypeSet::contains(BASIC.union(SNOW))`
  accepts either matching bit. Exact set equality detects Basic without Snow.
  Nine targeted mutations were ultimately detected. A green regression needs
  a failing counterexample before its asserted boundary is trusted.
- **Snapshot a simultaneous rule before any object leaves.** A no-regeneration
  static must apply to every affected victim of one destroy-all or lethal SBA
  event even when its source is processed first. Test both object orders;
  source departure in an earlier instruction is a different boundary.
- **Exercise the real choice before measuring a count.** Directly setting a
  chosen-opponent field missed the projection-cache bug at the entry decision.
  The real action must invalidate projections before count-dependent P/T is
  read. Include an opponent whose board gives a distinct answer.
- **Keep history authoritative and scans conditional.** Damage-source and
  recipient versions distinguish current incarnations; marked damage cannot
  reconstruct those facts in a public view. Keep public readers unknown and
  use an effect-table hint to avoid scanning every battlefield on ordinary
  SBA passes with no regeneration prohibition.
- **Name unsupported timing separately from supported characteristic edits.**
  Adding/removing Snow is a layer-four supertype change. General World
  tenure requires its own timing history; the primitive alone is not proof
  that every rule about every supertype is complete.
- **Match the reader's memory boundary to runtime facts.** A runtime that
  records damage recipients across a resolution cannot safely implement a
  script that resets damage memory after an earlier instruction. Accept the
  initial damage instruction and refuse later memory resets explicitly.
  Keep source-static no-regeneration predicates exact, and refuse World
  supertype changes until the world rule tracks continuous tenure.
- **Source identity determines prevention.** Energy Storm cares whether the
  damage source is an instant or sorcery spell. “Ability damage is never
  prevented” is too broad: an ability can attribute damage to a spell.
  Negative controls need a nonspell source, not only an ability on the stack.
- **Reader completion and coverage promotion are separate events.** Reclaim
  only the explicitly named, already implemented definitions after fully
  reading their real upstream scripts. Preserve each identity, printing pin,
  path and tested coverage; prove a preview leaves every relevant file hash
  unchanged. Four ownership transfers do not mean four additional cards were
  implemented, and an extended reader does not justify a broad regeneration.
- **New modifier variants belong in the verification census too.** An
  exhaustive production match does not update the independent test population.
  Add each new modifier to the enum-vs-list check and reconcile the locking
  classification by behavior: supertype edits lock their sets; regeneration
  prohibition changes a rule. Preserve membership checks alongside exact totals.

## 2026-10-26 — Bounded Ice Age closeout

Eighteen existing Partial definitions are now Implemented against `origin/main`;
70 Ice Age rule cards remain open. Count 373 Oracle IDs separately from the
383 printings, and preserve the user's narrowed scope rather than claim a
whole-set completion. Rules gate: **21,050 passed, 17 skipped, exit 0**.
Full desktop gate: **23,301 passed, 25 skipped; four API tests passed; exit 0**.
Formatting, Clippy, tests, cache, validation and API checks all passed.
Feature gate: **Passed, exit 0**. Client: 2,290 passed, including 39 devctl
checks. Gateway devtable Clippy/tests, dev-dylink, test-support, train without
play, fuzz, mutate, verification-hooks-shipped and crypto-bigint-scope all passed.

Completed instruction-draw history is not by itself a future rule input for
loop detection. Active loot must still hash its normalized eligible hand
positions; snapshot identity retains the complete ledger. Direct zone-mutation
fixtures must settle characteristics before measuring a no-new-events boundary.

## 2026-10-24 — Urza's Destiny Batch 2 (`Goblin Festival` through `Phyrexian Negator`)

- **Cards**: Added the next 25 absent first-printing identities in `uds` ledger order (`Goblin Festival` through `Phyrexian Negator`): **15 `Coverage::Implemented`, 4 `Coverage::Partial`, 6 `Coverage::Unimplemented` (`--hold-back`)**.
  - **Machine-owned (`Coverage::Implemented`, 5)**: `Incendiary`, `Junk Diver`, `Keldon Vandals`, `Magnify`, `Mark of Fury`.
  - **Hand-owned (`Coverage::Implemented`, 10)**: `Goblin Festival`, `Goblin Marshal`, `Hulking Ogre`, `Hunting Moa`, `Keldon Champion`, `Metathran Elite`, `Metathran Soldier`, `Multani's Decree`, `Pattern of Rebirth`, `Phyrexian Negator`.
  - **Hand-owned (`Coverage::Partial`, 4)**: `Ivy Seer`, `Jasmine Seer`, `Metalworker`, `Nightshade Seer` — each retains only its creature body with its optional reveal-any-number-of-hand-cards activated ability omitted because variable-count hand reveal is not yet in the DSL.
  - **Held back (`Coverage::Unimplemented`, 6)**: `Impatience` (blank enchantment without per-turn spell-cast tracking per active player), `Iridescent Drake` (ETB Aura attachment from graveyard not supported), `Landslide` (sorcery with no effect without sacrifice-any-number-of-Mountains cost/resolution), `Lurking Jackals` (blank enchantment without opponent life-threshold trigger and permanent type replacement), `Momentum` (Aura with no effect without power-boost per growth counter on the Aura), `Opalescence` (blank enchantment without global non-Aura enchantment animation to MV/MV creatures).
- **Engine (`crates/baylee-engine/src/resolve/immediate.rs`)**:
  - `Effect::GainControlAll` with `Filter::This` now checks `subjects::source(state, res).filter(|s| subjects::on_battlefield(state, *s))` instead of matching any battlefield permanent with `res.source` (whose arena `ObjectId` slot is reused with an incremented `version` if the card leaves and re-enters the battlefield before resolution, CR 400.7). Tested via `goblin_festival_new_incarnation_is_not_given_away_when_flip_loses`.
- **Reader (`crates/baylee-cards-codegen/src/scriptgen/`)**:
  - `keywords.rs`: Added `K:Echo:<cost>` transcoding to `AbilityDef::Echo { cost: mana!(...) }` (unblocking `Keldon Vandals` as machine-owned and future Echo creatures).
  - `triggers.rs`: When a `Phase$ End of Turn` trigger script has `ValidPlayer$ You` (as on `Mark of Fury`), check the Oracle text for `"at the beginning of the end step"` (without `"your end step"`) and emit `PlayerRel::EachPlayer`.
  - `chains.rs` / `targets.rs`: Claimed `TargetsWithDefinedController` on `ChangeZone` from `Graveyard` when it is `"You"` or when it is `"TriggeredCardController"` and the target filter already narrows to `PlayerRel::You` (unblocking `Junk Diver` as machine-owned).
- **Tools**:
  - `cargo tools hooks require-card-tests` reads a JSON hook payload from `stdin` (e.g., `echo '{}' | cargo tools hooks require-card-tests`); invoking it interactively without piping `stdin` blocks waiting for EOF.

## 2026-10-24 — Urza's Destiny Batch 3 (`Plague Dogs` through `Telepathic Spies`)

- **Cards**: Added the next 25 absent first-printing identities in `uds` ledger order (`Plague Dogs` through `Telepathic Spies`): **9 `Coverage::Implemented`, 7 `Coverage::Partial`, 9 `Coverage::Unimplemented` (stubs)**.
  - **Machine-owned (`Coverage::Implemented`, 5)**: `Plague Dogs`, `Powder Keg`, `Private Research`, `Solidarity`, `Taunting Elf`.
  - **Hand-owned (`Coverage::Implemented`, 4)**: `Reckless Abandon`, `Repercussion`, `Sigil of Sleep`, `Telepathic Spies`.
  - **Hand-owned (`Coverage::Partial`, 7)**: `Quash`, `Rapid Decay`, `Rayne, Academy Chancellor`, `Replenish`, `Scour`, `Sowing Salt`, `Splinter`.
  - **Unimplemented stubs (`Coverage::Unimplemented`, 9)**: `Plow Under` (putting two target lands on top of their owners' libraries not supported), `Rofellos's Gift`, `Scent of Brine`, `Scent of Cinder`, `Scent of Ivy`, `Scent of Jasmine`, `Scent of Nightshade` (variable-count hand reveal and revealed-card scaling not supported), `Scrying Glass` (choosing a number greater than 0 and a color and counting matching revealed hand cards not supported), `Storage Matrix` (untap-step type choice and untap restriction not supported).
- **Reader (`crates/baylee-cards-codegen/src/scriptgen/`)**:
  - `chains.rs`: Extended `"DestroyAll"` to transcode `.cmcEQX` across comma-separated alternatives of `ValidCards` into `Effect::DestroyAllWithManaValue { filter, value }` when `NoRegen` is absent (unblocking `Powder Keg` as machine-owned).
  - `constants.rs`: Mapped `"All creatures able to block CARDNAME do so."` in `keyword_static` to `Modifier::MustBeBlockedByAllAble` (unblocking `Taunting Elf` as machine-owned).

## 2026-10-25 — Urza's Destiny Batch 4 (`Thorn Elemental` through `Yawgmoth's Bargain`)

- **Cards**: Added the final 10 absent first-printing identities in `uds` ledger order (`Thorn Elemental` through `Yawgmoth's Bargain`), completing Urza's Destiny (`uds`): **7 `Coverage::Implemented`, 1 `Coverage::Partial`, 2 `Coverage::Unimplemented` (stubs)**.
  - **Machine-owned (`Coverage::Implemented`, 6)**: `Thran Foundry`, `Thran Golem`, `Trumpet Blast`, `Wall of Glare`, `Yavimaya Elder`, `Yavimaya Enchantress`.
  - **Hand-owned (`Coverage::Implemented`, 1)**: `Urza's Incubator`.
  - **Hand-owned (`Coverage::Partial`, 1)**: `Thorn Elemental` (7/7 Elemental body; optional unblocked combat-damage assignment `"You may have this creature assign its combat damage as though it weren't blocked."` is not in the DSL).
  - **Unimplemented stubs (`Coverage::Unimplemented`, 2)**: `Wake of Destruction` (destroying all other *lands* with the same name as target land is not in the DSL; `DestroyOthersNamedLike` destroys all permanents with that name), `Yawgmoth's Bargain` (mandatory `Skip your draw step` replacement is not in `ReplacementRule`).
- **Reader (`crates/baylee-cards-codegen/src/scriptgen/`)**:
  - `mana.rs`: Added `Exile<1/CARDNAME>` / `Exile<1/CARDNAME/...>` cost transcoding to `CostPart::ExileSelf` (unblocking `Thran Foundry` as machine-owned).
  - `targets.rs`: Added `"enchanted" => "Filter::Enchanted"` and `"!enchanted" | "unEnchanted" => "Filter::Not(&Filter::Enchanted)"` filter atoms (unblocking `Thran Golem` as machine-owned).
  - `statics.rs`: Added `CanBlockAny$ True` -> `Modifier::CanBlockAnyNumber` in `block_modifiers` (unblocking `Wall of Glare` as machine-owned) and `AddPower$ X | AddToughness$ X` with `SVar:X:Count$Valid <valid>` -> `Modifier::ModifyPTByCount(PtCount::{YouControl|OnBattlefield}(...))` in `pt_modifiers` (unblocking `Yavimaya Enchantress` as machine-owned).
  - `effects.rs` / `triggers.rs`: Added `ShuffleNonMandatory$ True` -> `optional = true` in `search_library` and accepted `OptionalDecider$ TriggeredCardController` on `Filter::This`/`ETB` triggers whose Oracle text contains `"you may"` (unblocking `Yavimaya Elder` as machine-owned).

## 2026-10-26 — Mercadian Masques (`mmq`), Nemesis (`nem`), Prophecy (`pcy`) Batches 1–2, and Self-PR Review

- **Cards**:
  - Completed **Mercadian Masques (`mmq`)** across 8 batches (186 cards: **101 `Coverage::Implemented`, 22 `Coverage::Partial`, 63 `Coverage::Unimplemented`**).
  - Completed **Nemesis (`nem`)** across 4 batches (100 cards: **55 `Coverage::Implemented`, 13 `Coverage::Partial`, 32 `Coverage::Unimplemented`**).
  - Completed **Prophecy (`pcy`)** Batches 1–2 (50 cards, `Abolish` through `Mercenary Informer`: **16 `Coverage::Implemented`, 8 `Coverage::Partial`, 26 `Coverage::Unimplemented`**).
- **Reader (`crates/baylee-cards-codegen/src/scriptgen/`)**:
  - Extended `scriptgen` across `mmq`, `nem`, and `pcy` Batches 1–2 to transcode `Sac<2/Land>`, `Discard<2/...>`, `SacValid$` (`Effect::SacrificeFilter`), `UnlessCost$` on player targets (`PlayerRel::Chosen`), `&`-delimited `AddType$` tokens (`Living Terrain`), `TapOrUntap` / `TapOrUntapAll`, `RevealHand$` + `Discard` chains (`Effect::RevealHandDiscard`), `PumpAll` with `ValidCards$` subtype filters, and Rebel/Mercenary permanent search recruitment (`Effect::SearchLibraryToBattlefield` with `Filter::CmcAtMost`).
  - Fixed `crates/baylee-cards-codegen/src/commander.rs` (`is_legendary_commander_face`) so flip creatures whose secondary face is legendary (`Erayo`, `Bushi Tenderfoot`, `Rune-Tail`, etc.) are not misclassified as `CommanderRule::Legendary` unless their primary face (index 0) is legendary (CR 903.3, 710.2).
- **Self-PR Review (`origin/main..HEAD`)**:
  - Promoted `Vine Dryad` and `Unmask` (`mmq` Batch 7) from `Coverage::Partial` to `Coverage::Implemented` using `AlternativeCost { cost: cost!(ExileFromHand(&...)), condition: AltCondition::Always }`, and added pitch-cast tests verifying that only a matching-color card in hand pays the alternative cost.
  - Moved `abilities = &[...]` from `face!(...)` to card level (`card!(...)`) on 11 single-faced cards from `uds` Batch 3 (`Quash`, `Rapid Decay`, `Rayne, Academy Chancellor`, `Reckless Abandon`, `Repercussion`, `Replenish`, `Scour`, `Sigil of Sleep`, `Sowing Salt`, `Splinter`, `Telepathic Spies`) so `def.abilities`, `generated_lines.rs`, and `activation_census` see all 12 abilities.
  - Added missing `// NOT SUPPORTED:` header comments to all hand-owned `Coverage::Partial` card files in the branch diff.
  - Note on targeting prompts in card tests: spells with `TargetSpec::AnyPlayer` prompt `Pending::ChoosePlayer` (answered with `PlayerAction::ChoosePlayer(pid)`), whereas activated abilities with `TargetSpec::AnyPlayer` prompt `Pending::ChooseTargets` (answered with `PlayerAction::ChooseTargets { objects: vec![], players: vec![pid] }`).

## 2026-10-26 — Reviewed `agy/uds-2` integration

- Distinguish successful destruction from entering a graveyard. Decree counts
  destruction receipts across replaced destinations, then gains the total life
  once; repeating a gain instruction changes the number of life-gain events.
- A counted sacrifice chooses its whole batch before moving any permanent.
  A departing dies-trigger source must observe simultaneous creature deaths.
- Preserve nested payment context, APNAP ordering and exact source incarnations.
  Canonical tests need literal subject identity and observable gameplay,
  including legal and refused combat choices.
- Integration evidence: the test-module creation race and two test compile
  errors were corrected; focused 35/35 passed. All eight deliberate mutations
  were detected, all sources restored exactly, and restored 35/35 passed.
  All 54 pending source reviews are closed. Rules: **21,355 passed, 17 skipped**;
  full: **23,606 passed, 25 skipped**, plus four API tests; features: **passed**,
  including 2,290 client tests (39 devctl). All gates exited 0. Both final gates
  ran on `2dca3227cc796b87db78ec74d112019c4e7cefe7`.
- Preserve infrastructure failures separately from source regressions. The
  initial feature gate lost its shared PostgreSQL container; five isolated
  social-report tests failed at DB connection. Restarting that container restored
  the unchanged binary to 5/5; the complete feature rerun then passed. No source
  or concurrency change was needed.
- Table generation twice and its check changed no files; the 399-card Write/Stop
  hook cleared its debt. Registry validation changed from 10,693 cards /
  10,352 finished / 341 stubs to **11,083 / 10,600 / 483**, with 0 untested.
  These totals and this merge do not claim set completion.

## Desktop feedback follow-up (25 October 2026)

- Optional triggers with no chosen target are discarded only when a probe
  resolution on a cloned state completes with no effect and no follow-up
  choice. The comparison normalizes only damage-batch allocation bookkeeping;
  any other changed field keeps the trigger. Tuktuk Scrapper covers both the
  answered empty target and no legal target, while a synthetic trigger with an
  untargeted draw proves that independent effects still resolve.
- Restricted mana is now planned by purpose. The ordinary plan wins first;
  otherwise one filter-admitted restricted source is tapped last, so the
  planner never evaluates restricted pool mana. Jasmine Dragon Tea Shop covers
  Ally spells and Ally-source activation, with non-Ally negative controls.
  Registry-backed hand identities are mandatory in these fixtures.
- New games choose the starting seat uniformly from the preset's seeded RNG
  before shuffling. Controlled tests may override it. Missing fields in old
  presets retain historical seat 0, while new randomized presets serialize
  `starting_player: null` for identical replays.
- Review (29 October 2026): the probe compared only `GameState::snapshot_hash`,
  which leaves the journal out on purpose. `Effect::RevealHand` writes nothing
  but a journal entry, so a declined target would have discarded a public
  reveal with it. A "no effect" proof has to cover every channel an effect can
  write to; the probe now also requires an unchanged journal length.
- Review (29 October 2026): the setting first entered `Engine::snapshot_hash`
  as `extra * 31 + flag`. Old records read the flag as `false`, yet the
  multiplication still moves `extra` whenever it is non-zero, so every stored
  replay would have diverged at its opening hash. At the PR's own base `extra`
  happened to be zero and the fixtures passed; `record::seen` caught it only
  after the rebase. A new input to a replay hash must contribute nothing for
  its legacy value, so the term is now added only when the setting is on.
