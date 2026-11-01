//! Barishi — {2}{G}{G} — Creature — Elemental
//! Oracle: When this creature dies, exile it, then shuffle all creature cards from your graveyard into your library.
//! Set: WTH #119 — Weatherlight | Scryfall ID: f263eb80-f8f2-4b32-8e8b-a297de9f3666 | Oracle ID: ffa93a79-7a10-4331-91de-eabbc4fee17e
// NOT SUPPORTED: shuffling all creature cards from your graveyard into your library is not yet modeled.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BARISHI,
    oracle_id = "ffa93a79-7a10-4331-91de-eabbc4fee17e",
    scryfall_id = "f263eb80-f8f2-4b32-8e8b-a297de9f3666",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "exiles itself when it dies; shuffling all creature cards from your graveyard into your library is not yet modeled",
    ),
    faces = &[face!(
        name = "Barishi",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL],
        power = Some(4),
        toughness = Some(3),
    ),],
    abilities = &[triggered!(
        Trigger::Dies(&Filter::This),
        &[Effect::ExileSource]
    )],
);
