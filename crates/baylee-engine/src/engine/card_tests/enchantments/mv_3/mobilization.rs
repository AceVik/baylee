//! `cards/enchantments/mv_3/mobilization.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Mobilization` is an enchantment costing `{2}{W}` under `Coverage::Implemented`.
/// It prints "Soldier creatures have vigilance" and "{2}{W}: Create a 1/1 white Soldier creature token."
/// When activated off floating mana, ability 1 creates a 1/1 white Soldier token.
/// Its static ability grants that token vigilance.
#[test]
fn mobilization_creates_soldier_token_and_grants_vigilance() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mobilization(), plains(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    assert!(
        tokens_of(&engine, p0).is_empty(),
        "no tokens on battlefield initially"
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White),
        3,
        "three white mana available"
    );

    // Ability 0 is the static vigilance grant; ability 1 is the token creation.
    activate(&mut engine, p0, mobilization(), 1);
    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1, "one token was created");
    let soldier = tokens[0];
    assert_eq!(pt(&engine, soldier), (1, 1), "Soldier token is a 1/1");
    assert!(
        keywords(&engine, soldier).contains(KeywordSet::VIGILANCE),
        "Mobilization grants vigilance to the Soldier token"
    );
}
