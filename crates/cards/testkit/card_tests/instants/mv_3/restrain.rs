//! `cards/instants/mv_3/restrain.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "5aa66cb0-86b9-4e85-9a17-df78416d682d"
fn restrain() -> CardIndex {
    card_index("5aa66cb0-86b9-4e85-9a17-df78416d682d")
}

/// Restrain ({2}{W}): targets only an attacking creature (excluding
/// non-attackers), prevents all combat damage that creature would deal this
/// turn while other attackers still deal damage, and draws a card.
#[test]
fn restrain_prevents_target_attackers_combat_damage_and_draws() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[restrain()])
        .battlefield(
            1,
            &[
                index::HILL_GIANT,
                index::GRIZZLY_BEARS,
                index::SAVANNAH_LIONS,
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, P1);

    let giant = lk::obj(&engine, P1, index::HILL_GIANT);
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let lions = lk::obj(&engine, P1, index::SAVANNAH_LIONS);

    // Giant and Bears attack; Lions stays home.
    let _ = lk::attack(&mut engine, &[giant, bears]);
    lk::block(&mut engine, &[]);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == P0),
    );
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();

    cast_from_hand(&mut engine, P0, restrain());
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&giant) && options.contains(&bears));
    assert!(
        !options.contains(&lions),
        "non-attacking creature is not a legal target: {options:?}"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before,
        "cast 1 spell, drew 1 card -> net 0"
    );

    lk::through_combat(&mut engine);
    assert_eq!(
        lk::life(&engine, P0),
        18,
        "Giant's 3 combat damage was prevented; Bears's 2 still hit"
    );
}
