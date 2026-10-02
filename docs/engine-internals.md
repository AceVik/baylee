# Engine Internals

(Deep spec; filled during M1–M2. Sections are normative.)

## Object model
Arena of `GameObject`s (`ObjectId = slot:24 | generation:8`). Kinds: Card,
Spell, Permanent, Token, Emblem, AbilityOnStack. Zone is a property; zones
hold ordered ids (library order IS the data). Every card instance carries
`card: (CardIndex, PrintRef)` — `PrintRef` is presentation-only.

The *printed* characteristics are shared, not inlined: an object holds an
`Arc<Characteristics>` handed out by `GameState::bases`, so every copy of a
card in a deck, every token of the same kind and every ability of the same
name on the stack point at one allocation. **Every write goes through
`GameObject::base_mut`**, which splits the sharing first; assigning
`obj.base` a fresh face is the only other legal way to change one. This is
what keeps a `GameObject` at 272 bytes and `GameState::clone` — the AI's
per-ply primitive — from copying the same 256 bytes a thousand times.

## Layers & continuous effects
Computed characteristics are cached projections: printed/copiable base →
apply matching `ContinuousEffect`s by layer (1 copy, 2 control, 3 text,
4 type, 5 color, 6 ability, 7a–e power/toughness), timestamp order,
dependency topological order within a layer. Cache validity = one
`u64` generation compare. Durations: `WhileSourceOnBattlefield`
(deregistered structurally on the source's zone change), `UntilEndOfTurn`,
`Indefinitely`, conditions. An emblem's static abilities function in the
command zone (CR 114.4): `sync_static_effects` registers them once, as
`Indefinitely` effects (`progress::emblem_statics`), and nothing removes
them, because an emblem never leaves. Subtypes are a 1024-bit bitmap (changeling =
one mask OR, not a scan), and the ids in it are **append-only** since #43:
`ALL_CREATURE_TYPES` is the mask a changeling gets and it is a generated
list rather than a range, because a new creature type no longer sits next to
the old ones. `docs/card-identity.md` §"`SubtypeId`" is normative.

**An object keeps two clocks, and a transform moves only one.** Both read
the game's one counter (`GameState::next_timestamp`).
`GameObject::timestamp` is the timestamp of CR 613.7: taken as the object
enters a zone (613.7d) and again each time the permanent transforms
(613.7g, `GameState::transform`). The effects of its static abilities take
it when `sync_static_effects` registers them (613.7a), and `transform` drops
the old face's effects first, so the new face's statics are registered with
the new stamp and apply after anything created while the other face was up.
A daybound card entering at night enters with its back face up
(CR 702.145b, 712.14a), and that is not a transform: nothing was turned
over (701.27a transforms a permanent). So the entry scan calls
`GameState::turn_over`, the half of `transform` that drops the statics of
the face going down and switches faces. It adds no stamp, since the arrival
already took one (613.7d), and journals no `Transformed`, which a "transforms
into" trigger (701.27e) and the log's "transformed" line read
(`a_werewolf_that_enters_at_night_has_not_transformed`, and in gamehost
`a_werewolf_entering_at_night_is_not_logged_as_transformed`).
`GameObject::controlled_since` is the moment its controller took it: taken
on arrival and again at every change of control
(`restart_summoning_sickness`). Summoning sickness (`combat::summoning_sick`,
CR 302.6) reads only this field, because a permanent that transforms is the
same object under the same controller (CR 712.18). The two used to be one
field, and that is why a transform could not take its timestamp without
making the permanent summoning-sick. Two of the events CR 613.7 lists are
not implemented: an Aura, Equipment or Fortification becoming attached
(613.7e), and turning face up or face down (613.7f). A Saga taking a lore
counter restamps `timestamp` (`progress.rs`, both lore sites) with no rule
behind it, because CR 613.7c stamps the counter and not the object; since
the split it no longer makes a Saga creature summoning-sick. Tests:
`mechanics_tests::transforms` covers the ordering and the sickness,
`mechanics_tests::tokens` both token writers.

**A `Pending` is published from a settled board, and the machine is what
settles it — including after an action.** The invalidation half has always
been right: `move_object` marks the projection stale in both directions for
the battlefield and the stack, and every counter writer does the same. What
was missing is anyone to recompute it, because the recompute runs in
`run_machine`, whose first line returns while an answer is awaited — and
`after_action` used to set exactly that flag, publishing the acting player's
priority (CR 117.3c) itself. Between one action and the next, then, the
machine did not run at all.

Three separate things were owed in that gap and none of them happened. The
layer projection stayed one action old, which is the report this arrived as:
*"I play a land and it does not inherit the artifact, hexproof, indestructible
static effects on the field"* — a land being the only permanent that reaches
the battlefield without passing through the stack, though a spell put onto the
stack invalidated the same projection and was shown just as stale. The
state-based actions CR 117.5 owes before any player receives priority did not
run. And the abilities the action triggered were not put on the stack, so a
land printing *"when this land enters, it deals 1 damage to target opponent"*
handed its controller priority over an empty stack with the question unasked —
and a spell cast in that window would have been stacked *under* a trigger that
preceded it.

So `after_action` records only that priority is owed
(`Engine::regrant_priority`, `passes` reset) and publishes nothing;
`priority_round`'s first arm hands it over at step 5, where every other
priority in the game is handed over, after the machine has done the work the
action made for it. An action is therefore an ordinary re-entry into the
machine rather than an exception to it, which is the property the three
failures above all came from lacking.

The mulligan window is the one place a question is still published without
the machine, which does not run before turn 1. So `Engine::new` and
`settle_mulligans` do the machine's first step themselves,
`sync_static_effects` and then `refresh_characteristics`: a starting
battlefield's static abilities apply to the board every seat keeps its hand
beside (CR 604.2), and the statics of a player who concedes in the window
leave with their permanents (CR 800.4a) before anybody is asked again
(`a_starting_battlefield_s_statics_apply_while_the_mulligans_are_open`).

Inside the machine, one step publishes a question after it has moved the
board behind step 0a: 0b, `apply_enter_modifiers`, which asks as-it-enters
choices (a colour, a creature type, a shockland's life, a clone's choice).
Before its first question it has applied every arrival's replacements that
ask nobody (below), so it may already have put a daybound permanent entering
at night back face up (CR 702.145b), and `GameState::turn_over` drops the
statics of the face going down and leaves the new face's to the next scan.
It may also have given a Room its door, or put counters on an arrival. So
when 0b leaves a question out, the machine does what `Engine::new` does, a
sync and a refresh, before it returns. Every other flip leads back to 0a
before a question: a resolution's transform (`apply_pending_face_changes`, at
the end of `finish_resolution`) is followed by the machine or by
`run_until_choice`, a delayed transform (3b) and daybound's own fixpoint
(2c) both `continue`
(`mechanics_tests::transforms::a_question_asked_as_permanents_enter_sees_the_face_that_entered`).

**Permanents that enter together get all their replacements before anyone
is asked.** They are one event, and each one's own replacements modify how
it enters (CR 614.12, 614.12a). So 0b walks every arrival since
`entry_scan_seq` first and applies what needs no answer: entering tapped,
counters, a planeswalker's loyalty (CR 306.5b), a Saga's lore counter
(CR 714.3a), daybound's back face, a Room's door. What asks (an
`EnterModifier` that chooses, or a clone's copy choice, `EntryAsk`) is
queued in `Engine::entry_questions` and asked one at a time: the active
player's first, then in turn order (CR 101.4), and one player's in the order
the permanents entered. CR 101.4c would let that player choose the order;
that is not offered. Each question is asked when its turn comes, so a
shockland is asked against the life the ones before it left, and one its
controller can no longer pay for enters tapped without a question
(CR 614.12b). The scan used to publish the first question from the middle of
its loop with the cursor already past every arrival, so every arrival behind
it got nothing: a Urza's Saga fetched beside Steam Vents had no lore counter,
and a planeswalker behind a shockland entered with no loyalty and died
(CR 704.5i). Tests: in `enter_tests`,
`a_saga_fetched_beside_a_shockland_enters_with_its_lore_counter`,
`a_planeswalker_entering_behind_a_shockland_enters_with_its_loyalty`,
`a_second_shockland_is_asked_against_the_life_the_first_one_left` and
`shocklands_of_two_players_entering_together_are_asked_in_apnap_order`;
`mechanics_tests::transforms::a_question_asked_as_permanents_enter_waits_for_the_ones_behind_it`.

Animate Artifact is one continuous effect with two layer parts.
`AnimateNoncreatureArtifact` is registered at layer 4; `LayerPlan` also places
its index in layer 7b. Each object projection remembers which such effects
started applying in layer 4 and continues only those in 7b (CR 613.6). The
implicit noncreature check participates in type-layer dependencies; it is not
repeated after the effect has made the object a creature. A type layer containing
this modifier selects effects against the current object projection, recalculating
dependencies involving the conditional animation after each application. A type
removal that currently changes nothing does not impose a dependency. Internal
edges of dependency loops are ignored before choosing the next timestamp, even
when another independent effect is ready. The ordinary precomputed plan remains
the path for layers without this conditional animation. Mana value comes from
the in-progress copiable characteristics, so copying an artifact changes the
base P/T appropriately. This temporary projection bookkeeping adds no replay
state or object footprint. Separate static abilities remain separate effects.

A projection reads the *board*, and there are two ways for it to read a
stale one. `recompute_with` walks **one object through all the layers**, so
while it runs, that object's cached characteristics are still the previous
projection — and a modifier that counts permanents (`ModifyPTPerCount`) used
to read its own source off that cache. Ashaya, Soul of the Wild makes your
nontoken creatures into lands at layer 4 and is then as big as the lands you
control at 7a, so it has to count itself: it came down one short. The object
under projection is now matched against the in-progress characteristics
(`eval::matches_projected`), which is what CR 613.1 says. Every other object
is read from its cache, which is this refresh's only once the walk has
reached it: an Elf later in the list was still the last refresh's Elf, so a
freshly cast one left Ashaya one short until something else invalidated it.
A count therefore marks its projection (`Projection::read_board`), and the
refresh projects the counting objects again after the walk, repeating while
one moved (a counter can count another), bounded by their number. That is
CR 613.1 again, not a dependency: CR 613.8a asks for two effects in the same
layer or sublayer, and a count in layer 7 and the type change it reads in
layer 4 are not. The other way is the generation
compare itself: it watches the effect **table**, so an input the filters read
that is *not* an effect leaves every projection stale. Naming a creature type
is one — Steely Resolve's static is registered as the enchantment enters and
the type is chosen one question later — so `ChooseSubtype` calls
`GameState::invalidate_projections`, as anything writing a counter already
does. Tap and combat status are others, announced through
`GameState::board_state_changed` only when an effect reads them: a filter
naming `Tapped`, `Attacking`, `Blocking` or `Unblocked`, or a count of what
the defending player controls (`PtCount::DefendingPlayerControls`, CR
508.5), which changes as attackers are declared and as combat ends.
`card_tests::rules::a_cached_projection_is_what_a_fresh_one_would_compute`
is the guard for both: a recompute may not disagree with the cache.

**A characteristic-defining ability works in every zone** (CR 604.3), and
a static is registered only while its source is on the battlefield, so a
card whose power and toughness are `*` was its printed 0 in a library, a
hand or a graveyard. Recruiter of the Guard offered a 3/3 Ashaya as a
creature with toughness 2 or less. `GameState::printed_pt_cda` holds, per
card object, the `Modifier::CharacteristicPT` its front face prints on
itself (CR 712.8a: off the battlefield a card has only its front face),
read once in `create_card`. `layers::recompute_with` applies it in layer 7a
wherever the card is *not* on the battlefield; there the registered static
does, so an effect that removes abilities still removes it. Those cards
join the ids of every refresh, and any move of one invalidates the
projection, since the move cleared its cache (a drawn Ashaya read 0/0 until
something else moved). While a cross-zone effect is registered (Maskwood
Nexus reaches creature cards in every zone) that is every object: the
refresh projects them all, and every move invalidates (a creature card drawn
under a Nexus kept its printed subtypes). Only a printed `Filter::This` P/T with no condition
qualifies; `Modifier::SetPTToCount` is granted, and CR 604.3a counts only
printed, token-made, copied or text-changed characteristic-defining abilities.

Layer 2 is not cached separately: the refresh writes the projected
controller straight into `GameObject::controller`, so every rule that asks
"who controls this" reads one field and none of them has to know that
layers exist. `base_controller` is the controller by default: the player a
permanent entered the battlefield under, or who cast the spell. It is written
only by `GameObject::set_controller`, and only where an object arrives.
**Every change of control is a layer-2 effect** (CR 613.1b), including the
ones that last the whole game: Gilded Drake's exchange, Wishclaw Talisman's
handover, Homeward Path and the control rotation all go through
`resolve::gain_control`, which registers `Modifier::GainControl` for the
player gaining it, `Duration::Indefinitely`, on the one object and its
version. They used to overwrite `base_controller`, which lost the one fact a
player leaving the game turns on (below). The effects keep their
timestamps, so a later taker wins (CR 613.7) and an earlier one's control
returns when the later one ends. `sync_static_effects` drops an indefinite
effect whose object has moved on, since it names an object that no longer
exists (CR 400.7). When the controller moves in either direction the
object's `controlled_since` is restarted, because CR 302.6 wants control
held *continuously* since the turn began. Its timestamp stays: a change of
control is not one of the events CR 613.7 gives an object a new one for.

**A static ability's "you" is whoever controls its source now** (CR 109.5,
611.3a); an effect a resolving spell or ability made keeps the player who
controlled it then (CR 611.2). `ContinuousEffect::origin` says which, and it
is a required field so that every registration site answers: the statics
`sync_static_effects` and `keep_own_statics` register, a copy's "except it
has …" and a token's quoted ability are `Static`; everything a resolution or
a replacement did is `Resolution`, including the lose-all-abilities rider
that lasts while its source stays on the battlefield. For a `Static` effect
`controller` is projection output like `GameObject::controller`: the refresh
writes the source's controller into it, and into every
`ReplacementEntry::controller` (all of them are statics), so each of the
twenty-odd readers keeps asking one field. Only a source on the battlefield
is followed; one that has left keeps its last controller there until the
next sync drops what it registered, which is its last-known information.

The refresh therefore walks the board until a walk moves no controller.
One walk projects one object at a time, and it used to read the stored
controller of any object it had not reached yet, and of the object it was
projecting, so a creature just taken was not pumped by its taker's anthem
until something else invalidated the projection. Each walk first points
every static at its source's controller; the walk that moves nothing read
exactly the controllers it wrote. That is one walk when no control changed,
two when one did, and three for a static that gives control (Control Magic)
whose source changed hands (CR 613.8a). The bound, control effects plus two,
only stops a dependency loop. Summoning sickness restarts once per
permanent whose controller ended the refresh different from how it began it.

## Combat
An attack names a `Defender` — a player or one of the defending player's
planeswalkers (CR 506.2); battles will be the third case, and every match
on the enum is written so adding one is a compile error. The engine
enumerates both halves of the declaration into `Pending::ChooseAttackers` —
which creatures may attack and which defenders may be attacked — and
validates a declaration against those same lists, so a client cannot name
an attacker or a defender the engine did not offer.
`Pending::ChooseBlockers` carries a `BlockOption` per creature that may
block, naming the attackers it may block: evasion is a pairing question
(flying, menace, protection), so a flat list of "creatures that may block"
would be a lie for half of them. Combat damage aimed at a planeswalker
takes loyalty counters off it (CR 306.8); trample past blockers goes to
whatever the creature is attacking (CR 702.19b), and a planeswalker that
has left the battlefield absorbs nothing — the attack stands (CR 506.4c)
but no damage is dealt and no lifelink is paid.

### What must attack, and what may attack whom (CR 508.1c–d)
Two sentences change a declaration of attackers, and both are statics the
layers carry as rules modifiers (`Layer::Text`, beside the other
modifiers that change what a creature may do):

- **A requirement**, `Modifier::AttacksEachCombat` ("attacks each combat if
  able"). A grant is an until-end-of-turn effect with the same modifier, so
  losing the creature's own abilities does not lose one another permanent
  gave it.
- **A restriction on the pair**, `Modifier::CantAttackUnlessDefenderControls`
  ("can't attack unless defending player controls an Island"): the filter
  is asked of the permanents of the player the creature would attack, and
  of a planeswalker's controller when it attacks one.

`combat::AttackRules` collects both once per declaration. The offer drops
a creature that no defender allows, names in `limits` a creature some
defenders do not allow (with the ones it may attack), and lists in
`required` the creatures that must attack. `declare_attackers` refuses a
pair a restriction forbids, and refuses a declaration that leaves out a
creature that must attack and could: untapped, able to attack under CR
508.1a, and allowed at least one defender. That is the whole of CR
508.1d's maximum here, because this engine has no attack costs and no
restriction on how many creatures attack, so obeying one requirement never
costs another. The clock's answer (`choice::timeout_answer`) declares the
required creatures and nothing else, each at the first defender it may
attack; the house AI keeps its own choice and adds what the rules make it
(`combat::obey_attack_rules`).

### Historical land count at turn start

`Engine::begin_turn` records `PerTurn::untapped_lands_at_start` after selecting
this turn's active player, before turn-start expiry and untap-step actions.
It counts visible untapped lands using their controller and projected types;
phased-out permanents are absent. The normal per-turn reset is followed by
storing this snapshot. First turns, extra turns and skipped untap steps use
the same path. `Amount::UntappedLandsAtTurnStart` reads it throughout the turn,
independently of whether the requesting ability existed at the boundary.
The value participates in both the snapshot hash and the loop signature.

### Immutable departure context

A battlefield departure's `JournalEntry::departure` records its controller,
toughness and attached permanents. It survives later moves of the object within
one resolution. `PendingTrigger::event_departure` carries the numeric/controller
context into `Rider::EventDeparture` on the stack; Creature Bond reads it through
`Amount::EventLastToughness` and `PlayerRel::ControllerOfEvent`. Thus reanimation
before collection or resolution cannot substitute new characteristics.

`Effect::DestroyAll` snapshots every member before destruction starts, then uses
`sba::destroy_all` and `move_object_with_departure`. This retains an Aura's
attachment even if the destruction loop happens to visit that Aura before its
host. Individual moves use the ordinary LKI snapshot. Indestructibility and
regeneration still use the same destruction checks. Pending event context and
queued captures participate in the engine snapshot; the stack rider participates
in both state snapshot and loop signature. Existing serialized journal entries
without a departure remain readable via the field's serde default.

### Damage dealt to a player

`GameState::damage_player` is the door for damage that reaches a player, as
`change_life` is for life: combat damage (`combat::deal_damage_to_player`)
and an effect's (`resolve::life::deal_to_player`) both come through it once
the prevention shields have had their say. It loses the life (CR 120.3a),
adds to `PerTurn::damage_dealt_to` and journals `DamageDealt`. The tally is
kept here and not in `change_life`, because it counts damage and not life:
a payment is no damage, and a player whose life can't change is still dealt
it. `Amount::DamageDealtToYouThisTurn` reads it.

`Trigger::PlayerDealtDamage` fires once for all the combat damage of a step
to one player (CR 510.2, 603.2c), on the first of its journal entries, and
its event amount is the step's total to that player (`trigger::event_damage_of`
takes the trigger for that reason). A source's trigger in the same batch
(`DealsCombatDamageToOpponent`) keeps its own share.

At a table of several defending players (CR 802.2, the attack multiple
players option: Team vs. Team's default, 808.3a, and one of Free-for-All's
three options, 806.2b), "each defending player
in APNAP order declares blockers", each all their blocks before the next
(802.4), and "those creatures can block only creatures attacking that
player, a planeswalker that player controls" (802.4a). `can_block` asks
`combat::blocking_player` of the attacker's `Defender` (a departed
planeswalker's last known controller, CR 506.4c, 802.2a), so the offer,
`declare_blockers` and `answer_fault` agree on it.
`Engine::next_defending_player` walks turn order from the active player
(CR 101.4) over the seats something attacks; `declare_blockers` asks the
next one itself (`CombatDeclared::BlockersBy`) rather than returning to the
machine, so no block trigger reaches the stack before the last declaration
(509.2a).

## What must block, and how many (CR 509.1a, 509.1c)
Four rules modifiers change a declaration of blockers, statics or
until-end-of-turn effects on `Layer::Text` like the attack ones:

- **How many attackers a creature may block.** CR 509.1a gives each
  blocker one. `Modifier::CanBlockAdditional(n)` adds `n` (two such effects
  add up) and `Modifier::CanBlockAnyNumber` lifts the limit.
- **Requirements, each about one pair.** `Modifier::MustBeBlockedByAllAble`
  on an attacker (Lure) asks each creature able to block it to do so;
  `Modifier::BlocksEachAttackerIfAble` on a blocker (Blaze of Glory) asks it
  to block each attacker. `BlockRules::demands(blocker, attacker)` counts the
  requirements asking for one pair, so two lures on one attacker ask twice,
  and a declaration obeys the sum over its pairs.

`combat::block_options` is the offer (the `BlockOption`s), and it is also
the universe the maximum is taken in: a pair outside it breaks a restriction
(evasion, protection, a menace attacker no two creatures could block).
`combat::BlockRules` collects the four once per declaration. The question
names, in `capacity`, each offered creature that may block more than one
attacker, and in `obeying` one legal declaration that obeys as many
requirements as the engine holds a declaration to. `declare_blockers`
refuses a pair named twice, a blocker over its limit, and a declaration that
obeys fewer requirements than `obeying` does.

`BlockRules::obeying` gives each blocker the attackers most requirements ask
of it, up to its limit, attackers without menace first; a menace attacker
left with one blocker then gets a second from any creature with room that
may block it, or loses the one it has (CR 702.111b). Without a menace
attacker that a requirement names, the blockers do not touch one another
and this is the maximum CR 509.1c asks for. With one, a blocker's help costs
it its own requirements, and the constructed declaration can fall short of
the best one. Two attackers each enchanted with Lure, one on the ground with
menace and one with flying; two blockers with reach and one without, each
able to block one attacker. `obeying` sends both reach creatures to the
flier (attackers without menace first), leaves the third alone on the
menace attacker, finds it no helper with room and drops it: two obeyed.
One reach creature and the one without reach on the menace attacker, the
other reach creature on the flier, obey three. **An engine
simplification**, lenient only — a declaration
obeying at least as many as `obeying` is accepted, so no legal declaration
is ever refused and the engine never asks for more than it can name. The
clock answers with `obeying` (`choice::timeout_answer`); the house AI keeps
its own blocks for every blocker `obeying` does not use and drops a menace
block left alone (`combat::obey_block_rules`); client-core preselects
`obeying` and never declines blocks by itself while it is not empty.

Counting is over the declared pairs, not the pairs banding adds afterwards
(CR 702.22h makes the band blocked as the block is made, after 509.1c has
checked the declaration). Block triggers are per pair already (CR 509.3b,
509.3d), so a creature blocking two attackers triggers twice; a blocker's
damage divided among the attackers it blocks is asked of its controller
(CR 510.1d, `divisions_owed`).

### Windows in the turn (CR 506.7)
"Cast this spell only before the combat damage step", "activate only during
an opponent's turn, before attackers are declared", "activate only during
your upkeep": each is a `Condition`, asked where the permission is asked.

- **Where.** An activated ability carries it as `condition` (the
  `ActivatedConditional` twin). A spell carries it on `AbilityDef::Spell`'s
  own `condition`, asked by `casting::spell_condition_allows` beside the
  timing the card's type gives it (CR 601.3), in the offer (`can_cast_form`)
  and in the cast wizard. An instant restricted to combat is still cast
  whenever an instant could be, inside its window.
- **Which.** `DuringStep(kind)` is the step; `BeforeStep(kind)` compares
  `TurnInfo::position` with `turn::position_of(kind)`, a place in turn order
  (CR 500.1), so "before the combat damage step" still holds in a declare
  attackers step that has no combat damage step after it and is over at
  end of combat (CR 506.7a, 506.7e). `OpponentsTurn` asks the active
  player's relation to "you", which a teammate's turn does not satisfy;
  `All` joins them. `Step::kind` is the one door from the engine's steps to
  the ones a card names, for `Trigger::StepBegin` as well: both combat
  damage steps are "the combat damage step".
- **What a card with a window asks of the turn.** `PerTurn.attacked` holds
  each creature declared as an attacker, with its version, written by
  `declare_attackers` and nothing else (`Filter::AttackedThisTurn`): a
  creature put onto the battlefield attacking never attacked (CR 508.4), and
  one that left and came back is a new object (CR 400.7).
  `Filter::ControlledSinceTurnBegan` is summoning sickness's measure without
  the creature or haste clauses (CR 302.6). `Effect::IfEventObjectMatches`
  asks a delayed trigger's "that creature" (Berserk's "if it attacked this
  turn").

This engine has one combat phase a turn, so CR 506.7c–d (which of several
combats a window means) never arises.

### Bands, and who divides combat damage (CR 702.22)
Banding is a bit (`KeywordSet::BANDING`) and three questions
(`engine/banding.rs`); "bands with other" (702.22b) is a family a bit cannot
carry, so the reader refuses it and those cards stay unread.

- **The band is announced with the attack** (CR 508.1e). `declare_attackers`
  ends with `ask_band`: each attacker with banding that is in no band yet is
  asked, in declaration order, which other unbanded attackers of the same
  defender join it (`Pending::ChooseCards` with `ChoicePrompt::Band`, none
  allowed). An answer with two creatures without banding is refused and the
  question stands (702.22c, 702.22d). The question stands before the machine
  collects attack triggers, which trigger only on the whole declaration
  (508.1m). Each member is journalled as `GameEvent::Banded`, a log line every
  seat reads, and the view carries the bands as `CombatView::bands`.
- **The band is combat state**, `AttackerInfo::band`: it lasts the combat even
  if banding is lost (702.22e) and a creature removed from combat leaves it
  (702.22f, the attacker entry goes).
- **A block on one member blocks the band** (702.22h):
  `spread_blocks_through_bands` runs after the declared blocks are made and
  asks no legality of the pairs it adds, since the rule's own example is a
  flier's mate blocked by what could block only the flier. Each added pair
  is journalled as `BecameBlocker`, so block triggers fire per pair.
- **Divisions are asked as the damage step begins** (`ask_combat_division`,
  from the priority round that would leave declare blockers or the
  first-strike step). Nobody holds priority between the answer and the
  damage (510.1, 510.2), so no answer meets a board it was not given. The
  last share calls `advance_step` directly: the priority round was complete
  when the first share was asked. `combat::divisions_owed` says who divides:
  an attacker blocked by two or more is divided by its controller, "divided
  as its controller chooses among them" (510.1c), unless one of them has
  banding: then by the defending player (702.22j), among the blockers only,
  so trample puts nothing past them; a blocker on two or more creatures is
  divided by the active player if one of them has banding (702.22k), and
  otherwise by its own controller (510.1d). A blocker deals its power once,
  split across what it blocks, never once per pair. The recorded
  `Division`s are hashed and cleared once the damage is dealt. The house AI
  gives each creature what finishes it, in turn, and the last the rest
  (`NumberPrompt::CombatDamage`); client-core names both creatures in the
  question.

What is still decided for the player: an attacker with trample blocked by
two or more creatures without banding is not asked. The engine assigns
lethal damage to each blocker in declaration order and the rest to what it
attacks, one of the assignments CR 702.19b lets its controller make ("once
all those blocking creatures are assigned lethal damage, any excess damage
is assigned as its controller chooses"); a share question whose last
recipient takes the rest cannot say that bound
(`banding_tests::an_attacker_blocked_by_two_is_divided_by_its_controller`
pins the question without trample). No effect in the pool makes a creature
become blocked, so 702.22i has no door yet.

## Teams: an opponent is a side
A seat carries a `team` from the preset. `GameState::side_of` answers which
side it plays for — its team, or itself when it has none — and `Side` is an
enum rather than an `Option<u8>` so that two teamless seats cannot compare
equal. Every rule that says *opponent* (CR 102.3) asks
`GameState::is_opponent`: who may be attacked and whose planeswalkers, "each
opponent", "target opponent", hexproof (CR 702.11a — a teammate may target
it), "during an opponent's turn", Teferi's sorcery-speed lock, Ashiok,
Opposition Agent, an opponent's graveyard. Rules that say *each other player*
— a draw offer, a symmetrical effect — deliberately do not, because a
teammate is not an opponent but is certainly another player.

The game is decided between sides, not heads: `game_result()` counts the
distinct sides still standing, so one side left is a win and none is a draw.
The winner is a `Victor` — a seat or a team — because a team wins as a team
however many of its members died getting there (CR 104.2c);
`Session::winning_seats` turns one back into the seat list `GameEnded`
carries. `GamePreset::validate` refuses a table where every seat shares a
team, which would otherwise be over at the first state-based-action pass.

`team` is deliberately absent from `snapshot_hash`: it is preset-constant, so
it can tell no two states of one game apart. Turns stay individual and life
totals stay separate — Two-Headed Giant (one turn per team, one life total,
blocking for a teammate) is a further step, not this one.

## Events, replacement, triggers
Proposed events are rewritten by applicable replacement effects (each at
most once per event, CR 614.5), applied, journaled; matching triggers are
collected and stacked APNAP (per-player ordering via ChoiceRequest).
SBAs run as a fixpoint before every priority grant (plus format SBAs).

### Attachments (CR 704.5m, 704.5n, 704.5p)

`eval::permits_enchantment` reads `CantBeEnchantedExceptSource` for Consecrate
Land. Attachment SBAs apply it alongside the enchant filter and protection.
The zone-entry door rejects a forbidden Aura before any battlefield move or ETB
journal entry: it stays in its non-stack zone, or goes from stack to graveyard.
An existing Aura's `AttachSelf` attempt leaves its old attachment intact when
forbidden. The source Aura itself is exempt; another copy is not. Targeting is
still governed separately by the enchant keyword and targeting restrictions.

`sba::run_attachment_sbas` asks the first sentence of CR 704.5p before
anything else: a battle or creature attached to an object or player becomes
unattached and stays on the battlefield, whatever else it is. That is how an
Equipment an effect animates (Karn, the Great Creator's +1) comes off the
creature it equips; an Equipment's own host rule (CR 301.5b, 704.5n) is
satisfied by that creature and would keep it on. Only then are an Aura's host
(its enchant filter, CR 303.4c; illegal or missing → graveyard, 704.5m) and an
Equipment's (a creature; illegal → unattached, 704.5n) asked, and the second
sentence of 704.5p takes any other noncreature, nonbattle permanent off what
it is attached to. Reconfigure keeps its Equipment on because it stops being a
creature while attached (CR 702.151b), which the card states as a static
conditioned on `Filter::IsAttached`. An Aura creature with no host stays on
the battlefield as the stand-in for an unattached bestowed Aura (CR 702.103f);
there is no bestow yet, and any other Aura creature belongs in the graveyard
(CR 303.4d). A Fortification's host is not read at all: the second sentence
of 704.5p spares it and the host check asks only Auras and Equipment, so one
attached to a nonland (CR 301.6) stays attached, where 704.5n would unattach
it. No pool card is a Fortification.

### A delayed trigger that watches an object (CR 603.7)
Earthbend (CR 701.66a) leaves "when that land dies or is put into exile,
return it to the battlefield tapped under your control" behind. It is a
`DelayedTrigger` like the step-timed ones, with `DelayedWhen::DiesOrIsExiled
{ card, version, after }` and `DelayedAction::Trigger { source, effects }`,
but it is never polled at a step: `trigger::watch_triggers` reads it off the
journal with the other triggers, and fires it on the first time that object
leaves the battlefield after sequence `after` (CR 603.7a, 603.7b), if that
was to a graveyard or into exile. It goes on the stack as a synthetic trigger
whose event object is the card and whose source and controller are the
earthbending ability's (CR 603.7e). `queue_new_triggers` then removes every
watch whose object is no longer on the battlefield at `version`, fired or
not: a land bounced to hand is a new object (CR 400.7). The animation binds
the object by version, so the land that returns is a plain land. The return
checks only that the card is in a graveyard or in exile (CR 603.7c); a card
moved from the graveyard into exile in response would come back from exile,
because a synthetic trigger carries no version.

### A permanent spell keeps what was done to it (CR 400.7a)
An effect a resolution registers on one object names it by id and version
(`EffectFilter::ObjectIs`), and the permanent a spell becomes is a new object
(CR 400.7). The one exception is CR 400.7a: an effect from a spell or ability
that changed a permanent spell on the stack goes on applying to the
permanent. `GameState::move_object` re-points those effects, the
`EffectOrigin::Resolution` ones naming the spell's version, on the move from
the stack to the battlefield (`EffectTable::follow_into_permanent`), and no
other move. A Lace cast at a creature spell makes a creature of the new
colour.

### The source on the stack has no version (CR 400.7)
"An object that moves from one zone to another becomes a new object with no
memory of, or relation to, its previous existence" (CR 400.7). An ability
on the stack names its source by id alone (`Resolution.source`), and an id
survives a move: the card that left and came back is at the same id, one
version on. Two scenarios in the pool show it, both through Kenrith, the
Returned King ("{4}{B}: Put target creature card from a graveyard onto the
battlefield under its owner's control"), and both are **known defects**:

- **Scavenging Ghoul.** Its end-step trigger ("put a corpse counter on this
  creature for each creature that died this turn") is on the stack; the
  Ghoul dies in response and Kenrith returns it. `this_object(res)` answers
  `res.source`, compares no version, and the counters land on the new Ghoul,
  an object the ability has no relation to (`resolve::counters`,
  `Effect::AddCounter`).
- **Circle of Protection: Blue.** A Prodigal Sorcerer's ping is on the stack
  and the Circle's controller has chosen the Sorcerer ("the next time a
  blue source of your choice would deal damage to you this turn"). The
  Sorcerer dies and Kenrith returns it before the ping resolves. The
  damage comes from the id, now on the battlefield two versions on (v+2),
  and `ChosenSource::deals` accepts there only the same version or the one
  a chosen spell became: the shield misses damage from the very source
  chosen. Left in the graveyard, the Sorcerer would have been read as it
  last was and the damage prevented (CR 609.7a).

The fix is a version on the stack object and on `Resolution.source`,
compared where the source is read; it is engine-core work and not yet done.

### A triggered mana ability resolves as it triggers (CR 605.4a)
"Whenever you tap a creature for mana, add an additional {G}" is a mana
ability (CR 605.1b: no target, triggers from a mana ability, could add mana;
`AbilityDef::is_triggered_mana_ability`). `Trigger::TappedForMana` matches a
`ManaProduced` whose nearest earlier journal entry about the same object is
its `ObjectTapped` under `Cause::Cost` — the pair every {T} mana ability
writes (CR 106.12, 106.12a) — so the second colour of one activation and a
tap to attack both miss. Who tapped is the event's `player`, held against the
trigger's `by` relation, and the tapped permanent is the trigger's event
object (`trigger::event_object_of`), which is how "its controller" and "that
player" (`PlayerRel::ControllerOfEvent`) find a seat. A triggered mana
ability adds to its own controller's pool unless its effect names another
(`Effect::AddManaFor`, `resolve::mana::add_to`): Gauntlet of Might's {R} for
an opponent's Mountain is the opponent's. `collect_triggers` resolves every queued triggered
mana ability first, through `resolve::run` with `mana_ability: true`, before
any ordinary trigger is asked about: the mana is in the pool when the player
who tapped next has priority, and nothing went on the stack. One that asked
a question would suspend like a colour-choice mana ability; none in the pool
does.

### Miracle can make its mana after the cast choices

Accepting Miracle no longer requires floating mana. After X and targets,
`cast_or_make_miracle_mana` opens a mana-only payment window when the pool
cannot pay and a mana source is available. `PaymentContinuation::Miracle`
holds the completed wizard outside both the active wizard and resolution
slots, so a mana ability can ask a color or cost question safely. The public
debt is the complete chosen cost, including X. Passing attempts that cast
once; an insufficient payment keeps the card in hand, clears announced X,
and spends no part of the spell's cost. Its answers are included in the replay
snapshot. The engine tests exercise an X spell with two targets and a Lotus
Petal color question; `gamehost/tests/ai_miracle.rs` verifies the AI's actual
Temporal Mastery payment and extra turn through the projected debt.

### An upkeep payment is asked after the upkeep's priority (CR 503.1a)
Echo and a pact's "at the beginning of your next upkeep, pay …; if you
don't, you lose the game" are delayed actions, not stack objects, and they
used to run as the upkeep began — before anybody held priority, against a
pool the untap step had just emptied (CR 500.5) and in which nobody could
have made mana (CR 502.4). A payment that the pool could not cover was then
a loss or a sacrifice with no question asked: Pact of Negation lost the game
on eight untapped Islands. An upkeep trigger is put on the stack before the
active player receives priority and resolves after it (CR 503.1a,
CR 117.3a), so `queue_upkeep_delayed` now sets these two aside in
`upkeep_payments`, and `priority_round` answers them where they would have
resolved: when the upkeep's round closes on an empty stack, before
`advance_step` empties the pool — `offer_miracle`'s moment, for
`offer_miracle`'s reason. A player who floats the mana in that window is
asked and pays; one who does not has not paid.

The payment is still not a stack object, so nothing on the board says it is
coming: a client or the house AI sees an empty upkeep. Making it one — with
the CR 605.3a window a `PlayerMayPay` tax already opens — is the rest of this.

### The cleanup step checks once, and may give priority (CR 514.3a)
A cleanup step is its two turn-based actions and then one check. First the
active player discards to their maximum hand size (CR 514.1). Then damage
wears off and "until end of turn" effects end (CR 514.2). An effect that
ends at 514.2 still applies while the discard is being asked for.
`Engine::cleanup_step` does 514.1 and `cleanup_ends_the_turns_effects` does
514.2, either straight after it or from the discard's answer.

The check is `run_machine`'s own next pass, in its usual order: 0a (with the
CR 800.4c exile below), the state-based actions, then the triggers.
`Engine::cleanup` remembers where the step stands:

- `Due`: the turn-based actions are still to come.
- `Checking`: the pass is the check.
- `Open`: the check performed a state-based action or put a triggered
  ability on the stack.

In `Checking`, `progress_step` ends the turn (`end_cleanup`) if the stack is
empty and nothing was noted. Otherwise the step is `Open` and gives the
active player priority like any other. A round that closes on an empty stack
goes through `advance_step`, whose Cleanup arm begins another cleanup step
(`Due` again): mana empties, its 514.2 ends what the window made, and its own
check decides again. The field survives a question, so a window whose
trigger has resolved is not mistaken for the check.

"Performed" is noted where it happens (`cleanup_check_acted`), because the
board does not always show it afterwards. `sba::run`'s outcome is noted, and
so is `finished_sagas`. So are the two removals in `collect_triggers`: no
mode chosen, and no legal target (CR 603.3d). Each of those abilities was
waiting to be put on the stack, which is what the check asks. The journal
can't stand in for this. An Equipment coming off a land whose animation
ended (CR 704.5n) and a +1/+1 counter cancelling a -1/-1 (CR 704.5q) are
state-based actions that journal nothing and move nothing, and each one
opens the window (`cleanup_tests`, Mutavault and Leonin Scimitar).

A cleanup step that makes another one every time is a loop through priority.
The machine's own watch starts over at every question, so the one that sees
it is `Engine::apply`'s action watch (`crate::loops`).

### The clause that is asked twice (CR 603.4)
A triggered ability may print an **intervening `if`** — the `if` between the
trigger event and the effect, as in "at the beginning of your upkeep, if this
land is tapped, put a storage counter on it". It is one clause and two
checks, and this engine makes both through one reader,
`eval::intervening_if`:

- **When it would trigger.** `trigger::collect` skips the ability entirely
  while the clause is false, in both of its loops (battlefield/look-back and
  the command zone's emblems), and *before* `trigger_count` — a trigger
  multiplier doubles a trigger, not a non-trigger.
- **When it would resolve.** `Progress::resolve_stack_top` asks again before
  it records anything, and an ability whose clause has stopped being true is
  removed from the stack and does nothing: `GameEvent::StackObjectDidNotResolve`,
  then the object ceases to exist the way a countered ability does
  (CR 608.2n). The order matters — the journal used to record
  `StackObjectResolved` before the branch that decides, and an entry saying
  an ability resolved followed by one saying it did not is a different rule
  to everything that reads the log.

Both checks ask the clause of the **ability's own controller** and its own
source, read off the object on the stack rather than off the permanent: the
two have been separate objects since it was put there (CR 113.7a), and a
source that has left the battlefield is exactly the case a clause about it
has to be able to fail on.

CR 608.2b's target re-check goes through the same door and is asked in the
same place: a spell or ability all of whose targets have become illegal also
does not resolve. `Engine::target_legality` asks it, above the spell/ability
split rather than inside either branch, because an Aura is a targeted
*permanent* spell and a check in one branch would miss the other. It asks with
`eval::stack_target_options` (`eval::target_options` and
`eval::target_player_options`, with the object's own controller and source),
which are the enumerations that offered those targets in the first place —
one predicate read from both ends, so an offer and a re-check cannot disagree
about what was choosable. A change of targets (CR 115.7, `resolve::retarget`)
asks the same function, so a redirected spell is offered only what the
re-check would call legal (#247).

Partial legality is handled and not merely survived: the legal subset is
written back to the object once, before any `Resolution` is built, which is
safe because a `TargetReq` carries one spec and nothing reads `targets` by
index. What is not handled is the rule's own example — "for every instance of
the word 'target'" needs two separate instances, and this DSL has no way to
spell a second one.

A spell leaves by `Engine::leave_stack_without_resolving` and not by
`finalize_spell`: rebound (CR 702.88) and an Adventure (CR 715.3d) exile a
card *as it resolves*, and one that never resolved has done neither.
Flashback is the rider that does apply, because CR 702.34a exiles the card
"any time it would leave the stack".

### State triggers (CR 603.8)
"When you control no Islands, sacrifice this creature" triggers on a state
and not on an event: `Trigger::State(&Condition)`. No journal entry matches
it (`trigger::hits` answers 0), and `trigger::state_triggers` walks the
battlefield on every pass of `queue_new_triggers`, asking the condition
with the permanent's controller as "you". An ability that is still waiting
in the trigger queue, or is on the stack (an `AbilityOnStack` whose
`AbilityLoc` names the same source and index), does not trigger again;
once it has left the stack it triggers at once if the state still matches.
The condition is the trigger's and not an intervening "if": the ability
resolves even when the state has ended by then.

Two limits. The in-flight check keys on the source's id, which this engine
keeps across a zone change, so a permanent that left and came back while
its ability is on the stack waits for that ability to leave before its own
can trigger (CR 603.8 would let the new object trigger at once; the same
missing version as an event object on the stack). And a copy of the
ability carries the same `AbilityLoc`, so it too holds the next trigger
back until it has left the stack.

### The one check in the fixpoint that is not a state-based action
Daybound and nightbound (CR 702.145c–g) are checked as their own step of
`Progress::run_machine`, between the state-based actions and the trigger
collection, and both halves of that placement are deliberate.

They are not SBAs — CR 702.145c and f say "this happens immediately and
isn't a state-based action" in as many words — and they need the card
definition behind a permanent, which `sba::run` has no lookup for.

The step immediately above them, `Progress::finished_sagas`, is there for
only the second of those two reasons. Sacrificing a Saga whose lore counters
have reached its final chapter (CR 714.4) *is* a state-based action, and it
sits outside `sba::run` because it has to read the permanent's abilities to
find out what that chapter number is. It used to sit somewhere else
entirely — on the way out of the last chapter's resolution — which left a
Saga on the battlefield for the rest of the game if that chapter was
countered. CR 714.4's second clause is why it still cannot fire from there:
a Saga is spared while it "isn't the source of a chapter ability that has
triggered but not yet left the stack", and a chapter that has triggered is
in the trigger queue before it is on the stack, which is why
`a_chapter_of_it_has_triggered` consults both. Reading only the stack would
sacrifice the Saga one step before CR 117.5 puts its last chapter on it.

Daybound and nightbound run
after the SBAs have settled so that a permanent about to die does not turn
over first, and before triggers are collected so that a permanent which does
turn over has done so before anything asks what triggered.

The step reports whether it changed anything, and the two shapes it must
refuse are why that answer has to be exact. A token has no card, and a clone
of a werewolf carries the copied daybound over a definition with one face
(CR 701.27c) — turning either over would rebuild its base from a face that
is not there. Both are skipped, and skipped *without* reporting a change: a
guard that reported one would send the fixpoint round forever on a permanent
it had just declined to touch.

`GameState::transform` is the door that journals `Transformed`. Its
neighbour `switch_face` does the same work silently and is what the modal
paths call, because choosing which face of an MDFC to cast or to play as a
land is not a transform (CR 712.11b and CR 712.12).

### A replacement that has to ask (CR 903.9b)
`GameState::move_object` is the one funnel every zone change goes through,
and it is synchronous: it cannot stop and ask a player anything. CR 903.9b
needs exactly that — "if a commander would be put into its owner's hand or
library, its owner **may** put it into the command zone instead" — so the
question is asked *before* the operation that would move it, and the answer
waits in `GameState::commander_redirect` for the funnel to spend.

The order is what makes it a replacement rather than a correction. Asking
afterwards and moving the card a second time ends in the same zone and is a
different game: "shuffle target creature into its owner's library, then that
player draws a card" draws from a library the commander is in, a hand size is
briefly wrong, and anything watching for a card entering a hand has already
fired. So `resolve::ask_commander_replace` suspends the resolution *at the
same program counter* with nothing yet mutated, collects one answer per
commander (the rule "may apply more than once to the same event", which a
mass bounce is), and the last answer re-enters the operation, which then runs
once with every answer in hand. Any effect that calls it must ask before its
first mutation, or the re-run does that mutation twice.

Where it does **not** reach yet, all of them paths where the move happens
inside a choice that has already been answered or outside a resolution
altogether: the turn-based and effect draws (`GameState::draw_cards`), a
search or wish that finds a commander in a library, the hand-to-library
put-backs (`AwaitingOp::PutBackOnTop`, `BottomFromHand`),
`CostPart::ReturnSelfToHand`, and `AwaitingOp::ReturnChosen` (the bounce
land's "return a land you control to its owner's hand"). Each fails safe — no
entry in `commander_redirect` means the printed move stands and no question is
asked.

The last of those is the one with a reason rather than an oversight, and it
is the reason the list is worth reading before adding a call. Asking from a
**continuation** arm does not work at all: the re-entry above is what makes
the rule a replacement, and the operation a continuation would re-enter is
the whole per-player chain — so the first player would be asked to choose
their permanent a second time. Its two siblings never want the call
(`SacrificeFilter` and `DestroyChosenForPlayers` both end in a graveyard,
which is CR 903.9a, a state-based action, and not this replacement at all),
so closing it means giving that one chain a way to ask before it moves
anything. Nothing reaches it today: the only filter the pool writes for that
effect is `Land.YouCtrl`.

### The replacements that multiply, and their three doors
Doubling Season and its kin do not rewrite an event; they multiply what an
effect produces (CR 614.16 for tokens, CR 614.16 for counters), so they live
in `engine/replacement.rs` and are read at the moment of production rather
than in the propose/apply funnel above. There are three doors and every
producing effect goes through one of them:
`replacement::put_counters`, `resolve::tokens::create_tokens` and
`resolve::tokens::create_token_copies`.

Doors, not helpers, because the bug is always the same one: a counter or a
token placed beside the rule is invisible to it, and nothing says so until
someone plays the pair. Thirteen call sites are behind the three doors
today — nine creating tokens, four placing counters — and every one of them
was outside until the commit that put it there: `AddCounterFilter` ("put a
+1/+1 counter on each other Ally you control", the shape most of the pool
writes), a planeswalker's starting loyalty, the three token-copy branches,
the two effects that hand tokens to *someone else*, and the sized token an
exiled card leaves behind.

The direction each rule reads is the part that is easy to get backwards, and
the two read opposite ways. The token rule filters the **affected
controller**: "if one or more tokens would be created under *your* control"
says nothing about whose spell is creating them, so Crib Swap's Shapeshifter
is doubled by the Doubling Season of the player whose creature was exiled,
never by the caster's. The counter rule filters the **object receiving
them**: "a permanent you control" is about the permanent, so an opponent's
spell putting a counter on my creature is doubled by mine.

`Amass` is the one deliberate hole and is documented at its call site: two
Armies would need a choice of which one takes the counters, and amass has
nowhere to ask it.

### Regeneration, the replacement that is not in the funnel

A regeneration shield (CR 701.19a) is a replacement effect and is read
nowhere near the propose/apply funnel above, because the event it replaces is
not proposed at all: destruction is reached from four places — the
lethal-damage state-based action in `sba.rs`, and the three resolving
effects in `resolve/` — and the one thing they share is `sba::destroy`. So the
shield is a count on the object (`Object::regeneration_shields`) and
`sba::destroy` is where it is spent — the creature is tapped through
`GameState::set_tapped`, its marked damage and its deathtouch flag are
cleared, and it is removed from combat. `sba::destroy_no_regen` is the same
door with the shield skipped, which is CR 701.19c and the eight cards in the
pool that print "it can't be regenerated".

Three things about it are easy to get backwards:

- **The shield answers destruction and nothing else.** Zero toughness
  (CR 704.5f), a planeswalker at no loyalty (704.5i), the legend rule
  (704.5j) and a sacrifice do not destroy, so they go through
  `put_into_graveyard` and a shield does not see them. `sba::destroy` is the
  *only* door that reads it, which is what keeps that true without a rule
  per case.
- **It is a count and not a flag**, because two activations in one turn are
  two shields and the second has to survive the first destruction.
- **It is cleared in the same two places `deathtouched` is** — the cleanup
  step (CR 514.2, so "this turn" means what it says) and leaving the
  battlefield — and it is in both `hash_object` and
  `hash_object_situation`, because a board with a shield up is not the same
  position as the board without one and loop detection would otherwise call
  them equal.

### Prevention shields, and the question the engine does not ask (CR 615)

"Prevent the next 3 damage that would be dealt to any target this turn" and
Fog leave a shield behind as they resolve (CR 615.1, 615.3), and the shield
waits for damage. The shields live in `GameState::shields` in the order they
were made, and `prevention::apply` is the one function that spends them:
every writer of damage asks it how much of what it is about to deal still
gets through, after the standing prevention it already asked (a permanent's
protection, which every writer asks, and Maze of Ith's `PreventDamageToIt`
and `PreventDamageFromIt`, which only combat's two ask: both cards that
carry them, Maze of Ith and Kor Haven, prevent combat damage only; none of
these is ever used up) and before anything is lost, marked or journalled. Four writers ask today — two
in `combat`, two in `resolve::life` — and damage prevented in full is never
dealt at all: no life change, no `DamageDealt`, no deathtouch, no lifelink.

- **A shield on a permanent is on that object** (id and version, CR 400.7):
  the creature that leaves and comes back is a new object with no shield.
- **Damage that can't be prevented passes every shield untouched** and
  reduces none of them (CR 615.12).
- **Every shield ends at the cleanup step** (CR 514.2); they all say "this
  turn". They are in `snapshot_hash`, `loop_signature` and the fingerprint.
- **A chosen-source shield** ("the next time a red source of your choice
  would deal damage to you", CR 615.8) is chosen as the ability resolves,
  from `prevention::source_options` (CR 609.7a: permanents, spells, and the
  source of an ability on the stack even once it has left), and waits for
  that source's next instance of damage to its controller. It rechecks the
  source's properties when the damage comes, against the source's last
  known characteristics if it has left, and a shield that prevents nothing
  is not used up (CR 609.7b). A damage source is an id, so which incarnation
  dealt the damage is read from where the id is now
  (`ChosenSource::deals` names the two corners that reading gets wrong).

- **Counters that prevent** (Rock Hydra, `Modifier::CountersPreventDamage`)
  are a static prevention effect, asked by both object doors through
  `prevention::absorb` after the shields and after any redirection: each 1
  damage takes a counter and is prevented while one is there. Damage that
  can't be prevented still takes the counters and is dealt in full, the
  removal being an effect of its own (CR 615.12), once for the event
  (615.12a).

The question the engine does not ask is CR 616.1's: when two shields could
apply to one event, the affected player (or the controller of the affected
permanent) chooses which applies first — and CR 615.7's last sentence, which
of several simultaneous sources one shield prevents. `prevention::rank`
applies them in a fixed order instead. For most pairs it is the order the
player would always pick: a shield that prevents nothing is not used up, so
the fuller shield first leaves the other standing (Fog before a Circle of
Protection, a Circle before Forcefield), and two "next N" shields spend the
same total either way. Two pairs are trades, and there the engine decides
what the player would be asked — **an engine simplification**: Reverse
Damage goes before Fog (the life now, rather than Reverse Damage kept for
that source's later damage: a creature chosen for Reverse Damage attacks
into a Fog, its combat damage spends Reverse Damage and gains its life,
and that creature's later damage that turn is dealt in full), and a
chosen-source shield before "the next N"
(the N kept for any source, rather than the chosen-source shield kept for
its one). A new kind of shield is placed in that order with its reason; a
pair for which the fixed order would often be the wrong answer needs the
question rather than a rank. Counters that prevent come after every
shield: a shield ends with the turn and a counter does not, so spending
the shield first is the choice a player would always make.

### Redirection: damage dealt to another instead (CR 614.9)

"All damage that would be dealt to you by unblocked creatures is dealt to
this creature instead" (Veteran Bodyguard) and "the next time a source of
your choice would deal damage to target creature this turn, that source
deals that damage to you instead" (Jade Monolith) are replacement effects
that move damage (CR 614.9). `prevention::redirect` answers where a writer's
damage goes instead, and all four writers ask it after the shields in front
of the first recipient and before anything is lost, marked or journalled; a
redirected amount goes through the door for the new recipient, where that
recipient's protection and shields meet it (Veteran Bodyguard with
protection from red takes nothing from a red attacker). The damage keeps its
source and whether it is combat damage, so deathtouch and lifelink read it
as ever; the player it was moved off is dealt nothing — no life, no
`per_turn.damage_dealt_to`, no "whenever you're dealt damage", no commander
damage.

- **The static** is `Modifier::RedirectDamageToYou(&from)`: damage a source
  matching `from` would deal to the effect's controller is dealt to the
  permanent the effect applies to. `from` is read on the source as it is
  then, or as it last was once it has left the battlefield (CR 609.7c): the
  ability of a red creature killed in response is still a red source's.
  Combat status is not remembered, so an unblocked creature that has left
  is no longer one. That is **a guess** where the rules do not settle it:
  CR 609.7c applies such an effect "to any sources that aren't on the
  battlefield that have that property", and CR 506.4 says only that a
  creature removed from combat "stops being an attacking, blocking,
  blocked, and/or unblocked creature". An unblocked Mogg Fanatic sacrificed
  ("Sacrifice this creature: It deals 1 damage to any target") at the
  Bodyguard's controller deals that 1 to the player; read by its last
  known information, as it was just before it left, it would be an
  unblocked creature's damage and go to the Bodyguard. The affected permanent is read then too
  (`effects::applies_to`), so Veteran Bodyguard's "as long as this creature
  is untapped" is in its affected filter, `And(This, Untapped)`, and a
  Bodyguard tapped earlier in the same resolution is already out of the way.
- **The shield** is `ShieldKind::RedirectNextFrom { source, to }`, made by
  `Effect::RedirectNextFromChosenSource` as it resolves: the source is
  chosen then (CR 609.7a), rechecked when the damage comes, used up by the
  damage it moves and kept by damage it does not (CR 609.7b), and on the
  creature as the object it was (CR 400.7). `prevention::apply` passes it by.
- **Once to an event** (CR 614.5): a writer starts a `prevention::Redirected`
  per event and hands it on with the damage, and a static that moved the
  damage is not asked again — Jade Monolith's shield on a Veteran Bodyguard
  sends the damage the Bodyguard took back to its controller, who is dealt
  it. A shield needs no entry: it is gone once it has moved something.
- **Nothing** is moved from or to a permanent that is no longer a creature
  on the battlefield, or to or from a player who has left the game (CR
  614.9); such a shield is still waiting afterwards.

**An engine simplification**, beside the one above: CR 616.1 lets the
affected player order redirection and prevention too, and the engine always
applies the shields first. That is a trade: preventing first spares the
creature a Bodyguard puts in the way and spends the shield; redirecting
first keeps the shield and costs the creature. Counters that prevent (Rock
Hydra) come after the redirection, so damage a Jade Monolith moves off the
Hydra costs it no counter. Among several redirections the oldest shield goes
first, then the oldest static, again without asking.

### The monarch's abilities have no source (CR 724.2)

`trigger::monarch_triggers` reads both off the events and queues synthetic
triggers from `ObjectId::NO_SOURCE`, controlled by the monarch they
triggered against; on the stack they are named "Monarch". The takeover is
`BecomeMonarch(ControllerOfTarget)`, read off the creature it carries.

### Undying and persist, and the question about an object that is gone

Both are keyword *triggered* abilities (CR 702.93a, CR 702.79a) and neither
is written as an `AbilityDef`: they are bits on `keywords`, and `trigger.rs`
reads them where it reads prowess and ward, pushing a `PendingTrigger` whose
effects come from a `&'static` list rather than from the card: setting the
bit is the whole of writing the card, and `keywords = KeywordSet::UNDYING` is
all of Young Wolf.

The bit that is read is the **printed** one. `move_object` clears the
object's layer cache (CR 400.7) and `Characteristics` falls back to the base,
so by the time this scan runs a continuous effect that *granted* undying has
stopped applying and left nothing behind — the same last-known-information
problem as the counters below, one field over, and not closed: it would need
a look-back store for the projected keywords. Mikaeus, the Unhallowed is the
one card in the pool that grants undying, and it is `Coverage::Partial` for
three reasons of which this is one.

What makes them different from prowess is the intervening `if`: "if it had no
+1/+1 counters on it" is a question about the creature **as it last existed
on the battlefield** (CR 603.4, checked when the ability would trigger), and
by then `move_object` has cleared its counters along with everything else
only a permanent has. Reading the card in the graveyard therefore answers
"no counters" every time and the creature returns for ever. So there is a
fourth look-back store beside `ltb_abilities`, `ltb_attachments` and
`ceased`: `GameState::ltb_counters` holds what a permanent wore on the way
out, is overwritten by the next departure of that object, and is in neither
hash — it is a record of what has already happened, so two positions that
differ only in it are the same position.

Three more things the rule needs, each of which was wrong first:

- **The stack object has to carry the event object.** A synthetic trigger's
  effect list names the creature with `TargetSpec::EventObject`, which
  `resolve::zones::spec_object` reads off `Resolution::event_object` — a
  field the non-synthetic branch of `progress::stack_triggers` wrote and the
  synthetic one did not. The trigger fired, reached the stack and resolved
  into nothing.
- **A token does not come back.** CR 111.7 — it ceased to exist — so the
  trigger is not pushed for a source with no card behind it.
- **The card has to still be in the graveyard.** CR 400.7: the return targets
  nothing, so nothing else would stop it pulling a card out of *exile* if
  somebody exiled it in response.

The fifth store is what the permanent **was**. `GameState::ltb_characteristics`
holds its projected characteristics as it left the battlefield, written and
cleared where `ltb_abilities` is, and the three leaves-the-battlefield
triggers (`Trigger::LeavesBattlefield`, `ExiledFromBattlefield`, `Dies`, one
match arm) ask their filter of it (`trigger::departed_matches`, CR 603.10a).
Before it, they asked the card in the graveyard: a Forest that Living Lands
had made a creature died as a land and "whenever a creature dies" never saw
it, and an Enduring Vitality that had come back as a non-creature
enchantment died as an enchantment creature card and came back again. Nothing else reads it; every
other question about a card off the battlefield is about the card as it is
now. The undying scan above still reads the printed keyword bits and could
read this store instead.

### "When you do": a trigger the resolution creates (CR 603.12)

A reflexive triggered ability is not in any card's ability list. The
resolution that caused its event creates it, and CR 603.12 has it check that
event "earlier during the resolution", never later. `Effect::Reflexive` is
written as the last op of the list, directly after the action it waits for,
and `resolve::reflexive::arm` does three things.

- **It reads what this resolution did from the journal.** `resolve_stack_top`
  records `StackObjectResolved` after the CR 603.4 and 608.2b checks and
  before any effect runs, and resolutions never interleave. So the entries
  after the latest marker for `res.on_stack` are this resolution's own.
  Nothing is carried on `Resolution`, and a question inside the action
  (Eden's "you may sacrifice", which splices its tail and resumes) changes
  nothing. A resolution whose `on_stack` is not on the stack, which is a mana
  ability's (CR 605.3b), reads nothing. Its `on_stack` is the permanent, and
  the permanent keeps the id of the spell it resolved from.
- **It counts the event, once per occurrence (CR 603.12a).** `SacrificedThis`
  is a departure of the source from the battlefield with `Cause::Effect`. A
  cost records `Cause::Cost`, so a payment window is never the action.
  `lints::every_reflexive_sits_where_it_can_trigger` is what makes "any
  departure by effect" mean "the sacrifice": it requires the sacrifice
  directly before the reflexive, and allows nothing before the sacrifice
  that could move the source.
- **It queues a synthetic trigger in `GameState::reflexive`.** The trigger
  has the source and controller of the ability that created it (CR 603.7e)
  and no event object. `finish_resolution` moves it into the trigger queue
  as its first act. Every completed stack resolution passes through there,
  and it runs before step 0b of `run_machine` can publish an as-enters
  question. From then on it is an ordinary synthetic trigger. It is stacked
  the next time a player would receive priority (CR 603.3), it asks for its
  target then (CR 603.3d, applying 601.2c), and it is removed when it has
  none.

Two older rules had to be right first, and each was wrong.

- **`SacrificeSelf` moved its source from anywhere.** A land bounced in
  response went from hand to graveyard, and a land whose control had changed
  was sacrificed by a player who no longer controlled it. CR 701.21a allows
  neither. `resolve::zones::can_sacrifice_self` checks zone, controller and
  phasing. `MayDo` reads the same predicate so that it does not offer an
  impossible "yes" (CR 608.2d).
- **A synthetic stack object answered `None` for its target requirement.**
  That meant CR 608.2b never re-checked a granted or reflexive target. The
  requirement is now written on the object when the trigger is stacked, and
  `stack_target_req` reads it back.

## Two instances of "target", and a board that moves mid-resolution

CR 115.3 counts targets per **instance of the word**: Khalni Ambush's "target
creature you control fights target creature you don't control" is two
requirements, each with its own filter, and the same object may be named once
for each. `res.targets` had been one flat list read as one instance by every
reader — ward, `BecomesTarget`, narrowing, the effect arms — so the second
instance is a **second list at every layer** and is never appended to the
first: `second_targets` on the DSL ability, on `GameObject` (with the
requirement it was chosen under, `second_target_req`), on `Resolution` and on
`CastWizard`. `GameObject::targets_object` is the one question "is this object
targeted by it at all", and the readers that meant that ask it. Both lists are
hashed, because two stacks that differ only in what the second instance named
are two positions.

It is asked as its own stage in both doors: `WizardStage::SecondTargets`
after `Targets` for a cast, and `PlanKind::ActivateAbilitySecondTargets` after
the first target for an activation, whose answer re-enters `start_activation`
the way every other plan does. The offer asks both: a spell whose second
instance has nothing legal to name is not castable (CR 601.2c), which
`casting::face_has_a_legal_target` and the activation offer both check.

Because one question is one instance, **an answer that names one thing twice
is refused at the door**, before anything moves — and not only for targets.
Every door taking a list asks `names_one_twice` in `actions.rs`: the target
question (and the convoke question, which arrives as the same variant), every
`ChooseCards`, the cleanup discard and the mulligan's bottom. Each of them had
checked the answer's length and its members and never compared them, so
`[c, c]` passed wherever two was a legal count and bought two of whatever the
next reader counted — two convoke mana for one tap, two delve mana for one
exile, a seven-card hand after a mulligan owed two. `answer_door_tests` has
one per door. The combat declarations take lists too and refuse a repeat in
`declare_attackers`/`declare_blockers` themselves.

Narrowing (CR 608.2b) is **per instance**. `progress::target_legality` asks
each list separately and returns `AllIllegal` only when every instance that
was asked lost everything; an instance that lost its one object is empty and
the spell still resolves for the other. That is also why the lists stay
apart — narrowing the first would otherwise shift the second's positions and
hand a `TargetSlot::Second` whatever used to be third. `Effect::Fight` checks
CR 701.14b on its own as well, battlefield and creature on both sides, because
a `TargetSlot::This` fighter is not a target and is never narrowed. That half
is a guard without a proof: no card in the pool fights with `This` yet, so
the first one (Golden Guardian) owes the test of its source leaving in
response.

The other half is that **an effect list can change what the next effect
reads**. The layer projection is a generation compare, refreshed between
engine steps — and a resolution is one step. Bridgeworks Battle's +2/+2 was
registered and its fight then read the stale power, so `resolve::run` now
calls `refresh_characteristics` before every effect. It costs a `u64` compare
when nothing changed, and it is the rule rather than the fight's: any
sentence that reads a characteristic after an earlier sentence changed it was
wrong the same way.

## Cards put into places: `Pending::Arrange`

"Put them back in any order" and "the rest on the bottom in any order" are
one question with different destinations, and so are scry and surveil — and
the piles of CR 700.3 once they move onto it (#202). A `Pending::Arrange` names
the cards and a list of `ArrangePile`s — a place, a `min`/`max` and whether
the order inside is the player's — and `PlayerAction::Arrange` answers with
one list per pile, in the order the question gave them. It replaced
`OrderObjects`, which carried a bare permutation and no destination, so a
client could not tell whether the first card it was handed would be the top
card or the one just above the bottom.

Two readings are fixed and are what `arrange_tests` hold against the library
itself rather than against the answer:

- **Library piles are listed top to bottom**, whichever end they go to: the
  first card of a `LibraryTop` pile is the new top card, and the last card of
  a `LibraryBottom` pile is the new bottom card. A pile reads the way the
  library will lie.
- **An answer is every offered card exactly once**, each pile within its
  bounds, checked by `choice::arrangement_fault` before anything moves. An
  unordered pile's order is not read.

A **scry** is two piles, the top and then the bottom, each taking any number
of the looked-at cards in an order the player chooses (CR 701.22a), and a
**surveil** is the top and then the graveyard (CR 701.25a) — an unordered
pile, because nothing reads the order in which cards enter a graveyard
together. Both used to be a `ChooseCards` naming the cards to send away,
with the rest left on top in the order they lay: the "in any order" of both
rules was approximated away, and a player could not scry two and keep the
second card above the first. The prompt (`ArrangePrompt::Scry` /
`Surveil`) is what a client titles the question with; the piles alone would
not say which rule is being applied. The library a scry puts cards back into
is the one they were looked at in, which for a "look at the top card of
target player's library" is not the controller's, and a surveiled card goes
to its **owner's** graveyard.

`choice::default_arrangement` is the answer with no preference — the cards
as offered, each pile filled to its minimum first — and is what the house AI
and the test kit give until they have an opinion. It returns `None` only for
piles whose bounds cannot hold the cards, which no question the engine asks
ever has. A look at a single card is still asked, though it has no order to
choose: a card is shown to its player only while a question about it is open
(the view's `looking_at` is read off the offer), so skipping the question
would take away the look the card prints. What is *not* asked is the rest of
a dig with one card left, which its player has just seen in the question
before.

## Phasing: the battlefield the rules see (CR 702.26b)
A phased-out permanent is treated as though it does not exist, except by
rules and effects that mention phased-out permanents.
`GameState::battlefield_seen` is that battlefield (`battlefield_view`
collects it). A raw `zones.list(ZoneLocation::Battlefield)` walk also yields
phased-out permanents, and `eval::matches` never reads `PHASED_OUT`. A count
over a raw walk therefore saw a phased-out Swamp: checklands and fastlands
asked `controls_count` exactly that way (#209).

Every such `.list` walk says why it may see phased-out permanents, in a
`// phasing:` comment within three lines above it (for example, the untap
step phases them in). Otherwise it is listed in `phasing_tests::UNAUDITED`.
Walks over every object (hashing, the projection refresh, cleanup) are not
battlefield queries and are not counted. The table must
match exactly: auditing a walk lowers its row, and a new unexplained walk
fails the test. It has been empty since 2026-09-29: "destroy all
creatures" (`Effect::DestroyAll`), mass bounce, counts and conditions,
control rotation, a Saga's counters and its sacrifice, day and night, the
untap step, and the rest now walk `battlefield_seen`.

What a phased-out permanent has does nothing either. The offer
(`compute_legal`) and the trigger scan (`trigger::collect`) walk
`battlefield_seen`, so none of its abilities is offered, a mana ability
included, and none of its triggered abilities triggers. Its static effects
are parked: `EffectTable::follow_phasing`, first in `sync_static_effects`,
moves them out of the table every reader sees and puts them back, same id,
same timestamp, same place in registration order, once it has phased in.
Phasing in is not entering (CR 702.26d), and some statics are registered
only once (a copy's own, a token's), so they are kept rather than rebuilt.
Its replacement rules are dropped by `sync_replacement_rules` and scanned
back from its abilities when it phases in. Effects a resolution made are
not parked. The leave probe (`docs/verification-hooks.md` §"L4: the leave
probe") checks all of this per card.

**Phasing out and in** is `GameState::phase_out` and `phase_in`
(`phasing.rs`), which `Effect::PhaseOut` and the untap step share.
`Effect::PhaseOut` phases out every target it has (Clever Concealment's
"any number"), or its source. The Auras, Equipment and Fortifications
attached to a permanent phasing out go with it, transitively, and carry
`Status::PHASED_OUT_INDIRECTLY` beside `PHASED_OUT` (CR 702.26g); one also
named directly phases out only indirectly (CR 702.26h). The set is read
before any status changes. The untap step phases in what the active player
controlled as it phased out (CR 502.1) and was not phased out indirectly;
`phase_in` brings along what phased out indirectly with it. Neither touches
`attached_to` (CR 702.26d, 702.26j): a directly phased-out Aura whose
creature has gone phases in attached to nothing there, and the attachment
state-based actions deal with it (CR 702.26i, 704.5m). The view shows the
four statuses of CR 110.5 (`Status::public`), so a client sees an
indirectly phased-out Aura as phased out.

**A phased-out permanent is not projected.** `refresh_characteristics`
leaves it out of the objects it walks, so nothing changes it while it is
phased out (CR 702.26b): an anthem that arrives meanwhile applies to it
once it has phased in, and not before. Its controller stays the one it
phased out under, which decides the untap step it phases in at: a creature
stolen until end of turn and phased out comes back at the thief's untap
step and is then its owner's again, because the theft ended while it was
away (CR 702.26f). The one reader that must see past the freeze is a
player leaving: the effects that gave them control end (CR 800.4a), so
`GameState::release_from_the_departed`, first in
`sba::exile_what_the_departed_control`, reads layer 2 again for a
phased-out permanent whose controller has left. What they control by
default is exiled with the rest (CR 702.26n); what they had through an
effect goes back and phases in at its controller's untap step. CR 702.26n
says "the next untap step after that player's next turn would have begun",
which can be a round later; the engine keeps no such turn. Nor does it
phase in an Aura that phased out indirectly with a permanent that then left
the game with its owner (CR 702.26k): the rules say nothing of it, and it
stays phased out. Both helpers invalidate the projection. One consequence is
seen only in the view: a permanent that phased out under an anthem that has
since left still shows the anthem's +1/+1 until it phases in, which is when
the rules look at it again.

A continuous effect from a resolution leaves a phased-out permanent out of
its set, "the permanent specifically" included (CR 702.26e): `bound_now`
walks `battlefield_seen`, and `this_to_affect` names nothing phased out, so
a set fixed while the permanent was away (CR 611.2c) stays without it after
it phases in. Targeting already read `battlefield_view`.

**"For as long as" (CR 702.26f), one duration of it.** An effect with a
"for as long as" duration that tracks a permanent ends when that permanent
phases out, "because they can no longer see it". One such duration has a
name: `Duration::WhileYouControlSource` (Extraction Specialist's "for as
long as you control this creature"). A phased-out source is one nobody
controls (CR 702.26b), so `end_control_durations` ends the effect at the
next pass as it would for a stolen source, and it does not begin again as
the source phases in. An effect that would begin while its source is phased
out ends at the same pass (CR 611.2b); nothing sees it in between.

**Not yet: the other "for as long as".** The table cannot tell the rest.
On a resolution's effect `Duration::WhileSourceOnBattlefield` means
two things. Tishana's Tidebinder's "loses all abilities for as long as this
creature remains on the battlefield" tracks the Tidebinder and should end
when it phases out. Urza's Saga "gains '{T}: Add {C}.'" has no duration at
all: it lasts as long as the Saga is the object it is (CR 400.7), and
phasing is not a zone change (CR 702.26d), so it should come back with the
Saga. Both are written the same way, neither is parked (only a static is),
and so the Tidebinder's effect outlives its phasing out. Ending it would
take:

- a duration that names what it tracks, apart from the lifetime of an
  effect on its own source: a `Duration` variant ("for as long as ~ remains
  on the battlefield") or the tracked object on `ContinuousEffect`;
- the DSL spelling for it, and the readers and cards that print "for as
  long as" moved onto it (Tidebinder's resolver writes the duration
  itself);
- in `GameState::phase_out`, removing every effect whose tracked object is
  in the set, never to come back (an effect that ended stays ended).

## Unusual casting
Rebound, suspend, miracle, flashback, evoke, adventures, plot, foretell,
madness, disturb decompose into: `CastPermission` (zone/cost/timing
override) + `PendingCast` (with expiry) + `DelayedTrigger` + `ExileRider`.
Keywords exist on stack objects (rebound can be granted).
Casting a permanent card from a graveyard under a player's permission
(Muldrotha, Wrenn and Realmbreaker's emblem) has one reader,
`casting::graveyard_cast_permission`, which the offer, `can_cast_form`, the
cast wizard and the view's graveyard price all ask. Muldrotha's plays are
written down per source and version in `PerTurn::graveyard_plays`; the
emblem's are not, and it is asked first, so a cast under it leaves
Muldrotha's allowance whole.

**A back face is cast at its own timing** (CR 601.3e). Only the face that
will be up on the stack is evaluated to see whether a modal double-faced
card can be cast (CR 712.11c), and only the alternative characteristics for
an Adventure (CR 715.3a). So `casting::can_cast_form` reads the front's
timing off its projected characteristics and each castable back face's off
that face (`casting::face_timing_allows`); the card is offered when either
may be cast now, affordable and with something to point at. The wizard's
`cast_options` keeps the same split: an option that casts the front needs
the front's timing, a `Face(i)` its own, and the list is renumbered after.
Vantress Visions is an instant on the back of an enchantment; read with the
front's timing it could only be cast on an empty stack, which for a spell
that targets an ability on the stack is never.

**A lock by name is a narrowing of the offer** (Pithing Needle, CR 602.5).
`Engine::compute_legal` builds `LegalActions` from every door an activation
comes through, printed, loyalty, granted and a card's in hand, and `apply`
refuses an activation the offer does not hold; so the lock is one pass over
the finished `legal.abilities` (`narrow_under_chosen_names`), beside split
second's, and not a guard in each door. The names are read from the
`Modifier::ChosenNameCantActivate` effects in force, each through its
source's `chosen_name`, and compared as interned names (`Names::find`): a
name no object of the game has carried was never interned and locks nothing,
and a source's projected name is what is compared, so a copy answers to the
name it copies. A mana ability, a special action
(`choice::is_special_action`: `TURN_FACE_UP` and the two unlock slots) and
`PREPARED_CAST` stay.
Nothing projected reads the name, so choosing one invalidates no projection;
the name is cleared as its permanent leaves the battlefield (CR 400.7).

**A Room's doors are designations on the permanent** (CR 709.5c,
`GameObject::doors`, one byte: a Room bit and the two unlocked halves). They
are given where enter modifiers are (`apply_enter_modifiers`), like "enters
transformed", so the trigger scan later in the same pass sees them: cast as
a half, that door is unlocked and `GameEvent::DoorUnlocked` is journalled
(CR 709.5d, 709.5h); not cast (a setup placement, a reanimation), neither
is. The unlock itself is a special action (CR 116.2m, 709.5e) under two
reserved indices below `TURN_FACE_UP` (`choice::unlock_door`,
`choice::door_to_unlock`), offered only at sorcery timing and only once the
half's mana cost floats, as turning a permanent face up is; it uses no
stack and journals the same event (`engine/room.rs`). What a door state
means is `GameState::set_doors`: one half is that half's face (a
`switch_face`), both is the left face with both mana costs and keywords,
neither is the left face with no name, cost, colour or keyword; the rules
text is `CardDef::door_abilities`, read by `GameObject::printed_abilities`,
and `printed_face` is `None` while no door is open. `set_doors` drops the
Room's static effects and replacement rules so the next
`sync_static_effects` registers what the new doors print: that scan only
adds while a permanent stays on the battlefield, and a Room placed uncast
had been scanned as its left half first. The printed front is stashed in
`original_base`, which the move off the battlefield restores, and the doors
are cleared there (CR 400.7). Off the battlefield the card is its left face,
as every multi-face card is here; the combined characteristics of CR 709.4
are not modelled. With both doors open the list's printed face is the left
one, so an ability of the right half has no stack-text line in that state
(Forgotten Cellar's trigger after an unlock from the Closet shows its card,
not its sentence).

**A copy of an ability is a clone of it** (CR 707.10,
`resolve::copy_target_ability`). Every decision made for the original rides
on its object, so the copy is that object cloned under a new id, newly
timestamped, controlled by the player who copied it, with the same
`AbilityLoc` and so the same source (CR 707.10b). An ability pushed from its
definition carries no `target_req`; the copy is given one
(`object::ability_target_req`, the arm list `stack_target_req` also reads)
so `retarget::start_copy` can ask about new targets against it (CR 707.10c).
It journals no `AbilityTriggered`, and `record_new_targets` journals what it
ends up targeting, with nothing counted as already targeted. A synthetic
ability's effects live in `Engine::synthetic_fx`, out of the resolver's
reach, so the resolver names `(original, copy)` in
`GameState::synthetic_copies` and `finish_resolution` hands the copy the
original's effects; the original is below the copy on the stack, so its
entry is still there. That list is hashed, because the question about new
targets is out before the resolution that made the copy ends.

**Replicate is a count on the spell and a trigger with that many copies**
(CR 702.56a, Lose Focus). The cast wizard's `Replicate` stage, after
`Kicker` and before `Targets`, asks `Pending::ChooseNumber` with
`NumberPrompt::Replicate { cost }`, and the answer is told from an X by the
stage it arrives in. The bound (`replicate_bound`) is the largest count the
floating pool pays beside the rest of the cast, through the same
`can_pay_mana` and `spend_for` the payment in `finish_cast` uses, with the
generic mana delve, convoke or a paid waterbend could still take off counted
as paid: an upper bound, so the question never offers less than could be
paid, and a count the payment cannot cover unwinds the cast (CR 601.2h), as
an X too large does. No payment covered skips the question. The count is
added to the total (CR 601.2f), is paid even by a free cast (CR 118.9d,
`pays_mana`), and is written on the spell as `GameObject::replicated`, which
is hashed. `trigger::replicate_triggers` reads it off `SpellCast`: one
synthetic trigger whose effects are the first `n` entries of the static
`REPLICATE_COPIES`, so the count is fixed as the spell is cast. Each
`Effect::CopyThisSpell` builds the copy with `resolve::copy_spell`, the one
spell-copy constructor `CopyTargetSpell` also uses (mode, X, face, kicker,
object and player targets, CR 707.10), from the spell's last known
information once it has left the stack (CR 608.2h), and asks about new
targets with `retarget::start_copy` (CR 707.10c). A copy carries the
count, as it carries every decision made for the spell (CR 707.10), and
copies nothing further: a copy is not cast, so no `SpellCast` names it.

**A back face's mana value is its front face's** (CR 202.3b). A nonmodal
double-faced card's back face has no mana cost, and up on the battlefield
(CR 712.8e) or cast transformed (CR 712.8c) its mana value is computed
from the front face's: Ravager of the Fells is a four, not a zero, and a
disturbed Benevolent Geist a two. A disturb face keeps its disturb cost in
`mana_cost`, since that is what it is cast for, so the cost is not the
answer either. `Characteristics::from_face` writes the front face's mana
value into `front_mana_value` on a back face no one may cast from hand
(the flag `xtask validate` holds against Scryfall's layout), and every
reader asks `Characteristics::mana_value()`, never `mana_cost.cmc()`. A
copy of such a face has mana value 0: `layers::copiable_values`, the one
door every copy takes its values through, writes the 0. Nothing counts
devotion yet; whatever does must not count a disturb face's `mana_cost`,
because the back face has no mana cost.

## Loop detection
A real endless loop is a *repeat*, not a long run. Every mandatory loop in
Magic goes through the stack, so the players are asked every time round and
passing is all they can do: the detector therefore watches the situation as
each answer arrives (`Engine::apply`), with a second watch inside the
decision-free segment (`run_machine`) purely as a hang guard.

`GameState::loop_signature` is what it compares — the rules-visible
situation, blind to object identity and timestamps. `snapshot_hash` cannot
be used: slots are never recycled and timestamps only go up, so a genuine
loop never hashes the same twice.

`loops::LoopWatch` runs Brent's cycle-finding algorithm over that stream:
one stored value, no history buffer. Nothing is hashed for the first 4096
answers, and only every 256th after that — sampling leaves an eventually
periodic sequence eventually periodic, so the cycle is still found. A first
match is put on probation and re-checked one period later, so a coincidence
is not a loop.

Large-but-finite piles of work (the ally deck's thousand rally triggers) are
never flagged: every iteration consumes something, so the situation changes
every time round.

Policy (house rule): `RunOnceThenBreak` (default) withholds the triggers
feeding the loop — in `collect_triggers` and again where a trigger whose
target was already chosen would reach the stack — until the stack has
drained. The loop's effect has happened once and then stops; play continues,
and a card that starts the loop again next upkeep is broken again next
upkeep. A loop detected while a break is still in force means the break did
not take (it is driven by replacement effects or SBAs, not triggers), and
that falls back to `CompRulesDraw`: CR 104.4b, the game is a draw. Every
detection is journalled as `LoopDetected { period, broken }`.

## Custom modes (Rhai)
`ScriptedModifier` implements `FormatModifier` via a sandboxed Rhai script
(fuel-limited, engine RNG only, all mutations through the event pipeline).
Hooks: triggers, replacements, delayed triggers; API: zones, dice,
choices, free casts, emblems, player keywords, skip-step, library segments.

## Determinism & performance
Seeded ChaCha8; no HashMap iteration; journal = replay/resume/crash-
recovery source of truth. Budgets: legal_actions < 50 µs, engine clone
< 5 µs, full AI game < 2 ms.

### The replay contract

A game is its preset (seed included) plus every `(seat, action)` handed to
`Engine::apply`, in the order they were handed in. The seat is part of the
record and is never read back out of `pending()`: before turn 1 every seat is
asked its mulligan at once (`engine::mulligan`, house rule 4) and answers in
whatever order its answers arrive, so the record's order is the order of
arrival.

Randomness comes from one seed and two kinds of stream:

- **The table's stream** (ChaCha stream 0) shuffles the opening libraries and
  draws everything random from turn 1 on.
- **A seat's mulligan stream** (`GameRng::for_seat`: the same seed on stream
  `seat + 1`) shuffles that seat's library for each mulligan it takes, and is
  used for nothing else.

So a seat's hands depend only on its own answers, never on whether the seat
beside it answered first, and the table's stream leaves the mulligans at the
position it entered them however many were taken. `snapshot_hash` covers
the open mulligans: who is still deciding, and where each seat's stream
stands. `house_rules_tests` pins all three, and pins the opening deal of a
table that only keeps against the engine that asked in seat order.

`Engine::snapshot_hash` is not a complete comparison: it leaves out the
journal, the open question and its bookkeeping (`pending`, `pending_plan`, the
cast wizard, `priority_holder`, `awaiting_answer`, `resolve_next`,
`regrant_priority`, the activation checklist's scratch fields), the loop watch
and its latches, `delayed_queue` and most of `trigger_queue`, so two engines
that hash equal can still answer the next question differently; the complete
comparison is `Engine::fingerprint` (feature `fuzz`), which names every field.

## Control rotation at a multiplayer table

`Effect::ControlRotation` asks the controller which adjacent living seat to
receive nonland permanents from. That neighbour identifies the printed
left/right choice; `Pending::ChoosePlayer` carries exactly the two neighbours.
In a duel both directions coincide and no question is needed. The resolver
records all old controllers before changing any, leaves the source and lands
alone, and skips eliminated seats. The house-AI multiplayer soak exposed the
old `1 - controller` calculation, which overflowed as soon as seat 2 owned a
nonland. The four-seat regression exercises both directions and failed with
that original calculation restored.

## A player leaving the game (CR 800.4)

`sba::eliminate_player` follows CR 800.4a in order:

1. Everything the leaver owns leaves the game. It leaves without
   `move_object`, so `eliminate_player` invalidates the projection itself:
   what stays may have counted it, as Pyrogoyf counts the card types among
   cards in all graveyards.
2. Every `GainControl` effect for them ends, so what they took goes back to
   whoever controls it without them. A Gilded Drake'd creature goes back to
   its owner. Under a later thief it goes back to the earlier one. Their
   `SearchTakeover` effects end too: Opposition Agent's hold on a searching
   player is the one way the engine controls a player (CR 722.2). A search
   they were making for somebody else when they left goes back to that
   player, with the same cards and limits, to finish as their own search
   (CR 722.5).
3. What a permanent of theirs held "until it leaves the battlefield" comes
   back (CR 610.3, `GameState::return_what_departed_hosts_held`): their
   permanents left without a move, so this is the one departure
   `move_object` does not see. If they were the monarch, the crown passes
   as they leave (CR 724.4, `GameState::monarch_leaves`): to the active
   player, or, when the leaver is the active player, to the next player in
   turn order still in the game, and to nobody when nobody is left. It
   passes through `set_monarch`, so Palace Jailer's exile ends if the heir
   is an opponent of the player who exiled.
4. `sba::exile_what_the_departed_control` removes what they still control:
   an ability or a copy of a spell on the stack ceases to exist, and
   everything else is exiled through `move_object` with
   `Cause::PlayerLeft`. That leaves what they control by default: a creature
   they reanimated out of another player's graveyard, a spell of another
   player's they cast.

`every_card_that_controls_a_player_does_it_by_taking_over_a_search` reads
the Oracle text of every implemented and partial card and fails the day
one controls a player some other way (Mindslaver, Word of Command). Step 2
then has to end that effect as well.

CR 800.4c is the same rule seen later. A creature the leaver reanimated,
which somebody else had taken until end of turn, is exiled as that effect
ends. The same function does it, from `run_machine`'s step 0a, whenever the
refresh found the effect table changed and someone has left. That comes
before the game-over check and before `sba::run`, because this is not a
state-based action. Effects that end at the end of combat, as a turn or an
untap step begins, or as their source leaves are all followed by that pass
before anybody gets priority. So are the "until end of turn" ones: the
cleanup step's check is that pass, inside the step, so the exile is stamped
in the cleanup step. It is not a state-based action, so by itself it opens
no window. A trigger it causes does (CR 514.3a).

The residue is a resolution: an effect that ends in the middle of one exiles
at the next pass, not inside the resolution.

CR 800.4b has four doors. A `GainControl` effect for a player who has left
applies to nothing (`layers::apply`), and neither `gain_control` nor a
resolution's `CreateContinuousEffect` registers one. A search is never taken
over by a player who has left. A card put onto the battlefield "under your
control" by a leaver's resolution stays where it is. Tokens and copies of
spells were closed by #278.

## Graveyard triggers and card order

Nether Shadow uses a graveyard-active trigger, distinct from a battlefield
ability that observes a death. `TriggerZone::Graveyard` collects from the
owner's graveyard, with the owner as controller. Permanent-only trigger
multipliers do not apply. The source's zone-change version travels through
the pending trigger and stack rider; both the intervening condition and a
`ThisObject` return reject a different incarnation, even at the same object id.

Graveyard vectors store bottom first. `GraveyardCardsAbove` counts matching
cards after the source; tokens and spell copies are not cards. When a
participating card definition can read this order, `graveyard_order::capture`
records simultaneous arrivals grouped by owner. Owners answer in APNAP order
using the existing ordered `ArrangePlace::Graveyard` pile: **first card on top**.
Reordering changes neither zone-change versions nor the zone-change journal.
Games without an order-sensitive participating definition need no extra prompt.

Capture boundaries are effect instructions, completed choice instructions,
simultaneous payment groups and state-based-action passes. The current cost DSL
represents one multi-card sacrifice or discard by contiguous parts of that
kind; a different cost kind closes the group. Distinct discard and sacrifice
instructions therefore do not become one ordering choice. The queue and
staged legend choices are part of checkpoints and fingerprints. A failed
payment rolls back its arrivals and queue. Resolutions drain payment order
before their first effect and preserve nested continuations around an ordering
question. In particular, Wrath of God stays on the stack while the owner
orders destroyed creatures; the spell enters the graveyard afterwards.

Legend choices are selected before applying the SBA pass. Lethal creatures,
legend-rule deaths and finished Sagas from that same check share one event
(CR 704.3); an Aura that becomes illegal only because its host just died is
handled in the next pass. This is necessary for the owner's order choice
under CR 404.3, not just for a deterministic insertion order.

## Mana spending permissions

`ManaSpending` in core is a directed permission matrix over actual mana types
and the requirements they may pay. Its default is exact matching. Sunglasses
of Urza adds white-to-red for the source's current controller; Mycosynth Lattice
allows every actual type to pay colored requirements. Neither changes the
original mana, its snow provenance, the cost, or the payment record's colors.
Compatible permissions compose under CR 609.4a.

`casting::mana_spending(state, player)` reads live synchronized effects.
Affordability and payment use those same permissions, including restricted
mana and its spend riders. Gamehost exposes the matrix in `ManaPoolView` so
automatic plans can choose Plains for a red requirement while respecting white
requirements elsewhere in that cost. The player-facing pool still contains
white mana; this is a payment permission, not a conversion ability.
