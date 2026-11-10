//! Sandbar Crocodile — {4}{U} — Creature — Crocodile
//! Oracle: Phasing (This phases in or out before you untap during each of your untap steps. While it's phased out, it's treated as though it doesn't exist.)
//! Set: MIR #88 — Mirage | Scryfall ID: 450b79d9-5ab8-4699-8052-a278c316a5c3 | Oracle ID: e11926bf-ca14-4e1e-aca5-9ffedb91868c

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SANDBAR_CROCODILE,
    oracle_id = "e11926bf-ca14-4e1e-aca5-9ffedb91868c",
    scryfall_id = "450b79d9-5ab8-4699-8052-a278c316a5c3",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    keywords = KeywordSet::PHASING,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Sandbar Crocodile",
        mana_cost = mana!("{4}{U}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::CROCODILE],
        power = Some(6),
        toughness = Some(5),
    ),],
);
