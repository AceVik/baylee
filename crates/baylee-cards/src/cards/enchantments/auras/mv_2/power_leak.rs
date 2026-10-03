//! Power Leak — {1}{U} — Enchantment — Aura
//! Oracle: Enchant enchantment
//! Oracle: At the beginning of the upkeep of enchanted enchantment's controller, that player may pay any amount of mana. This Aura deals 2 damage to that player. Prevent X of that damage, where X is the amount of mana that player paid this way.
//! Set: 4ED #92 — Fourth Edition | Scryfall ID: 99f235d7-8f6b-4d17-bc36-6f2cb6d5deec | Oracle ID: dc2f0000-870b-487f-9623-618fc8eb9765

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::POWER_LEAK,
    oracle_id = "dc2f0000-870b-487f-9623-618fc8eb9765",
    scryfall_id = "99f235d7-8f6b-4d17-bc36-6f2cb6d5deec",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Power Leak",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::ENCHANTMENT)
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::ENCHANTMENT)))
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::ControllerOfAttached
            },
            &[Effect::PayManaToPreventDamage {
                // This upkeep's player remains the affected player when the
                // Aura moves, leaves, or changes controller in response.
                player: PlayerRel::ActivePlayer,
                amount: Amount::Fixed(2),
            }],
        ),
    ],
);
