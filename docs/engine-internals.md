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

The arena is copy-on-write a chunk at a time: slots live in two-slot chunks,
each behind its own `Arc`, a clone shares every chunk, and a write copies only
the chunk it lands in, preserving exact slot order and generations in each
snapshot. `Engine::apply` takes a full decision checkpoint per answer (a
refused or numerically failed answer restores it), so an answer pays for the
chunks it wrote and not for every object in the game. The checkpoint keeps the
event journal's *length*, not the journal: it only grows, and a restore cuts
it back, as `GameState::roll_back` does for a canceled payment. Names, retained
damage sources and the base cache are shared by a clone the same way. Measured
costs are in `docs/perf-baseline.md`.

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

`AddSupertype` and `RemoveSupertype` alter only the named supertypes in layer
4, including dependency ordering for effects that read those supertypes.
The world rule still uses entry timestamps for printed World permanents;
tracking time since gaining or regaining World through a continuous effect
is unsupported. The supertype modifiers alone do not implement that rule.

**An object keeps two clocks, and a transform moves only one.** Both read
the game's one counter (`GameState::next_timestamp`).
`GameObject::timestamp` is the timestamp of CR 613.7: taken as the object
enters a zone (613.7d) and again each time the permanent transforms
(613.7g, `GameState::transform`). The effects of its static abilities take
it when `sync_static_effects` registers them (613.7a), and `transform` drops
the old face's effects first, so the new face's statics are registered with
the new stamp and apply after anything created while the other face was up.
A daybound card entering at night enters with its back face up
(CR 702.145b, 712.11a), and that is not a transform: nothing was turned
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
same object under the same controller (CR 712.15). The two used to be one
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
`Engine::new` also runs step 0b's half that asks nobody,
`scan_arrivals`, over the starting battlefield: a planeswalker there has its
loyalty (CR 306.5b) and a tapland is tapped while the hands are kept, and an
arrival that asks (a shockland's life) waits in `entry_questions` for the
machine's first pass, which asks it before anything else (#304,
`a_starting_planeswalker_has_its_loyalty_while_the_mulligans_are_open`).

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
(CR 715.3a), daybound's back face, a Room's door. What asks (an
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
itself (CR 712.6a: off the battlefield a card has only its front face),
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

`Modifier::AssignsNoCombatDamage` excludes an affected attacker or blocker
from the assignment preview, damage-division questions and final assignments
(CR 510.1a, 101.1). It is not prevention and does not suppress noncombat
damage. In particular, a masked creature that would assign no damage does
not turn face up for that assignment. Strike-step membership remains separate
(CR 510.4); the restriction neither moves a first striker into the regular
step nor allows a double striker to assign damage in either step.

### What must attack, and what may attack whom (CR 508.1c–d)
Combat restrictions and requirements are statics the layers carry as rules
modifiers (`Layer::Text`, beside the other modifiers that change what a
creature may do):

- **A requirement**, `Modifier::AttacksEachCombat` ("attacks each combat if
  able"). A grant is an until-end-of-turn effect with the same modifier, so
  losing the creature's own abilities does not lose one another permanent
  gave it.
- **A restriction on the pair**, `Modifier::CantAttackUnlessDefenderControls`
  ("can't attack unless defending player controls an Island"): the filter
  is asked of the permanents of the player the creature would attack, and
  of a planeswalker's controller when it attacks one.
- **Comparative creature-count restrictions** (CR 508.1c, 509.1b),
  `Modifier::CantAttackUnlessYouControlMoreCreaturesThanDefender` and
  `Modifier::CantBlockUnlessYouControlMoreCreaturesThanAttacker`.
  `combat::CreatureCounts` counts each player's projected battlefield creatures
  once when the declaration rules are built. The comparison is strict, includes
  creatures outside the declaration, and reads the chosen defending player or
  the particular attacker's current controller rather than the active player.
  Block offers and direct declarations share the same predicate; mana-payment
  handling remains independent.

`combat::AttackRules` collects restrictions, requirements, attacker caps and
costs once per declaration. The offer drops a creature that no defender
allows, names in `limits` a creature some defenders do not allow (with the
ones it may attack), and lists in `required` the creatures that must attack.
`declare_attackers` refuses a forbidden pair or a declaration that leaves out
a creature that must attack and can attack for free. A player is never
required to pay a cost to make a required attacker attack (CR 508.1d);
`required_maximum` also accounts for attack-alone restrictions and any cap on
the number of attackers. The clock's answer (`choice::timeout_answer`)
declares the required creatures and nothing else, each at the first defender
it may attack; the house AI keeps its own choice and adds what the rules make
it (`combat::obey_attack_rules`).

Mana, life and sacrifice costs are determined for the complete declaration
before any of them is paid (CR 508.1h). Each sacrifice cost becomes a
`Pending::ChooseCards` question with `ChoicePrompt::CostSacrifice`: its
controller chooses exactly the required number of matching permanents they
control, and one permanent cannot pay two costs. A declaration whose costs no
assignment of permanents pays is refused, a tax out of the sources' reach
likewise, both before the first question: nothing takes a declaration back
once its sacrifices are being asked for. Each question offers only permanents
that leave every later cost payable, a matching rather than a walk down the
list, so a land spent on "a land" cannot strand "two Islands". Nothing is
sacrificed, tapped or declared between those answers. A mana payment window
retains the chosen permanents and the determined costs, which stay locked in
(CR 508.1h); as it closes the declaration is checked again, because a mana
ability may have tapped or sacrificed an attacker, and a short payment or a
declaration that no longer stands reverses the window without sacrificing
anything. The final payment is validated before the attackers are declared,
and a creature a cost sacrificed never becomes one (CR 508.1k).

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

### Damage-source color projection

`Modifier::DamageFromColoredSourcesIsColorless` changes how a source answers a
filter during a damage event; it is not a layer-five color change and does not
mutate the object's cached characteristics. Protection, prevention, redirection
and damage-trigger source filters call
`eval::matches_damage_source_with_context`, which clones the projected
characteristics only when an applicable effect matches one of the source's
colors, then clears only the clone's color set. The effect table's monotone hint
skips that scan when no such rule is registered.

Targeting, blocking, source choices made before a damage event, recipient
filters and all non-damage filter evaluation continue to use the source's actual
characteristics. Thus protection from a color loses its damage-prevention clause
under this effect but retains its enchant/equip, blocking and targeting clauses
(CR 702.16b). LKI and object-version selection still happen before this temporary
projection, so it neither enlarges nor replaces the source's identity.

### Damage history and later deaths

`damage_history` records actual positive permanent damage after prevention and
redirection. Both sides of each deduplicated pair carry an object handle and
zone-change version. Marked damage is independent: removing it during cleanup
does not erase what happened earlier in the turn. `PerTurn::reset` clears the
history when the turn ends.

A battlefield departure snapshots the victim and its surviving damage sources,
including their abilities, controller, grants and trigger multiplier. A group
of simultaneous departures takes these snapshots before moving its members.
`Trigger::DiesAfterDamageByThis` reads the snapshot rather than reconstructing
the earlier battlefield from objects that may already have moved again.
The source's version also follows the resulting ability onto the stack, so a
self-counter cannot affect a later incarnation. A phased-out source is absent
both when observing a death and when an effect would put a counter on it.

A zone-change trigger keeps two identities distinct. `event_object_identity`
can name the card in its destination zone for an instruction such as Rancor's
return, while the source version retains the permanent that left the
battlefield. A condition about what that permanent did earlier reads the
latter; it must not replace the former (CR 400.7).

`Trigger::DealtDamage` retains the damaged permanent as `EventObject` with
its event-time incarnation, and the received damage as `EventAmount`. As with
the player-damage trigger, simultaneous combat damage from several sources
is one event with their total (CR 510.2, 603.2c). Noncombat damage retains
each event's own amount. The amount remains available after lethal damage
moves the creature to its graveyard.

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
Flanking is collected from the attacker's keyword when each blocker is
declared. `layers::keyword_instances` replays the normal layer projection
to count its native instance and each applicable grant; removing the
keyword clears every earlier instance, and a later grant starts again.
Each instance triggers separately (CR 702.25b), unless the blocker itself
has flanking. The blocker is an implicit, untargeted event subject, pinned
to its incarnation when it blocked: blinking it in response does not shrink
the returned creature (CR 400.7).

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
  and a declaration obeys the sum over its pairs. A blocker that costs mana
  to block with (`Modifier::CantBlockUnlessPayManaPerCounter`) counts for
  none: "that player is not required to pay that cost, even if blocking with
  that creature would increase the number of requirements being obeyed"
  (CR 509.1c). It is offered while its controller could pay, but the maximum
  is the other blockers', and blocking with it obeys nothing in their place,
  as a taxed attacker obeys no "attacks if able" (CR 508.1d).

`combat::block_options` is the offer (the `BlockOption`s), and it is also
the universe the maximum is taken in: a pair outside it breaks a restriction
(evasion, protection, or an attacker with too few possible blockers to meet
its minimum). Menace supplies a minimum of two (CR 702.111b);
`Modifier::MinimumBlockers(n)` supplies `n`, and independent minimums combine
by taking the greatest. `combat::block_bound` publishes the result in
`Pending::ChooseBlockers::bounds`: a bounded attacker has zero blockers or at
least that many. `combat::BlockRules` collects the declaration rules once per
declaration. The question names, in `capacity`, each offered creature that
may block more than one attacker, and in `obeying` one legal declaration that
obeys as many requirements as the engine holds a declaration to.
`declare_blockers` refuses a pair named twice, a blocker over its limit, and
a declaration that obeys fewer requirements than `obeying` does.

`BlockRules::obeying` gives each blocker the attackers most requirements ask
of it, up to its limit, attackers without a minimum-blocker restriction first;
a bounded attacker with too few blockers then gets helpers from any creatures
with room that may block it, or loses the incomplete block (CR 509.1b,
702.111b). Without a bounded attacker that a requirement names, the blockers
do not touch one another and this is the maximum CR 509.1c asks for. With
one, a blocker's help costs
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
its own blocks for every blocker `obeying` does not use and then holds the
merged declaration to the question's bounds (`combat::keep_bounds`);
client-core preselects `obeying` and never declines blocks by itself while it
is not empty.

Counting is over the declared pairs, not the pairs banding adds afterwards
(CR 702.22h makes the band blocked as the block is made, after 509.1c has
checked the declaration). Block triggers are per pair already (CR 509.3b,
509.3d), so a creature blocking two attackers triggers twice; a blocker's
damage divided among the attackers it blocks is asked of its controller
(CR 510.1d, `divisions_owed`).

### Windows in the turn (CR 506.8)
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
  end of combat (CR 506.8a, 506.8e). `OpponentsTurn` asks the active
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

This engine has one combat phase a turn, so CR 506.8c–d (which of several
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

**An attacker with trample blocked by two or more without banding** is divided
by its controller under `HouseRules.divide_trample` (absent means off, and it is
not written while off; `ask_combat_division` hands it to `divisions_owed`, which
has no house rules of its own). A table asks; a training or evaluation game does
not. CR 702.19b: "Once all those blocking creatures are assigned lethal damage,
any excess damage is assigned as its controller chooses among those blocking
creatures and the player, planeswalker, or battle the creature is attacking", and
it need not assign lethal to all of them, "but in that case can't assign any
damage to the player or planeswalker". The same `ask_share`/`answer_share` serve,
one share per blocker in declaration order, and `OwedDivision::lethal` says
what is different:

- every blocker is asked, the last too, and what is left after it is not written
  into the `Division`: the trample branch of `assign_attacker_damage` sends
  `power - sum` to what the attacker attacks;
- the last blocker's `min` is `min(lethal, left)` while every earlier blocker has
  lethal, and it takes all that is left, unasked, once one does not;
- a share with `min == max` is written down and not asked, so a share of nothing
  is not asked either, where a plain division still asks it;
- with nothing to reach (a planeswalker that left: 702.19f, so
  `active_damage_recipient` is `None`) `lethal` is empty, and the question is the
  plain division, which puts all of it on the blockers.

`lethal_damage` is the auto path's: deathtouch 1 (702.2c), marked damage counted.
It does not count "damage from other creatures that's being assigned during the
same combat damage step" (702.19b), which only a creature blocking two attackers
can be owed: such a blocker has to be given more than the rules ask before the
rest goes on. A double striker is asked again in the second step, on what the
first step marked: `collect_combat_damage` spends the divisions.

Without the rule, and always for a single blocker, nobody is asked: the engine
assigns lethal damage to each blocker in declaration order and the rest to what
it attacks, one of the assignments 702.19b lets its controller make. Banding with
trample keeps its own path above (702.22j)
(`trample_division_tests`; `banding_tests::an_attacker_blocked_by_two_is_divided_by_its_controller`
pins the question without trample).

False Orders' re-block is the one effect in the pool that makes a creature become
blocked, and it spreads its block through the band as 702.22h and 702.22i say
(`AwaitingOp::BlockAttacker`).

### Piles that restrict blocks (Raging River)
`Trigger::OneOrMoreAttack` fires once for a declaration that holds a
matching attacker (CR 603.2c). `Effect::LeftRightPilesRestrictBlocks`
(`resolve/river.rs`) then asks each defending player, every opponent of the
attacking player during combat (CR 802.2) in turn order from the active
player (CR 101.4), to name their "left" pile among their creatures without
flying (`ChoicePrompt::LeftPile`; the rest are "right"). The piles are
public, since the creatures are on the battlefield. The controller then
labels each attacking creature they control, in declaration order, with
`Pending::ChoosePile { label: Some(attacker) }`: first pile "left", second
"right". Two empty piles are not asked about. A label is a blocking
restriction (CR 509.1b) the combat holds (`CombatState::pile_limits`, hashed,
cleared with the combat) and `combat::can_block` asks, so the offer,
`declare_blockers` and the clock agree. Only a creature with flying, or one
in the chosen pile, may block that attacker. A creature in no pile (one
that entered since) may not, and two Rivers' limits on one attacker both
bind.

### Blocks made by piles at random (Camouflage)
Camouflage replaces the declare-blockers turn-based action for the rest of
the turn (`PerTurn::camouflage`, set by
`Effect::BlockInPilesAtRandomThisTurn`). `ask_blockers` asks each
defending player, in the order they would declare (CR 802.4), to name one
pile at a time (`ChoicePrompt::CamouflagePile { pile, of }`, an existing
ChooseCards sheet; `PlanKind::CamouflagePiles`). There is one pile per
attacker of theirs. A creature goes into one pile, or into as many as the
attackers it could block ("may likewise be put into additional piles",
`BlockRules::capacity`). The piles go to different attackers at random,
drawn from the game's seeded generator (the owner's decision, 09.10.2026),
so a replay deals them alike. A creature then blocks its pile's attacker if
`BlockRules::can_declare` allows it, which is `combat::can_block` and the
restrictions that compare creature counts; a menace attacker is blocked by
its pile only if two or more of it can block. Requirements (CR 509.1c) bind a
declaration, and there is none. The blocks go through `make_blocks`, the
half of `declare_blockers` after its checks, so they are journalled, spread
through bands (CR 702.22h), and followed by the next defending player.

### Removed from combat, and blocked again (False Orders)
`Effect::RemoveTargetFromCombat` is CR 506.4's "an effect specifically
removes it from combat" (`CombatState::remove_from_combat`, the door a
departure uses too). An attacker it was blocking stays blocked (CR 509.1h),
except with `unblock`: "creatures it was blocking that had become blocked by
only that creature this combat become unblocked". "This combat" is a
history, not the blocks left, so `CombatState` keeps every block made
(`block_history`, hashed, cleared with the combat), and `blocked_only_by`
asks it. `Effect::TargetMayBlockAttackerOfChoice` then offers the caster the
attackers the creature could be blocking (those attacking its controller or
a planeswalker they control, CR 506.3e, 802.4a) as `ChoicePrompt::BlockWith`,
none allowed. The block is made and journalled as `BecameBlocker`, so block
triggers fire. Blocking restrictions are not asked: they bind a declaration
(CR 509.1b), and this is an effect.

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
collected and stacked APNAP, each player's by timestamp unless the game asks.
SBAs run as a fixpoint before every priority grant (plus format SBAs).

### The order of a player's triggers

CR 603.3b is a two-part process: first the triggered abilities whose trigger
condition isn't another ability triggering, then the rest, each part per player
in APNAP order and in any order they choose. No card in the pool triggers on an
ability triggering, so every queue here is the first part. Under
`HouseRules.order_triggers` the player at the front of the queue is asked before
any of their triggers asks a mode or a target (`progress::trigger_order`; the
wire is `docs/protocol.md` §"The order of triggers"). `PendingTrigger::in_chosen_order`
marks what they have settled: the queue's sort leaves those first, in the order
chosen, and nobody is asked about them again, so an answer outlives the target
questions and the re-sorts between it and the stack. A trigger found after the
answer waits behind them and is asked about in turn.

Only triggers that differ are asked about. Two are the same when their bodies are
equal (`Body`: effects, targets and intervening condition, not the trigger event),
their text changes are equal, their sources are one object or two that only the
handle tells apart, and their events are the same as far as the body reads them
(`EventRead`, off the words of the script). A body that reads nothing of its event
is the same ability whatever set it off, which keeps a board wipe under Blood Artist
from being a question; Dingus Egg reads only whose land it was. A word that misses
a reader costs a question and never a rule: those triggers go on by timestamp.
A trigger with no legal target is asked about like any other, since a trigger put
on the stack first may be what it could target.

A game without the rule marks nothing, folds nothing into `snapshot_hash` and
serialises no field, so stored records and the hashes they hold are untouched.

### Optional triggers with an empty target

With `HouseRules.auto_discard_empty_optional_triggers`, an optional trigger
whose target answer is empty is resolved first on a cloned state. Only a
complete resolution with no follow-up question, no journal entry and no state
change beyond the resolver's damage-batch bookkeeping removes the real stack
object, journals `StackObjectDidNotResolve`, and runs cleanup. The journal
counts because the snapshot hash omits it and an effect such as a revealed
hand writes nothing else. Any actual effect, another target or division, or
an unfinished question keeps the ordinary path. When on, the setting is part
of the continuation hash; off, it adds nothing, so an old record keeps every
hash it recorded. New games enable it; records without the field read it as
`false`, so their historical empty-target answers still have a question to
answer.

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

When a source's own departure trigger creates `AtNextEndStep` or
`AtNextCleanup`, the delayed action remembers the exact public-zone successor
that the departure recorded (CR 400.7e). This lets a dies trigger return its
card from the graveyard later. Moving that card out of the graveyard and back,
before or after the delayed action is created, does not retarget the action
to the new incarnation (CR 603.7c).

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

### Source incarnations on the stack (CR 400.7)

Stack abilities and their resolutions retain the source's zone-change version.
A card returning to the battlefield is a different source even when its arena
id is unchanged. The former Scavenging Ghoul self-counter leak is guarded by
`resolve::counters` checking the captured source version before adding counters;
the exact Ghoul/Kenrith scenario is not yet a dedicated regression.

Source power and source-counter amounts read that exact incarnation, live while
it remains and from its departure snapshot after it leaves (CR 608.2h). Paying
a sacrifice cost can remove the source before its stack ability is created.
Immediate mana abilities retain the pre-payment source version in their
resolution context because they have no stack object. Counter amounts use the
counters remaining after any earlier counter-removal cost, and a color choice
retains the amount across suspension. `engine::cost_source_lki_tests` covers
these paths, a returning source, and power changing before resolution.

Damage uses the retained source incarnation and its last known characteristics.
The former Circle of Protection failure after a pinger leaves and returns is
covered by independent tests for all five Circles, including multiple historical
incarnations. Choosing an earlier source is distinct from choosing the current
permanent. Death triggers capture the source before departure, separately from
their new-zone event object. Granted abilities test their recipient against that
same historical incarnation. Explicit event-object damage is distinguished from
ordinary source damage even when their arena ids match. Internal event context
alone grants no source-choice eligibility: the ability must actually read it;
copies retain that eligibility. Omnath and undying regressions cover both sides.
These changes do not imply that every reference to another object is
version-aware: the Earthbend event-object case above and the state-trigger
suppression case below remain separate gaps. Exact target revalidation (#117)
is described below.

Ice Cauldron's linked abilities keep three identities separate (CR 400.7,
607.5). `Rider::ActivationManaNote` belongs to the source permanent's exact
incarnation and rules acquisition: a returned Cauldron has no old note. The
independent `PlayLifetime::WhileExiled` permission belongs to the exact exiled
card incarnation and ends when that incarnation leaves exile. Producing mana
records that exile's version in its provenance; payment accepts that incarnation
or its immediate stack successor (`version + 1`), never a same-named object or a
later exile incarnation. Declining a later optional exile replaces the noted
mana types but retains the previous last-exile reference. Both note halves and
the permission participate in deterministic state hashing.

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

A window that closes without its cast (a miracle declined after a tap, or
paid short; a `PaymentContinuation::Cast` likewise) gives back what was made
in it: the play was never completed, and CR 734.1 lets its player reverse the
mana abilities activated while making it. The window holds a `WindowStart`
(the journal's length, the payer's pool, the generated-mana obligations and
the trigger queue's length when it opened), and `Engine::give_back_window`
untaps every permanent tapped as a cost since then, puts the pool and the
obligations back, and drops the triggers the taps queued. The journal is not
cut back, because each tap was its own `apply` and a host has read past it:
the untaps are journalled as `ObjectUntapped { cause: Cost }`, which nothing
triggers on. A window in which anything but a tap and its mana happened (a
sacrificed Lotus Petal, life paid, a library touched) is left as it stands,
which 734.1's "may" allows. A cast started from a `Cast` window carries its
start into the window its wizard opens for the rest
(`CastWizard::window_start`), so both are given back together
(`combo_tests::miracle`, `conduit_of_worlds_paid_short_gives_back_the_forest_tapped_for_it`).

### "Can this seat make mana?" is asked of the window's list

Whether a payment window would have anything in it is
`LegalActions::has_mana_source` of the finished offer narrowed as a window
narrows it (`narrow_to_mana`): there `abilities` holds mana abilities
alone. Of any other list it answers a different question, because an
entry in `abilities` is any activation at all. Four offers asked it of
another list until 10 November 2026:

- **A granted action with a mana cost** the pool cannot pay (Nafs Asp's
  and Guardian Angel's "pay {1}") was offered while `compute_legal` was
  still building the offer, when `abilities` held every activation, also
  one no mana paid. A seat with no land untapped was offered the action,
  and `start_granted_action`'s window found nothing to pay with and refused
  it (deal 359 of the v6 priority probe). `settle_granted_offers` now
  decides at the end of `compute_legal`, on the finished offer narrowed and
  without the unpaid grants themselves, so a grant cannot vouch for itself.
- **The free-cast offers** (a linked or immediate free play, a transformed
  back face, a cast an effect commands) asked it of the finished,
  un-narrowed offer, where a Prodigal Sorcerer's tap reads as mana to come.
  They ask `free_cast_makes_mana`, which is `can_make_mana_for_cast`.

Neither the legal list nor this question is in `Engine::snapshot_hash`, so
no hash moves. A granted action that left the offer was one `apply`
refused, so no record replays differently for it; what a seat is offered
moves at exactly those positions, and a policy that reads the list (the
net's candidates) sees one answer fewer there.
`engine/granted_offer_tests.rs` plays the change in lockstep against the
engine as it was (`ASK_MANA_AS_BEFORE`): over a debt deck built to reach
it and the house decks, the hashes and answers are equal throughout, and
the questions differ only by granted actions the engine as it was offered
and then refused. Those games never reach a free cast with mana left to
pay and no land to make it; `granted_tests::a_free_cast_counts_on_mana_
only_a_window_could_make` holds the question there. A free-cast option
that left the offer was one whose payment found no mana, and a record
that chose one would no longer replay at that answer.

### Madness is a discard replacement, a trigger and an ordinary cast (CR 702.35)

`FaceDef::madness` holds the complete alternative `Cost`, not merely its mana.
`GameState::discard_card` is the shared door for an effect, a chosen, random or
self discard paid as a cost, and the cleanup step's turn-based discard. It
records exactly one `Discarded` event for the hand incarnation, replaces the
graveyard destination with exile and writes `MadnessExiled` with that exact
object version. Only a real graveyard successor enters
`per_turn.discarded_to_graveyard`: neither the exile nor the later fallback is
a second discard into the graveyard.

`trigger::discarded` turns that event into a normal, counterable triggered
ability carrying `Effect::Madness`; it is not a direct free cast from the
journal. Resolving it asks `YesNoPrompt::Madness`. Accepting enters the
ordinary cast wizard with `CastWizard::alternative_cost = Madness`, separately
from its mode or modes. The shared `cast_wizard::madness` preflight enumerates
legal modes and targets, checks every non-mana part and prices increases and
reductions against the alternative mana cost. X, targets, additional costs and
the mana-producing payment window therefore behave as they do on other casts
(CR 601.2b, 601.2f–h, 702.35b). A cast with no legal target or an impossible
non-mana payment is not offered. Waiving the printed mana cost does not waive
an external tax or additional cost.

Declining, or cancelling an accepted cast, invokes
`GameState::madness_to_graveyard` only for the still-exiled object version the
trigger named (CR 400.7, 702.35a). Countering the trigger leaves the card in
exile; countering the resulting spell uses the ordinary spell-countering
rules and puts it into the graveyard. A copied trigger gets its own decision,
but only one copy can find and cast the exiled incarnation. Player departure
cancels the pending question and payment without manufacturing a cast
(CR 800.4). `engine/madness_tests.rs` plays these boundaries, including effect,
cost, self-cost and cleanup discards; Basking Rootwalla's canonical test plays
its `{0}` cost during an opponent's turn, and `gamehost/tests/ai_madness.rs`
walks the actual public view and house answer.

### An upkeep payment is a triggered ability on the stack (CR 702.30a, CR 603.7)
Echo and a pact's "at the beginning of your next upkeep, pay …; if you don't,
you lose the game" trigger as the upkeep begins and go on the stack before the
active player receives priority (CR 503.1a, CR 117.3a), in the same APNAP batch
as everything else that triggered there (CR 603.3b). Priority goes round over
them, they can be answered, and what they ask is asked as they resolve. A client,
a language-model seat and the house AI read them as they read any other stack
object: an ability of that card, at its place in the card's list.

**Echo** is a keyword with a cost and no effect list (`AbilityDef::Echo`), and
three places write out what it means. `trigger::triggered_parts` reads it as a
step trigger with an intervening `if`, so it fires, doubles and is lost as a
printed trigger is: a permanent with no echo at the beginning of the upkeep (a
Humility) has nothing to trigger, and a copy has its own. `intervening_clause`
gives `intervening_if_failed` the same clause for CR 603.4's second check, and
`Engine::demand_echo` is its body as it resolves.

The clause is `Condition::SourceCameUnderYourControlSinceYourLastUpkeep`. A
player's `upkeep_began` is the stamp as their latest upkeep began and
`last_upkeep_began` the one before it, which is what "your last upkeep" means
in the step they are in; `GameObject::controlled_since` is compared against the
second. It restarts at every change of control, so a permanent taken owes at
its new controller's next upkeep and one handed back owes again, whether or not
it was paid for before. A player with no last upkeep yet has a stamp of zero, so
everything came after it, the opening board included. Neither stamp is in the
state hash: they are read off the turn clock, and a game with no echo in it
hashes as it always has (§"The replay contract"). As it resolves the clause
also asks for the incarnation that triggered, on the battlefield, under the
ability's controller. A permanent gone, blinked or taken in response is removed
from the stack with nothing asked: the last of those still "came under your
control", but all the ability could do is ask for mana and sacrifice nothing
(CR 701.21a).

The body is `Effect::PlayerMayPayManaOr` with `SacrificeSelf`, the tax Phantasmal
Forces prints, and its CR 605.3a window: the question comes with the pool empty
and a yes opens the window to make the mana in. One thing is echo's own. A player
with neither the mana nor anything to make it with is past asking, and the body
is `SacrificeSelf` alone. Asked of the pool only, the question used to go
unasked beside untapped lands.

**A pact's price** is the delayed triggered ability its spell creates
(CR 603.7), and the card lists it as a second ability so that it has a place and
a sentence: `Effect::AtNextUpkeepPrinted { ability: 1 }` makes a
`DelayedAction::PrintedTrigger`, and at that upkeep it joins the trigger queue as
that ability of that face, with the list it needs kept beside it (a copy of the
spell, which made one too, has ceased to exist by then). The listed ability is a
`triggered!` on an instant, which functions on the battlefield and so never
fires on its own. Its body is `Effect::PayManaOrLose`: "pay", so a pool that
covers it is spent without a word, and anything else is `YesNoPrompt::PayPact`,
put whatever the board, with `source: None` because a standing "no" would be a
standing loss. It has the tax's window, and an unpaid close is `Effect::LoseGame`.

Both used to be delayed actions and not stack objects. First they ran as the
upkeep began, against a pool the untap step had just emptied (CR 500.5) and in
which nobody could have made mana (CR 502.4), and Pact of Negation lost the game
on eight untapped Islands. Then they were set aside and answered when the
upkeep's round closed on an empty stack, which kept that fix and showed nobody
anything: a language-model seat reported a Keldon Champion's echo missing from
the stack, and APNAP order against the upkeep's real triggers was wrong. Echo was
also registered as the permanent entered, so it stayed with the player it entered
under: a stolen permanent asked its old controller for mana, and one that lost
its abilities still came due.

Cumulative upkeep (CR 702.24a) never shared any of this. It is written out as a
printed `triggered!` with `Effect::AddCounter` and `Effect::PlayerMayPayOr`, and
has been an ordinary stack object all along.

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
about what was choosable. A `TargetReq` naming `Player(You)` is an explicit
self-target: only the controller is offered, their own hexproof permits it,
and protection from everything forbids it both when choosing and when
resolving. A bare `Player(You)` in an effect remains an implicit subject;
without a target requirement it asks no target choice or target re-check.
A change of targets (CR 115.7, `resolve::retarget`)
asks the same function, so a redirected spell is offered only what the
re-check would call legal (#247).

Target slots bind object and zone-change version when announced, before costs
can move them (#117). Both groups are checked against those exact references and
current legality. A missing reference is illegal; resolution does not invent one
from the current object. Partial legality narrows a local resolution copy while
the original announcement remains available to later copies.

Retargeting compares exact pairs, including when an old illegal target and a
newly chosen returned incarnation share one arena id. Fixed divided damage keeps
its shares paired to those identities; even division recomputes over the legal
resolution targets. New-target events and Ward use the same distinction.
Historical target projections carry their own entitled identity and do not
link a target arrow to a newer object. Integration acceptance is tracked in
`docs/set-progress.md`.

Untargeted self-effects now carry an exact resolution subject. Direct
`Filter::This` pumps/continuous effects and self regeneration, prevention,
untap, counters and sacrifice cannot affect a returned incarnation. Actual
public-zone moves by this effect or its cost establish an explicit successor
(CR 400.7j), retained through nested and suspended instructions. Damage source
LKI is separate. Per-turn ability counts key by incarnation, and delayed
sacrifices retain that incarnation. Focused behavioral coverage includes the
four Alpha pumpers, Dragon Whelp's activation timing and delayed sacrifices,
Drudge Skeletons, Sedge Troll, Rock Hydra, Basalt Monolith and Enduring Vitality.

The current pool source census found 31 literal composite `This` occurrences
(including nested duplicates), all in static abilities, trigger predicates or
entry conditions. Alpha instances are Veteran Bodyguard, Ironclaw Orcs,
Juggernaut, Winter Orb and the five Wards; shared `YOUR_ALLIES` is used only by
trigger predicates. None uses a resolving `PumpFilter`, `SetPTFilter` or
`CreateContinuousEffect` with composite `This`. This milestone does **not**
claim general incarnation binding for future resolving `And`/`Or`/`Not`
filters containing `This`; those must gain an explicit resolution-context
reader before such a card is implemented.

A spell leaves by `Engine::leave_stack_without_resolving` and not by
`finalize_spell`: rebound (CR 702.88) and an Adventure (CR 716.3d) exile a
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
have reached its final chapter (CR 715.4) *is* a state-based action, and it
sits outside `sba::run` because it has to read the permanent's abilities to
find out what that chapter number is. It used to sit somewhere else
entirely — on the way out of the last chapter's resolution — which left a
Saga on the battlefield for the rest of the game if that chapter was
countered. CR 715.4's second clause is why it still cannot fire from there:
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
(CR 701.27d) — turning either over would rebuild its base from a face that
is not there. Both are skipped, and skipped *without* reporting a change: a
guard that reported one would send the fixpoint round forever on a permanent
it had just declined to touch.

`GameState::transform` is the door that journals `Transformed`. Its
neighbour `switch_face` does the same work silently and is what the modal
paths call, because choosing which face of an MDFC to cast or to play as a
land is not a transform (CR 712.8b and CR 712.9).

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

`resolve::dig` follows that protocol for both private `LookAtTopFind` and
public `RevealTopFind`. Its `RestDestination` keeps the private effects'
bottom-of-library ordering separate from a public effect's graveyard, exile
or other printed destination. `finish_with_destination` resolves every
selected `Find`, plans both the selected and unselected moves, and only then
asks the commander questions; no card from either group moves before the last
answer. A public top reveal is journalled before the choice and is not repeated
when a selection resumes. No match finishes the remainder immediately rather
than publishing an empty choice. Graveyard and exile moves ask nothing here:
CR 903.9a's question is the later state-based action, not this replacement.
The direct public/private resolver tests and Malevolent Rumble's two-commander
card test pin those boundaries.

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

### A draw that waits to be asked (CR 121.2, 614.11a)
Island Sanctuary replaces "a card" its controller "would draw during
[their] draw step", which is not only the turn-based draw (CR 504.1): a
Howling Mine's additional card, an Ancestral Recall cast in that step and a
Lich's draws for a gain there are draws too, and each card of "draw three
cards" is a draw of its own (CR 121.2). `GameState::draw_cards` cannot ask,
so it does not make such a draw: it puts `(player, n)` on
`GameState::draws_to_offer`, and every draw behind one already waiting goes
there as well, so the order the instructions gave holds (CR 121.2c). The
queue is hashed by `snapshot_hash`; `loop_signature` leaves it out because
it is empty whenever priority is granted.

Two drainers ask about it, card by card, through one pair of functions
(`state/draw_offer.rs`: `next_draw_offer`, `draw_offer_answered`):

- **A resolution**, at the top of each turn of `resolve::run`'s loop and
  before it completes, asks about the first waiting draw before the next
  instruction runs (CR 614.11a), suspending on `AwaitingOp::SkipDraw`. The
  instruction that drew has already run, so the answer advances nothing. A
  mana ability does not ask; nor does an instruction whose own question is
  out, which keeps its question and has the draw asked after it.
- **The machine** asks about the turn-based draw (`offer_queued_draw`, called
  by the draw step) and, at the top of `run_machine`, about anything queued
  where no resolution could ask, with `PlanKind::SkipDraw`.

Either way the question is the Sanctuary's own `YesNo { MayDo }` keyed by its
printed ability, so a standing answer covers every draw. Yes skips one card
and makes the restriction (once: a second skip before the controller's next
turn adds nothing); no leaves the card to the controller's next Sanctuary
not yet asked about it (CR 614.5) and draws it when none is left. The skip
is offered with an empty library too (CR 614.11), and nobody draws for a
player who has left (CR 800.4a: what they own has left with them).

#### Every replacement of a draw (K12: dredge, Chains, Lamp)
The same queue carries every replacement of a draw, card by card:
Island Sanctuary's skip, dredge from the drawing player's graveyard
(`ReplacementRule::Dredge(n)`, CR 702.52a-b, read off the printed rules of
the cards there, never registered from the battlefield), Chains of
Mephistopheles (`ReplacementRule::DiscardInsteadOfLaterDraws`, every
player's draws but the first of a draw step,
`GameState::next_draw_is_first_in_draw_step`), and the next-draw effects of
Ring of Ma'rûf (`Modifier::NextDrawWish`) and Aladdin's Lamp
(`Modifier::NextDrawFromTop { count }`, made by `Effect::NextDrawLooksAtTop`).
`GameState::draw_may_be_replaced` decides whether a draw waits.

What the front draw has been through is `GameState::draw_front`
(`FrontDraw`: Sanctuaries and dredge declined, Chains already applied to it,
the cards a Lamp is looking at), hashed, and reset when the card is done
(`take_front_draw`). One step at a time (`front_draw_step`):

1. A Sanctuary not declined: `YesNo { MayDo }` as before.
2. Dredge cards on offer and nothing mandatory: `ChooseCards { prompt:
   Dredge, min: 0, max: 1 }` over those cards; none is the plain draw.
3. More than one way on (dredge beside a mandatory one, or two mandatory
   ones whose order matters): `ChooseCards { prompt: DrawReplacement, min:
   1, max: 1 }` over their sources, dredge cards first (CR 616.1, 616.1e).
   Replacements that do the same thing (two Chains of one controller, two
   Rings, two Lamps of one X) are interchangeable and are not asked about.
4. One mandatory replacement: applied. Chains with an empty hand mill one
   and replace the draw (the card's ruling); with one card it is discarded;
   with more, `ChooseCards { prompt: Discard, min: 1, max: 1 }` over the
   hand. A Lamp looks at the top `count` and, with two or more, asks
   `ChooseCards { prompt: PutOnBottom, min = max = len - 1 }`; the rest go
   to the bottom in a random order. A Ring asks its `Wish`.
5. Nothing: the card is drawn (a whole multiple draw at once when no
   replacement has begun on it and no Chains could treat its cards
   differently).

Chains and Lamp leave a draw behind, which every replacement not yet applied
to it may modify (CR 614.5, 616.1f, 616.2): the optional ones are asked
again about it, so declining a dredge and then meeting the Chains still lets
the player dredge the card the Chains just made them discard. Both drainers
answer through `draw_offer_answered` (yes or no) and `draw_choice_answered`
(cards); `AwaitingOp::SkipDraw` and `PlanKind::SkipDraw` carry only the
question.

An instruction that does something to "the card drawn" and whose draw had to
wait keeps its place (`Resolution::draw_watch`, as a loot keeps
`Resolution::loot`) and runs again once the draw is settled, acting only on
a card drawn plainly: `GameState::draws_replaced` counts the draws a
replacement modified (CR 614.11b). Sindbad is the one such instruction.

Not modelled: a Chains discard is not offered to Library of Leng or
Nephalia Academy (those replacements are asked only by an effect's discard
inside a resolution, `resolve/discard.rs`), and
two Lamp effects of one Lamp in one turn are applied oldest first rather
than in the player's order.

### Complete replacements of a life gain (CR 119.10, 614.5, 616.1)

`GameState::change_life` sends every positive change through
`state/life_gain.rs`. `Modifier::LifeGainDrawsInstead` (Lich) turns the whole
gain into that many draws; `Modifier::LifeGainNoLifeInstead` (Sulfuric Vortex)
turns it into nothing. Neither leaves a `LifeChanged` event or adds to
`life_gained_amount`. Several replacements with the same outcome apply only
once, without a redundant question (CR 614.5). When both outcomes apply, the
affected player chooses one source through `ChooseCards {
LifeGainReplacement, min: 1, max: 1 }`; after either complete replacement
there is no life gain left for the other (CR 616.1, 616.1e-f).

A contested gain and every later gain wait in `GameState::life_gains_to_offer`
as `PendingLifeGain { player, amount, cause }`. A resolution drains that queue
before its next instruction (`AwaitingOp::LifeGainReplacement`); the machine
drains gains queued outside one through `PlanKind::LifeGainReplacement`.
Lich's draws use the ordinary replacement-aware `draw_cards` path, so those
draws and their own choices finish before the next waiting life gain (CR
616.2). A replacement that disappears while queued no longer applies, and a
player who cannot gain life cannot receive either replacement (CR 119.7).
Damage-floor calculations likewise count no replaced gain as additional loss
allowance. The queue is included in snapshot hashes, fingerprints and loop
progress, and an active replacement continuation forbids forks.

The shared object-choice protocol names public sources, not internal
`EffectId`s. A pool lint therefore rejects different kinds of complete
life-gain replacement on the same card, including granted modifiers; the
runtime also rejects an ambiguous hand-built state instead of silently using
registration order. Separate sources such as Lich and Sulfuric Vortex remain
fully selectable. If a future card needs two such kinds on one permanent, the
choice must gain effect identity rather than reusing one indistinguishable
source answer.

### A discard asked about before it is made (Library of Leng)
"If an effect causes you to discard a card, discard it, but you may put it
on top of your library instead of into your graveyard." It replaces where a
discarded card goes and nothing else: the card is still discarded (CR
701.9a, `GameEvent::Discarded` is journalled), and one put into the library
is not revealed, so its characteristics are undefined to whatever reads the
discard (CR 701.9c) and the log names it to its discarder only.

The question comes before anything moves, as CR 903.9b's does
(`resolve/discard.rs`, modelled on `ask_commander_replace`): a card that
reached the graveyard and was then put back would be a different game. Every
door an effect's discard goes through names its cards first —
`DiscardRandom` (after the random draw, which is never repeated),
`DiscardHand`, Balance's hands (`equalize`), and the two choices whose
answers discard, `DiscardChain` (`DiscardForPlayers`, `RevealHandDiscard`)
and `DiscardThenDraw`. Each discarding player who controls a Library of
Leng is asked one `Pending::Arrange` (`ArrangePrompt::DiscardToLibrary`):
a graveyard pile first, so the answer with no preference is the discard as
printed, and a library pile in any order. The answers wait in
`GameState::discard_answers`, and `GameState::discard_card`, the effect
discards' one door, spends them, placing the library pile as it was listed
(`discards_on_top`). What runs after the answer is `DiscardThen::Cards` (the
named cards, then the next instruction) or `DiscardThen::Choice` (the choice
answered again with the same cards; its second visit finds them answered).
A cost's discard and the cleanup step's (CR 514.1) are no effect's and are
never asked.

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

A rule lasts as long as its source is on the battlefield, and within a
resolution that is decided per instruction: `resolve::run` calls
`replacement::expire_departed_rules` before each instruction and after the
last, never inside one. "Destroy target permanent. Its controller creates a
token" with the Doubling Season as the target makes one token (#291), and a
rule whose source goes first in a simultaneous event still applies to the
rest of that event by last-known information (the Dauthi Voidwalker in a
Toxic Deluge).

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
door with the shield skipped (CR 701.19c).

Ordinary destruction also checks the turn's exact-object regeneration
prohibitions and active `Modifier::CantBeRegenerated` rule effects before
offering or spending a shield. A continuous prohibition reads its filter at
destruction: `Filter::DealtDamageBySourceThisTurn` names the exact current
recipients that source actually damaged this turn, so removing the static
source removes its prohibition. `Effect::DamagedThisWayCantBeRegenerated`
instead records the creature recipients of positive damage dealt by this
resolution's source after prevention and redirection. That prohibition lasts
through the turn independently of the source and does not follow a recipient
through a zone change. Neither form consumes a shield it disallows.

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

### Damage, prevention and redirection (CR 120.4, 615.7, 616.1)

Every combat damage step and every damage instruction first collects its
simultaneous assignments. `damage::DamageWork` then applies prevention and
redirection before committing any damage result. Combat retains this work in
`Engine::combat_damage`; a resolving spell or ability retains it in
`AwaitingOp::Damage`. Finishing a choice resumes that exact instruction or
combat step, without granting priority or checking state-based actions midway.
Fight collects both creatures' power before either result changes counters.

`ChooseDamageEffect` asks the affected player, or an affected permanent's
controller, which applicable effect to apply. After each answer the engine
recomputes applicability and the chooser. Different players decide in APNAP
order, restarting that order when redirection makes an earlier player's
choice applicable (CR 101.4d). One effect alone needs no ordering question,
and neither do several whose order cannot change the event
(`damage::order_matters`): each only prevents (protection, Fog, a shield,
bought prevention), no two modify the same damage part, and no effect is
offered for two recipients, so each applies to its own damage whichever
comes first. Protection from black on two creatures under Plague Spitter is
an effect on each creature and nothing to choose between (CR 616.1 has the
controller choose among the effects on one object). One redirection may be
among them when nothing at its destination has a budget, every option there
being prevention that is never spent (protection, Fog): Pariah sending a
player's Pestilence damage to a Paladin en-Vec beside the Paladin's own
meets the protection in every order. A second redirection, a redirection
onto a shield, a reflection, an amount replacement, a face-up turn or a
counter removal keeps the question, as does a shared shield. Until 11
November 2026 every
chooser offered two different effects was asked, so a record from before
holds that answer where a build after it asks nothing.
An infinite shield such as Fog remains in force; a finite shield spends only
its actual prevention. A chosen-source shield that prevents or redirects
nothing remains available (CR 609.7b).

When one finite shield cannot prevent all damage from several simultaneous
sources, `AllocatePrevention` offers each source/recipient part and requires
an exact total (CR 615.7). The player can split the shield between sources;
this changes which source deals damage, including its deathtouch and lifelink.
The action rejects duplicate or unknown parts, excessive shares and incorrect
totals before mutation. `DamageChoiceId { batch, step }` rejects answers from
an earlier decision even if the numeric effect IDs happen to coincide.

Each part retains its source incarnation, recipient incarnation and applied
effect history through redirection. A static or shield cannot redirect the
same part again in the same event (CR 614.5), and the new recipient chooses
among that recipient's applicable effects. A departed source is read from its
retained last known characteristics, including when its card has since
returned as a new permanent. Those histories and suspended continuations join
the deterministic snapshot/fuzz fingerprints; `GameObject` gains no field.

All damage, deathtouch marks, commander-damage totals and lifelink results are
committed only after every choice is complete. Life gained as an additional
prevention effect and counters removed by a prevention effect are retained as
results until that commit, so a later player's choice sees no partially
applied life/counter results (CR 120.4, 615.5). Reserved counter removals reduce
the capacity available to subsequent prevention applications. Unpreventable
damage does not spend ordinary shield capacity, but still performs additional
counter-removal effects once per point (CR 615.12–615.12a).

`ShieldStore` gives each resolved shield a stable identity and optional printed
ability provenance. Its identities are never reused during a game, including
after cleanup. A shield on a permanent names its version (CR 400.7), and
cleanup removes shields whose duration is this turn. Prevention bought while
resolving an instruction stays inside that instruction's damage work: it
follows redirected damage and cannot prevent damage from a later instruction.

Damage-source selection uses `DamageSourceRef` (object plus zone-change
version) and a stable `SourceChoiceId`. A current permanent and earlier
incarnations referenced by stack abilities are independently selectable.
`source_memory` retains exact stack, shield, delayed and linked references;
an archived snapshot alone does not make a source eligible. A chosen permanent
spell follows its recorded resolution into a permanent, not an assumed version
increment. The chooser's historical view carries its own entitled identity and
characteristics instead of looking up the current object. Target revalidation
uses the separate exact-target contract described above (#117).

### The monarch's abilities have no source (CR 726.2)

`trigger::monarch_triggers` reads both off the events and queues synthetic
triggers from `ObjectId::NO_SOURCE`, controlled by the monarch they
triggered against; on the stack they are named "Monarch". The takeover is
`BecomeMonarch(ControllerOfTarget)`, read off the creature it carries.

`GameState::set_monarch` is the one door to the designation. It journals
`GameEvent::BecameMonarch` when the crown moves, and only then (a monarch
told to become the monarch again does not become it), which is what the
game log's line and Palace Jailer's CR 610.3b check read: an exile "until an
opponent becomes the monarch" that resolves after an opponent was crowned
since it was put on the stack exiles nothing
(`resolve::zones::an_opponent_was_crowned_since`). Players who lose in one
state-based check leave together (`sba::run` finds every loss first), so
the crown passes over all of them to the heir (CR 726.4), never through one.

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

### "This way": what a resolution has done (CR 608.2c)

`Amount::ThisWay` / `ThisWayBy` and `ThisWay::…` are read off the journal by
`resolve::this_way`, as `resolve::reflexive` reads its event: the entries after
this resolution's `StackObjectResolved` marker are its own, because
resolutions never interleave and nothing happens while one waits for an
answer. `Resolution::this_way_from` narrows the window to one pass of
`Effect::RepeatWhile` (Grist): the op runs its pass spliced into the program
and is itself spliced after it as the check, so a question inside the pass
suspends and resumes like any other, and `this_way_from` being set is how the
op tells its check from its first entry. A died or returned permanent is
matched on `ltb_characteristics` (CR 608.2h); damage is the source's own,
after prevention.

Each-player instructions (`DealDamage` to players, `LoseLife`, `GainLifeFor`,
`DrawCardsFor`) read their amount once per player through
`resolve::amount_for_player`, every player's before anything happens, with
`PlayerRel::Recipient` naming that player; everywhere else `Recipient` is
nobody. `Condition::AmountAtLeast` inside `Effect::IfCondition` reads through
`amount2`, so the resolution-only amounts work there.

### Ending the game by effect (CR 104.1, 104.2b, 104.4c)

`Effect::WinGame` / `DrawGame` write `GameState::game_end` (the first ending
stands); `Engine::game_result` reads it before anything else, right after an
agreed draw, so the game ends the next time the machine looks — immediately,
in rules terms. A win names the controller's team when they play for one.
`Trigger::LastCounterRemoved` reads `CounterChanged::by`, which every
player-driven removal now records (effects: the resolving controller; costs:
the payer; a counter move: the mover). Rules removals (stun, damage to a
planeswalker, SBAs) record nobody.

**A decided game asks nothing.** The machine's own game-over check comes
after the questions a pass may raise first (a counter order, a graveyard
order, an entry's choice), so a state-based check that decided the game
went on to ask, for one, the owner of the cards that died beside the loss to
put them in order (CR 404.3). `Engine::end_a_decided_game` runs after the
machine stops on a question, and after a resolution leaves one standing: once
`game_result` has an answer, the game is over instead, on the board the
question would have been asked on, so that board hashes as it did. A
concession already ended the game whatever was being asked; this is the same
rule for every other ending (CR 104.1). A record from before 11 November 2026
may end with an answer to such a question, which a build after it refuses
because the game is over.

### The defending player of an attacking source (CR 508.5)

`PlayerRel::DefendingPlayer` is `eval::defending_player_of(source)`: the player
the source attacks, or the controller of the planeswalker it attacks (last
known, CR 506.4c). A source that did not attack this combat names the one
opponent of a two-player game during combat (CR 506.2) and nobody at a larger
table.
`Filter::ControlledByDefendingPlayer` read on an ability of an attacking
creature is exact as well: the player that creature attacks (Floral Spuzzem's
"target artifact defending player controls" at a table of three). Both read
`CombatState::last_defender_of`, which keeps what each creature attacked this
combat after it is removed from combat ("the player that creature was
attacking before it was removed from combat", CR 508.5). Read on
anything else (a spell such as False Orders) it stays coarse: any opponent of
the active player during combat. `mechanics_tests::combat_memory` plays the
three-seat case.

### What combat and damage leave behind (wave 2)

Four records, each read by name and never inferred from the journal:

- `PerTurn::blocks` — every block declared this turn, as a `BlockRecord` of
  the blocker and the attacker (each a `DamageSourceRef`, the object as it was)
  and the attacker's controller at that moment. Written in
  `GameState::declare_block`, cleared with the turn. `Filter::BlockedByReferenceThisTurn`,
  `BlockedReferenceThisTurn` and `BlockedThisTurnBy` compare both versions, so
  a creature that left and came back is a new object that was never blocked
  (CR 400.7); `Effect::ReturnPerBlockedDeath` reads the controller.
  `Condition::SourceWasBlockedThisTurn` compares the source's captured event
  incarnation at both trigger collection and resolution (CR 603.4).
- `PerTurn::damage_log` — every damage event this turn after prevention
  (CR 615.1: prevented damage was never dealt), with the amount, combat or
  not, the recipient (and the version of an object recipient) and an
  `Arc<GameObject>` snapshot of the source as it dealt it. Written in
  `damage_history::log_damage`. `Amount::DamageThisTurn` matches its `by`
  filter against the snapshot, so "damage dealt by artifacts" counts a
  creature that was an artifact then; `Filter::DealtDamageToSourceThisTurn`
  and `Effect::DealHalfOfSpellDamage` (damage a spell dealt while on the
  stack) read it too. A dies trigger's "to it" counts the incarnation that
  died (the last version damaged before it left).
- `GameState::ltb_combat` — who a permanent was in combat with as it left the
  battlefield, written and cleared where `ltb_characteristics` is.
  `Filter::BlockingSource` / `BlockedBySource` fall back to it once the source
  is gone (CR 608.2h: Abu Ja'far's dies trigger destroys the creatures that
  were blocking it, though it died out of combat).
- `DelayedWhen::DealtDamageThisTurn` — `Effect::WheneverTargetDealtDamageThisTurn`'s
  delayed trigger (CR 603.7): it watches the first target as the object it is
  now and fires on each damage event to it whose source the filter matches as
  it deals it; "that much" is the event amount. It lapses with the turn.

### A trigger with two creatures: event object and subject

`Trigger::AttachedBlocksOrBecomesBlockedBy` (Infinite Authority) is an Aura's
own ability about the creature it is attached to: its controller is the
Aura's (CR 603.3a), and the creature losing its abilities changes nothing.
One event per blocker–attacker pair (CR 509.3b, 509.3d); the other creature
is the event object, and the attached creature, as the object it was then,
is `PendingTrigger::event_subject`. On the stack it rides as
`Rider::EventSubject`; a delayed trigger made in its resolution
(`DelayedAction::TriggerAbout::subject`) carries it on, and
`Effect::AtNextEndStepAboutSubject` makes a delayed end-step trigger about
it, in which it is the event object and the reference. A subject that has
left the battlefield since is a new object (CR 400.7) and is not affected.
`Trigger::AttachedBlocksOrBecomesBlockedByOneOrMore` shares the same source
and attached-object lookup but groups matching declarations by side: once
when the attached creature blocks and once when it becomes blocked, rather
than once per opposing creature (CR 509.3a, 509.3c).

### The reference object (`RuleContext::reference`)

A relational filter (`Filter::Reference`, `BlockingReference`,
`BlockedByReferenceThisTurn`, `BlockedReferenceThisTurn`,
`SharesTypeWithReference`) reads one object besides the source:
`Resolution::rule_context` names the first target, else the triggering event's
object (a delayed trigger made by `Effect::AtEndOfCombat { about, .. }` carries
its `about` object as that, which is how Glyph of Doom's end-of-combat trigger
still knows its Wall). While a second target is chosen (cast wizard,
activation, the resolution-time legality check in `progress/stack.rs`) the
reference is the first target, so `second_targets` may relate to it (Gauntlets
of Chaos, Glyph of Delusion). A continuous effect whose filter reads the
reference keeps it: `EffectFilter::DslAbout(filter, id, version)` evaluates
the filter with that object as the reference while it is still that object
(Feint's "that creature and each creature blocking it" covers a later
blocker, CR 611.2c); an effect that locks its set is bound to the objects as
it resolves (`resolve::bound_now_with_context`). Castability and activation
offers ask a relative second target per legal first target
(`casting::second_is_reachable`, `Engine::ability_has_targets`). Nothing outside a resolution or a target choice has a reference, and
there every such filter matches nothing.

### Tokens put onto the battlefield attacking (CR 508.4)

`Effect::CreateTokenAttacking` creates its tokens tapped, then
(`resolve/attacking_tokens.rs`), during a combat of their controller's turn,
asks per token which defending player or planeswalker it attacks: no question
with one option; else `Pending::ChoosePlayer` (skipped with one player) and,
when that player controls planeswalkers, `Pending::ChooseCards` over them (none
chosen = the player). The tokens join combat through
`CombatState::declare_attackers` and `remember_defender`, and nothing else: no
`BecameAttacker` is journalled and `per_turn.attacked` does not name them
(CR 508.3a, they never "attacked"), and the declaration's requirements and
restrictions are never asked (CR 508.4c). Outside combat, or for a player who
is not the active one (CR 506.3b), they only enter tapped. "Whenever you
attack this turn" is `DelayedWhen::YouAttackThisTurn`, read off the journal's
`BecameAttacker` entries once per scan and dropped with the turn.

### What a player did during their last turn (Arboria)

`Player::last_turn_acted` is written as each player's turn ends
(`remember_completed_turn_attackers`): they cast a spell
(`per_turn.spells_cast`, countered or not) or a nontoken permanent entered the
battlefield under their control (`per_turn.put_nontoken_permanent`, set in
`GameState::move_object`). Who *put* a permanent onto the battlefield and under
whose control it entered differ only for an effect that puts a permanent onto
the battlefield under another player's control; the record reads the second.
`Modifier::CantAttackPlayerIdleLastTurn` is asked as a pair restriction in
`AttackRules::allows`, for players only (ruling: planeswalkers may be
attacked).

### A departed reference keeps its history (CR 608.2h)

A delayed trigger about an object (`DelayedAction::TriggerAbout`) is queued
with that object and the identity it had, whether or not it is still there:
"that creature" affects nothing once the identity is no longer current (CR
603.7c), but what the effect reads about it is its last information.
`eval::last_battlefield_version` is a permanent's version, or, once it has
left the battlefield, the version its departure recorded in the journal;
`eval::blocked_this_turn` compares block records against it, so Glyph of
Doom's Wall that died in combat still names the creatures it blocked, and
`damaged_incarnation` (Brine Hag, Blazing Effigy) reads the incarnation that
died. `CR 202.3e`: `Characteristics::x_on_stack`, written by the projection,
makes a spell's mana value count X while it is on the stack.

### Improvise and its grant

Improvise (CR 702.126a) shares convoke's tap question
(`TargetPrompt::Convoke`): `casting::tap_to_pay_sources` is creatures for a
convoke face, artifacts for a face with improvise or while
`per_turn.improvise_next_spell` is set for the caster (Archway of
Innovation), each permanent once. The grant is spent in
`cast_wizard/commit.rs` by the player's next cast, whatever it tapped.

### Convoke pays coloured mana

A creature tapped for convoke pays one generic mana **or one coloured mana
of one of its colours** (CR 702.51a); an artifact tapped for improvise pays
generic mana only (CR 702.126a). `ManaCost::convoked_colored` takes the
colours each tap pays with (`casting::tap_colors`) and removes the coloured
symbols they can pay, as many as can be (a maximum matching, so a white-and-
blue creature is kept for the `{U}` a white one cannot pay); a tap that paid
no colour pays generic mana, which comes off the count the caller has left.
`{W}`, hybrid, Phyrexian and `{2/W}` symbols take their colour's tap; `{C}`,
snow and X take none. One function for the four readers that must agree:
the offer's probes (`casting::keyword_priced`, used by `can_cast` and
`cast_options`), the tap question's bound (the coloured symbols the sources
can pay plus the generic delve has left), the payment (`pay_and_cast`) and
the payment window's owed cost (`Engine::wizard_paid`). Until #230 the taps
paid generic mana alone: Chord of Calling (`{X}{G}{G}{G}`) was castable only
when the lands made all its green. The free-cast and escape probes
(`free_face_affordable`, `free_cost_affordable`, escape) still count the taps
as generic, which can only leave a legal cast unoffered, never offer one the
payment refuses.

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

### For each permanent, its controller (CR 608.2h, 101.4)

`Effect::ForEachMatching` runs its instructions once per permanent its filter
matches, with that permanent as `EventObject` and its controller as
`EventPlayer` (`resolve/for_each.rs`). The permanents are counted once, as
the instruction begins (CR 608.2h), and their controllers take their turns
in APNAP order (CR 101.4): the active player's permanents first, then each
other player's in turn order, one player's in battlefield order.

When the instructions sacrifice through `SacrificeFilter` — Fade Away: "For
each creature, its controller sacrifices a permanent of their choice unless
they pay {1}" — the iteration is a `for_each::Count`, carried in
`SubjectContext::counted`. A sacrifice the instructions reach is not made:
`for_each::hold` owes it. When a player's last permanent has had its turn,
that player chooses each permanent owed (`AwaitingOp::HeldSacrifice`, a
`ChooseCards` of one), never one already chosen, and after the last player
everything chosen is sacrificed at once (`sacrifice_together`), each only
while its chooser still controls it (CR 701.21a). So each player answers all
their payments, then chooses all their sacrifices, before the next player is
asked anything. Fade Away's ruling of 2004-10-04 says it in these words:
each player counts their creatures as it begins, the active player announces
first, all sacrifices are done at the same time, and "you can't sacrifice
one creature in order to pay for another and thereby avoid payment for the
first". A counted creature that has left — chosen for another, or
sacrificed for mana to pay for another (CR 605.3a) — still has its turn and
its controller still owes for it. The payments come before the choices so
that nothing chosen can still be sacrificed for mana afterwards. A player
asked later is not shown what an earlier player chose, which CR 101.4b says
they know; the chosen permanents are still on the battlefield until the end,
as the Dungeon's tax choices are.

Instructions that do not sacrifice (Heroism, Tidal Flats, Dread Wight,
Fighting Chance) act on the permanent itself, and a permanent that has left
before its turn is passed over: it is a new object (CR 400.7). They iterate
as before, through `SubjectContext::for_each_remaining`, in APNAP order; at
a table of two none of them matches permanents of both players, so their
order and their hashes are what they were. `SubjectContext::counted` is
hashed only while a count is open.

Carbon's ladder game seg2 attempt-000000033 met the iteration as it was: on
turn 12 the nonactive player was asked before and between the active
player's choices, in battlefield order, and on turn 22 a player with two
creatures and no mana chose one for the other, the second was skipped as
gone, and one permanent was lost instead of two.
`engine::for_each_count_tests` plays both on the engine and, through
`for_each::ONE_AT_A_TIME`, on the engine as it was.

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

The untap step syncs the table itself, right after it phases permanents in
and out and before the active player untaps (CR 502.1, then 502.3): the
progress loop's next pass would come after the untap. So a Sleep Paralysis
that phased in with its creature keeps it tapped in that very step, and a
permanent with phasing that phased out no longer holds anything tapped. An
untap step in which nothing phased does not sync. Until 11 November 2026
the parked statics came back only after the untap, so a record from before
in which an Aura or another static that keeps permanents tapped phased in
plays differently from that untap step on.

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

**Externally imposed non-mana costs belong to the announced price.** Spell
legality and the cast wizard call `casting::additional_spell_costs` against the
same `projected_spell` used by mana pricing, then append the owned parts after
the selected face's printed and alternative costs. Activated abilities keep
the equivalent complete list in `ActivationPrice`; it is frozen before payment
and carried by mana-window and counter continuations. Offer, X bounds, object
menus, snapshot fingerprints and final payment therefore cannot silently drop
an imposed sacrifice or ask for a different price later. The spell path's
augmenting-path matcher proves that distinct objects can pay all object parts;
the payment path consumes the same ordered answers and records LKI through the
ordinary cost payer.

**Foretell is two actions with two prices** (CR 702.143a). `FaceDef::foretell`
holds only the later cast's printed mana cost. While its owner has priority on
their own turn, `engine/foretell` offers a {2} special action in
`LegalActions::foretellable`; this is not sorcery timing, so combat and a
nonempty stack do not close it. With the {2} still in the lands it is a
planning hint, `LegalActions::unpaid_foretell`, while a mana ability is on
offer (CR 605.3a): the seat taps, then foretells. The action uses no stack, exiles that hand card
face down and marks the exact incarnation with `Rider::Foretold` and the turn.
Only its owner may inspect it, and only that incarnation may be cast after that
turn for the printed foretell cost, with the card's ordinary timing. It has no
time counters and no linked host. `Amount::XIfSourceWasForetold` reads how the
resolving spell was cast, so Starnheim Unleashed also covers its ordinary cast.
The rider survives a move from exile to exile only: the cast writes it again on
the spell (CR 702.143c), and a foretold spell exiled from the stack is a new
object with no foretell permission (CR 400.7; `foretell_tests`).

The desktop plans {2} for the special action rather than the later cast cost,
offers both choices where ordinary casting is also possible, and treats an
offered or planned action as something to do for auto-pass purposes. Codegen
accepts only `K:Foretell:<complete mana cost>`; a non-mana or extended spelling
is refused instead of producing a partially translated card.

Casting a permanent card from a graveyard under a player's permission
(Muldrotha, Wrenn and Realmbreaker's emblem) has one reader,
`casting::graveyard_cast_permission`, which the offer, `can_cast_form`, the
cast wizard and the view's graveyard price all ask. Muldrotha's plays are
written down per source and version in `PerTurn::graveyard_plays`; the
emblem's are not, and it is asked first, so a cast under it leaves
Muldrotha's allowance whole.

**A back face is cast at its own timing** (CR 601.3e). Only the face that
will be up on the stack is evaluated to see whether a modal double-faced
card can be cast (CR 712.8c), and only the alternative characteristics for
an Adventure (CR 716.3a). So `casting::can_cast_form` reads the front's
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
The name is cleared as its permanent leaves the battlefield (CR 400.7).

A cast prohibition by name uses the same remembered rider but a different
lock. `EnterModifier::ChooseCardNameExcept(types)` publishes the existing
`Pending::ChooseCardName` with `excluded_types`; the ordinary name-action
validation accepts only a printed face without those types. Meddling Mage
combines that with `PlayersCantCast { who: EachPlayer, filter: NamedBySource }`.
`casting::cast_is_forbidden` checks both the offer and the direct action, so
neither player can cast the named spell while differently named spells and
land plays remain legal. A spell cast as one face has that face's
characteristics alone (CR 709.3a, 716.3a), so every lock is asked of the face
being cast: `casting::face_is_forbidden`, read beside `face_timing_allows`
by `can_cast` and `cast_options`. A forbidden front leaves a back face that
is not, and a forbidden back face is not offered. A face whose name no object
of the game has carried projects as a name nothing is called
(`Names::printed`), never as the card's front. Choosing a name invalidates
projections, and the rider participates in object versioning and the state
hash.

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

**Buyback and multikicker are questions on top of a way of casting** (CR
702.27a, 702.33c), not ways of casting: each adds to whichever way was chosen,
a free cast's and an alternative cost's included (CR 118.8a, 118.9d), so
neither is a `CastModeKind`. `cast_wizard/announced_costs.rs` asks them, in the
`Buyback` and `Multikicker` stages between `Kicker` and `Replicate`, after X as
the kicker is (CR 601.2b lists all of it as one announcement, and nothing
reads the order). A question is asked only when a yes could be paid: parts
through `casting::additional_payment_capacity_for`, with a `CostChoice` that
says buyback was announced, and mana either floating or still to be made.
Mana to be made counts (CR 601.2g), so a multikicker's bound is
`REPLICATE_CEILING` while a source is left, as an X's is, and a count the
window cannot reach is reversed there (CR 601.2h, 734.1). `CastWizard::buyback`
holds what comes off the printed mana, a number since the wizard rides in every
`Engine` by value, and
`wizard_total_before_reduction` adds both, so the frozen reductions of CR
601.2f reach them. Non-mana parts are paid with the mandatory ones, a random
discard last (CR 601.2h), and a buyback's `Sacrifice` does not touch the
sacrifice receipt, which is the printed additional cost's (CR 607.2i).

The ending is one branch of `finalize_spell`, which is why no other door needs
to know: `Rider::Buyback` is read there, at CR 608.2n, and a spell countered
(CR 701.6a) or with no target left (CR 608.2b) leaves by another door. A copy
carries the rider (CR 707.10) and is sent nowhere: `is_card` is false, it takes
a copy's ending (CR 704.5e). The rider goes when the object leaves the stack by
any door (CR 400.7).

**A resolved spell's competing endings are its controller's to choose** (CR
616.1, 616.1e). Buyback (CR 702.27a), rebound (702.88a), an Adventure (715.3d)
and whatever `replacement::graveyard_destination` finds (Yawgmoth's Will,
Dauthi Voidwalker, a one-shot graveyard permission) all replace CR 608.2n's one
move, none is of the kinds CR 616.1a–d put first, and once one is applied the
card is not going to the graveyard and nothing is left for the rest (616.1f).
`Engine::spell_endings` lists them. With two or more,
`Engine::ask_spell_ending` asks the spell's controller, a
`Pending::ChooseCastMode` of `CastModeKind::SpellEnding` held by
`PlanKind::SpellEnding`, and the spell stays on the stack with its work done
until the answer calls `Engine::finish_spell`. A plan and not a suspended
resolution, because a spell with no effects reaches `finalize_spell` with no
`Resolution` at all. It is asked after `finish_resolution` has emptied
`state.reflexive`, which must be empty while a question is out. `asked` is the
controller, so a controlled player's answer is their controller's through
`controlled_actor` (CR 724.5). A concession asks again from what is left (it
expires the departed's rules first, since the leaving was an event of its own,
#291); with one ending left nobody is asked and it applies.

Chosen `SpellEnding::Exile` goes to `move_object` as a move to the graveyard,
because the move is what applies the replacement and counts its counters. An
Adventure that takes another ending turns back to its front face first (CR
715.4), and rebound then casts that creature free.

Flashback (and what shares its rider) is not offered beside any of them: it
exiles "instead of putting it anywhere else" (CR 702.34a),
so the hand a buyback chose would be exiled too and every order ends alike. Not
asked for that reason, and known not to be exact: flashback beside Dauthi
Voidwalker exiles without the void counter, where CR 616.1e would let the
controller take either. No spell is worse off for it, and it matters only to the
player controlling them.

`GameObject::times_kicked` beside `kicked` is the count: a multikicker's, or 1
for a kicker. Both stay with the permanent the spell became (CR 400.7d), both
go when it arrives from anywhere else (`apply_enter_modifiers`, with the X), and
an `ETB` trigger takes them onto the ability on the stack with the X, so
`Effect::IfKicked` asks one object whatever is resolving.

**A back face's mana value is its front face's** (CR 202.3b). A nonmodal
double-faced card's back face has no mana cost, and up on the battlefield
(CR 712.6d) or cast transformed (CR 712.6b) its mana value is computed
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

## Custom modes

Not built. An early plan had a sandboxed script (Rhai) describe a format's
own rules; no script engine is linked, and none is planned. A format's rules
are Rust in the engine, and a new mode starts as a design in
`docs/proposals/` (#176).

## Determinism & performance
Seeded ChaCha8; no HashMap iteration; journal = replay/resume/crash-
recovery source of truth. Budgets: legal_actions < 50 µs, engine clone
< 5 µs, full AI game < 2 ms.

### The replay contract

**Until the first release candidate this contract does not bind** (owner, 6.11.2026). Baylee is
in beta, so a change may move the state hash or stored games; refresh the fixture with
`refresh_fixture_hashes` and say so in the commit. It binds again from the first release
candidate on. Recorded LLM games stay useful as training data either way: an older record
converts on the frozen runtime of its build.

A game is its preset (seed included) plus every `(seat, action)` handed to
`Engine::apply`, in the order they were handed in. The seat is part of the
record and is never read back out of `pending()`: before turn 1 every seat is
asked its mulligan at once (`engine::mulligan`, house rule 4) and answers in
whatever order its answers arrive, so the record's order is the order of
arrival.

Randomness comes from one seed and two kinds of stream:

- **The starting seat** is drawn first from the table's stream, uniformly
  across the seats (`HouseRules.starting_player == None`), before any opening
  library is shuffled. A controlled test may name it explicitly instead. Old
  records without the field keep historical seat 0; new randomized presets
  serialize it as `null`, so a replay makes the same choice again.
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

A record keeps the hash after every input, so the host hashes the game after
every answer. The hash takes the arena in as one digest per chunk, and each
retained damage source as one digest, so a part nobody wrote costs eight bytes.
`GameState` remembers those digests (`state::hash::SnapshotMemo`) and feeds them
again while the chunk is the same allocation; holding the chunk is what makes
"same allocation" mean "same content", since any write to a shared chunk copies
it. The stream, and so every hash a record holds, is exactly the one digesting
every part gives (`state::tests::the_remembered_hash_is_the_written_hash`,
`hasher::tests`); once this contract binds again, a change that alters it breaks
every stored record's replay and is a decision, not an optimisation.

So a rule that is new writes nothing into a game that never meets it. A field
the hash reads writes nothing while it is unset, or stays out if it follows from
what the hash already covers (`Player::upkeep_began`, read off the turn clock).
`derive(Hash)` writes a variant's number, so a new variant goes at the end of its
enum and one that leaves must not move the rest: an `Effect` takes its slot, and
`DelayedAction` says the number of the first variant after its gap
(`a_delayed_action_keeps_the_number_a_written_record_holds`). A step of
`Engine::snapshot_hash` that has nothing left to fold in stays, as the
multiplication it was. `record::seen`'s acceptance record is not refreshed to
get past any of this: it is the check that a record already written still
replays, and the fixture is for a rule that changed that game.

`Engine::snapshot_hash` is not a complete comparison: it leaves out the
journal, the open question and its bookkeeping (`pending`, `pending_plan`, the
cast wizard, `priority_holder`, `awaiting_answer`, `resolve_next`,
`regrant_priority`, the activation checklist's scratch fields), the loop watch
and its latches, `delayed_queue` and most of `trigger_queue`, so two engines
that hash equal can still answer the next question differently; the complete
comparison is `Engine::fingerprint` (feature `fuzz`), which names every field.

## Forking a simulation (training and evaluation)

`Engine::fork_for(seat, decks, policy_seed)` (`engine/simulation.rs`) is
the lookahead seam for whoever already owns the real engine: the host's own
house search (`baylee-gamehost`'s `session/foresight/`, which the engine
server and the client's own table run within each level's bound and the
host's, `docs/house-ai.md` §"Foresight at a host") and the evaluator,
`eval4`, which runs its games in-process through
`baylee_gamehost::session::Session`. A seat playing over the gateway has no
engine to fork, and `docs/proposals/lookahead-design.md` remains the design
for that one. It needs `L: CardLookup + Clone`; the registry lookups are
unit structs. It exists only under the engine's `fork` feature (named
`simulation` until 12.11.2026), which gamehost's `foresight` turns on, and
with it the hosting builds. What `cargo dev gate shipped-features` keeps
out of every hosting build is the harness around it, gamehost's
`simulation` (`Session::fork_for` to drive a copy by hand, probes, a
harness's clock). The training crates turn that on through
`baylee-train/onnx`, never a default:
`cargo build-servers` and `build-client` select `--workspace`, so a default
of any member, a training crate's too, unifies into every shipped binary.
`baylee-train/play` is a default and would have leaked it there. The gate's
`verification-hooks-unified` step reads the whole workspace's graph inverted
on the engine and fails on exactly that; it was checked red with
`simulation` in `play`.

A harness that plays through a session forks the session instead:
`Session::fork_for(seat, decks, policy_seed)` (`decks` passed through;
`baylee-gamehost`,
`session/simulation.rs`, under gamehost's `simulation` feature and in its own
tests through a dev edge) wraps the engine's fork in a `SimulationSession`.
Every chair keeps its kind — a driven chair stays driven, and `release`
hands it to the agent it kept — and every house agent is the chair's own,
reseeded from the policy seed (never from the table's game id or deal).
Agent views and house answers are built by the same code as the table's
(`session/table.rs`), so the searching seat is shown what the table would
show it and the house plays as it plays there; there is no record, log,
envelope, clock or spectator, and `act_slice`/`pump_at_most` return counts.
It takes `&self`; `session/tests/simulation.rs` pins that a table forked and
played on records, byte for byte, what its twin records.

A fork is the whole engine copied field by field, so it carries the complete
driver continuation — priority holder, passes, a regrant owed, combat
declaration progress, the cleanup stage, loyalty used, the loop watch's step
count — and no resume point has to be rebuilt. `Engine` is deliberately not
`Clone`: an undisguised copy is the real game's hidden cards and random
stream, so the copy (`simulation::driver::duplicate`) is private and
`fork_for` disguises it before returning. Then, in the fork:

- **Hidden cards are dealt again.** Pools per owner: each opponent's hand
  and library together (the seat knows each one's size and nothing about
  which card is where), each opponent's cards outside the game, and the
  seat's own library. A pool's identities (card, print, rules print, deck
  choices) are sorted by identity — never by `ObjectId` or position — then
  shuffled on the policy seed's `belief` stream and dealt into the same
  zones at the same sizes as **new objects** with fresh ids, version 0 and
  timestamp 0. The old objects are removed from the fork's arena. Ids are
  handed out in deck-list order at setup, so a kept id would name its card
  to anyone holding the list; new ids are allocated in zone order and say
  nothing. What the seat can see now stays out of the deal, the same
  objects in the same places: a hand revealed by a static (`hand_revealed`:
  Revelation, Stromgald Spy), the hand of a player the seat controls
  (`may_inspect_private`, CR 724.4), and a library top a static reveals
  (`library_top_revealed`: Courser of Kruphix, Field of Dreams) — dealing
  those would put a card there the seat knows is not, and with Courser
  change which lands can be played from the top. What the seat knew earlier
  about a hidden card (a top it saw, a card revealed from a hand and kept
  there) is lost with it — losing knowledge is accepted, leaking it is not.
- **Randomness** comes from the policy seed's `future` stream
  (`GameRng::stream`: the policy seed's ChaCha8 key on a stream named for
  its purpose). The real seed, word position and call count are not in the
  fork.
- **The journal** keeps its length and sequence numbers, because rules read
  back into it (spells cast this turn, damage dealt since a source arrived,
  an event's object by its sequence number) and the driver's cursors are
  sequence numbers. `GameStarted`'s seed is zeroed; a move between two zones
  the seat cannot see, and every mention of a removed id, name
  `ObjectId::NO_SOURCE`; a developer command loses its text.
- **Notes about hidden cards** that no rule in the fork can read any more
  are dropped: the copies `damage_sources` keeps of every object as it left
  a zone (`remember_damage_source`, pruned only at the end of the turn), for
  each one taken in a library, outside the game or in another player's
  hand — a draw keeps a copy of the card drawn; `per_turn.drawn`,
  `instruction_drawn`, `wand_of_ith_targets` and `ability_fires` entries
  naming a removed id; the dealt cards' entries in `printed_pt_cda` and
  `offboard_static_candidates`, which the new objects take afresh; and the
  `effect_text_overrides` of effects whose source the deal removed (parked
  ones included, collected before the statics are re-synced, and dropped
  only if the effect is still gone after the sync) or that quote a removed
  card, as the end of the turn would drop them
  (`prune_damage_sources`) — each names a removed card. No other override
  goes: a phased-out permanent's statics are parked, not gone (CR
  702.26b), and keep theirs.
- **Settings**: no seat has capabilities; every other seat's standing
  answers and priority holds are the defaults (they are that seat's private
  setting); the forking seat keeps its own.
- **Derived caches are rebuilt**, not copied: statics are synced
  (`sync_board_rules`, which drops the removed objects' effects and
  registers the dealt cards' off-battlefield statics), the projection is
  invalidated and refreshed, and the priority offer is computed again.

`DeckKnowledge` says where the hidden cards are dealt from. Under `Known`
the multiset dealt is read off the real game, which is the opponent's
submitted deck less what has been seen; that is the seat's to read only
when every deck list is public to it (a published precon, a deck the harness
chose). The interned names and the base cache are functions of the submitted
lists and public play, so such a fork keeps them. `Unknown` is refused.
`Prior(&PublicPrior)` is for a seat that knows its own list and no other,
which is every fair level of the house; it is described below.

Every field of `GameState` is classified in one place,
`GameState::audit_fork` (`state/fork.rs`), which destructures the state
without `..` as the snapshot hash does, so a new field does not compile
until somebody has said what a fork does with it: **carried** (public, or a
function of the deck lists), **idle** (a continuation that must be empty —
the entry pipeline, the state-based-action and counter-order choices, a
graveyard-order batch, the draw, discard, miracle and copy queues, token
cleanup — or the fork is refused), **rewritten** (by the deal, the journal
rewrite or the settle), or **checked** for a public record naming a hidden
card. The driver's fields are classified the same way in
`simulation::driver`.

**The invariant:** for fixed seat-visible facts and policy seed, the fork's
hidden zones and future randomness are the same whatever the real hidden
cards and RNG; forking and advancing the fork leave the real engine's
state, hash and RNG unchanged. `engine/simulation_tests.rs` pins both
halves against twin games, together with continuation (the same actions
take source and fork to the same board; a pass already made is kept) and
every refusal below.

**Dealing from a public prior.** A `PublicPrior` holds, for each other
player, a weighted list of cards (`PublicPrior::new().with_player(player,
cards)`; a card listed twice is one entry with the weights added, and the
order of the list does not matter). The caller builds it from what the seat
can see — its own view and its own deck list — and asserts that nothing
else went into it; the engine cannot check that, and a prior computed from
an opponent's list is that list dealt back into the fork. Under
`DeckKnowledge::Prior` the seat's own library is dealt as under `Known`,
from its own hidden cards. Every pool of another owner — a hand the seat
does not see, a library below a revealed top, the cards outside the game —
keeps its zones' sizes, which are public, and has each slot drawn from that
owner's list, with replacement and with probability proportional to weight,
on the policy seed's `belief` stream. A drawn card is a card as a deck entry
with no printing of its own would make it: `PrintRef::UNKNOWN`, no physical
rules print, the default deck choices, and its rules and printed face
admitted through the door setup admits a deck entry by
(`GameState::admit_printed_card`: linked rules, the interned face, its own
power and toughness, its off-battlefield statics), since no deck at the
table need hold it. So the fork takes the engine's `CardLookup`.

The real cards of such a pool are removed unread. `redeal` marks the pool
`Source::Prior`, takes from the game its zones' sizes and its ids — the ids
name what the fork removes and what the journal rewrite forgets — and never
asks an object of it what it is, what it carries or how often it has moved.
Whatever a card there carried (a rider from a reveal, counters, a stale
status) goes with it, where under `Known` it refuses the fork: the mark
belonged to a card the fork does not hold. `SimulationSession` follows:
under a prior the house in every chair but the seat's is told the prior's
cards as its deck, not the real list.

One fact about every deck list is decided again: setup turns graveyard
ordering on (CR 404.3) when any list holds a card that reads the order of a
graveyard, and from then on owners are asked to order simultaneous
arrivals. A fork on a prior asks that of its own cards after the deal
(`GameState::decide_graveyard_order_from_cards`), so a card in a list the
seat has never seen turns nothing on and one the prior deals does.

**The invariant under a prior:** for fixed seat-visible facts, prior and
policy seed, the fork is the same whatever the real hidden cards, **deck
lists** and RNG: the same card under the same id in every slot of every
zone, the same offer, the same journal, the same stream, and the same game
as both are played on with the same answers.
`engine/simulation_tests/prior.rs` pins it against twins whose lists share
no card, with the missing, empty, weightless and unknown-card priors
(`ForkRefusal::UnusablePrior`, each a fact about the prior and the zone
sizes), the seat's own library, determinism and the untouched source.

What remains of the real lists in such a fork is in three places, none of
which a rule can read, each of which a reader of the raw engine can, and
each of which is in the snapshot hash and the fingerprint — so a fork's hash
must not steer a fair seat's play. The **name table** keeps every name the
game interned and the **base cache** every card's rules and printed face,
the other decks' included. Rules reach them only through a card or a name
the fork itself holds: a name's number is compared for equality and never
ordered or iterated by (the legend rule groups by it, "N different names"
sorts and counts), `Names::find` and the cache's lookup are asked about a
card in the fork or a face of one, and a name the real list interned
compares equal to no object once its cards are gone. But the numbers the
dealt cards' names take count the names interned before them, so two forks
of twins print the same game under different numbers unless the lists
interned as many names and the prior lists cards of neither.
The **effect table** keeps its count of registrations, which is the id the
next effect takes, and its generation, which keys the projection: a hidden
card with a static that works in a hand or a library (Grist, the Hunger
Tide) has moved both. Its effect and its text overrides are dropped by the
settle; the two counters stay, and no rule reads them but to tell two
effects apart or to know a cache is stale. The table's monotone hints are
the same kind of record.

**Refusals under a prior** are the refusals below, sorted by what they are
a function of. Kept as they are, because the seat saw the thing happen: no
such seat, the game being over, the opening hands, every suspended
operation and idle continuation, the loop watch, anything but priority or a
declaration (what the engine is in the middle of, and whom it is asking
what, is taken as visible to every seat); a face-down object or card in a
public zone; a commander in a hidden zone, and everything public that
points at a hidden card, each judged by the id alone. A continuous effect
about a hidden card of another owner is refused whichever incarnation it
names, where under `Known` only the current one is: which incarnation the
card is in counts its hidden moves. Not asked at all of another owner's
hidden cards, because the answer is that owner's secret: rules state of
their own (`MarkedHiddenCard`, every mark `plain` lists) and a static that
works where the card hides. The seat's own library is asked both, as under
`Known`. Neither kind: `PrivatePolicy`, another seat's standing answer in
the journal, is a function of that seat's private setting and not of a
card; it is refused under a prior as under `Known`, and tells the seat that
such an answer has been given once.

The invariant is about the hidden **arrangement**, not about **history**.
A public object carries how it got where it is: a card's `version` counts
every zone change it has made, the hidden ones included — a card put on the
bottom by a London mulligan, or put back by Brainstorm and drawn again,
shows more moves than its neighbour once it is public — and the effect
table's identities and generation count every registration. A fork keeps
that history as the game has it (a reader of the raw engine can count a
public card's past moves, which the seat's view does not show); it deals
the present, where every hidden card is, and does not rewrite what
happened.

**Refused** (`ForkRefusal`), because v1 cannot deal under them without
leaking or dropping a rules fact: the opening mulligans (they run on streams
of the real seed); any question but ordinary priority or a combat
declaration (declare attackers, declare blockers: a turn-based action asked
at the start of its step with nothing suspended, whose offer — attackers,
defenders, the blocks each blocker may make — is public); any suspended
operation, named by its driver field (a resolution, a payment window, a
cast or activation in progress, queued triggers, entry questions, delayed
or upkeep work, a library-top announcement, a draw or discard being asked,
a reflexive trigger, triggers not yet scanned, and a miracle check queued —
which the opening draws and each turn's first draw leave in
`pending_miracle` until the engine has offered it (`offer_miracle`), so the
upkeep of turn 1 is refused and a main phase is not, and a player's first
draw in another's turn (their Brainstorm in my main phase) leaves it until
everyone has passed on the empty stack (CR 702.94a)); a loop watch
holding a signature (it hashes the real hidden cards) or breaking a loop;
any face-down object (CR 708.2) and face-down exile (CR 406.3); a commander
in a hidden zone; a hidden card with rules state of its own (counters, a
status, riders, a copy's base, a face other than its front) — but not a
stale `kind`: a permanent bounced to a hand is still marked a permanent
there; a hidden card with a static that works in a hand or a library
(`ZoneRef::Hand`, `Library`, `NotBattlefield`: Grist, the Hunger Tide),
because every move between the two registers it again, moving the effect
table's identities and generation and leaving a text override naming the
card — a record of where it has been that a deal cannot rewrite; anything
public pointing at a hidden card — an object's targets, its event object,
its attachment, the source of the ability it is, what was discarded,
sacrificed or tapped to pay for it, its riders; a play permission, a
discovered card, a payment for a card, a delayed trigger's condition or
action, a continuous effect about the card's current incarnation; another
seat's standing answer in the journal (`AutoAnswered`; never in `eval4`,
whose seats send no automation settings); an unknown deck. Every refusal
depends on what the seat can see — when draws happened, what is public,
what the pool's multiset is — never on where a hidden card is.

**An activation is in progress from its press until it ends**, and what it
captured for its push is part of it: `activating_abilities` (the source's
ability list, read before a cost can move the source — the Glasspool Mimic
case) and `activating_grant` (a granted ability's definition). Each way an
activation ends empties both: the push takes them, and a mana ability, which
goes on no stack and resolves at once (CR 605.3b), and a reversal after an
answer (`reverse_activation`) release them
(`Engine::release_activation_capture`). So the fork's refusal on these two
fields names an activation still being made. Until 10 November 2026 a mana
ability and a reversal left the capture standing, keyed by its source, until
the next push or activation took or overwrote it, and the settled priority
after any mana ability but a basic land type's (which taps without the
activation checklist) was refused as `Continuation("activating_abilities")`:
2,901 of 6,859 roots one `eval4 --search` run attempted. In play the release
changes nothing: the only later readers of a stale capture were the next push
from the same source (a trigger) and what is read of that source's activated
abilities before its next activation captures afresh (the offer's
conditions, words and target walls, a mode question). Each of them now reads
the source itself, which gives the same list unless the source's abilities
or words changed in between, where the stale list was the wrong one. The
capture is in neither snapshot hash (it is in `Engine::fingerprint` and the
decision checkpoint). The lockstep games in
`engine/activation_capture_tests.rs` hold that: every house deck plays its
neighbour with the release and without it (a test-only switch), the two take
the same answers to the same snapshot hashes and differ in the two capture
fields alone, and four in five of the forks taken in them were refused
before.

**The activator** (`activation_target_choice`: who is activating, for a
target question another seat may answer, and the seats it named) is held
the same way, and its release does move hashes, because it is in
`Engine::snapshot_hash`. `start_activation` used to name it before three
returns that are no activation — turning a permanent face up (CR 116.2b) and
unlocking a door (CR 116.2m), special actions taken through the activation
door, and a prepared cast — and a loyalty activation, which finishes in
`finish_loyalty_activation`, pushed its ability and kept it. Each left the
activator standing with nothing in flight until the next activation was
pressed, and every position in between hashed it in and was refused as
`Continuation("activation_target_choice")`. From commit eb40dc593 (the
**activator boundary**) it is named after those returns and released where
a loyalty ability is pushed, as every other activation releases it. Nothing
reads it but its own target question, the hash, the fork and the prints, so
play is unchanged; the hashes of exactly those positions move.
`engine/activation_choice_tests.rs` plays the change in lockstep against the
engine as it was, on a deck built to reach every path and on the house
decks: the same answers, the same hashes everywhere but at those positions,
and there the activator is the whole difference, and the hash the old engine
wrote is `Engine::snapshot_hash_naming_stale_activator` of the new one.

A record written before the boundary carries the old hash at every input
after a loyalty ability, a face-up, a door unlock or a prepared cast (during
the cast too), up to the next activation pressed (or the next granted mana
ability), and no other. **Convert records written before eb40dc593 with the
runtime that wrote them.** A later `convert4` holds to that: its exact audit
(`record::replay_exact`) diverges at those inputs, and where every
divergence is that activator (`convert4::activator_boundary::refusal`) the
game is refused by name — the build that wrote it, the build converting
it, the boundary and the rule — and the run stops after
`rejected-games.json` lists every such game; it never leaves the game out
as an ordinary divergence, and `--llm-bc-skip-unreplayable` does not take
it. The header's build string (`version+build.N (commit)`) names a commit
but not where it stands to the boundary, so the hashes decide. Only a record
that opens with the converting build's hash meets this refusal; one that
opens otherwise is another build's to `convert4` already (checked by `seen`
in an LLM-BC conversion, an opening divergence in every other).
`record::replay` on a later build accepts such a record through `seen`, as it
does every hash a change moves. Datasets already converted stay valid as
converted. `boundary_scan4 --root <dir> | --run <run> | --dataset
<meta.json> --out <jsonl>` (`baylee-train`) lists, per run, the games that
cross the boundary (read from their actions alone), the games that open with
its own build's hash, and the games a `convert4` of that build refuses
(both).

The twin-game test compares the two forks with `Engine::fingerprint` (every
field) and their whole journals, so a new field or note that carries a real
hidden card into a fork fails it as soon as the fixture exercises it — that
is how the `damage_sources` copies were found, and the review's Grist
override. It exercises draws, land plays, casting and passing, sideboards,
a card defining its own power and toughness and one with a graveyard
static; mechanics it does not reach (searches, wishes, cards put back into
a library) are covered by the refusals and the field-by-field rules above,
not by a twin comparison yet.

### The offer boundary (e1d85f5d0)

Three changes of 11.11.2026 change what a seat is *offered* and not what the
engine does. From e1d85f5d0 `LegalActions::cast_ways` names every way of
casting a castable or payable card that lies outside the hand and the command
zone (its flashback, escape, disturb face, foretell cost), from protocol 43
`restricted_taps` and `restricted_floating` name what restricted mana may pay
for (CR 106.6), and from 1e29094fb the planners, the house's included, read
each `{2/C}` of a cost on its own (CR 107.4e). `LegalActions` is in neither
snapshot hash, no answer is taken or refused differently, and every record
replays. What moves is what binds a question by its JSON: a shadow label's
`pending_sha256` and view hash, and a house answer asked again where a mixed
`{2/C}` cost now plans. **Convert shadow labels written before e1d85f5d0 with
the runtime that wrote them** (`docs/trained-ai.md`); `convert4` names the
boundary when it meets one (`convert4::shadow_bc::offer_boundary`).

### The question boundary (c55a8c6eb, a5eb549ca)

Two changes of 12.11.2026 take questions away. From c55a8c6eb no order is
asked between damage effects that cannot change the event (§"Damage,
prevention and redirection", `damage::order_matters`: protection on two
creatures, a redirection onto protection), and from a5eb549ca nothing is
asked once the game is decided (§"Ending the game by effect",
`end_a_decided_game`). No position both builds reach hashes differently:
the engine as it was, answered either way, reaches the board this one
reaches without asking, and at a question after the decision it hashed the
board this one ends on (`protection_flow_tests`, `game_end_tests`). But a
record written before holds the answers. A later build parts from it at the
input before a removed order question and meets its hash again after the
record's answers to it, and is over where the record still answers. **Convert
records written before c55a8c6eb with the runtime that wrote them.** A later
`convert4` refuses such a record by name (`convert4::question_boundary`:
`QUESTION_REFUSAL`, the builds, the first input and `QUESTION_RULE`) when it
opens with the converting build's hash, and the run stops after
`rejected-games.json` lists every such game, as at the activator boundary;
`--llm-bc-skip-unreplayable` does not take it. A record of another build that
departs only that way is named in its seen audit's reason, which stays one
the flag may leave out. The recognition replays the record and accepts only
those two departures: answers to damage-order questions between a hash this
build parts from and the record's return to this build's hash, and answers
to questions alone after this build's game is over, up to the end line.

`engine::recorded_lockstep_tests` (ignored) plays recorded runs on both
engines in lockstep, the old one through the cfg(test) switches, and lists
the records that part. Over all 192,984 games under `~/baylee-data/v6` on
12.11.2026: 1,406 records answer a removed order question and 5 answer after
the decision, of which 287 replayed exactly on the build before
(241 in `train/v6d-phase-d-run` and `-run2` eval records, 31 in
`v6f-v5-benchmark` eval records, 14 in `v6t-campaign/distill/gate`, 1 in
`v6s-league`); the other 1,124, the LLM-corpus game in `run-haiku-bulk-b`
among them, were already another build's. The lists are in
`~/baylee-data/v6/train/question-boundary-c55a8c6eb/` (`lockstep-parted.jsonl`
with whether each replayed exactly before, `lockstep-all.jsonl`).

### The protocol-44 boundary

Three changes of 12.11.2026, from the Carbon seats' reports, change what a
seat is told and not what the engine does. An offer names a suspend whose mana
is still in the lands (`LegalActions::unpaid_suspend`, beside `suspendable`,
filled in the hand scan of `compute_legal` and kept only while a mana ability
is on offer, as `unpaid_foretell` is), a view names the division a spell on
the stack announced (`PlayerView::divided`, read from `GameState::divided`,
beside the stack so a stack object keeps its shape and the house its position
hash), and a log line names its step (`LogEntry::step`, so a combat's two damage
steps are told apart). `queued_trigger` names a source that has ceased to exist
from the incarnation the trigger saw (`GameState::damage_source`, which keeps
a gone token after `ceased` is cleared). `LegalActions`, views and the log are
in neither snapshot hash, no answer is taken or refused differently, and every
record replays. What moves is what binds a question by its JSON: a shadow
label's `pending_sha256` and view hash where the offer names an unpaid
suspend. **Convert shadow labels written before protocol 44
with the runtime that wrote them**; `convert4` names the boundary when it
meets one (`convert4::shadow_bc::protocol44_boundary`).

Two reports of the same block are answered by the seat and not here, because
the engine's answer must not move. The question "pay {21}?" is put whether or
not it can be paid (`resolve/choices.rs`, CR 605.3a: the mana may be made in
the window); a seat leaves the yes out where nothing it has untapped reaches
the price (CR 118.3, 608.2d). And a division of combat damage is asked as
declare blockers (or the first-strike damage step) hands over to the damage
step (`banding::ask_combat_division`), so the view stands in the step before;
the seat heads the question with the damage step it belongs to (CR 510.1).

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
   control of a player ends too, both ways the engine keeps one as an
   effect: Opposition Agent's `SearchTakeover` hold on a searching player
   (CR 724.2), and Mindslaver's control of a player's next turn
   (`GameState::turn_controls`, CR 724.1), waiting or under way, whether
   they are its controller or the player it controls. A search they were
   making for somebody else when they left goes back to that player, with
   the same cards and limits, to finish as their own search (CR 723.5).
3. What a permanent of theirs held "until it leaves the battlefield" comes
   back (CR 610.3, `GameState::return_what_departed_hosts_held`): their
   permanents left without a move, so this is the one departure
   `move_object` does not see. If they were the monarch, the crown passes
   as they leave (CR 726.4, `GameState::monarch_leaves`): to the active
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

`every_card_that_controls_a_player_has_a_departure_rule` reads the Oracle
text of every implemented and partial card and fails the day one controls
a player some other way than the three it knows: Opposition Agent's search
takeover, Mindslaver's `Effect::ControlPlayerNextTurn`, and Word of
Command's `Effect::ControlPlayerPlayCard`, which is no effect a departure
has to end, since it is read off the play under way at every decision
(`Engine::controlled_actor`) and is gone with it. Step 2 then has to end
the new one as well.

A turn control waits through every turn its player does not take, other
players' and extra turns among them, and through a turn of theirs that is
skipped, which never begins (CR 724.1b). As the player's turn begins,
`Engine::begin_turn` stamps the control with that turn's number
(`PlayerTurnControl::began`); `controlled_actor` reads it only during the
turn it was stamped with; and the next turn's beginning ends it, whoever
takes that turn, the player's own extra turn included ("the effect doesn't
end until the beginning of the next turn", CR 724.1). Of two waiting for
the same player, the later works (CR 724.1a). Keyed on turn numbers alone
it was dropped once two turns passed before the player's (seat 0's on seat
3's, at four seats, or after its controller's two extra turns) and lasted
on through an extra turn the player took right after
(`player_control_tests`).

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

### Timed permissions for exiled cards

`GameState::play_permissions` records the benefiting player and the exact card
incarnation. It lives outside `PerTurn`: Nivix lasts until that player's next
turn begins; Cori Mountain Monastery lasts through that turn's cleanup. Actual
turn starts advance these windows after skip replacements, so skipped turns do
not consume them and extra turns do. Cleanup expires all `ThisTurn` permissions
and prunes entries whose card has changed incarnation (CR 400.7, 514.2, 614.10a).

An optional spell-type restriction is evaluated against the face being cast,
including Adventure characteristics (CR 601.3e). A cast-only permission cannot
play a land. An unrestricted play permission can offer both spell and land
faces, subject to ordinary timing and land limits. Waiving the mana cost does
not waive cost increases or additional costs. Restricted mana and spend riders
also inspect the prospective spell face during both the offer and payment;
creature-only mana cannot pay the tax on a noncreature Adventure.
