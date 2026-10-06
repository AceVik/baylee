//! `cards/lands/manlands/restless_vinestalk.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Restless Vinestalk: "This land enters tapped." / "{T}: Add {G} or {U}." / "{3}{G}{U}: Until end of turn, this land becomes a 5/5 green and blue Plant creature with trample. It's still a land."
/// Under `Coverage::Implemented`, all printed characteristics of the land and its animation are fully realized.
/// Playing this land causes it to enter tapped, and paying `{3}{G}{U}` after untapping animates it into a 5/5 Plant creature with trample.
#[test]
fn restless_vinestalk_animates_into_plant_with_trample() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(318, forest())
        .battlefield(0, &[forest(), forest(), forest(), island(), island()])
        .hand(0, &[restless_vinestalk()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, restless_vinestalk());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    tap_mana_except(&mut engine, p0, land);
    activate(&mut engine, p0, restless_vinestalk(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, land), (5, 5));
    let types = engine.state().object(land).unwrap().characteristics().types;
    assert!(types.contains(TypeSet::CREATURE));
    assert!(types.contains(TypeSet::LAND));
    assert!(keywords(&engine, land).contains(KeywordSet::TRAMPLE));
    assert!(!is_tapped(&engine, land));
}

/// Restless Vinestalk: "Whenever this land attacks, up to one other target
/// creature has base power and toughness 3/3 until end of turn."
#[test]
fn restless_vinestalk_attack_sets_another_creatures_base_power_and_toughness() {
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(4102, forest())
        .battlefield(
            0,
            &[
                restless_vinestalk(),
                forest(),
                forest(),
                island(),
                island(),
                island(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    animate_and_attack(&mut engine, restless_vinestalk(), &[]);
    let elves = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves");
    assert_eq!(pt(&engine, elves), (1, 1));
    aim_trigger_at(&mut engine, elves);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, elves), (3, 3), "base 3/3 until end of turn");
}
