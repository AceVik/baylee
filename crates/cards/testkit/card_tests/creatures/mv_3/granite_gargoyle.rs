//! `cards/creatures/mv_3/granite_gargoyle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Granite Gargoyle prints two things: flying on a 2/2 body for `{2}{R}`, and
/// "`{R}`: This creature gets +0/+1 until end of turn" — an ability with no
/// once-per-turn clause, so it stacks. Both halves are read off one board of
/// five Mountains: three pay for the creature and the two left in the pool buy
/// the pump twice. `(2, 3)` then `(2, 4)` is the only pair of numbers that shows
/// the toughness moving while the power stays exactly where it was printed — a
/// `(3, 3)` would mean the pump was written +1/+1, and a single activation
/// alone could not tell a repeatable ability from a one-shot.
#[test]
fn granite_gargoyle_pumps_its_own_toughness_once_per_red() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(1701, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[granite_gargoyle()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, granite_gargoyle());
    pass_until(&mut engine, stack_is_empty);
    let gargoyle = on_battlefield(&engine, p0, granite_gargoyle()).expect("the Gargoyle resolved");
    assert!(
        keywords(&engine, gargoyle).contains(KeywordSet::FLYING),
        "the printed flying reaches the permanent"
    );
    assert_eq!(pt(&engine, gargoyle), (2, 2), "the body it prints");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        2,
        "five Mountains paid {{2}}{{R}} and left two red floating"
    );

    // Ability 0 is the only line the card prints and its whole price is the
    // {R}: no {T} is on it, so the Gargoyle stays untapped and may pay the
    // same price again.
    activate(&mut engine, p0, granite_gargoyle(), 0);
    assert!(
        !stack_is_empty(&engine),
        "a pump is no mana ability, so it uses the stack"
    );
    assert_eq!(
        pt(&engine, gargoyle),
        (2, 2),
        "and nothing has happened while the targetless ability is still there"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, gargoyle),
        (2, 3),
        "{{R}} buys +0/+1, and not a point of power"
    );

    activate(&mut engine, p0, granite_gargoyle(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, gargoyle),
        (2, 4),
        "the ability carries no once-per-turn clause, so the second {{R}} \
         stacks on the first"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "both red were spent"
    );
    assert!(
        !is_tapped(&engine, gargoyle),
        "the price was mana and never its own {{T}}"
    );
}
