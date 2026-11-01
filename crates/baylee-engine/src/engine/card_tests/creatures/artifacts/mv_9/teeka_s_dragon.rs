//! `cards/creatures/artifacts/mv_9/teeka_s_dragon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn teeka_s_dragon() -> CardIndex {
    card_index("6fb6e37e-d774-4a50-8ea3-b955535c2065")
}

/// Teeka's Dragon (`{9}`, `Coverage::Implemented`): 5/5 Artifact Creature —
/// Dragon with flying, trample, and rampage 4 (+4/+4 for each creature blocking
/// it beyond the first).
#[test]
fn teeka_s_dragon_has_flying_trample_and_rampage_4() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[teeka_s_dragon()])
        .battlefield(1, &[wind_drake(), wind_drake()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let dragon = on_battlefield(&engine, p0, teeka_s_dragon()).unwrap();
    assert_eq!(pt(&engine, dragon), (5, 5));
    assert!(keywords(&engine, dragon).contains(KeywordSet::FLYING));
    assert!(keywords(&engine, dragon).contains(KeywordSet::TRAMPLE));

    let drakes = all_on_battlefield(&engine, p1, wind_drake());

    // Attack p1 and get double-blocked by two Wind Drakes -> rampage 4 triggers (+4/+4 -> 9/9).
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(dragon, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(drakes[0], dragon), (drakes[1], dragon)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, dragon), (9, 9));
}

/// Teeka's Dragon — rampage 4 counts the creatures blocking it "beyond the
/// first": one blocker gives nothing.
#[test]
fn teeka_s_dragon_gets_no_bonus_from_a_single_blocker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[teeka_s_dragon()])
        .battlefield(1, &[wind_drake(), wind_drake()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let dragon = on_battlefield(&engine, p0, teeka_s_dragon()).unwrap();
    let drakes = all_on_battlefield(&engine, p1, wind_drake());

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(dragon, Defender::Player(p1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(drakes[0], dragon)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, dragon),
        (5, 5),
        "one blocker is not beyond the first"
    );
}
