//! Merfolk Raiders — {1}{U} — Creature — Merfolk Soldier
//! Oracle: Islandwalk (This creature can't be blocked as long as defending player controls an Island.)
//! Oracle: Phasing (This phases in or out before you untap during each of your untap steps. While it's phased out, it's treated as though it doesn't exist.)
//! Set: MIR #75 — Mirage | Scryfall ID: d0f34166-8946-4bcc-84af-3540c42ac7f7 | Oracle ID: 19edc98c-edc2-47bf-b2da-00f536d7c158
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MERFOLK_RAIDERS,
    oracle_id = "19edc98c-edc2-47bf-b2da-00f536d7c158",
    scryfall_id = "d0f34166-8946-4bcc-84af-3540c42ac7f7",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::ISLANDWALK.union(KeywordSet::PHASING),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Merfolk Raiders",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::MERFOLK, subtypes::creature::SOLDIER],
        power = Some(2),
        toughness = Some(3),
    ),],
);
