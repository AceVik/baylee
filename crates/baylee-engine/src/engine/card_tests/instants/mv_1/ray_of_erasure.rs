//! `cards/instants/mv_1/ray_of_erasure.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn ray_of_erasure() -> CardIndex {
    card_index("29d828fa-a813-48f6-8dcb-43f6047b88a3")
}

fn hand_size(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(seat)).len()
}

/// Ray of Erasure (29d828fa-a813-48f6-8dcb-43f6047b88a3) — {U} Instant:
/// - "Target player mills a card."
/// - "Draw a card at the beginning of the next turn's upkeep."
#[test]
fn ray_of_erasure_mills_target_player_one_card_and_draws_at_next_upkeep() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island()])
        .hand(0, &[ray_of_erasure()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let p1_lib_before = library_size(&engine, P1);
    let p0_lib_before = library_size(&engine, P0);
    let p0_hand_before = hand_size(&engine, P0);

    tap_all_mana(&mut engine, P0);
    cast_with_floating(&mut engine, P0, ray_of_erasure());
    engine.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, P1),
        p1_lib_before - 1,
        "targeted player P1 mills 1 card"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(P1)).len(),
        1,
        "the milled card is in P1's graveyard"
    );
    assert_eq!(
        library_size(&engine, P0),
        p0_lib_before,
        "untargeted player P0's library is untouched"
    );

    // At the beginning of the next turn's upkeep (P1's upkeep, turn 2), P0 draws a card.
    pass_until(&mut engine, |e| {
        e.state().turn.number == 2 && e.state().turn.step == Step::Upkeep && stack_is_empty(e)
    });
    assert_eq!(
        hand_size(&engine, P0),
        p0_hand_before,
        "P0 draws a card at the beginning of the next turn's upkeep"
    );
}
