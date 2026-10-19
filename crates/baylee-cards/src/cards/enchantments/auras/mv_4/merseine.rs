//! Merseine — {2}{U}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: This Aura enters with three net counters on it.
//! Oracle: Enchanted creature doesn't untap during its controller's untap step if this Aura has a net counter on it.
//! Oracle: Pay enchanted creature's mana cost: Remove a net counter from this Aura. Only the controller of the enchanted creature may activate this ability.
//! Set: FEM #23a — Fallen Empires | Scryfall ID: b1e96895-ef1d-44fa-b263-bce833fc3109 | Oracle ID: b6b6e085-eef8-486d-9d66-1b7f1ee7d0dc
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MERSEINE,
    oracle_id = "b6b6e085-eef8-486d-9d66-1b7f1ee7d0dc",
    scryfall_id = "b1e96895-ef1d-44fa-b263-bce833fc3109",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Merseine",
        mana_cost = mana!("{2}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
        enter_modifiers = &[EnterModifier::WithCounters {
            kind: counters::NET,
            amount: Amount::Fixed(3),
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
            Modifier::DoesNotUntap,
            condition = Some(Condition::CountersOnSelf(counters::NET, 1)),
        ),
        activated!(
            cost!(PayAttachedManaCost),
            &[Effect::RemoveCounterSelf {
                kind: counters::NET,
                n: 1,
            }],
            zone = ActivationZone::BattlefieldAttachedController,
        ),
    ],
);
