//! Atraxa, Grand Unifier — {3}{G}{W}{U}{B} — Legendary Creature — Phyrexian Angel
//! Oracle: Flying, vigilance, deathtouch, lifelink
//! Oracle: When Atraxa enters, reveal the top ten cards of your library. For each card type, you may put a card of that type from among the revealed cards into your hand. Put the rest on the bottom of your library in a random order. (Artifact, battle, creature, enchantment, instant, land, planeswalker, and sorcery are card types.)
//! Set: ONE #196 — Phyrexia: All Will Be One | Scryfall ID: 4a1f905f-1d55-4d02-9d24-e58070793d3f | Oracle ID: abbcb153-0763-44c6-964f-b4ff0eb64257
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ATRAXA_GRAND_UNIFIER,
    oracle_id = "abbcb153-0763-44c6-964f-b4ff0eb64257",
    scryfall_id = "4a1f905f-1d55-4d02-9d24-e58070793d3f",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green, Color::Blue, Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Atraxa, Grand Unifier",
        mana_cost = mana!("{3}{G}{W}{U}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::PHYREXIAN, subtypes::creature::ANGEL],
        power = Some(7),
        toughness = Some(7),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
