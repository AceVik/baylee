//! Angel of the Dire Hour — {5}{W}{W} — Creature — Angel
//! Oracle: Flash
//! Oracle: Flying
//! Oracle: When this creature enters, if you cast it from your hand, exile all attacking creatures.
//! Set: JMP #85 — Jumpstart | Scryfall ID: b7ab81d4-aaf7-4c72-9651-5e1482126928 | Oracle ID: 8b1e6b7c-892d-4058-b183-9121cdd623f0
// IMPLEMENTED — flash, flying, and a hand-cast conditional attacking-creature exile.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ANGEL_OF_THE_DIRE_HOUR,
    oracle_id = "8b1e6b7c-892d-4058-b183-9121cdd623f0",
    scryfall_id = "b7ab81d4-aaf7-4c72-9651-5e1482126928",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::FLASH.union(KeywordSet::FLYING),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Angel of the Dire Hour",
        mana_cost = mana!("{5}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ANGEL],
        power = Some(5),
        toughness = Some(4),
    ),],
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::ExileAll {
            filter: &Filter::Attacking,
            linked: false,
        }],
        condition = Some(Condition::SourceMatches(&Filter::CastFromHand)),
    )],
);
