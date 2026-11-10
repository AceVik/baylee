//! `cards/creatures/artifacts/mv_1/sphinx_s_herald.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn sphinx_s_herald() -> CardIndex {
    card_index("6452873b-1bb2-44b3-b630-88f15962b6e8")
}

/// "{2}{U}, {T}, Sacrifice a white creature, a blue creature, and a black
/// creature: …" The price is three different creatures: with the white and
/// the blue but no black, the ability is not on the offer; with all three it
/// is, and paying takes all three.
#[test]
fn sphinx_s_herald_wants_a_white_a_blue_and_a_black_creature_as_its_price() {
    let p0 = PlayerId::new(0);
    let mut without_black = main_with(&[sphinx_s_herald(), serra_angel(), killer_whale()], &[]);
    let herald = on_battlefield(&without_black, p0, sphinx_s_herald()).expect("seated");
    float_mana(&mut without_black, ManaColor::Blue, 1);
    float_mana(&mut without_black, ManaColor::Colorless, 2);
    assert!(
        !is_offered(&without_black, herald, 0),
        "no black creature to sacrifice"
    );

    let mut engine = main_with(
        &[
            sphinx_s_herald(),
            serra_angel(),
            killer_whale(),
            black_knight(),
        ],
        &[],
    );
    let herald = on_battlefield(&engine, p0, sphinx_s_herald()).expect("seated");
    float_mana(&mut engine, ManaColor::Blue, 1);
    assert!(
        !is_offered(&engine, herald, 0),
        "{{2}}{{U}} wants three mana"
    );
    float_mana(&mut engine, ManaColor::Colorless, 2);
    assert!(is_offered(&engine, herald, 0), "all three creatures stand");
}
