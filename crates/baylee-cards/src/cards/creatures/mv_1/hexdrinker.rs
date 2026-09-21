//! Hexdrinker — {G} — Creature — Snake
//! Oracle: Level up {1} ({1}: Put a level counter on this. Level up only as a sorcery.)
//! Oracle: LEVEL 3-7
//! Oracle: 4/4
//! Oracle: Protection from instants
//! Oracle: LEVEL 8+
//! Oracle: 6/6
//! Oracle: Protection from everything
//! Set: MH1 #168 — Modern Horizons | Scryfall ID: 89f5cc05-5d9d-4709-b3c5-a6249c294acc | Oracle ID: 69bc2afd-9f53-47f2-b9c8-f12732784e10
// PARTIAL — level up is written as the activated ability it is (CR 702.87a);
// the two level bands are not expressible, see the NOT SUPPORTED note below.

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
    coverage = Coverage::Partial(
        "the level bands are inexpressible: no modifier sets power/toughness or grants protection while a level-counter count is in a range",
    ),
    abilities = &[
        // NOT SUPPORTED: "LEVEL 3-7 — 4/4, protection from instants" and
        // "LEVEL 8+ — 6/6, protection from everything". A level band is a
        // static ability gated on a *range* of level counters; the only
        // counter-conditional modifiers (AddTypeIfCountersAtLeast,
        // AddKeywordIfCountersAtLeast) carry no P/T and no ProtectionFrom,
        // and are bounded below only, so neither band can be written.
        activated!(
            cost!("{1}"),
            &[Effect::AddCounter {
                kind: CounterKind::Level,
                amount: Amount::Fixed(1),
            }],
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
