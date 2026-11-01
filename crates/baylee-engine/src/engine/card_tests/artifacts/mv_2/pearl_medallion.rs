//! `cards/artifacts/mv_2/pearl_medallion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);

fn pearl_medallion() -> CardIndex {
    card_index("19015380-1332-4960-8cc6-0732009525a2")
}

/// Pearl Medallion (`19015380-1332-4960-8cc6-0732009525a2`) — {2} Artifact:
/// "White spells you cast cost {1} less to cast."
#[test]
fn pearl_medallion_reduces_white_spell_cost_by_one() {
    // Mesa Pegasus costs {1}{W}; with Pearl Medallion in play it is castable for {W}.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[pearl_medallion(), plains()])
        .hand(0, &[index::MESA_PEGASUS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, index::MESA_PEGASUS);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, P0, index::MESA_PEGASUS).is_some());
}

/// The reduction belongs to its controller and to the printed spell color.
#[test]
fn reduction_does_not_help_the_opponent_or_other_colors() {
    use crate::engine::card_tests::legends_kit as lk;
    use baylee_core::generated::index as cards;
    for (seat, spell, color, amount) in [
        (PlayerId::new(1), cards::MESA_PEGASUS, ManaColor::White, 1),
        (PlayerId::new(0), cards::HILL_GIANT, ManaColor::Red, 3),
    ] {
        let p0 = PlayerId::new(0);
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[pearl_medallion()])
            .hand(usize::from(seat.get()), &[spell])
            .start();
        keep_mulligans(&mut e);
        reach_their_main_phase(&mut e, seat);
        lk::float(&mut e, seat, &[(color, amount)]);
        let id = in_hand(&e, seat, spell).unwrap();
        let Pending::Priority { player, legal } = e.pending() else {
            panic!("priority");
        };
        assert_eq!(*player, seat);
        assert!(
            !legal.castable.contains(&id) && !legal.payable.contains(&id),
            "the spell still costs its full price"
        );
        assert!(on_battlefield(&e, p0, pearl_medallion()).is_some());
    }
}
