//! `cards/creatures/mv_7/oyobi_who_split_the_heavens.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Oyobi, Who Split the Heavens` is a `{6}{W}` 3/6 legendary Spirit with flying under `Coverage::Implemented`.
/// Whenever its controller casts a Spirit or Arcane spell, its triggered ability creates a 3/3 white Spirit creature token with flying.
/// With Oyobi on the battlefield, casting `kami_of_tattered_shoji()` (a Spirit spell) fires the `Trigger::SpellCast` ability,
/// which resolves into a 3/3 flying Spirit creature token before the cast creature itself enters the battlefield.
#[test]
fn oyobi_creates_spirit_token_on_casting_spirit_spell() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                oyobi_who_split_the_heavens(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .hand(0, &[kami_of_tattered_shoji()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let oyobi = on_battlefield(&engine, p0, oyobi_who_split_the_heavens())
        .expect("Oyobi is on battlefield");
    assert_eq!(pt(&engine, oyobi), (3, 6), "Oyobi is a 3/6");
    assert!(
        keywords(&engine, oyobi).contains(KeywordSet::FLYING),
        "Oyobi has flying"
    );
    assert!(
        tokens_of(&engine, p0).is_empty(),
        "no tokens on battlefield initially"
    );

    cast_from_hand(&mut engine, p0, kami_of_tattered_shoji());

    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(
        tokens.len(),
        1,
        "one Spirit token was created from casting a Spirit spell"
    );
    let token = tokens[0];
    assert_eq!(pt(&engine, token), (3, 3), "Spirit token is a 3/3");
    assert!(
        keywords(&engine, token).contains(KeywordSet::FLYING),
        "Spirit token has flying"
    );
    assert!(
        on_battlefield(&engine, p0, kami_of_tattered_shoji()).is_some(),
        "the cast Spirit creature resolved and entered the battlefield"
    );
}
