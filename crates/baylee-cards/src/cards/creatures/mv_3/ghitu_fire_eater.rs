//! Ghitu Fire-Eater — {2}{R} — Creature — Human Nomad
//! Oracle: {T}, Sacrifice this creature: It deals damage equal to its power to any target.
//! Set: 7ED #184 — Seventh Edition | Scryfall ID: 0770cc34-0f38-4773-8633-6907f44436c4 | Oracle ID: 3e792338-4a21-4a06-ad94-39931118e06c
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GHITU_FIRE_EATER,
    oracle_id = "3e792338-4a21-4a06-ad94-39931118e06c",
    scryfall_id = "0770cc34-0f38-4773-8633-6907f44436c4",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Ghitu Fire-Eater",
        mana_cost = mana!("{2}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::NOMAD],
        power = Some(2),
        toughness = Some(2),
    ),],
    abilities = &[activated!(
        cost!(TapSelf, SacrificeSelf),
        &[Effect::DealDamage {
            amount: Amount::SourcePower,
            target: TargetSpec::AnyTarget,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
    )],
);
