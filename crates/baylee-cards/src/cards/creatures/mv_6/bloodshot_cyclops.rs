//! Bloodshot Cyclops — {5}{R} — Creature — Cyclops Giant
//! Oracle: {T}, Sacrifice a creature: This creature deals damage equal to the sacrificed creature's power to any target.
//! Set: 8ED #179 — Eighth Edition | Scryfall ID: b021f1ea-4db1-40fb-a5dc-af13be53ad15 | Oracle ID: 84d47913-53ac-4d4d-bac3-bc0306d43e12
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BLOODSHOT_CYCLOPS,
    oracle_id = "84d47913-53ac-4d4d-bac3-bc0306d43e12",
    scryfall_id = "b021f1ea-4db1-40fb-a5dc-af13be53ad15",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Bloodshot Cyclops",
        mana_cost = mana!("{5}{R}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::CYCLOPS, subtypes::creature::GIANT],
        power = Some(4),
        toughness = Some(4),
    ),],
    abilities = &[activated!(
        cost!(TapSelf, Sacrifice(&Filter::YOUR_CREATURE)),
        &[Effect::DealDamage {
            amount: Amount::SacrificedPower,
            target: TargetSpec::AnyTarget,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
    )],
);
