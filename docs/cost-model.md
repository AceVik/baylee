# Cost Model

A cost is a mana part plus non-mana parts:

`Cost` contains a `ManaCost` and a static slice of `CostPart`. The authoritative
vocabulary lives in `crates/baylee-cards-dsl/src/cost.rs`; `docs/card-dsl.md`
documents the supported authoring forms. Card files normally use
`cost!("{1}{G}", TapSelf, SacrificeSelf)`, `Cost::FREE` or `Cost::TAP`.

The cost families are tap/untap (`TapSelf`, `UntapSelf`, `TapOther`, `TapX`, `TapN`,
`Crew`, `Saddle`), sacrifice (`SacrificeSelf`, `Sacrifice`, `SacrificeX`,
`SacrificeAll`), discard (`Discard`, `DiscardSelf`, `DiscardRandom`), zone
movement (`ExileSelf`, `ExileFromHand`, `ExileFromGraveyard`,
`ReturnSelfToHand`, `ReturnToHand`), life (`PayLife`, `PayLifeX`,
`PayCommanderColors`), energy (`PayEnergy`, `PayEnergyX`) and counters
(`RemoveCounterSelf`, `RemoveCounterSelfX`, `PutCounterSelf`).

Activation X is one announcement shared by the mana and non-mana parts.
`ActivationX` supplies an optional lower bound or a card-defined amount.
Bounds account for fixed resource payments and repeated variable parts;
payment revalidates them. Counted-object selections are collected over as many
pages as needed and paid only after the complete selection is valid. Payment
checkpoints restore the state when a later part cannot be paid. Exhaust is
recorded only after successful payment, separately from per-turn limits.

**The order of the parts is the printed order, and it is load-bearing.**
`Engine::pay_cost` walks them left to right, and four of them — `SacrificeSelf`,
`DiscardSelf`, `ExileSelf`, `ReturnSelfToHand` — move the source out of the
zone it is being paid in. Anything after one of those is asked of an object
that is no longer there, and each fails a different quiet way: `TapSelf` taps
nothing and journals that it did, and `RemoveCounterSelf` finds no counters,
refuses, and leaves the cost half paid with the permanent already in the
graveyard. Magic prints these in one order only, so a cost written the other
way round is a transcription error rather than a card — which is why it is a
pool lint (`baylee-cards::lints::no_cost_asks_for_a_permanent_it_has_already_spent`)
and not a refusal at the moment of activation.

**A part that names an object never names the source twice.** Object-selection
parts ask the player through `cost_wizard`, and the menu they are asked
from is `cost_wizard::menu`, which is the board less the source whenever the
same cost already spends the source the same way: `TapSelf` beside
`TapOther` or `TapX`, `SacrificeSelf` beside `Sacrifice`, `SacrificeX`
or `SacrificeAll`, and the return, discard and
exile pairs likewise. Selesnya Evangel's "{1}, {T}, Tap an untapped creature
you control" is the pool's case — alone on the table it is not offered,
because its own `{T}` leaves nothing untapped to tap (CR 118.3). A different
kind of payment keeps the source on the menu: a creature that taps may still
be the creature sacrificed. And the battlefield the menu reads is the one
the rules see, so a phased-out permanent pays nothing (CR 702.26b).

**A counter paid as a cost is not multiplied.** CR 614.16 and the
counter-doubling replacements are about counters being *put* on a permanent,
and Magic prints nothing that multiplies a removal — so `RemoveCounterSelf`
goes through `replacement::remove_counters`, which takes no multiplier at all.
The counters a permanent *arrives* with are the opposite case: they are a
replacement effect (CR 614.1c), so `EnterModifier::WithCounters` goes through
`replacement::put_counters` and a Doubling Season applies. One Vivid land
under one Doubling Season enters with four charge counters and still pays one
per activation.

The two differ once more on where their number comes from. A cost is a number
the card prints; an arrival takes an `Amount`, because "this creature enters
with X +1/+1 counters on it" is the X announced as the *spell* was cast
(CR 107.3m) and the value of X for the permanent itself is 0. So Walking
Ballista's entry counters are read off the object it came down as and only
when it came down off the stack, while the counter its ping spends is one
counter under any X at all.

Four orthogonal concepts:

1. **Alternative costs** (at most one per cast, CR 601.2b): pitch
   (Force of Will), overload, evoke, miracle, flashback, conditional free
   ("{0} if you control your commander"). Conditions checked at legality.
2. **Additional costs** (any number; optional ones are cast choices):
   kicker, spree, escalate, mandatory extras.
3. **Payment assists** (change how mana is paid, not the cost): convoke,
   delve, improvise.
4. **Total-cost pipeline** (CR 601.2f–h): base or alternative → + additional
   → + increases → − reductions → floors (Trinisphere). Applies to `{0}`
   alternative costs too.

Casting follows CR 601.2a–h as a stepwise `CastPlan` assembled through
`ChoiceRequest`s: modes → alternative/additional costs → targets → X →
total cost → payment.

Delayed payment (Pact of Negation) is a delayed triggered ability, not a
cost — see `docs/engine-internals.md` (unusual casting).
