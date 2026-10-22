//! Mistmoon Griffin — {3}{W} — Creature — Griffin
//! Oracle: Flying
//! Oracle: When this creature dies, exile it, then return the top creature card of your graveyard to the battlefield.
//! Set: VMA #34 — Vintage Masters | Scryfall ID: 0831f747-b3e5-4e5f-9cc2-ad581a90797a | Oracle ID: 7fe9d30f-c778-4769-a3ae-3cb48cd23e07
// PARTIAL — Exiles itself when it dies and returns a creature card from your graveyard to the battlefield; restricting the returned card to the top creature card of your graveyard is not yet modeled.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MISTMOON_GRIFFIN,
    oracle_id = "7fe9d30f-c778-4769-a3ae-3cb48cd23e07",
    scryfall_id = "0831f747-b3e5-4e5f-9cc2-ad581a90797a",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLYING,
    coverage = Coverage::Partial(
        "Exiles itself when it dies and returns a creature card from your graveyard to the battlefield; restricting the returned card to the top creature card of your graveyard is not yet modeled."
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
            Effect::ReturnFromGraveyardOf {
                whose: PlayerRel::You,
                filter: &Filter::CREATURE
            }
        ]
    ),],
);
