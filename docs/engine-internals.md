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
`Indefinitely`, conditions. Subtypes are a 1024-bit bitmap (changeling =
one mask OR, not a scan), and the ids in it are **append-only** since #43:
`ALL_CREATURE_TYPES` is the mask a changeling gets and it is a generated
list rather than a range, because a new creature type no longer sits next to
the old ones. `docs/card-identity.md` §"`SubtypeId`" is normative.

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

A projection reads the *board*, and there are two ways for it to read a
stale one. `recompute_with` walks **one object through all the layers**, so
while it runs, that object's cached characteristics are still the previous
projection — and a modifier that counts permanents (`ModifyPTPerCount`) used
to read its own source off that cache. Ashaya, Soul of the Wild makes your
nontoken creatures into lands at layer 4 and is then as big as the lands you
control at 7c, so it has to count itself: it came down one short. The object
under projection is now matched against the in-progress characteristics
(`eval::matches_projected`), which is what CR 613.1 says; every other object
is read from its own finished projection. The other way is the generation
compare itself: it watches the effect **table**, so an input the filters read
that is *not* an effect leaves every projection stale. Naming a creature type
is one — Steely Resolve's static is registered as the enchantment enters and
the type is chosen one question later — so `ChooseSubtype` calls
`GameState::invalidate_projections`, as anything writing a counter already
does. `card_tests::rules::a_cached_projection_is_what_a_fresh_one_would_compute`
is the guard for both: a recompute may not disagree with the cache.

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
object's timestamp is bumped, because CR 302.6 wants control held
*continuously* since the turn began.

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
fails the test.

## Unusual casting
Rebound, suspend, miracle, flashback, evoke, adventures, plot, foretell,
madness, disturb decompose into: `CastPermission` (zone/cost/timing
override) + `PendingCast` (with expiry) + `DelayedTrigger` + `ExileRider`.
Keywords exist on stack objects (rebound can be granted).

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

1. Everything the leaver owns leaves the game.
2. Every `GainControl` effect for them ends, so what they took goes back to
   whoever controls it without them. A Gilded Drake'd creature goes back to
   its owner. Under a later thief it goes back to the earlier one. Their
   `SearchTakeover` effects end too: Opposition Agent's hold on a searching
   player is the one way the engine controls a player (CR 722.2). A search
   they were making for somebody else when they left goes back to that
   player, with the same cards and limits, to finish as their own search
   (CR 722.5).
3. `sba::exile_what_the_departed_control` removes what they still control:
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
ends. The same function does it, called from two places:

- `run_machine`'s step 0a, whenever the refresh found the effect table
  changed and someone has left. That comes before the game-over check and
  before `sba::run`, because this is not a state-based action. Effects that
  end at the end of combat, as a turn or an untap step begins, or as their
  source leaves are all followed by that pass before anybody gets priority.
- `cleanup_step`, right after until-end-of-turn effects end, because that
  step goes straight on into the next turn (`end_cleanup`), and step 0a
  would first look after that turn has begun.

The residue is a resolution: an effect that ends in the middle of one exiles
at the next pass, not inside the resolution. And, like everything else that
happens in the cleanup step, an exile there gives nobody priority and starts
no second cleanup step (CR 514.3a is not implemented). A trigger it causes
waits for the next turn's first priority.

CR 800.4b has four doors. A `GainControl` effect for a player who has left
applies to nothing (`layers::apply`), and neither `gain_control` nor a
resolution's `CreateContinuousEffect` registers one. A search is never taken
over by a player who has left. A card put onto the battlefield "under your
control" by a leaver's resolution stays where it is. Tokens and copies of
spells were closed by #278.

