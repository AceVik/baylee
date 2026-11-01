//! Mistmoon Griffin — {3}{W} — Creature — Griffin
//! Oracle: Flying
//! Oracle: When this creature dies, exile it, then return the top creature card of your graveyard to the battlefield.
//! Set: VMA #34 — Vintage Masters | Scryfall ID: 0831f747-b3e5-4e5f-9cc2-ad581a90797a | Oracle ID: 7fe9d30f-c778-4769-a3ae-3cb48cd23e07
// PARTIAL — Flying and self-exile on death; returning the top creature card of your graveyard is omitted because the return effect permits choosing any creature card.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MISTMOON_GRIFFIN,
    oracle_id = "7fe9d30f-c778-4769-a3ae-3cb48cd23e07",
    scryfall_id = "0831f747-b3e5-4e5f-9cc2-ad581a90797a",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Partial(
        "Flying and self-exile on death; returning the top creature card of your graveyard is omitted because the return effect permits choosing any creature card."
    ),
    faces = &[face!(
        name = "Mistmoon Griffin",
        mana_cost = mana!("{3}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::GRIFFIN],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[
            Effect::ExileSource,
            // NOT SUPPORTED: Return the top creature card of your graveyard to the battlefield.
            // ReturnFromGraveyardOf would allow choosing a different card.
        ]
    )],
);
