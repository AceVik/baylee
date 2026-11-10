//! Energy Vortex — {3}{U}{U} — Enchantment
//! Oracle: As this enchantment enters, choose an opponent.
//! Oracle: At the beginning of your upkeep, remove all vortex counters from this enchantment.
//! Oracle: At the beginning of the chosen player's upkeep, this enchantment deals 3 damage to that player unless they pay {1} for each vortex counter on this enchantment.
//! Oracle: {X}: Put X vortex counters on this enchantment. Activate only during your upkeep.
//! Set: MIR #64 — Mirage | Scryfall ID: 87e060d7-42ed-49ab-bc5c-2f3210cbd0d1 | Oracle ID: bcb2f489-9d58-424d-96c1-a124a8d25a14
// IMPLEMENTED — chooses an opponent on entry, removes vortex counters on your upkeep, taxes or damages the chosen opponent on their upkeep, and adds X vortex counters during your upkeep.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ENERGY_VORTEX,
    oracle_id = "bcb2f489-9d58-424d-96c1-a124a8d25a14",
    scryfall_id = "87e060d7-42ed-49ab-bc5c-2f3210cbd0d1",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Energy Vortex",
        mana_cost = mana!("{3}{U}{U}"),
        types = TypeSet::ENCHANTMENT,
        enter_modifiers = &[EnterModifier::ChooseOpponent],
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::RemoveAllCountersSelf {
                kind: counters::VORTEX,
            }],
        ),
        triggered!(
            Trigger::StepBeginChosenOpponent {
                step: StepKind::Upkeep,
            },
            &[Effect::PlayerMayPayOr {
                player: PlayerRel::ActivePlayer,
                mana: Amount::CountersOnSource(counters::VORTEX),
                effect: &Effect::DealDamage {
                    amount: Amount::Fixed(3),
                    target: TargetSpec::Player(PlayerRel::ActivePlayer),
                },
            }],
        ),
        activated!(
            cost!("{X}"),
            &[Effect::AddCounter {
                kind: counters::VORTEX,
                amount: Amount::X,
            }],
            condition = Some(Condition::All(&[
                Condition::YourTurn,
                Condition::DuringStep(StepKind::Upkeep),
            ])),
        ),
    ],
);
