//! Elephant Resurgence — {1}{G} — Sorcery
//! Oracle: Each player creates a green Elephant creature token. Those creatures have "This token's power and toughness are each equal to the number of creature cards in its controller's graveyard."
//! Set: PCY #113 — Prophecy | Scryfall ID: 22147f72-7ff8-40c4-9bdd-df41dce17dad | Oracle ID: e2d54c59-7722-4520-adca-cd02a0871ab2
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ELEPHANT_RESURGENCE,
    oracle_id = "e2d54c59-7722-4520-adca-cd02a0871ab2",
    scryfall_id = "22147f72-7ff8-40c4-9bdd-df41dce17dad",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Elephant Resurgence",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
