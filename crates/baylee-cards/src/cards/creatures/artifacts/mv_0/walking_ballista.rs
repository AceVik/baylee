//! Walking Ballista — {X}{X} — Artifact Creature — Construct
//! Oracle: This creature enters with X +1/+1 counters on it.
//! Oracle: {4}: Put a +1/+1 counter on this creature.
//! Oracle: Remove a +1/+1 counter from this creature: It deals 1 damage to any target.
//! Set: 2XM #306 — Double Masters | Scryfall ID: 5272436e-74f0-44c4-a291-ea8ebc3f1525 | Oracle ID: 4b515bb0-f275-4400-8032-3173b799ab40
// PARTIAL — both activated abilities are built; the entry counters are not
// expressible, see the NOT SUPPORTED note under the card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WALKING_BALLISTA,
    oracle_id = "4b515bb0-f275-4400-8032-3173b799ab40",
    scryfall_id = "5272436e-74f0-44c4-a291-ea8ebc3f1525",
    faces = &[face!(
        name = "Walking Ballista",
        mana_cost = mana!("{X}{X}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::creature::CONSTRUCT],
        power = Some(0),
        toughness = Some(0),
    ),],
    coverage = Coverage::Partial(
        "enters with X +1/+1 counters — the as-it-enters modifiers carry no \
         counters and no Amount, so the X announced as the spell is cast has \
         nowhere to go"
    ),
    abilities = &[
        activated!(
            cost!("{4}"),
            &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }]
        ),
        activated!(
            cost!(RemoveCounterSelf {
                kind: CounterKind::P1P1,
                n: 1
            }),
            &[Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::AnyTarget,
            }],
            target = Some(TargetSpec::AnyTarget)
        ),
    ],
);

// NOT SUPPORTED: "This creature enters with X +1/+1 counters on it."
//
// The as-it-enters vocabulary (`FaceDef::enter_modifiers`) is Tapped,
// TappedUnless, TappedUnlessCount, TappedOrPayLife and ChooseSubtype: none of
// them carries counters, and none of them carries an `Amount`. The X of
// {X}{X} is announced as the spell is cast, and the nearest variant that
// exists — `EnterModifier::WithCounters` — names counters but no computed
// count, which is the same shape the cost side had to split into
// `RemoveCounterSelf` and `RemoveCounterSelfX` to be able to say.
//
// Both activated abilities are complete: {4} adds a +1/+1 counter to the
// source (no target is named, so `AddCounter` puts it on the source), and
// removing a +1/+1 counter as the cost of an activation is paid by the
// engine (`RemoveCounterSelf` off the source, no prompt), after which the
// ping points at `TargetSpec::AnyTarget` — one choice over creatures,
// planeswalkers, battles and players (CR 115.4).
