//! Leovold, Emissary of Trest — {B}{G}{U} — Legendary Creature — Elf Advisor
//! Oracle: Each opponent can't draw more than one card each turn.
//! Oracle: Whenever you or a permanent you control becomes the target of a spell or ability an opponent controls, you may draw a card.
//! Set: UMA #202 — Ultimate Masters | Scryfall ID: cedfc5b7-9242-4680-b284-debc8b5a9bc7 | Oracle ID: d5d91377-fd66-4dbe-a092-07f2ea379ca7
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LEOVOLD_EMISSARY_OF_TREST,
    oracle_id = "d5d91377-fd66-4dbe-a092-07f2ea379ca7",
    scryfall_id = "cedfc5b7-9242-4680-b284-debc8b5a9bc7",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green, Color::Blue]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Leovold, Emissary of Trest",
        mana_cost = mana!("{B}{G}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::ELF, subtypes::creature::ADVISOR],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
