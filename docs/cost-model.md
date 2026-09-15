# Cost Model

A cost is a mana part plus non-mana parts:

```rust
struct Cost { mana: ManaCost, parts: &'static [CostPart] }
enum CostPart { TapSelf, UntapSelf, SacrificeSelf, Sacrifice(&'static Filter),
                PayLife(u16), PayLifeX, Discard(&'static Filter), DiscardSelf,
                ExileSelf, ExileFromHand(&'static Filter), ReturnSelfToHand }
```

That is the type as it stands (`baylee-cards-dsl/src/cost.rs`), and the
sketch it replaces is why it is written out here: the parts are `*Self` where
a cost pays with the source and take a `Filter` only where a choice is
genuinely open. A card file never writes the struct — `cost!("{1}{G}",
TapSelf, SacrificeSelf)` reads left to right the way the card prints it, with
`Cost::FREE` and `Cost::TAP` for the empty cost and a bare `{T}`.
`docs/card-dsl.md` is normative on the spelling, including which four parts
an activated ability may not carry.

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
