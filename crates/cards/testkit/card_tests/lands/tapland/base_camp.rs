//! `cards/lands/tapland/base_camp.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn base_camp_enters_tapped_and_adds_restricted_party_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest()).hand(0, &[base_camp()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, base_camp());
    assert!(entered_tapped(&engine, land));

    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, base_camp(), 1);
    let Pending::ChooseColor { .. } = engine.pending().clone() else {
        panic!("expected ChooseColor, got {:?}", engine.pending());
    };
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::Red);
    assert_eq!(pool.available(ManaColor::Red), 0);
    assert_eq!(pool.total(), 1);
    assert!(is_tapped(&engine, land));
}
