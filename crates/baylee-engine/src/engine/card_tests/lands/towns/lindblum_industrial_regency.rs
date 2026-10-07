//! `cards/lands/towns/lindblum_industrial_regency.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lindblum, Industrial Regency prints `This land enters tapped` and `{T}: Add {R}` on its land face,
/// alongside the Adventure instant Mage Siege.
/// The card is marked `Coverage::Partial` because the Adventure token and damage effects are unsupported.
/// Playing Lindblum enters tapped, untaps on the subsequent turn cycle, and taps for one red mana.
#[test]
fn lindblum_industrial_regency_enters_tapped_and_taps_for_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[lindblum_industrial_regency()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, lindblum_industrial_regency());
    assert!(entered_tapped(&engine, land));

    let from = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.number > from
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority");
    };
    assert!(legal.abilities.contains(&(land, 0)));

    activate(&mut engine, p0, lindblum_industrial_regency(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert!(is_tapped(&engine, land));
}
