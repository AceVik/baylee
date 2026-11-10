//! Demonic Torment — {2}{B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature can't attack.
//! Oracle: Prevent all combat damage that would be dealt by enchanted creature.
//! Set: ME3 #62 — Masters Edition III | Scryfall ID: ac439803-71cc-4139-9314-2c0fff2ef876 | Oracle ID: 2172b724-9004-488e-88d3-a5fc48c50e41
// IMPLEMENTED — enchant creature; it can't attack and all combat damage it
// would deal is prevented.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::DEMONIC_TORMENT,
    oracle_id = "2172b724-9004-488e-88d3-a5fc48c50e41",
    scryfall_id = "ac439803-71cc-4139-9314-2c0fff2ef876",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Demonic Torment",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE)
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::AddKeyword(KeywordSet::CANT_ATTACK)
        ),
        static_ability!(Filter::AttachedToBySource, Modifier::PreventDamageFromIt),
    ],
);
