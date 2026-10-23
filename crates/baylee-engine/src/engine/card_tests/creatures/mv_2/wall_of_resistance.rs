//! `cards/creatures/mv_2/wall_of_resistance.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn wall_of_resistance() -> CardIndex {
    card_index("e6f5f80c-2620-4558-8a5f-8fc36cea4793")
}

fn shock() -> CardIndex {
    baylee_core::generated::index::SHOCK
}

/// Wall of Resistance (`{1}{W}`, `Coverage::Implemented`): 0/3 Wall with
/// defender and flying; at the beginning of each end step, if it was dealt
/// damage this turn, put a +0/+1 counter on it.
#[test]
fn wall_of_resistance_gets_plus_zero_plus_one_counter_at_end_step_if_dealt_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[wall_of_resistance(), mountain()])
        .hand(0, &[shock()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let wall = on_battlefield(&engine, p0, wall_of_resistance()).unwrap();
    assert!(keywords(&engine, wall).contains(KeywordSet::DEFENDER));
    assert!(keywords(&engine, wall).contains(KeywordSet::FLYING));
    assert_eq!(pt(&engine, wall), (0, 3));

    // Deal 2 damage to Wall of Resistance with Shock.
    cast_from_hand(&mut engine, p0, shock());
    aim_at(&mut engine, p0, wall);
    pass_until(&mut engine, stack_is_empty);

    // At p0's end step, Wall of Resistance gets a +0/+1 counter -> 0/4.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(pt(&engine, wall), (0, 4));
    // A whole turn cycle without damage adds nothing.
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(pt(&engine, wall), (0, 4), "not dealt damage, no counter");
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Wall of Resistance — "each end step": damage taken blocking on the
/// opponent's turn earns the counter at that turn's end step too.
#[test]
fn wall_of_resistance_gets_its_counter_at_the_opponents_end_step_too() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[wall_of_resistance()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let wall = on_battlefield(&engine, p0, wall_of_resistance()).unwrap();
    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();

    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bear, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(wall, bear)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::CombatEnd
    });
    assert_eq!(pt(&engine, wall), (0, 3), "damaged, counter not yet earned");

    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, wall),
        (0, 4),
        "the opponent's end step put the counter on"
    );
}
