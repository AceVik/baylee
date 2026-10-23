#[allow(clippy::wildcard_imports)]
use super::*;

fn waylay() -> CardIndex {
    card_index("f0db78c7-09b5-4981-8ca8-6dbab597c5d9")
}

#[test]
fn waylay_creates_three_knight_tokens() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .hand(0, &[waylay()])
        .battlefield(0, &[plains(), plains(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, waylay());
    pass_until(&mut engine, stack_is_empty);

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 3);
    assert_eq!(pt(&engine, tokens[0]), (2, 2));
}
