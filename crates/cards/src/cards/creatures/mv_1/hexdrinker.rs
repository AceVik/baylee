//! Hexdrinker — {G} — Creature — Snake
//! Oracle: Level up {1} ({1}: Put a level counter on this. Level up only as a sorcery.)
//! Oracle: LEVEL 3-7
//! Oracle: 4/4
//! Oracle: Protection from instants
//! Oracle: LEVEL 8+
//! Oracle: 6/6
//! Oracle: Protection from everything
//! Set: MH1 #168 — Modern Horizons | Scryfall ID: 89f5cc05-5d9d-4709-b3c5-a6249c294acc | Oracle ID: 69bc2afd-9f53-47f2-b9c8-f12732784e10
// IMPLEMENTED — level up (CR 702.87a) and both level bands (CR 711.2a/b),
// each a static ability that holds only while its counter count does.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::HEXDRINKER,
    oracle_id = "69bc2afd-9f53-47f2-b9c8-f12732784e10",
    scryfall_id = "89f5cc05-5d9d-4709-b3c5-a6249c294acc",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Hexdrinker",
        mana_cost = mana!("{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SNAKE],
        power = Some(2),
        toughness = Some(1),
    ),],
    abilities = &[
        activated!(
            cost!("{1}"),
            &[Effect::AddCounter {
                kind: CounterKind::Level,
                amount: Amount::Fixed(1),
            }],
            timing = ActivationTiming::SorcerySpeed,
        ),
        // LEVEL 3-7: 4/4, protection from instants.
        static_ability!(
            Filter::This,
            Modifier::SetPT(4, 4),
            condition = Some(Condition::CountersOnSelfBetween(CounterKind::Level, 3, 7))
        ),
        static_ability!(
            Filter::This,
            Modifier::ProtectionFrom(&Filter::HasType(TypeSet::INSTANT)),
            condition = Some(Condition::CountersOnSelfBetween(CounterKind::Level, 3, 7))
        ),
        // LEVEL 8+: 6/6, protection from everything (CR 702.16j).
        static_ability!(
            Filter::This,
            Modifier::SetPT(6, 6),
            condition = Some(Condition::CountersOnSelf(CounterKind::Level, 8))
        ),
        static_ability!(
            Filter::This,
            Modifier::ProtectionFrom(&Filter::Any),
            condition = Some(Condition::CountersOnSelf(CounterKind::Level, 8))
        ),
    ],
    coverage = Coverage::Implemented,
);
