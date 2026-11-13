//! Midnight Ritual — {X}{2}{B} — Sorcery
//! Oracle: Exile X target creature cards from your graveyard. For each creature card exiled this way, create a 2/2 black Zombie creature token.
//! Set: 10E #158 — Tenth Edition | Scryfall ID: 4239c4e8-41f3-47fd-bd01-fa0b28976e29 | Oracle ID: 389ad507-8a14-4856-8d19-c90e3104aca8
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::MIDNIGHT_RITUAL,
    oracle_id = "389ad507-8a14-4856-8d19-c90e3104aca8",
    scryfall_id = "4239c4e8-41f3-47fd-bd01-fa0b28976e29",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Midnight Ritual",
        mana_cost = mana!("{X}{2}{B}"),
        types = TypeSet::SORCERY,
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
