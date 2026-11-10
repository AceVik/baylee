//! Urborg Panther — {2}{B} — Creature — Nightstalker Cat
//! Oracle: {B}, Sacrifice this creature: Destroy target creature blocking it.
//! Oracle: Sacrifice a creature named Feral Shadow, a creature named Breathstealer, and this creature: Search your library for a card named Spirit of the Night, put that card onto the battlefield, then shuffle.
//! Set: MIR #150 — Mirage | Scryfall ID: edc9ff0f-adec-4d39-b281-98c5862f506b | Oracle ID: 0678beaf-564c-4e98-b45f-a544925683a1
// IMPLEMENTED — {B}, Sacrifice: destroy target creature blocking it; sacrifice Feral Shadow, Breathstealer, and Urborg Panther to search for Spirit of the Night onto the battlefield.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static CREATURE_BLOCKING_SOURCE: Filter = Filter::And(&[Filter::CREATURE, Filter::BlockingSource]);

static FERAL_SHADOW: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Named("Feral Shadow"),
    Filter::ControlledByYou,
]);

static BREATHSTEALER: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Named("Breathstealer"),
    Filter::ControlledByYou,
]);

static SPIRIT_OF_THE_NIGHT: Filter = Filter::Named("Spirit of the Night");

card!(
    index = index::URBORG_PANTHER,
    oracle_id = "0678beaf-564c-4e98-b45f-a544925683a1",
    scryfall_id = "edc9ff0f-adec-4d39-b281-98c5862f506b",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Urborg Panther",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::NIGHTSTALKER, subtypes::creature::CAT],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[
        activated!(
            cost!("{B}", SacrificeSelf),
            &[Effect::destroy(TargetSpec::Object(
                &CREATURE_BLOCKING_SOURCE
            ))],
            target = Some(TargetSpec::Object(&CREATURE_BLOCKING_SOURCE)),
        ),
        activated!(
            cost!(
                Sacrifice(&FERAL_SHADOW),
                Sacrifice(&BREATHSTEALER),
                SacrificeSelf
            ),
            &[Effect::SearchLibrary {
                filter: &SPIRIT_OF_THE_NIGHT,
                finds: &[Find::BATTLEFIELD],
                optional: false,
            }],
        ),
    ],
);
