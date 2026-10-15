//! `cards/creatures/artifacts/mv_4/venomcrawler.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn venomcrawler() -> CardIndex {
    card_index("f3a07ae4-b0f8-43e6-adba-217f220cc2d0")
}

/// Venomcrawler — {3}{B} 2/2 with lifelink: "Whenever another creature
/// dies, put a +1/+1 counter on this creature." A Lightning Bolt on a Bears
/// beside it gives it one counter (3/3).
#[test]
fn venomcrawler_grows_when_another_creature_dies() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[venomcrawler(), grizzly_bears(), mountain()])
        .hand(0, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let crawler = on_battlefield(&engine, p0, venomcrawler()).expect("seated");
    let bears = on_battlefield(&engine, p0, grizzly_bears()).expect("seated");
    assert_eq!(pt(&engine, crawler), (2, 2));

    bolt_to_death(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, grizzly_bears()).is_some());
    assert_eq!(counters_on(&engine, crawler, CounterKind::P1P1), 1);
    assert_eq!(pt(&engine, crawler), (3, 3));
}

/// Lifelink: an unblocked Venomcrawler hits for 2 and its controller gains
/// the 2 (20 to 22, 20 to 18).
#[test]
fn venomcrawler_gains_life_from_its_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[venomcrawler()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let crawler = on_battlefield(&engine, p0, venomcrawler()).expect("seated");
    assert!(keywords(&engine, crawler).contains(KeywordSet::LIFELINK));
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0),
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(crawler, Defender::Player(p1))],
            },
        )
        .expect("it attacks");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("no blocks");
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert_eq!(engine.state().players[1].life, 18);
    assert_eq!(engine.state().players[0].life, 22, "lifelink");
}
