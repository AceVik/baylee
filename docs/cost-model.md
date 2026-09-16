# Cost Model

A cost is a mana part plus non-mana parts:

```rust
struct Cost { mana: ManaCost, parts: &'static [CostPart] }
enum CostPart { TapSelf, UntapSelf, SacrificeSelf, Sacrifice(&'static Filter),
                PayLife(u16), PayLifeX, Discard(&'static Filter), DiscardSelf,
                ExileSelf, ExileFromHand(&'static Filter), ReturnSelfToHand,
                TapOther(&'static Filter),
                RemoveCounterSelf { kind: CounterKind, n: u16 } }
```

That is the type as it stands (`baylee-cards-dsl/src/cost.rs`), and the
sketch it replaces is why it is written out here: the parts are `*Self` where
a cost pays with the source and take a `Filter` only where a choice is
genuinely open. A card file never writes the struct — `cost!("{1}{G}",
TapSelf, SacrificeSelf)` reads left to right the way the card prints it, with
`Cost::FREE` and `Cost::TAP` for the empty cost and a bare `{T}`, and a part
with named fields keeps its braces (`cost!(TapSelf, RemoveCounterSelf { kind:
CounterKind::Charge, n: 1 })`). `docs/card-dsl.md` is normative on the
spelling, including which four parts an activated ability may not carry.

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

**A counter paid as a cost is not multiplied.** CR 614.16 and the
counter-doubling replacements are about counters being *put* on a permanent,
and Magic prints nothing that multiplies a removal — so `RemoveCounterSelf`
goes through `replacement::remove_counters`, which takes no multiplier at all.
The counters a permanent *arrives* with are the opposite case: they are a
replacement effect (CR 614.1c), so `EnterModifier::WithCounters` goes through
`replacement::put_counters` and a Doubling Season applies. One Vivid land
under one Doubling Season enters with four charge counters and still pays one
per activation.

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
