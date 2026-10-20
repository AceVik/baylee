//! Fylgja — {W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: This Aura enters with four healing counters on it.
//! Oracle: Remove a healing counter from this Aura: Prevent the next 1 damage that would be dealt to enchanted creature this turn.
//! Oracle: {2}{W}: Put a healing counter on this Aura.
//! Set: ICE #26 — Ice Age | Scryfall ID: 3c6358a1-37f0-4b40-93d4-4f1652c38404 | Oracle ID: df0e5dd8-65e0-474c-ac32-6e0f6f16b8e6
// GENERATED STUB — implement abilities + tests, see docs/card-dsl.md.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

// NOT SUPPORTED: Prevent the next 1 damage that would be dealt to enchanted creature this turn without targeting (modeled as targeting enchanted creature).
card!(
    index = index::FYLGJA,
    oracle_id = "df0e5dd8-65e0-474c-ac32-6e0f6f16b8e6",
    scryfall_id = "3c6358a1-37f0-4b40-93d4-4f1652c38404",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage =
        Coverage::Partial("untargeted prevent next 1 damage to enchanted creature not supported"),
    faces = &[face!(
        name = "Fylgja",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: counters::HEALING,
            amount: Amount::Fixed(4),
        }],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
        ),
        activated!(
            Cost {
                mana: ManaCost::ZERO,
                parts: &[CostPart::RemoveCounterSelf {
                    kind: counters::HEALING,
                    n: 1,
                }],
            },
            &[Effect::PreventNextDamage {
                target: TargetSpec::Object(&Filter::CREATURE),
                amount: Amount::Fixed(1),
            }],
            target = Some(TargetSpec::Object(&Filter::AttachedToBySource)),
        ),
        activated!(
            cost!("{2}{W}"),
            &[Effect::AddCounter {
                kind: counters::HEALING,
                amount: Amount::Fixed(1),
            }],
        ),
    ],
);
