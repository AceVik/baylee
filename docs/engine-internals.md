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
planeswalkers (CR 508.1a); battles will be the third case, and every match
on the enum is written so adding one is a compile error. The engine
enumerates the legal defenders into `Pending::ChooseAttackers` and
validates a declaration against that same list, so a client cannot name a
defender the engine did not offer. Combat damage aimed at a planeswalker
takes loyalty counters off it (CR 306.8); trample past blockers goes to
whatever the creature is attacking (CR 702.19b), and a planeswalker that
has left the battlefield absorbs nothing — the attack stands (CR 506.4c)
but no damage is dealt and no lifelink is paid.

## Events, replacement, triggers
Proposed events are rewritten by applicable replacement effects (each at
most once per event, CR 614.5), applied, journaled; matching triggers are
collected and stacked APNAP (per-player ordering via ChoiceRequest).
SBAs run as a fixpoint before every priority grant (plus format SBAs).

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
