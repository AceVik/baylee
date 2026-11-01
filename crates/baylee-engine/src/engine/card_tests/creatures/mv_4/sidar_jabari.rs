//! `cards/creatures/mv_4/sidar_jabari.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

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

/// "Tap target creature **defending player controls**": the menu leaves out
/// the attacker's own creatures. The untapped Bears that blocks is a creature
/// without flanking, so flanking shrinks it.
#[test]
fn sidar_jabari_taps_only_their_creature_and_flanking_shrinks_the_other_blocker() {
    let mut engine = lk::game(
        SEED,
        &[sidar_jabari(), index::GRIZZLY_BEARS],
        &[index::HILL_GIANT, index::GRIZZLY_BEARS],
        &[],
        &[],
    );
    let jabari = lk::obj(&engine, P0, sidar_jabari());
    let my_bears = lk::obj(&engine, P0, index::GRIZZLY_BEARS);
    let giant = lk::obj(&engine, P1, index::HILL_GIANT);
    let their_bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);

    lk::attackers_offered(&mut engine);
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(jabari, Defender::Player(P1))],
            },
        )
        .unwrap();
    let menu = aim_at(&mut engine, P0, giant);
    assert!(menu.contains(&giant) && menu.contains(&their_bears));
    assert!(
        !menu.contains(&my_bears) && !menu.contains(&jabari),
        "only creatures the defending player controls"
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    assert!(is_tapped(&engine, giant));
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(their_bears, jabari)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, their_bears), (1, 1));
}
