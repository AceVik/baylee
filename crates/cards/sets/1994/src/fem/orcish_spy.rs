//! Orcish Spy — {R} — Creature — Orc Rogue
//! Oracle: {T}: Look at the top three cards of target player's library.
//! Set: 8ED #208 — Eighth Edition | Scryfall ID: 7f3948e0-8597-4927-90ce-affa949e3013 | Oracle ID: 0d058991-3751-4afe-80c9-79f0c6e10282
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ORCISH_SPY,
    oracle_id = "0d058991-3751-4afe-80c9-79f0c6e10282",
    scryfall_id = "7f3948e0-8597-4927-90ce-affa949e3013",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Orcish Spy",
        mana_cost = mana!("{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ORC, subtypes::creature::ROGUE],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        Cost::TAP,
        &[Effect::LookAtTopOf {
            player: PlayerRel::Chosen,
            count: 3,
        }],
        target = Some(TargetSpec::AnyPlayer),
    ),],
);
