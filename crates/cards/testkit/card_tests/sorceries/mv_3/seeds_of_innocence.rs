//! `cards/sorceries/mv_3/seeds_of_innocence.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn seeds_of_innocence() -> CardIndex {
    card_index("f5ebad5c-9675-4a37-b76c-760416083939")
}

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

fn sol_ring() -> CardIndex {
    card_index("6ad8011d-3471-4369-9d68-b264cc027487")
}

fn jayemdae_tome() -> CardIndex {
    card_index("39ee576a-0803-4063-9c84-f2b537e4d44c")
}

fn darksteel_gargoyle() -> CardIndex {
    card_index("73010421-374f-458e-aa88-248ef8ae4f8b")
}

#[test]
fn seeds_of_innocence_destroys_artifacts_and_controllers_gain_mana_value_life() {
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), sol_ring()])
        .battlefield(1, &[jayemdae_tome(), darksteel_gargoyle()])
        .hand(0, &[seeds_of_innocence()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);

    cast_from_hand(&mut e, P0, seeds_of_innocence());
    pass_until(&mut e, stack_is_empty);

    assert!(on_battlefield(&e, P0, sol_ring()).is_none());
    assert!(on_battlefield(&e, P1, jayemdae_tome()).is_none());
    assert!(
        on_battlefield(&e, P1, darksteel_gargoyle()).is_some(),
        "indestructible artifact survives"
    );
    assert_eq!(
        e.state().players[0].life,
        21,
        "P0 gains 1 for Sol Ring (MV 1)"
    );
    assert_eq!(
        e.state().players[1].life,
        31,
        "P1 gains 4 for Tome (MV 4) + 7 for surviving indestructible Gargoyle (MV 7)"
    );
}
