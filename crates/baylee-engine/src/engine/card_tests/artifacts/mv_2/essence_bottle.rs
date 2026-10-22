//! `cards/artifacts/mv_2/essence_bottle.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::counters;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn essence_bottle() -> CardIndex {
    card_index("76f839b0-1d4c-4ad9-b44f-a8a13d6d109f")
}

/// Essence Bottle (`76f839b0-1d4c-4ad9-b44f-a8a13d6d109f`) — {2} Artifact (Partial):
/// "{3}, {T}: Put an elixir counter on this artifact. {T}, Remove all elixir counters from this artifact: You gain 2 life for each elixir counter removed this way."
#[test]
fn essence_bottle_adds_elixir_counter_and_redeems_for_two_life_each() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[essence_bottle(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let bottle = on_battlefield(&engine, P0, essence_bottle()).unwrap();
    tap_all_mana(&mut engine, P0);
    activate(&mut engine, P0, essence_bottle(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine
            .state()
            .object(bottle)
            .unwrap()
            .counters
            .get(counters::ELIXIR),
        1
    );

    // Next turn, untap and activate the second ability to gain 2 life and remove all elixir counters.
    reach_their_main_phase(&mut engine, P1);
    reach_their_main_phase(&mut engine, P0);
    activate(&mut engine, P0, essence_bottle(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(bottle)
            .unwrap()
            .counters
            .get(counters::ELIXIR),
        0
    );
    assert_eq!(engine.state().players[0].life, 22);
}
