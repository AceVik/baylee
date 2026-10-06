//! `cards/lands/manlands/restless_ridgeline.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Restless Ridgeline: "This land enters tapped." / "{T}: Add {R} or {G}." / "{2}{R}{G}: This land becomes a 3/4 red and green Dinosaur creature until end of turn. It's still a land."
/// Under `Coverage::Implemented`, all printed characteristics of the land and its animation are fully realized.
/// Playing this land causes it to enter tapped, and paying `{2}{R}{G}` after untapping animates it into a 3/4 Dinosaur creature.
#[test]
fn restless_ridgeline_animates_into_dinosaur() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(315, forest())
        .battlefield(0, &[mountain(), mountain(), forest(), forest()])
        .hand(0, &[restless_ridgeline()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, restless_ridgeline());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    tap_mana_except(&mut engine, p0, land);
    activate(&mut engine, p0, restless_ridgeline(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, land), (3, 4));
    let types = engine.state().object(land).unwrap().characteristics().types;
    assert!(types.contains(TypeSet::CREATURE));
    assert!(types.contains(TypeSet::LAND));
    assert!(!is_tapped(&engine, land));
}

/// Restless Ridgeline: "Whenever this land attacks, another target attacking
/// creature gets +2/+0 until end of turn. Untap that creature."
#[test]
fn restless_ridgeline_attack_pumps_and_untaps_another_attacker() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4104, forest())
        .battlefield(
            0,
            &[
                restless_ridgeline(),
                mountain(),
                mountain(),
                forest(),
                forest(),
                restless_bears(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    let elves = on_battlefield(&engine, p0, restless_bears()).expect("my Bears");
    animate_and_attack(&mut engine, restless_ridgeline(), &[elves]);
    assert!(is_tapped(&engine, elves), "attacking tapped the Elves");
    aim_trigger_at(&mut engine, elves);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, elves), (4, 2), "+2/+0");
    assert!(!is_tapped(&engine, elves), "untapped by the trigger");
}
