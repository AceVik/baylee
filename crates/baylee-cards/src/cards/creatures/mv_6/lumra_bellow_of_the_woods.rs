//! Lumra, Bellow of the Woods — {4}{G}{G} — Legendary Creature — Elemental Bear
//! Oracle: Reach, vigilance
//! Oracle: Lumra's power and toughness are each equal to the number of lands you control.
//! Oracle: When Lumra enters, mill four cards. Then return all land cards from your graveyard to the battlefield tapped.
//! Set: BLB #183 — Bloomburrow | Scryfall ID: ae4f3aaf-3960-48cd-b34b-32e4ae5ae088 | Oracle ID: 97a84e9d-bfc4-4ca2-b1e8-908dba56ccdb
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LUMRA_BELLOW_OF_THE_WOODS,
    oracle_id = "97a84e9d-bfc4-4ca2-b1e8-908dba56ccdb",
    scryfall_id = "ae4f3aaf-3960-48cd-b34b-32e4ae5ae088",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Lumra, Bellow of the Woods",
        mana_cost = mana!("{4}{G}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::ELEMENTAL, subtypes::creature::BEAR],
        power = Some(0),
        toughness = Some(0),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
