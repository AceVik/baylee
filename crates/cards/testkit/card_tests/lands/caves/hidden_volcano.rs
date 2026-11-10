//! `cards/lands/caves/hidden_volcano.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hidden Volcano: "This land enters tapped." / "{T}: Add {R}." / "{4}{R}, {T}, Sacrifice this land: Discover 4. Activate only as a sorcery."
/// Playing this land causes it to enter tapped, and after untapping on a subsequent turn it taps for red mana.
#[test]
fn hidden_volcano_enters_tapped_and_taps_for_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(205, forest())
        .hand(0, &[hidden_volcano()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, hidden_volcano());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    activate(&mut engine, p0, hidden_volcano(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert!(is_tapped(&engine, land));
}

#[test]
fn hidden_volcano_pays_its_colored_cost_and_discovers_four() {
    hidden_cave_discovers(hidden_volcano(), ManaColor::Red, false);
}

/// "Activate only as a sorcery" and "{T}" are part of the discover ability.
#[test]
fn hidden_volcano_discover_is_refused_on_a_stack_an_opponents_turn_or_a_tapped_cave() {
    hidden_cave_refuses_discover(hidden_volcano(), ManaColor::Red);
}
