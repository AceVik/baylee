//! `cards/creatures/artifacts/mv_4/snare_thopter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn snare_thopter() -> CardIndex {
    card_index("a3be0e16-6063-4cd0-87cb-606591a958c2")
}

fn gilded_sentinel() -> CardIndex {
    card_index("a382165d-d715-4836-b1ce-9e5d8dc56fb4")
}

/// Snare Thopter — {4} 3/2 with flying and haste. Cast the turn it is
/// needed, beside a Gilded Sentinel that has neither: only the Thopter is on
/// the attackers list, and it flies through a Grizzly Bears for 3.
#[test]
fn snare_thopter_attacks_the_turn_it_is_cast() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(); 8])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[snare_thopter(), gilded_sentinel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, snare_thopter());
    pass_until(&mut engine, stack_is_empty);
    cast_with_floating(&mut engine, p0, gilded_sentinel());
    pass_until(&mut engine, stack_is_empty);
    let thopter = on_battlefield(&engine, p0, snare_thopter()).expect("it resolved");
    let golem = on_battlefield(&engine, p0, gilded_sentinel()).expect("it resolved");
    assert_eq!(pt(&engine, thopter), (3, 2));
    assert!(keywords(&engine, thopter).contains(KeywordSet::FLYING));
    assert!(keywords(&engine, thopter).contains(KeywordSet::HASTE));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for this");
    };
    assert!(attackers.contains(&thopter), "haste: it may attack now");
    assert!(
        !attackers.contains(&golem),
        "the control: the Sentinel is summoning sick"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(thopter, Defender::Player(p1))],
            },
        )
        .expect("the Thopter attacks");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let Pending::ChooseBlockers { blockers, .. } = engine.pending().clone() else {
        unreachable!("the pass waited for this");
    };
    assert!(
        blockers.iter().all(|b| !b.attackers.contains(&thopter)),
        "flying: the Bears cannot block it"
    );
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("no blocks");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].life, 17);
}
