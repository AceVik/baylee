//! `cards/instants/mv_2/raise_the_alarm.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Raise the Alarm prints one line — "Create two 1/1 white Soldier creature
/// tokens" — and "two" is the half a single-token reading would lose. The
/// board is two Plains and nothing else, so the {1}{W} is a real payment out
/// of a pool the lands actually filled, and the count is read off the
/// battlefield after the spell has resolved rather than off the stack: one
/// token and two tokens are the same card until the effect has finished.
#[test]
fn raise_the_alarm_makes_two_white_soldier_tokens() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[raise_the_alarm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert!(
        tokens_of(&engine, p0).is_empty(),
        "nothing is on the board before the spell is cast"
    );

    cast_from_hand(&mut engine, p0, raise_the_alarm());
    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(
        tokens.len(),
        2,
        "one printed line, two Soldiers: {tokens:?}"
    );
    for token in tokens {
        let def = engine
            .state()
            .object(token)
            .expect("the token is an object")
            .token
            .expect("the token knows which token it is");
        assert_eq!(def.name, "Soldier", "the token the card names");
        assert_eq!(
            (def.power, def.toughness),
            (Some(1), Some(1)),
            "a 1/1, and not the two Bodies a token with no numbers would have"
        );
        assert!(
            def.colors.contains(baylee_core::color::Color::White),
            "a *white* Soldier"
        );
        assert!(
            types(&engine, token).contains(TypeSet::CREATURE),
            "and a creature, not merely a permanent"
        );
        assert_eq!(
            engine
                .state()
                .object(token)
                .expect("the token is still there")
                .controller,
            p0,
            "under the control of the seat that cast the spell"
        );
    }
    assert!(
        in_graveyard(&engine, p0, raise_the_alarm()).is_some(),
        "an instant that has resolved goes to its owner's graveyard"
    );
}
