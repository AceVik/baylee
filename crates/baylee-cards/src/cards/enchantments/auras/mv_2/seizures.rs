//! Seizures — {1}{B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Whenever enchanted creature becomes tapped, this Aura deals 3 damage to that creature's controller unless that player pays {3}.
//! Set: ICE #159 — Ice Age | Scryfall ID: da369c86-7e17-43d8-b626-b6842e3d2d50 | Oracle ID: 550921f5-ec65-460d-9ada-f2c5a4c4b2c4
use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SEIZURES,
    oracle_id = "550921f5-ec65-460d-9ada-f2c5a4c4b2c4",
    scryfall_id = "da369c86-7e17-43d8-b626-b6842e3d2d50",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Seizures",
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
            Trigger::BecomesTapped(&Filter::AttachedToBySource),
            &[Effect::PlayerMayPayOr {
                player: PlayerRel::ControllerOfEvent,
                mana: Amount::Fixed(3),
                effect: &Effect::DealDamage {
                    amount: Amount::Fixed(3),
                    target: TargetSpec::Player(PlayerRel::ControllerOfEvent),
                },
            }],
        ),
    ],
);
