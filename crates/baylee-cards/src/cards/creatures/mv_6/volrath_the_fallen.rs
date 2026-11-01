//! Volrath the Fallen — {3}{B}{B}{B} — Legendary Creature — Phyrexian Shapeshifter
//! Oracle: {1}{B}, Discard a creature card: Volrath gets +X/+X until end of turn, where X is the discarded card's mana value.
//! Set: NEM #75 — Nemesis | Scryfall ID: 08bdd66e-9ca1-456e-a61c-7c96cf6f7c56 | Oracle ID: 4e11c838-68a7-4c97-b064-a46833cf1d2f
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::VOLRATH_THE_FALLEN,
    oracle_id = "4e11c838-68a7-4c97-b064-a46833cf1d2f",
    scryfall_id = "08bdd66e-9ca1-456e-a61c-7c96cf6f7c56",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Volrath the Fallen",
        mana_cost = mana!("{3}{B}{B}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[
            subtypes::creature::PHYREXIAN,
            subtypes::creature::SHAPESHIFTER
        ],
        power = Some(6),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
