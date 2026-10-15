//! `cards/enchantments/mv_4/primal_order.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn primal_order() -> CardIndex {
    card_index("63136274-df97-49ae-9efe-1385410e744b")
}

/// "At the beginning of each player's upkeep, this enchantment deals damage
/// to that player equal to the number of nonbasic lands they control": my
/// two Badlands cost me 2 (my Forest costs nothing), and the opponent's one
/// Badlands costs them 1, not my count.
#[test]
fn primal_order_counts_the_upkeeping_players_nonbasic_lands() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[primal_order(), badlands(), badlands(), forest()])
        .battlefield(1, &[badlands()])
        .start();
    keep_mulligans(&mut engine);

    walk_to_step_trigger(&mut engine, p0, Step::Upkeep);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p0), 18, "two nonbasic lands, two damage");
    assert_eq!(life_of(&engine, p1), 20, "the other player is not hit");

    walk_to_step_trigger(&mut engine, p1, Step::Upkeep);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p1), 19, "one nonbasic land, one damage");
    assert_eq!(life_of(&engine, p0), 18, "my life is left alone");
}
