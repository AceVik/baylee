//! Genesis — {4}{G} — Creature — Incarnation
//! Oracle: At the beginning of your upkeep, if this creature is in your graveyard, you may pay {2}{G}. If you do, return target creature card from your graveyard to your hand.
//! Set: MH1 #166 — Modern Horizons | Scryfall ID: 9b741d88-513a-4f47-a4fd-af42c5e44b6d | Oracle ID: 15afdd88-05fc-4335-8a55-c6ee702f3bef
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GENESIS,
    oracle_id = "15afdd88-05fc-4335-8a55-c6ee702f3bef",
    scryfall_id = "9b741d88-513a-4f47-a4fd-af42c5e44b6d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Genesis",
        mana_cost = mana!("{4}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::INCARNATION],
        power = Some(4),
        toughness = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
