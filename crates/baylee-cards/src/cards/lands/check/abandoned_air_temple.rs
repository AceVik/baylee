//! Abandoned Air Temple — (no cost) — Land
//! Oracle: This land enters tapped unless you control a basic land.
//! Oracle: {T}: Add {W}.
//! Oracle: {3}{W}, {T}: Put a +1/+1 counter on each creature you control.
//! {T}: Add {W}.
//! {3}{W}, {T}: Put a +1/+1 counter on each creature you control.
//! Set: TLA #263 — Avatar: The Last Airbender | Scryfall ID: 9c0433f9-8f1e-4a19-a83f-a41925f1b1a9 | Oracle ID: 9575d7ce-f26d-4b90-87a3-6329e9799572
// IMPLEMENTED — checkland variant + team pump activation.

static BASIC_LAND_YOU: Filter = Filter::And(&[
    Filter::ControlledByYou,
    Filter::LAND,
    Filter::HasSupertype(SupertypeSet::BASIC),
]);
static YOUR_CREATURES: Filter = Filter::And(&[Filter::ControlledByYou, Filter::CREATURE]);

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ABANDONED_AIR_TEMPLE,
    oracle_id = "9575d7ce-f26d-4b90-87a3-6329e9799572",
    scryfall_id = "9c0433f9-8f1e-4a19-a83f-a41925f1b1a9",
    faces = &[face!(
        name = "Abandoned Air Temple",
        types = TypeSet::LAND,
        enter_modifiers = &[EnterModifier::TappedUnless(&BASIC_LAND_YOU)],
    )],
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::White, 1)]),
        activated!(
            cost!("{3}{W}", TapSelf),
            &[Effect::AddCounterFilter {
                filter: &YOUR_CREATURES,
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }]
        ),
    ],
);
