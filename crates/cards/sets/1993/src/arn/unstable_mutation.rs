//! Unstable Mutation — {U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +3/+3.
//! Oracle: At the beginning of the upkeep of enchanted creature's controller, put a -1/-1 counter on that creature.
//! Set: UMA #80 — Ultimate Masters | Scryfall ID: da66b93a-11a6-4861-a775-d309b7adc461 | Oracle ID: 278b237e-9699-43eb-a03e-0b68eccc08b3
// IMPLEMENTED — enchant creature; +3/+3 to the enchanted creature; and a
// -1/-1 counter on it at the beginning of its controller's upkeep.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::UNSTABLE_MUTATION,
    oracle_id = "278b237e-9699-43eb-a03e-0b68eccc08b3",
    scryfall_id = "da66b93a-11a6-4861-a775-d309b7adc461",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Unstable Mutation",
        mana_cost = mana!("{U}"),
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
        static_ability!(Filter::AttachedToBySource, Modifier::ModifyPT(3, 3)),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::ControllerOfAttached
            },
            &[Effect::AddCounterFilter {
                filter: &Filter::AttachedToBySource,
                kind: CounterKind::M1M1,
                amount: Amount::Fixed(1),
            }]
        ),
    ],
);
