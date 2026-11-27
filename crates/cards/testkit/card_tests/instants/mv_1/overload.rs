//! `cards/instants/mv_1/overload.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn overload() -> CardIndex {
    card_index("07159efc-c69f-4164-a8ca-9da641dbf702")
}

fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

fn jayemdae_tome() -> CardIndex {
    card_index("39ee576a-0803-4063-9c84-f2b537e4d44c")
}

/// Overload ({R} Instant, Kicker {2}):
/// - Unkicked destroys artifact with MV <= 2, leaves MV 4 artifact alone even though it can target it.
/// - Kicked destroys artifact with MV <= 5.
#[test]
fn overload_respects_mv_two_unkicked_and_mv_five_kicked() {
    // Unkicked vs MV 1 (Sol Ring) -> destroyed.
    let mut e1 = lk::game(SEED, &[mountain()], &[sol_ring()], &[overload()], &[]);
    let ring = lk::obj(&e1, P1, sol_ring());
    tap_all_mana(&mut e1, P0);
    cast_with_floating(&mut e1, P0, overload());
    lk::aim(&mut e1, &[ring], &[]);
    pass_until(&mut e1, stack_is_empty);
    assert!(in_graveyard(&e1, P1, sol_ring()).is_some());

    // Unkicked vs MV 4 (Jayemdae Tome) -> legal target, NOT destroyed.
    let mut e2 = lk::game(
        SEED,
        &[mountain(), mountain(), mountain()],
        &[jayemdae_tome()],
        &[overload()],
        &[],
    );
    let tome = lk::obj(&e2, P1, jayemdae_tome());
    tap_all_mana(&mut e2, P0);
    cast_with_floating(&mut e2, P0, overload());
    e2.apply(P0, PlayerAction::YesNo(false)).unwrap();
    lk::aim(&mut e2, &[tome], &[]);
    pass_until(&mut e2, stack_is_empty);
    assert!(on_battlefield(&e2, P1, jayemdae_tome()).is_some());

    // Kicked vs MV 4 (Jayemdae Tome) -> destroyed.
    let mut e3 = lk::game(
        SEED,
        &[mountain(), mountain(), mountain()],
        &[jayemdae_tome()],
        &[overload()],
        &[],
    );
    let tome3 = lk::obj(&e3, P1, jayemdae_tome());
    tap_all_mana(&mut e3, P0);
    cast_with_floating(&mut e3, P0, overload());
    e3.apply(P0, PlayerAction::YesNo(true)).unwrap();
    lk::aim(&mut e3, &[tome3], &[]);
    pass_until(&mut e3, stack_is_empty);
    assert!(in_graveyard(&e3, P1, jayemdae_tome()).is_some());
}
