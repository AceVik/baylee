//! Uro, Titan of Nature's Wrath — {1}{G}{U} — Legendary Creature — Elder Giant
//! Oracle: When Uro enters, sacrifice it unless it escaped.
//! Oracle: Whenever Uro enters or attacks, you gain 3 life and draw a card, then you may put a land card from your hand onto the battlefield.
//! Oracle: Escape—{G}{G}{U}{U}, Exile five other cards from your graveyard. (You may cast this card from your graveyard for its escape cost.)
//! Set: THB #229 — Theros Beyond Death | Scryfall ID: a0b6a71e-56cb-4d25-8f2b-7a4f1b60900d | Oracle ID: ee302659-59ed-4eef-babe-451b9ccf7f14
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::URO_TITAN_OF_NATURE_S_WRATH,
    oracle_id = "ee302659-59ed-4eef-babe-451b9ccf7f14",
    scryfall_id = "a0b6a71e-56cb-4d25-8f2b-7a4f1b60900d",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Uro, Titan of Nature's Wrath",
        mana_cost = mana!("{1}{G}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::ELDER, subtypes::creature::GIANT],
        power = Some(6),
        toughness = Some(6),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
