//! `cards/instants/mv_1/whispers_of_the_muse.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn whispers_of_the_muse() -> CardIndex {
    card_index("a4a8af4e-8686-45ba-94fd-ccf6cc8adaae")
}

/// p0 has `lands` Islands and Whispers of the Muse in hand; p1 has two Islands
/// and a Counterspell. p0's main phase.
fn whispers_over(lands: usize) -> Engine<RegistryLookup> {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &vec![island(); lands])
        .battlefield(1, &[island(), island()])
        .hand(0, &[whispers_of_the_muse()])
        .hand(1, &[counterspell()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    engine
}

fn hand_size(engine: &Engine<RegistryLookup>) -> usize {
    engine.state().zones.list(ZoneLocation::Hand(P0)).len()
}

/// `Whispers of the Muse` ({U}, Instant):
/// `Buyback {5}`
/// `Draw a card.`
#[test]
fn whispers_of_the_muse_draws_a_card() {
    let mut engine = whispers_over(1);

    let hand_before = hand_size(&engine);
    cast_from_hand(&mut engine, P0, whispers_of_the_muse());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(hand_size(&engine), hand_before);
}

/// "Buyback {5}": the {5} is paid on top, and the card comes home beside the
/// card it drew.
#[test]
fn whispers_of_the_muse_with_its_buyback_paid_comes_back_to_hand() {
    let mut engine = whispers_over(6);
    let library = library_size(&engine, P0);

    cast_from_hand(&mut engine, P0, whispers_of_the_muse());
    answer_buyback(&mut engine, P0, true);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{U}} and {{5}} are the six floating"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(library_size(&engine, P0), library - 1);
    assert_eq!(hand_size(&engine), 2, "the drawn card and the spell");
    assert_bought_back(&engine, P0, whispers_of_the_muse());
}

/// Declined, it costs five less and is a Whispers that is spent.
#[test]
fn whispers_of_the_muse_without_its_buyback_goes_to_the_graveyard() {
    let mut engine = whispers_over(6);
    let library = library_size(&engine, P0);

    cast_from_hand(&mut engine, P0, whispers_of_the_muse());
    answer_buyback(&mut engine, P0, false);
    assert_eq!(engine.state().players[0].mana_pool.total(), 5);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(library_size(&engine, P0), library - 1);
    assert_eq!(hand_size(&engine), 1, "the drawn card alone");
    assert_not_bought_back(&engine, P0, whispers_of_the_muse());
}

/// Countered, it does not resolve: buyback paid, card gone, nothing drawn.
/// "Draw a card" targets nothing, so no target can leave it (CR 608.2b).
#[test]
fn whispers_of_the_muse_countered_with_its_buyback_paid_goes_to_the_graveyard() {
    let mut engine = whispers_over(6);
    let library = library_size(&engine, P0);

    cast_from_hand(&mut engine, P0, whispers_of_the_muse());
    answer_buyback(&mut engine, P0, true);
    counter_the_top_spell(&mut engine, P0, P1);

    assert_eq!(library_size(&engine, P0), library);
    assert_eq!(hand_size(&engine), 0);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_not_bought_back(&engine, P0, whispers_of_the_muse());
}
