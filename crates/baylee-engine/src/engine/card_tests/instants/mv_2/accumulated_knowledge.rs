//! `cards/instants/mv_2/accumulated_knowledge.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

/// Accumulated Knowledge ({1}{U} Instant):
/// "Draw a card, then draw cards equal to the number of cards named Accumulated Knowledge in all graveyards."
#[test]
fn accumulated_knowledge_draws_one_plus_copies_in_all_graveyards() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, card_index("1ca25379-d194-4670-a7bf-525ad11cf436"))
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(
            0,
            &[
                card_index("1ca25379-d194-4670-a7bf-525ad11cf436"),
                card_index("1ca25379-d194-4670-a7bf-525ad11cf436"),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_len = |e: &Engine<RegistryLookup>, seat: PlayerId| {
        e.state().zones.list(ZoneLocation::Hand(seat)).len()
    };

    // Seed 1 copy of Accumulated Knowledge into P1's graveyard.
    seed_graveyard(&mut engine, p1, 1);
    let before = hand_len(&engine, p0);

    // First cast: 1 (base) + 1 (in P1's graveyard) = 2 cards drawn; hand grows by +1 net.
    tap_mana_where(&mut engine, p0, |_| true);
    cast_with_floating(
        &mut engine,
        p0,
        card_index("1ca25379-d194-4670-a7bf-525ad11cf436"),
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(hand_len(&engine, p0), before + 1);

    // Second cast: 1 (base) + 2 (1 in P1's graveyard + 1 in P0's graveyard) = 3 cards drawn; hand grows by +2 net.
    cast_with_floating(
        &mut engine,
        p0,
        card_index("1ca25379-d194-4670-a7bf-525ad11cf436"),
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(hand_len(&engine, p0), before + 3);
}
