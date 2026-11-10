//! `cards/creatures/mv_6/massacre_wurm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Massacre Wurm at a table of three: the -2/-2 reaches every creature an
/// opponent controls and none of its caster's, and each opponent loses 2
/// life per creature of **theirs** that died — "that player", not every
/// opponent.
#[test]
fn massacre_wurm_shrinks_every_opponents_creatures_and_each_loses_for_their_own_dead() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::table(SEED, swamp(), 3)
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                steadfast_guard(),
            ],
        )
        .battlefield(1, &[steadfast_guard(), thundering_giant()])
        .battlefield(2, &[steadfast_guard()])
        .hand(0, &[massacre_wurm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let giant = on_battlefield(&engine, PlayerId::new(1), thundering_giant()).unwrap();
    assert_eq!(pt(&engine, giant), (4, 3));
    cast_from_hand(&mut engine, p0, massacre_wurm());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, massacre_wurm()).is_some());
    assert!(
        on_battlefield(&engine, p0, steadfast_guard()).is_some(),
        "the caster's own 2/2 is not an opponent's creature"
    );
    assert!(on_battlefield(&engine, PlayerId::new(1), steadfast_guard()).is_none());
    assert!(on_battlefield(&engine, PlayerId::new(2), steadfast_guard()).is_none());
    assert_eq!(pt(&engine, giant), (2, 1), "a survivor wears the -2/-2");
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[1].life, 18, "one of seat 1's died");
    assert_eq!(engine.state().players[2].life, 18, "one of seat 2's died");

    // Until end of turn, and only for what was there as it resolved.
    pass_until(&mut engine, |e| e.state().turn.active == PlayerId::new(1));
    assert_eq!(pt(&engine, giant), (4, 3), "the -2/-2 ended with the turn");
}

/// A token that dies still costs its controller 2 life, though it has
/// ceased to exist by the time the trigger resolves — "that player" is
/// read as the creature last was. Thragtusk's Beast is the token.
#[test]
fn massacre_wurm_charges_for_a_dead_token_and_ignores_its_controllers_own_dead() {
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[massacre_wurm(), steadfast_guard()])
        .battlefield(1, &[thragtusk()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, PlayerId::new(0));
    let tusk = on_battlefield(&engine, p1, thragtusk()).unwrap();
    kill(&mut engine, tusk);
    assert_eq!(engine.state().players[1].life, 18, "Thragtusk was theirs");
    let beast = tokens_of(&engine, p1);
    assert_eq!(beast.len(), 1);
    kill(&mut engine, beast[0]);
    assert!(tokens_of(&engine, p1).is_empty(), "the token is gone");
    assert_eq!(engine.state().players[1].life, 16, "and still cost 2 life");

    let mine = on_battlefield(&engine, PlayerId::new(0), steadfast_guard()).unwrap();
    kill(&mut engine, mine);
    assert_eq!(
        engine.state().players[0].life,
        20,
        "not an opponent's creature"
    );
    assert_eq!(engine.state().players[1].life, 16);
}
