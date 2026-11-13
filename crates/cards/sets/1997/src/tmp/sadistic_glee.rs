//! Sadistic Glee — {B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Whenever a creature dies, put a +1/+1 counter on enchanted creature.
//! Set: TMP #153 — Tempest | Scryfall ID: d9e1959c-b87b-4e17-a0d2-0489ea79220b | Oracle ID: 2af22962-8f66-4868-a175-ad5d7f4c9d58
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SADISTIC_GLEE,
    oracle_id = "2af22962-8f66-4868-a175-ad5d7f4c9d58",
    scryfall_id = "d9e1959c-b87b-4e17-a0d2-0489ea79220b",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Sadistic Glee",
        mana_cost = mana!("{B}"),
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
            Trigger::Dies(&Filter::CREATURE),
            &[Effect::AddCounterFilter {
                filter: &Filter::AttachedToBySource,
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            }],
        ),
    ],
);
