//! `cards/enchantments/mv_4/collapsing_borders.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn collapsing_borders() -> CardIndex {
    card_index("9fea3f42-643f-43c8-a9a9-4516dd72af5b")
}

fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

/// Collapsing Borders ({3}{R} Enchantment):
/// "Domain — At the beginning of each player's upkeep, that player gains 1 life
/// for each basic land type among lands they control. Then this enchantment deals 3 damage to that player."
#[test]
fn collapsing_borders_gains_domain_life_then_deals_three_each_upkeep() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), forest(), plains(), island()])
        .battlefield(1, &[swamp()])
        .hand(0, &[collapsing_borders()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, collapsing_borders());
    pass_until(&mut engine, stack_is_empty);

    // P1 upkeep: 1 basic land type (Swamp) -> +1 life, then 3 damage -> 18 life.
    reach_their_main_phase(&mut engine, P1);
    assert_eq!(life_of(&engine, P1), 18);

    // P0 upkeep: 4 basic land types -> +4 life (24), then 3 damage -> 21 life.
    reach_their_main_phase(&mut engine, P0);
    assert_eq!(life_of(&engine, P0), 21);
}
