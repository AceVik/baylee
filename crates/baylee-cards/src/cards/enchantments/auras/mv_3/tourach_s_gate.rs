//! Tourach's Gate — {1}{B}{B} — Enchantment — Aura
//! Oracle: Enchant land you control
//! Oracle: Sacrifice a Thrull: Put three time counters on this Aura.
//! Oracle: At the beginning of your upkeep, remove a time counter from this Aura. If there are no time counters on this Aura, sacrifice it.
//! Oracle: Tap enchanted land: Attacking creatures you control get +2/-1 until end of turn. Activate only if enchanted land is untapped.
//! Set: FEM #48 — Fallen Empires | Scryfall ID: d77f6401-a9fb-449c-b511-6fb837055bb4 | Oracle ID: 03e27ebd-6141-4002-b41f-0bf9d6ec2908
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

const THRULL: Filter = Filter::HasSubtype(subtypes::creature::THRULL);
const ATTACKING_CREATURE: Filter = Filter::And(&[Filter::CREATURE, Filter::Attacking]);

card!(
    index = index::TOURACH_S_GATE,
    oracle_id = "03e27ebd-6141-4002-b41f-0bf9d6ec2908",
    scryfall_id = "d77f6401-a9fb-449c-b511-6fb837055bb4",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Tourach's Gate",
        mana_cost = mana!("{1}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::YOUR_LAND),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::YOUR_LAND))),
        ),
        activated!(
            cost!(Sacrifice(&THRULL)),
            &[Effect::AddCounterSource {
                kind: CounterKind::Time,
                amount: Amount::Fixed(3),
            }],
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[
                Effect::RemoveCounterSelf {
                    kind: CounterKind::Time,
                    n: 1,
                },
                Effect::IfCondition {
                    condition: Condition::Not(&Condition::CountersOnSelf(CounterKind::Time, 1)),
                    then: &[Effect::SacrificeSelf],
                    otherwise: &[],
                },
            ],
        ),
        activated!(
            cost!(TapOther(&Filter::AttachedToBySource)),
            &[Effect::PumpFilter {
                filter: &ATTACKING_CREATURE,
                controlled_by: Some(PlayerRel::You),
                power: Amount::Fixed(2),
                toughness: Amount::NegXFixed(1),
                keywords: KeywordSet::EMPTY,
                duration: Duration::UntilEndOfTurn,
            }],
        ),
    ],
);
