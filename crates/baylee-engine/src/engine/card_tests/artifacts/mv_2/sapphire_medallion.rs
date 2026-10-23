//! `cards/artifacts/mv_2/sapphire_medallion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::PHANTASMAL_FORCES;

fn sapphire_medallion() -> CardIndex {
    card_index("f1ef5698-14bd-42bf-aae5-2870699e4186")
}

/// `Sapphire Medallion` ({2}, Artifact, `Coverage::Implemented`):
/// `Blue spells you cast cost {1} less to cast.`
#[test]
fn sapphire_medallion_reduces_the_generic_cost_of_blue_spells_you_cast_by_one() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        // 3 Islands + Sapphire Medallion: enough to cast Phantasmal Forces ({3}{U} -> {2}{U}).
        .battlefield(0, &[island(), island(), island(), sapphire_medallion()])
        .hand(0, &[PHANTASMAL_FORCES])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, PHANTASMAL_FORCES);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, PHANTASMAL_FORCES).is_some());
}

/// The reduction belongs to its controller and to the printed spell color.
#[test]
fn reduction_does_not_help_the_opponent_or_other_colors() {
    use crate::engine::card_tests::legends_kit as lk;
    use baylee_core::generated::index as cards;
    for (seat, spell, color, amount) in [
        (
            PlayerId::new(1),
            cards::PHANTASMAL_FORCES,
            ManaColor::Blue,
            3,
        ),
        (PlayerId::new(0), cards::HILL_GIANT, ManaColor::Red, 3),
    ] {
        let p0 = PlayerId::new(0);
        let mut e = Duel::new(SEED, forest())
            .battlefield(0, &[sapphire_medallion()])
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
        assert!(on_battlefield(&e, p0, sapphire_medallion()).is_some());
    }
}
