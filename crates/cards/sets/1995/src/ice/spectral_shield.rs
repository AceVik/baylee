//! Spectral Shield — {1}{W}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +0/+2 and can't be the target of spells.
//! Set: ME3 #176 — Masters Edition III | Scryfall ID: 57281028-c4ec-4875-9b15-956b153ced41 | Oracle ID: 37d9c065-598d-4ee3-abfc-625bd6169ff1

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SPECTRAL_SHIELD,
    oracle_id = "37d9c065-598d-4ee3-abfc-625bd6169ff1",
    scryfall_id = "57281028-c4ec-4875-9b15-956b153ced41",
    color_identity = ColorSet::from_slice(&[Color::Blue, Color::White]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Spectral Shield",
        mana_cost = mana!("{1}{W}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        // CR 303.4a: Enchant creature.
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        // CR 613.4c: Enchanted creature gets +0/+2.
        static_ability!(Filter::AttachedToBySource, Modifier::ModifyPT(0, 2)),
        // CR 115.1: Enchanted creature can't be the target of spells.
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::CantBeTargetedBy(&Filter::InZone(ZoneRef::Stack))
        ),
    ],
);
