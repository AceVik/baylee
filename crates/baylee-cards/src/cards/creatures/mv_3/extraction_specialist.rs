//! Extraction Specialist — {2}{W} — Creature — Human Rogue
//! Oracle: Lifelink
//! Oracle: When this creature enters, return target creature card with mana value 2 or less from your graveyard to the battlefield. That creature can't attack or block for as long as you control this creature.
//! Set: SNC #12 — Streets of New Capenna | Scryfall ID: b404d6c7-0b65-4c6a-b141-9dffbeb120db | Oracle ID: 4164034a-5e59-4e40-a150-2c1000b0bd0d
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::EXTRACTION_SPECIALIST,
    oracle_id = "4164034a-5e59-4e40-a150-2c1000b0bd0d",
    scryfall_id = "b404d6c7-0b65-4c6a-b141-9dffbeb120db",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Extraction Specialist",
        mana_cost = mana!("{2}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::ROGUE],
        power = Some(3),
        toughness = Some(2),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
