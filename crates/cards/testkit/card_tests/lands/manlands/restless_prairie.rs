//! `cards/lands/manlands/restless_prairie.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Restless Prairie: "This land enters tapped." / "{T}: Add {G} or {W}." / "{2}{G}{W}: This land becomes a 3/3 green and white Llama creature until end of turn. It's still a land."
/// Under `Coverage::Implemented`, all printed characteristics of the land and its animation are fully realized.
/// Playing this land causes it to enter tapped, and paying `{2}{G}{W}` after untapping animates it into a 3/3 Llama creature.
#[test]
fn restless_prairie_animates_into_llama() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(314, forest())
        .battlefield(0, &[forest(), forest(), plains(), plains()])
        .hand(0, &[restless_prairie()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, restless_prairie());
    assert!(entered_tapped(&engine, land));

    pass_until(&mut engine, |e| {
        e.state().turn.number >= 3
            && e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert!(!is_tapped(&engine, land));

    tap_mana_except(&mut engine, p0, land);
    activate(&mut engine, p0, restless_prairie(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, land), (3, 3));
    let types = engine.state().object(land).unwrap().characteristics().types;
    assert!(types.contains(TypeSet::CREATURE));
    assert!(types.contains(TypeSet::LAND));
    assert!(!is_tapped(&engine, land));
}

/// Restless Prairie: "Whenever this land attacks, other creatures you
/// control get +1/+1 until end of turn."
#[test]
fn restless_prairie_attack_pumps_the_other_creatures_you_control() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(4103, forest())
        .battlefield(
            0,
            &[
                restless_prairie(),
                forest(),
                forest(),
                plains(),
                plains(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves");
    let land = animate_and_attack(&mut engine, restless_prairie(), &[]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, mine), (2, 2), "my other creature gets +1/+1");
    assert_eq!(pt(&engine, theirs), (1, 1), "theirs does not");
    assert_eq!(
        pt(&engine, land),
        (3, 3),
        "\"other\": the land is not pumped"
    );
}
