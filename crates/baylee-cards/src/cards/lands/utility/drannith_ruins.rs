//! Drannith Ruins — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}, {T}: Put two +1/+1 counters on target non-Human creature that entered this turn.
//! Set: MAT #50 — March of the Machine: The Aftermath | Scryfall ID: 9c07dda8-06dd-499a-9825-dc6b9a73e455 | Oracle ID: d1f10cca-8dfa-4ea5-b227-4446cd8514a8
// PARTIAL — {T}: Add {C} and the counter ability are built; the target's
// "that entered this turn" restriction has no variant, so the ability reaches
// any non-Human creature.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes::creature;

card!(
    index = index::DRANNITH_RUINS,
    oracle_id = "d1f10cca-8dfa-4ea5-b227-4446cd8514a8",
    scryfall_id = "9c07dda8-06dd-499a-9825-dc6b9a73e455",
    faces = &[face!(name = "Drannith Ruins", types = TypeSet::LAND,),],
    coverage = Coverage::Partial(
        "the counter ability's target is not narrowed to creatures that entered this turn — no Filter variant reads when a permanent arrived"
    ),
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "target non-Human creature that entered this turn" —
        // `Filter` has no variant for a permanent that entered this turn, so
        // the target is every non-Human creature.
        activated!(
            cost!("{2}", TapSelf),
            &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(2),
            }],
            target = Some(TargetSpec::Object(&Filter::And(&[
                Filter::CREATURE,
                Filter::Not(&Filter::HasSubtype(creature::HUMAN)),
            ]))),
        ),
    ],
);
