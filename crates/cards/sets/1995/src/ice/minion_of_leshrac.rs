//! Minion of Leshrac — {4}{B}{B}{B} — Creature — Demon Minion
//! Oracle: Protection from black
//! Oracle: At the beginning of your upkeep, this creature deals 5 damage to you unless you sacrifice a creature other than this creature. If this creature deals damage to you this way, tap it.
//! Oracle: {T}: Destroy target creature or land.
//! Set: ME2 #104 — Masters Edition II | Scryfall ID: a4bbed56-50c9-4a57-b0f2-ddf4a94fbdb8 | Oracle ID: 8af3b981-6eda-46b0-a7b8-226bc719d137
// IMPLEMENTED — protection from black, upkeep sacrifice-another-creature-or-take-5-and-tap trigger, and {T} destroy creature or land.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BLACK: Filter = Filter::HasColor(ColorSet::from_slice(&[Color::Black]));
static CREATURE_OR_LAND: Filter = Filter::HasType(TypeSet::CREATURE.union(TypeSet::LAND));

card!(
    index = index::MINION_OF_LESHRAC,
    oracle_id = "8af3b981-6eda-46b0-a7b8-226bc719d137",
    scryfall_id = "a4bbed56-50c9-4a57-b0f2-ddf4a94fbdb8",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Minion of Leshrac",
        mana_cost = mana!("{4}{B}{B}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::DEMON, subtypes::creature::MINION],
        power = Some(5),
        toughness = Some(5),
    ),],
    abilities = &[
        static_ability!(Filter::This, Modifier::ProtectionFrom(&BLACK)),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[
                Effect::PlayerMayPayCostOr {
                    player: PlayerRel::You,
                    cost: &CostPart::Sacrifice(&Filter::ANOTHER_CREATURE_YOU_CONTROL),
                    effect: &Effect::DealDamage {
                        amount: Amount::Fixed(5),
                        target: TargetSpec::Player(PlayerRel::You),
                    },
                },
                Effect::IfCondition {
                    condition: Condition::AmountAtLeast(
                        Amount::ThisWay(ThisWay::DamageDealtToYou),
                        1,
                    ),
                    then: &[Effect::TapSelf],
                    otherwise: &[],
                },
            ],
        ),
        activated!(
            Cost::TAP,
            &[Effect::destroy(TargetSpec::Object(&CREATURE_OR_LAND))],
            target = Some(TargetSpec::Object(&CREATURE_OR_LAND)),
        ),
    ],
);
