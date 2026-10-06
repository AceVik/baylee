//! `cards/lands/artifacts/power_depot.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Power Depot prints `This land enters tapped`, `{T}: Add {C}`, `{T}: Add one mana of any color.
/// Spend this mana only to cast artifact spells or activate abilities of artifacts`, and `Modular 1`.
/// The card is marked `Coverage::Partial` because modular dies triggers and ability restrictions are unsupported.
/// Power Depot enters tapped wearing one `CounterKind::P1P1` counter; upon untapping, activating ability index 1
/// prompts for a color and places one restricted mana into `pool.restricted()` rather than `pool.available()`.
#[test]
fn power_depot_enters_tapped_with_counter_and_adds_restricted_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest()).hand(0, &[power_depot()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, power_depot());
    assert!(entered_tapped(&engine, land));
    assert_eq!(counters_on(&engine, land, CounterKind::P1P1), 1);
    let t = types(&engine, land);
    assert!(t.contains(TypeSet::ARTIFACT));
    assert!(t.contains(TypeSet::LAND));

    let from = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > from
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, power_depot(), 1);

    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice");
    };
    assert_eq!(options.len(), 5);

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 0);
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::Blue);
    assert!(is_tapped(&engine, land));
}
