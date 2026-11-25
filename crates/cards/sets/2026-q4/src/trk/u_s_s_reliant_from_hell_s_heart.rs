//! U.S.S. Reliant, From Hell's Heart — {2}{B}{B} — Legendary Artifact — Spacecraft
//! Oracle: Whenever U.S.S. Reliant enters or deals combat damage to a player, return target creature card with mana value 4 or less from your graveyard to the battlefield with a finality counter on it.
//! Oracle: Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 10+.)
//! Oracle: 10+ | Flying
//! Set: TRK #135 — Star Trek | Scryfall ID: a4c6fc55-f585-4529-b549-31d4d838c9af | Oracle ID: edaa05ba-8b15-4156-b725-1e0c418a9ae5
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::U_S_S_RELIANT_FROM_HELL_S_HEART,
    oracle_id = "edaa05ba-8b15-4156-b725-1e0c418a9ae5",
    scryfall_id = "a4c6fc55-f585-4529-b549-31d4d838c9af",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "U.S.S. Reliant, From Hell's Heart",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::ARTIFACT,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::artifact::SPACECRAFT],
        power = Some(4),
        toughness = Some(3),
    ),],
);

// TODO(card): implement abilities, see docs/card-dsl.md.
