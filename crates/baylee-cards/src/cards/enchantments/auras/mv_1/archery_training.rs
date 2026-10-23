//! Archery Training — {W} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: At the beginning of your upkeep, you may put an arrow counter on this Aura.
//! Oracle: Enchanted creature has "{T}: This creature deals X damage to target attacking or blocking creature, where X is the number of arrow counters on Archery Training."
//! Set: UDS #2 — Urza's Destiny | Scryfall ID: 151232e6-68cc-4cac-a532-9ade8e925961 | Oracle ID: acab4fa9-d9ab-4a89-a50f-0cee9d7ca0c3
// PARTIAL: puts an arrow counter on Archery Training at your upkeep and grants enchanted creature '{T}: deal 1 damage to target attacking or blocking creature'; reading the number of arrow counters on the granting Aura from a granted ability is not in Amount.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ATTACKING_OR_BLOCKING_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Or(&[Filter::Attacking, Filter::Blocking]),
]);

static ARCHERY_TRAINING_SHOT: AbilityDef = activated!(
    Cost::TAP,
    &[Effect::DealDamage {
        amount: Amount::Fixed(1),
        target: TargetSpec::Object(&ATTACKING_OR_BLOCKING_CREATURE),
    }],
    targets = Some(TargetReq::one(TargetSpec::Object(
        &ATTACKING_OR_BLOCKING_CREATURE,
    ))),
);

card!(
    index = index::ARCHERY_TRAINING,
    oracle_id = "acab4fa9-d9ab-4a89-a50f-0cee9d7ca0c3",
    scryfall_id = "151232e6-68cc-4cac-a532-9ade8e925961",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "puts an arrow counter on Archery Training at your upkeep and grants enchanted creature '{T}: deal 1 damage to target attacking or blocking creature'; reading the number of arrow counters on the granting Aura from a granted ability is not in Amount",
    ),
    faces = &[face!(
        name = "Archery Training",
        mana_cost = mana!("{W}"),
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
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::MayDo {
                effects: &[Effect::AddCounter {
                    kind: counters::ARROW,
                    amount: Amount::Fixed(1),
                }],
            }],
        ),
        static_ability!(
            Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]),
            Modifier::GrantActivatedDefinition(&ARCHERY_TRAINING_SHOT),
        ),
    ],
);
