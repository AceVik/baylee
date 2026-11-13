//! Tayam, Luminous Enigma — {1}{W}{B}{G} — Legendary Creature — Nightmare Beast
//! Oracle: Each other creature you control enters with an additional vigilance counter on it.
//! Oracle: {3}, Remove three counters from among creatures you control: Mill three cards, then return a permanent card with mana value 3 or less from your graveyard to the battlefield.
//! Set: C20 #16 — Commander 2020 | Scryfall ID: 05b837a2-5773-4340-87f9-b4d6a43deb27 | Oracle ID: 84be8354-0bfa-4df1-868a-13197bd49191
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TAYAM_LUMINOUS_ENIGMA,
    oracle_id = "84be8354-0bfa-4df1-868a-13197bd49191",
    scryfall_id = "05b837a2-5773-4340-87f9-b4d6a43deb27",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green, Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Tayam, Luminous Enigma",
        mana_cost = mana!("{1}{W}{B}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::NIGHTMARE, subtypes::creature::BEAST],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
