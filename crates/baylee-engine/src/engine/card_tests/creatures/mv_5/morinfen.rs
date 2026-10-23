//! `cards/creatures/mv_5/morinfen.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::counters;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn morinfen() -> CardIndex {
    card_index("d69f3aaf-8462-432d-afaf-4176ac04962b")
}

/// The question P0's upkeep asks, and how much life it names.
fn upkeep_price(e: &mut Engine<RegistryLookup>, turn: u32) -> u32 {
    pass_until(e, |e| {
        e.state().turn.number == turn && matches!(e.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayLife { amount },
        ..
    } = e.pending().clone()
    else {
        panic!("the upkeep asks to pay life: {:?}", e.pending())
    };
    assert_eq!(player, P0);
    amount
}

/// Morinfen (d69f3aaf-8462-432d-afaf-4176ac04962b) — "Cumulative upkeep—
/// Pay 1 life." Each of its controller's upkeeps adds an age counter and asks
/// for 1 life per counter: 1, then 2, then (declined) it is sacrificed and
/// nothing is paid. The opponent's upkeep does nothing.
#[test]
fn morinfen_costs_one_more_life_each_of_its_controllers_upkeeps() {
    let mut e = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[morinfen()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    cast_from_hand(&mut e, P0, morinfen());
    pass_until(&mut e, stack_is_empty);
    let morin = on_battlefield(&e, P0, morinfen()).expect("Morinfen is out");

    // P1's upkeep is not "your upkeep": no counter, no life.
    reach_their_main_phase(&mut e, P1);
    assert_eq!(counters_on(&e, morin, counters::AGE), 0);
    assert_eq!(e.state().players[0].life, 20);

    assert_eq!(upkeep_price(&mut e, 3), 1, "one age counter, one life");
    e.apply(P0, PlayerAction::YesNo(true)).expect("paid");
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 19);
    assert_eq!(counters_on(&e, morin, counters::AGE), 1);

    assert_eq!(upkeep_price(&mut e, 5), 2, "two age counters, two life");
    e.apply(P0, PlayerAction::YesNo(true)).expect("paid");
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 17);
    assert_eq!(counters_on(&e, morin, counters::AGE), 2);

    assert_eq!(upkeep_price(&mut e, 7), 3, "three age counters, three life");
    e.apply(P0, PlayerAction::YesNo(false)).expect("declined");
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 17, "nothing was paid");
    assert!(
        in_graveyard(&e, P0, morinfen()).is_some(),
        "sacrificed unless the cost is paid"
    );
}
