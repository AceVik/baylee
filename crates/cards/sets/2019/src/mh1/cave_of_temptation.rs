//! Cave of Temptation — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Add one mana of any color.
//! Oracle: {4}, {T}, Sacrifice this land: Put two +1/+1 counters on target creature. Activate only as a sorcery.
//! Set: MH1 #237 — Modern Horizons | Scryfall ID: d86e9149-6fd9-44fc-b765-3e646c7d83d6 | Oracle ID: 75540897-53f6-433b-bd70-9851551df6ef
// IMPLEMENTED — all three lines: {C} from a bare tap, one mana of any color
// for {1} and a tap, and a sorcery-speed {4}, {T}, Sacrifice activation that
// puts two +1/+1 counters on a target creature.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CAVE_OF_TEMPTATION,
    oracle_id = "75540897-53f6-433b-bd70-9851551df6ef",
    scryfall_id = "d86e9149-6fd9-44fc-b765-3e646c7d83d6",
    faces = &[face!(name = "Cave of Temptation", types = TypeSet::LAND,),],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        mana_ability!(cost!("{1}", TapSelf), &[Effect::mana_of_any_color()]),
        activated!(
            cost!("{4}", TapSelf, SacrificeSelf),
            &[Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(2),
            }],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
