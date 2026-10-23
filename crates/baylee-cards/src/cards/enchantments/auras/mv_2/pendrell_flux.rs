//! Pendrell Flux — {1}{U} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: Enchanted creature has "At the beginning of your upkeep, sacrifice this creature unless you pay its mana cost."
//! Set: USG #87 — Urza's Saga | Scryfall ID: 34847e03-c529-415e-9d49-fd4647ca8892 | Oracle ID: fe902db2-3b1d-4266-88eb-307f878fc317
// PARTIAL

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static PENDRELL_FLUX_TAX: &[Effect] = &[Effect::PlayerMayPayOr {
    player: PlayerRel::You,
    mana: Amount::Fixed(1),
    effect: &Effect::SacrificeSelf,
}];

card!(
    index = index::PENDRELL_FLUX,
    oracle_id = "fe902db2-3b1d-4266-88eb-307f878fc317",
    scryfall_id = "34847e03-c529-415e-9d49-fd4647ca8892",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "paying a permanent's exact printed mana cost in Effect::PlayerMayPayOr is not supported; grants enchanted creature an upkeep trigger to sacrifice itself unless its controller pays {1}",
    ),
    faces = &[face!(
        name = "Pendrell Flux",
        mana_cost = mana!("{1}{U}"),
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
            Modifier::GrantTriggered {
                trigger: Trigger::StepBegin {
                    step: StepKind::Upkeep,
                    whose: PlayerRel::You,
                },
                effects: PENDRELL_FLUX_TAX,
                target: None,
            },
        ),
    ],
);
