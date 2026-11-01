//! Legacy Weapon — {7} — Legendary Artifact
//! Oracle: {W}{U}{B}{R}{G}: Exile target permanent.
//! Oracle: If Legacy Weapon would be put into a graveyard from anywhere, reveal Legacy Weapon and shuffle it into its owner's library instead.
//! Set: DMR #229 — Dominaria Remastered | Scryfall ID: afd35d5c-7f28-4842-85e8-692d6c391c3a | Oracle ID: aa1a248d-2f76-4f15-b065-08f299af07b9
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LEGACY_WEAPON,
    oracle_id = "aa1a248d-2f76-4f15-b065-08f299af07b9",
    scryfall_id = "afd35d5c-7f28-4842-85e8-692d6c391c3a",
    color_identity = ColorSet::from_slice(&[
        Color::Black,
        Color::Green,
        Color::Red,
        Color::Blue,
        Color::White
    ]),
    faces = &[face!(
        name = "Legacy Weapon",
        mana_cost = mana!("{7}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
