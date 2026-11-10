//! `cards/instants/mv_1/sprout.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sprout is one line — `{G}` for "Create a 1/1 green Saproling creature
/// token" — so the whole card is the token, and the only way to tell a token
/// from a spell that did nothing is to read the permanent that arrived. One
/// Forest pays for it, which makes the pool reading exact: the Saproling
/// stands on a board that held nothing before the cast, it is a *creature*
/// (an Effect that merely announced a token would leave the type line empty),
/// and its 1/1 green body is the printing rather than a default. The spell
/// itself is looked for in the graveyard afterwards, which is where a resolved
/// instant goes and where a case that never resolved would not be.
#[test]
fn sprout_creates_a_one_one_green_saproling_token() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest()])
        .hand(0, &[sprout()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert!(
        tokens_of(&engine, p0).is_empty(),
        "the board holds no token before the spell is cast"
    );

    cast_from_hand(&mut engine, p0, sprout());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, sprout()).is_some(),
        "a resolved instant goes to its owner's graveyard"
    );
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one activation of the spell, one token");
    let saproling = engine
        .state()
        .object(tokens[0])
        .expect("the token is on the battlefield")
        .token
        .expect("it knows which token it is");
    assert_eq!(saproling.name, "Saproling");
    assert_eq!(
        (saproling.power, saproling.toughness),
        (Some(1), Some(1)),
        "a 1/1, and not a bodyless token the state-based checks would eat"
    );
    assert!(
        saproling.colors.contains(baylee_core::color::Color::Green),
        "green, which no other part of the board could have supplied"
    );
    assert!(
        types(&engine, tokens[0]).contains(TypeSet::CREATURE),
        "and it is a creature, not merely a permanent that appeared"
    );
}
