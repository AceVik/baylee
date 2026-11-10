//! Battlefield Scrounger — {3}{G}{G} — Creature — Centaur
//! Oracle: Threshold — Put three cards from your graveyard on the bottom of your library: This creature gets +3/+3 until end of turn. Activate only once each turn and only if there are seven or more cards in your graveyard.
//! Set: DMR #150 — Dominaria Remastered | Scryfall ID: f5a9f234-c1c1-4b4b-8385-6e0ced9977f1 | Oracle ID: 92ac7531-10bb-47a2-9dd0-378d547b4ed6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BATTLEFIELD_SCROUNGER,
    oracle_id = "92ac7531-10bb-47a2-9dd0-378d547b4ed6",
    scryfall_id = "f5a9f234-c1c1-4b4b-8385-6e0ced9977f1",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Battlefield Scrounger",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::CENTAUR],
        power = Some(3),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
