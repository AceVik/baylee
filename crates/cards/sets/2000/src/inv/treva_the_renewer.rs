//! Treva, the Renewer — {3}{G}{W}{U} — Legendary Creature — Dragon
//! Oracle: Flying
//! Oracle: Whenever Treva deals combat damage to a player, you may pay {2}{W}. If you do, choose a color, then you gain 1 life for each permanent of that color.
//! Set: DDE #49 — Duel Decks: Phyrexia vs. the Coalition | Scryfall ID: 74ee66b2-5b69-461b-9590-1143adc692af | Oracle ID: 72524f3c-674b-4ebf-9526-18737b354010
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TREVA_THE_RENEWER,
    oracle_id = "72524f3c-674b-4ebf-9526-18737b354010",
    scryfall_id = "74ee66b2-5b69-461b-9590-1143adc692af",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::Blue, Color::White]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Treva, the Renewer",
        mana_cost = mana!("{3}{G}{W}{U}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::DRAGON],
        power = Some(6),
        toughness = Some(6),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
