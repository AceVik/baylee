//! Elder Land Wurm — {4}{W}{W}{W} — Creature — Dragon Wurm
//! Oracle: Defender, trample
//! Oracle: When this creature blocks, it loses defender.
//! Set: ME1 #11 — Masters Edition | Scryfall ID: 369b3ebc-572b-4614-aacb-4990309d8848 | Oracle ID: 5948381a-419d-4b8b-8ea2-ea623cb0d606

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ELDER_LAND_WURM,
    oracle_id = "5948381a-419d-4b8b-8ea2-ea623cb0d606",
    scryfall_id = "369b3ebc-572b-4614-aacb-4990309d8848",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Elder Land Wurm",
        mana_cost = mana!("{4}{W}{W}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DRAGON, subtypes::creature::WURM],
        power = Some(5),
        toughness = Some(5),
    ),],
    keywords = KeywordSet::DEFENDER.union(KeywordSet::TRAMPLE),
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::Blocks(&Filter::This),
        &[Effect::continuous(
            &Filter::This,
            Modifier::RemoveKeyword(KeywordSet::DEFENDER),
            Duration::Indefinitely,
        )]
    )],
);
