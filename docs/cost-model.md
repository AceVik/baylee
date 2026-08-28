# Cost Model

A cost is a mana part plus non-mana parts:

```rust
struct Cost { mana: ManaCost, parts: SmallVec<[CostPart; 4]> }
enum CostPart { Tap(Filter), Untap(Filter), Sacrifice(Filter), ExileFrom(Zone, Filter),
                Discard(Filter), PayLife(u16), RemoveCounter{..}, Mill(u16),
                ReturnToHand(Filter), Reveal(Filter), Custom(..) }
```

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
