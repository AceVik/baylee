//! Binding Agony — {1}{B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Whenever enchanted creature is dealt damage, this Aura deals that much damage to that creature's controller.
//! Set: MIR #106 — Mirage | Scryfall ID: b620a9f4-358d-4436-85b8-b0c16602ff57 | Oracle ID: dcd41ed1-4a12-416b-8ba0-51a9c0927ddb
// PARTIAL — deals 1 damage to the enchanted creature's controller each time it is dealt damage, not that much damage.
// NOT SUPPORTED: "this Aura deals that much damage to that creature's controller." A trigger on damage dealt to a permanent does not carry the amount (the journal reads the amount only for damage to a player and for Trigger::DealsDamage), so the Aura deals a fixed 1: less than printed whenever the creature is dealt 2 or more.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::BINDING_AGONY,
    oracle_id = "dcd41ed1-4a12-416b-8ba0-51a9c0927ddb",
    scryfall_id = "b620a9f4-358d-4436-85b8-b0c16602ff57",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "enchants target creature and triggers when enchanted creature is dealt damage to deal 1 damage to its controller; dealing damage equal to the exact amount of damage dealt to the enchanted creature is not supported",
    ),
    faces = &[face!(
        name = "Binding Agony",
        mana_cost = mana!("{1}{B}"),
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
            Trigger::DealtDamage(&Filter::AttachedToBySource),
            &[Effect::DealDamage {
                target: TargetSpec::Player(PlayerRel::ControllerOfAttached),
                amount: Amount::Fixed(1),
            }],
        ),
    ],
);
