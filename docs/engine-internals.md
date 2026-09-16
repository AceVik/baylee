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
`Indefinitely`, conditions. Subtypes are a 512-bit bitmap (changeling =
set-all in O(1)).

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

Layer 2 is not cached separately: the refresh writes the projected
controller straight into `GameObject::controller`, so every rule that asks
"who controls this" reads one field and none of them has to know that
layers exist. `base_controller` holds what a control effect will hand back
and is written only by `GameObject::set_controller` — a permanent handover
(entering the battlefield, Gilded Drake, Homeward Path). When the
controller moves in either direction the object's timestamp is bumped,
because CR 302.6 wants control held *continuously* since the turn began.

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
put-backs (`AwaitingOp::PutBackOnTop`, `BottomFromHand`), and
`CostPart::ReturnSelfToHand`. Each fails safe — no entry in
`commander_redirect` means the printed move stands and no question is asked.

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
