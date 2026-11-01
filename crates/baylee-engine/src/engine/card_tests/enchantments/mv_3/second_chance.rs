//! `cards/enchantments/mv_3/second_chance.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn second_chance() -> CardIndex {
    card_index("aa19bbf0-7de3-4ec3-a98b-2ac7533c6b0b")
}

#[test]
fn second_chance_sacrifices_at_low_life_for_extra_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[second_chance()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, second_chance());
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, second_chance()).is_some(),
        "above 5 life it stays"
    );
    assert!(engine.state().extra_turns.is_empty());
    engine.dev_state_mut(p0).unwrap().players[0].life = 5;

    reach_their_main_phase(&mut engine, p1);
    assert!(
        on_battlefield(&engine, p0, second_chance()).is_some(),
        "not the opponent's upkeep"
    );
    reach_their_main_phase(&mut engine, p0);

    assert!(in_graveyard(&engine, p0, second_chance()).is_some());
    assert_eq!(engine.state().extra_turns, vec![p0]);
}
