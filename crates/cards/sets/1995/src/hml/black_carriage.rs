//! Black Carriage — {3}{B}{B} — Creature — Horse
//! Oracle: Trample
//! Oracle: This creature doesn't untap during your untap step.
//! Oracle: Sacrifice a creature: Untap this creature. Activate only during your upkeep.
//! Set: HML #42 — Homelands | Scryfall ID: 87068116-6000-44ee-b47f-f5cb8c233bb2 | Oracle ID: 6bf5dcf1-5c93-42f4-adaf-a6f2e5807596
// IMPLEMENTED — trample, doesn't untap during your untap step, and sacrifice a creature during your upkeep to untap.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BLACK_CARRIAGE,
    oracle_id = "6bf5dcf1-5c93-42f4-adaf-a6f2e5807596",
    scryfall_id = "87068116-6000-44ee-b47f-f5cb8c233bb2",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    keywords = KeywordSet::TRAMPLE,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Black Carriage",
        mana_cost = mana!("{3}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HORSE],
        power = Some(4),
        toughness = Some(4),
    ),],
    abilities = &[
        static_ability!(Filter::This, Modifier::DoesNotUntap),
        activated!(
            cost!(Sacrifice(&Filter::CREATURE)),
            &[Effect::UntapSelf],
            condition = Some(Condition::All(&[
                Condition::YourTurn,
                Condition::DuringStep(StepKind::Upkeep),
            ]))
        ),
    ],
);
