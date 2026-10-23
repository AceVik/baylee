//! `cards/sorceries/mv_2/elven_rite.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn elven_rite() -> CardIndex {
    card_index("755dbf29-7484-444f-b5df-29ec4b4bbc82")
}

/// Elven Rite (`755dbf29-7484-444f-b5df-29ec4b4bbc82`) — {1}{G} Sorcery (Partial):
/// "Put two +1/+1 counters on target creature."
#[test]
fn elven_rite_puts_two_plus_one_counters_on_target_creature() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                index::GRIZZLY_BEARS,
                index::SAVANNAH_LIONS,
            ],
        )
        .hand(0, &[elven_rite()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bears = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    cast_from_hand(&mut engine, P0, elven_rite());
    aim_at(&mut engine, P0, bears);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bears), (4, 4));
    let lions = on_battlefield(&engine, P0, index::SAVANNAH_LIONS).unwrap();
    assert_eq!(pt(&engine, lions), (2, 1));
}
