//! `cards/lands/caves/sunken_citadel.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn sunken_citadel_chooses_color_and_taps_for_one_of_it() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[sunken_citadel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let citadel = play_land(&mut engine, p0, sunken_citadel());
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(entered_tapped(&engine, citadel));

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, citadel));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(citadel, 1)),
        "the two-mana ability is available with its land-source restriction"
    );

    activate(&mut engine, p0, sunken_citadel(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1
    );
    assert!(is_tapped(&engine, citadel));
}
