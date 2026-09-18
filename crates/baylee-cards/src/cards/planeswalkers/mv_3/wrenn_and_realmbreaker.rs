//! Wrenn and Realmbreaker — {1}{G}{G} — Legendary Planeswalker — Wrenn
//! Oracle: Lands you control have "{T}: Add one mana of any color."
//! Oracle: +1: Up to one target land you control becomes a 3/3 Elemental creature with vigilance, hexproof, and haste until your next turn. It's still a land.
//! Oracle: −2: Mill three cards. You may put a permanent card from among the milled cards into your hand.
//! Oracle: −7: You get an emblem with "You may play lands and cast permanent spells from your graveyard."
//! Set: MOM #217 — March of the Machine | Scryfall ID: 6f807d91-b157-44e8-a431-49782184f876 | Oracle ID: 4566fb92-448e-4b3f-9045-9d74323c35d1
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::WRENN_AND_REALMBREAKER,
    oracle_id = "4566fb92-448e-4b3f-9045-9d74323c35d1",
    scryfall_id = "6f807d91-b157-44e8-a431-49782184f876",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    faces = &[face!(
        name = "Wrenn and Realmbreaker",
        mana_cost = mana!("{1}{G}{G}"),
        types = TypeSet::PLANESWALKER,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::planeswalker::WRENN],
        loyalty = Some(4),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
