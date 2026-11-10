//! Essence Flare — {U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +2/+0.
//! Oracle: At the beginning of the upkeep of enchanted creature's controller, put a -0/-1 counter on that creature.
//! Set: ME2 #48 — Masters Edition II | Scryfall ID: 388ab202-e044-4059-84c9-a995c66f053c | Oracle ID: 9bdc79c9-c8b0-4db1-89d9-0ca7920b6576
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const M0M1: CounterKind = CounterKind::Minus {
    power: 0,
    toughness: 1,
};

card!(
    index = index::ESSENCE_FLARE,
    oracle_id = "9bdc79c9-c8b0-4db1-89d9-0ca7920b6576",
    scryfall_id = "388ab202-e044-4059-84c9-a995c66f053c",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Essence Flare",
        mana_cost = mana!("{U}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        static_ability!(Filter::AttachedToBySource, Modifier::ModifyPT(2, 0)),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::ControllerOfAttached,
            },
            // "That creature": the one this Aura enchanted, even if the Aura
            // has left the battlefield before the trigger resolves (its last
            // attachment), which `AddCounterFilter(AttachedToBySource)` does
            // not find.
            &[Effect::ChangeAttachedCounters {
                kind: M0M1,
                amount: Amount::Fixed(1),
                remove: false,
            }],
        ),
    ],
);
