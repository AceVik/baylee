//! `cards/creatures/mv_1/elvish_berserker.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn elvish_berserker() -> CardIndex {
    card_index("c9046281-dedb-4246-8e5d-7786fe0c41c7")
}

/// Elvish Berserker (`c9046281-dedb-4246-8e5d-7786fe0c41c7`) — {G} Creature — Elf Berserker (Implemented):
/// "Whenever this creature becomes blocked, it gets +1/+1 until end of turn for each creature blocking it."
#[test]
fn elvish_berserker_gets_plus_1_plus_1_for_each_creature_blocking_it() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[elvish_berserker()])
        .battlefield(1, &[index::WALL_OF_STONE, index::WALL_OF_WOOD])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let berserker = on_battlefield(&engine, P0, elvish_berserker()).unwrap();
    let w1 = on_battlefield(&engine, P1, index::WALL_OF_STONE).unwrap();
    let w2 = on_battlefield(&engine, P1, index::WALL_OF_WOOD).unwrap();

    kit::declare_attack(&mut engine, &[berserker], P1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(w1, berserker), (w2, berserker)],
            },
        )
        .unwrap();
    kit::resolve_the_stack(&mut engine, |_, _| None);

    assert_eq!(pt(&engine, berserker), (3, 3));
    kit::drive(
        &mut engine,
        |e| {
            e.state().turn.active == P1
                && e.state().turn.phase == Phase::FirstMain
                && matches!(e.pending(), Pending::Priority { player, .. } if *player == P1)
        },
        |_, pending| match pending {
            Pending::ChooseNumber {
                max,
                reason: crate::choice::NumberPrompt::CombatDamage { .. },
                ..
            } => Some(PlayerAction::ChooseNumber(*max)),
            _ => None,
        },
    );
    assert_eq!(pt(&engine, berserker), (1, 1));
}
