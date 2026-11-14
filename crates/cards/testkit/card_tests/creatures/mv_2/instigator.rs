//! `cards/creatures/mv_2/instigator.rs`, played.
//!
//! Oracle:
//! - "{1}{B}{B}, {T}, Discard a card: Creatures target player controls attack
//!   this turn if able."

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn instigator() -> CardIndex {
    card_index("0fe1a1f7-41c9-415e-a5e3-b5ba0c0271e5")
}

#[test]
fn instigator_forces_target_players_able_creatures_to_attack_this_turn() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp(), instigator()])
        .battlefield(1, &[plains(), plains(), index::GRIZZLY_BEARS])
        .hand(0, &[forest()])
        .hand(1, &[index::SAVANNAH_LIONS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let inst = on_battlefield(&engine, P0, instigator()).unwrap();
    assert_eq!(pt(&engine, inst), (1, 1));

    // Pass to P1's first main phase; P1 casts a summoning-sick Savannah Lions.
    reach_their_main_phase(&mut engine, P1);
    let lions_hand = in_hand(&engine, P1, index::SAVANNAH_LIONS).unwrap();
    tap_all_mana(&mut engine, P1);
    cast_with_floating(&mut engine, P1, index::SAVANNAH_LIONS);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, P1, index::SAVANNAH_LIONS).is_none());
    let _ = lions_hand;

    // Wait for P1's CombatBegin where P0 holds priority, then activate Instigator on P1.
    pass_until(&mut engine, |e| {
        e.state().turn.active == P1
            && e.state().turn.step == Step::CombatBegin
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, instigator(), 0);

    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!("expected ChooseTargets, got {:?}", engine.pending());
    };
    assert!(player_options.contains(&P1));
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();

    let Pending::ChooseCards {
        options,
        prompt: ChoicePrompt::CostDiscard,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected CostDiscard, got {:?}", engine.pending());
    };
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == P1),
    );

    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();
    let lions = on_battlefield(&engine, P1, index::SAVANNAH_LIONS).unwrap();
    let Pending::ChooseAttackers {
        attackers,
        required,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("stopped on ChooseAttackers");
    };
    assert!(required.contains(&bears), "non-sick Bears must attack");
    assert!(
        !attackers.contains(&lions),
        "summoning-sick Lions is unable"
    );
    assert!(
        engine
            .apply(P1, PlayerAction::DeclareAttackers { attackers: vec![] })
            .is_err(),
        "skipping forced Bears attack is illegal"
    );
    engine
        .apply(
            P1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(P0))],
            },
        )
        .unwrap();
}
