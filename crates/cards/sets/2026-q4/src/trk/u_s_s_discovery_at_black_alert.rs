//! U.S.S. Discovery, at Black Alert — {3}{G}{G} — Legendary Artifact — Spacecraft
//! Oracle: Whenever U.S.S. Discovery enters or attacks, discover 5. (Exile cards from the top of your library until you exile a nonland card with mana value 5 or less. Cast it without paying its mana cost or put it into your hand. Put the rest on the bottom in a random order.)
//! Oracle: Station
//! Oracle: 6+ | Flying
//! Set: TRK #221 — Star Trek | Scryfall ID: 69b1f87f-ff02-4945-9f07-18dab5035e2d | Oracle ID: 13dbfe3f-1e04-4c4f-953c-7c949dba9d33
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::U_S_S_DISCOVERY_AT_BLACK_ALERT,
    oracle_id = "13dbfe3f-1e04-4c4f-953c-7c949dba9d33",
    scryfall_id = "69b1f87f-ff02-4945-9f07-18dab5035e2d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "U.S.S. Discovery, at Black Alert",
        mana_cost = mana!("{3}{G}{G}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(5),
        toughness = Some(5),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
