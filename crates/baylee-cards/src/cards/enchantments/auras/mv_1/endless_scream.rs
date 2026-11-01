//! Endless Scream — {X}{B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: This Aura enters with X scream counters on it.
//! Oracle: Enchanted creature gets +1/+0 for each scream counter on this Aura.
//! Set: TMP #132 — Tempest | Scryfall ID: e9474231-34c5-4563-8a61-fd1bc2693f86 | Oracle ID: cde3e29c-31fd-4dce-a008-283bbe90bfc6
// PARTIAL — Enters with X scream counters on it and gives enchanted creature +1/+0 while this Aura has a scream counter on it; scaling +1/+0 per scream counter on the Aura source is not yet modeled.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ENDLESS_SCREAM,
    oracle_id = "cde3e29c-31fd-4dce-a008-283bbe90bfc6",
    scryfall_id = "e9474231-34c5-4563-8a61-fd1bc2693f86",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "Enters with X scream counters on it and gives enchanted creature +1/+0 while this Aura has a scream counter on it; scaling +1/+0 per scream counter on the Aura source is not yet modeled."
    ),
    faces = &[face!(
        name = "Endless Scream",
        mana_cost = mana!("{X}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: counters::SCREAM,
            amount: Amount::X,
        }],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::ModifyPT(1, 0),
            condition = Some(Condition::CountersOnSelf(counters::SCREAM, 1)),
        ),
    ],
);
