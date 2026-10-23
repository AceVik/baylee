//! `cards/creatures/mv_3/maddening_imp.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn maddening_imp() -> CardIndex {
    card_index("97dea59e-2090-4653-9ed9-86820a20605f")
}

/// Maddening Imp (`97dea59e-2090-4653-9ed9-86820a20605f`) — {2}{B} Creature — Imp:
/// "{T}: Non-Wall creatures the active player controls attack this turn if able. At the beginning of the next end step, destroy each of those creatures that didn't attack this turn. Activate only during an opponent's turn and only before combat."
#[test]
fn maddening_imp_forces_active_opponents_non_wall_creatures_to_attack() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[maddening_imp()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    // Pass priority on P1's first main phase to P0 so P0 can activate Maddening Imp before combat.
    engine.apply(P1, PlayerAction::PassPriority).unwrap();
    activate(&mut engine, P0, maddening_imp(), 0);
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    assert!(
        engine
            .apply(P1, PlayerAction::DeclareAttackers { attackers: vec![] })
            .is_err()
    );
}

/// The unsupported delayed destruction must not kill unrelated late arrivals.
#[test]
fn unsupported_destruction_does_not_kill_creatures_entering_later() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut e = lk::game(
        SEED,
        &[maddening_imp()],
        &[
            index::GRIZZLY_BEARS,
            index::LLANOWAR_ELVES,
            index::WALL_OF_WOOD,
        ],
        &[],
        &[index::HILL_GIANT],
    );
    reach_their_main_phase(&mut e, P1);
    let imp = on_battlefield(&e, P0, maddening_imp()).unwrap();
    // The activation is restricted to an opponent's precombat turn.
    let bears = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    activate(&mut e, P1, index::LLANOWAR_ELVES, 0);
    e.apply(P1, PlayerAction::PassPriority).unwrap();
    activate(&mut e, P0, maddening_imp(), 0);
    lk::settle(&mut e);
    pass_until(
        &mut e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P1),
    );
    lk::float(&mut e, P1, &[(ManaColor::Red, 4)]);
    cast_with_floating(&mut e, P1, index::HILL_GIANT);
    lk::settle(&mut e);
    lk::attackers_offered(&mut e);
    e.apply(
        P1,
        PlayerAction::DeclareAttackers {
            attackers: vec![(bears, Defender::Player(P0))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| e.state().turn.step == crate::turn::Step::End);
    lk::settle(&mut e);
    assert!(
        on_battlefield(&e, P1, index::LLANOWAR_ELVES).is_some(),
        "Partial omits the delayed destruction altogether"
    );
    assert!(
        on_battlefield(&e, P1, index::WALL_OF_WOOD).is_some(),
        "Walls were excluded"
    );
    assert!(
        on_battlefield(&e, P1, index::HILL_GIANT).is_some(),
        "late arrivals were not among those creatures"
    );
    assert!(
        on_battlefield(&e, P1, index::GRIZZLY_BEARS).is_some(),
        "the creature that attacked survives"
    );
    assert!(e.state().object(imp).is_some());
}

#[test]
fn imp_cannot_activate_on_its_controllers_turn_or_after_combat_begins() {
    use crate::engine::card_tests::legends_kit as lk;
    let mut e = lk::game(SEED, &[maddening_imp()], &[index::GRIZZLY_BEARS], &[], &[]);
    assert!(!lk::offered(&e, maddening_imp(), P0, 0));
    reach_their_main_phase(&mut e, P1);
    pass_until(&mut e, |e| {
        e.state().turn.step == crate::turn::Step::CombatBegin
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    assert!(!lk::offered(&e, maddening_imp(), P0, 0));
}
