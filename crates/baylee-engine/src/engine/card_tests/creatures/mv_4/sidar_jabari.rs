//! `cards/creatures/mv_4/sidar_jabari.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn sidar_jabari() -> CardIndex {
    card_index("cb637dfd-4022-43a4-8632-77e614436f38")
}

/// Sidar Jabari (`{3}{W}`, `Coverage::Implemented`): 2/2 legendary Human Knight
/// with flanking that taps target creature defending player controls whenever
/// it attacks.
#[test]
fn sidar_jabari_taps_defending_creature_on_attack() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[sidar_jabari()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let jabari = on_battlefield(&engine, p0, sidar_jabari()).unwrap();
    let bear = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    assert!(keywords(&engine, jabari).contains(KeywordSet::FLANKING));

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(jabari, Defender::Player(p1))],
            },
        )
        .unwrap();
    let menu = aim_at(&mut engine, p0, bear);
    assert_eq!(menu, vec![bear]);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, bear));
}
