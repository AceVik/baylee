//! Karona, False God — {1}{W}{U}{B}{R}{G} — Legendary Creature — Avatar
//! Oracle: Haste
//! Oracle: At the beginning of each player's upkeep, that player untaps Karona and gains control of it.
//! Oracle: Whenever Karona attacks, creatures of the creature type of your choice get +3/+3 until end of turn.
//! Set: SCG #138 — Scourge | Scryfall ID: de53d083-251e-42a4-9e2e-c2978c80615b | Oracle ID: 9ea1f10c-5ceb-4ac0-84c6-29046dea649a
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::KARONA_FALSE_GOD,
    oracle_id = "9ea1f10c-5ceb-4ac0-84c6-29046dea649a",
    scryfall_id = "de53d083-251e-42a4-9e2e-c2978c80615b",
    color_identity = ColorSet::from_slice(&[
        Color::Black,
        Color::Green,
        Color::Red,
        Color::Blue,
        Color::White
    ]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Karona, False God",
        mana_cost = mana!("{1}{W}{U}{B}{R}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::AVATAR],
        power = Some(5),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
