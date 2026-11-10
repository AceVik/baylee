//! Lancers en-Kor — {3}{W}{W} — Creature — Kor Soldier
//! Oracle: Trample
//! Oracle: {0}: The next 1 damage that would be dealt to this creature this turn is dealt to target creature you control instead.
//! Set: STH #8 — Stronghold | Scryfall ID: e7cd99c2-6d4a-48e8-8848-6fdc0788525d | Oracle ID: ef49dc78-9fd9-4cf8-af10-5a6b8ef7fc57
// PARTIAL: has trample; {0} ability to redirect the next 1 damage from this creature to target creature you control is not supported

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LANCERS_EN_KOR,
    oracle_id = "ef49dc78-9fd9-4cf8-af10-5a6b8ef7fc57",
    scryfall_id = "e7cd99c2-6d4a-48e8-8848-6fdc0788525d",
    color_identity = ColorSet::from_slice(&[Color::White]),
    keywords = KeywordSet::TRAMPLE,
    coverage = Coverage::Partial(
        "has trample; {0} ability to redirect the next 1 damage from this creature to target creature you control is not supported"
    ),
    faces = &[face!(
        name = "Lancers en-Kor",
        mana_cost = mana!("{3}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::KOR, subtypes::creature::SOLDIER],
        power = Some(3),
        toughness = Some(3),
    ),],
);
