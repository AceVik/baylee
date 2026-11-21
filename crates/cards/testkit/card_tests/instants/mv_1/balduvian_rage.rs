//! `cards/instants/mv_1/balduvian_rage.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;
use baylee_engine::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "1bc7d73b-0f54-4807-b13a-3def0c5e5616"
fn balduvian_rage() -> CardIndex {
    card_index("1bc7d73b-0f54-4807-b13a-3def0c5e5616")
}

/// Balduvian Rage ({X}{R}): targets ONLY an attacking creature (excluding
/// non-attacking creatures), pumps +X/+0 until end of turn, does not draw
/// immediately, and draws 1 card at the beginning of the next turn's upkeep.
#[test]
fn balduvian_rage_pumps_attacking_creature_by_x_and_draws_next_upkeep() {
    let mut engine = lk::game(
        SEED,
        &[index::GRIZZLY_BEARS, index::HILL_GIANT],
        &[],
        &[balduvian_rage()],
        &[],
    );
    let bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let giant = lk::obj(&engine, P0, index::HILL_GIANT);
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);

    // Attack with Bears only; Hill Giant stays back.
    let _ = lk::attack(&mut engine, &[bears]);
    lk::block(&mut engine, &[]);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(P0)).len();

    lk::float(&mut engine, P0, &[(ManaColor::Red, 4)]);
    cast_with_floating(&mut engine, P0, balduvian_rage());
    engine.apply(P0, PlayerAction::ChooseNumber(3)).unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&bears));
    assert!(
        !options.contains(&giant),
        "non-attacking creature is excluded: {options:?}"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![bears],
                players: vec![],
            },
        )
        .unwrap();
    lk::settle(&mut engine);

    assert_eq!(pt(&engine, bears), (5, 2), "2/2 + 3/0 = 5/2");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before - 1,
        "does not draw immediately"
    );

    lk::through_combat(&mut engine);
    assert_eq!(lk::life(&engine, P1), 15, "5/2 Bears dealt 5 combat damage");

    pass_until(&mut engine, |e| {
        e.state().turn.active == P1 && e.state().turn.step == Step::Upkeep
    });
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bears), (2, 2), "+3/+0 wore off at EOT");
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(P0)).len(),
        hand_before,
        "drew 1 card at the beginning of next turn's upkeep"
    );
}
