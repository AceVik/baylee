//! Funeral March — {1}{B}{B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: When enchanted creature leaves the battlefield, its controller sacrifices a creature of their choice.
//! Set: ME2 #91 — Masters Edition II | Scryfall ID: 376e1f71-47a0-4a94-90c6-4fef0e9f0026 | Oracle ID: 4d8afa5c-cef3-4e67-bd4c-ebd5c68b96a7
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FUNERAL_MARCH,
    oracle_id = "4d8afa5c-cef3-4e67-bd4c-ebd5c68b96a7",
    scryfall_id = "376e1f71-47a0-4a94-90c6-4fef0e9f0026",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Funeral March",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        triggered!(
            Trigger::LeavesBattlefield(&Filter::AttachedToBySource),
            &[Effect::SacrificeFilter {
                who: PlayerRel::ControllerOfEvent,
                filter: &Filter::CREATURE,
            }]
        ),
    ],
);
