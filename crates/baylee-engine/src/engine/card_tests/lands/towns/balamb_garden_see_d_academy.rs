//! `cards/lands/towns/balamb_garden_see_d_academy.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Balamb Garden, `SeeD` Academy // Balamb Garden, Airborne: "This land enters tapped." / "{T}: Add {G} or {U}." / "{5}{G}{U}, {T}: Transform this land..."
/// Under `Coverage::Partial`, the transform ability and back-face crew ability are omitted.
/// Playing the front face enters tapped, and after untapping on a subsequent turn it offers a choice between green and blue mana.
#[test]
fn balamb_garden_enters_tapped_and_taps_for_green_or_blue() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(222, forest())
        .hand(0, &[balamb_garden_see_d_academy()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, balamb_garden_see_d_academy());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, balamb_garden_see_d_academy(), 0);
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!("expected color choice, got {:?}", engine.pending());
    };
    assert!(options.contains(&ManaColor::Green));
    assert!(options.contains(&ManaColor::Blue));

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Blue), 1);
    assert!(is_tapped(&engine, land));
}
