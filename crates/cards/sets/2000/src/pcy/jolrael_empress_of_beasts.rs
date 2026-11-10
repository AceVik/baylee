//! Jolrael, Empress of Beasts — {3}{G}{G} — Legendary Creature — Human Spellshaper
//! Oracle: {2}{G}, {T}, Discard two cards: All lands target player controls become 3/3 creatures until end of turn. They're still lands.
//! Set: TSB #81 — Time Spiral Timeshifted | Scryfall ID: 01084157-60e5-4693-bbec-be60b0b4e04f | Oracle ID: 7972d118-c105-4b33-8c3b-ec85da538471
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::JOLRAEL_EMPRESS_OF_BEASTS,
    oracle_id = "7972d118-c105-4b33-8c3b-ec85da538471",
    scryfall_id = "01084157-60e5-4693-bbec-be60b0b4e04f",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Jolrael, Empress of Beasts",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::SPELLSHAPER],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
