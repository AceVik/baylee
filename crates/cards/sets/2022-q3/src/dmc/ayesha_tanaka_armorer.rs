//! Ayesha Tanaka, Armorer — {3}{W}{U} — Legendary Creature — Human Artificer
//! Oracle: Whenever Ayesha Tanaka attacks, look at the top four cards of your library. You may put any number of artifact cards with mana value less than or equal to Ayesha Tanaka's power from among them onto the battlefield tapped. Put the rest on the bottom of your library in a random order.
//! Oracle: Ayesha Tanaka can't be blocked as long as defending player controls three or more artifacts.
//! Set: DMC #29 — Dominaria United Commander | Scryfall ID: dab74812-89c9-42e2-8b9d-bf5b35a7cf7e | Oracle ID: 59fcd09c-4633-457e-b36a-488e94c1c941
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::AYESHA_TANAKA_ARMORER,
    oracle_id = "59fcd09c-4633-457e-b36a-488e94c1c941",
    scryfall_id = "dab74812-89c9-42e2-8b9d-bf5b35a7cf7e",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Ayesha Tanaka, Armorer",
        mana_cost = mana!("{3}{W}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::ARTIFICER],
        power = Some(2),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
