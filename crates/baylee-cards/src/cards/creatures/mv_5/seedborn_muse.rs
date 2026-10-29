//! Seedborn Muse — {3}{G}{G} — Creature — Spirit
//! Oracle: Untap all permanents you control during each other player's untap step.
//! Set: TDC #268 — Tarkir: Dragonstorm Commander | Scryfall ID: 76f58441-5674-4f1e-af9b-94e14ab7e619 | Oracle ID: 463865bc-087e-477b-9e86-84e77f1ad931
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SEEDBORN_MUSE,
    oracle_id = "463865bc-087e-477b-9e86-84e77f1ad931",
    scryfall_id = "76f58441-5674-4f1e-af9b-94e14ab7e619",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Seedborn Muse",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::SPIRIT],
        power = Some(2),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
