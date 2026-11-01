//! Cabal Interrogator — {1}{B} — Creature — Zombie Wizard
//! Oracle: {X}{B}, {T}: Target player reveals X cards from their hand and you choose one of them. That player discards that card. Activate only as a sorcery.
//! Set: SCG #57 — Scourge | Scryfall ID: 256a7a37-6f47-47a3-b149-5692aee8b34a | Oracle ID: 769ae101-73cc-4ac5-8f46-8af8e7de6581
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::CABAL_INTERROGATOR,
    oracle_id = "769ae101-73cc-4ac5-8f46-8af8e7de6581",
    scryfall_id = "256a7a37-6f47-47a3-b149-5692aee8b34a",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Cabal Interrogator",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ZOMBIE, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
