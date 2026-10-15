//! `cards/creatures/artifacts/mv_2/ornithopter_of_paradise.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn ornithopter_of_paradise() -> CardIndex {
    card_index("3a940dfa-a026-4969-8981-ef90cdfbe9ef")
}

/// Ornithopter of Paradise: a 0/2 with "Flying" and "{T}: Add one mana of any
/// color". The color is named by the controller on activation, and the tap
/// is the price: tapped, it is not offered again.
#[test]
fn ornithopter_of_paradise_taps_for_one_mana_of_the_color_named() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[ornithopter_of_paradise()], &[]);
    let thopter = on_battlefield(&engine, p0, ornithopter_of_paradise()).expect("seated");
    assert_eq!(pt(&engine, thopter), (0, 2));
    assert!(keywords(&engine, thopter).contains(KeywordSet::FLYING));
    assert!(offered(&engine, thopter, 0));
    activate(&mut engine, p0, ornithopter_of_paradise(), 0);
    name_a_color(&mut engine, ManaColor::Red);
    assert!(is_tapped(&engine, thopter));
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1, "the named color");
    assert_eq!(pool.total(), 1);
    assert!(!offered(&engine, thopter, 0), "tapped, it cannot tap again");
}

/// The color is chosen at each activation, not fixed by the card: on a later
/// turn a different one can be named.
#[test]
fn ornithopter_of_paradise_may_name_a_different_color_each_turn() {
    let p0 = PlayerId::new(0);
    let mut engine = table(&[], &[ornithopter_of_paradise()], &[]);
    let thopter = on_battlefield(&engine, p0, ornithopter_of_paradise()).expect("seated");
    activate(&mut engine, p0, ornithopter_of_paradise(), 0);
    name_a_color(&mut engine, ManaColor::White);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        1
    );
    pass_until(&mut engine, |e| {
        e.state().turn.number > 2
            && e.state().turn.active == p0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { .. })
    });
    assert!(!is_tapped(&engine, thopter), "it untapped");
    activate(&mut engine, p0, ornithopter_of_paradise(), 0);
    name_a_color(&mut engine, ManaColor::Blue);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert_eq!(pool.available(ManaColor::White), 0, "last turn's is gone");
}

/// Flying: a ground creature cannot block it.
#[test]
fn ornithopter_of_paradise_flies_over_a_ground_blocker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(
        &[],
        &[ornithopter_of_paradise()],
        &[grizzly_bears(), storm_crow()],
    );
    let thopter = on_battlefield(&engine, p0, ornithopter_of_paradise()).expect("seated");
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("seated");
    let options = attack_and_collect_blocks(&mut engine, thopter, p1);
    assert!(!options.iter().any(|o| o.blocker == bears));
}
