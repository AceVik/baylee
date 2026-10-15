//! `cards/enchantments/mv_5/polluted_bonds.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn polluted_bonds() -> CardIndex {
    card_index("13b33a41-c22b-4e7a-8323-abadf7f80961")
}

/// "Whenever a land an opponent controls enters, that player loses 2 life
/// and you gain 2 life." My own land does nothing; theirs drains them.
#[test]
fn polluted_bonds_drains_the_opponent_for_each_land_they_play() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[polluted_bonds()])
        .hand(0, &[forest()])
        .hand(1, &[plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    play_land(&mut engine, p0, forest());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, 20, "my own land");
    assert_eq!(engine.state().players[1].life, 20);

    reach_their_main_phase(&mut engine, p1);
    play_land(&mut engine, p1, plains());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[1].life, 18, "they lose 2");
    assert_eq!(engine.state().players[0].life, 22, "and I gain 2");
}
