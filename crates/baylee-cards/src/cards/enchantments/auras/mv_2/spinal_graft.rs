//! Spinal Graft — {1}{B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +3/+3.
//! Oracle: When enchanted creature becomes the target of a spell or ability, destroy that creature. It can't be regenerated.
//! Set: TPR #119 — Tempest Remastered | Scryfall ID: ace63a68-82cc-47fb-b403-7deaeabd6b3e | Oracle ID: 70b5eca0-c61b-4066-ae5f-90948912c04e
// PARTIAL: enchants a creature, gives it +3/+3, and grants it a trigger that destroys it without regeneration when it becomes the target of a spell or ability; triggering from the Aura itself when the enchanted creature becomes the target of a spell or ability is not on Trigger.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SPINAL_GRAFT,
    oracle_id = "70b5eca0-c61b-4066-ae5f-90948912c04e",
    scryfall_id = "ace63a68-82cc-47fb-b403-7deaeabd6b3e",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "enchants a creature, gives it +3/+3, and grants it a trigger that destroys it without regeneration when it becomes the target of a spell or ability; triggering from the Aura itself when the enchanted creature becomes the target of a spell or ability is not on Trigger",
    ),
    faces = &[face!(
        name = "Spinal Graft",
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
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::ModifyPT(3, 3),
        ),
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::GrantTriggered {
                trigger: Trigger::BecomesTarget,
                effects: &[Effect::destroy_no_regen(TargetSpec::ThisObject)],
                target: None,
            },
        ),
    ],
);
