//! `cards/creatures/mv_1/thallid.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn ready() -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[card_index("46abaabe-7015-4d82-a5aa-2706b1f51bed")])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    e
}

fn spores(e: &Engine<RegistryLookup>) -> u16 {
    let id = on_battlefield(e, P0, card_index("46abaabe-7015-4d82-a5aa-2706b1f51bed"))
        .expect("the Thallid is out");
    e.state()
        .object(id)
        .expect("it is out")
        .counters
        .get(baylee_cards_dsl::counters::SPORE)
}

/// Runs through player 1's turn to player 0's next upkeep and resolves the
/// upkeep triggers.
fn next_own_upkeep(e: &mut Engine<RegistryLookup>) {
    pass_until(e, |e| e.state().turn.active == P1);
    pass_until(e, |e| {
        e.state().turn.active == P0 && e.state().turn.step == Step::Upkeep
    });
    pass_until(e, stack_is_empty);
}

fn offers_saproling(e: &Engine<RegistryLookup>) -> bool {
    let id = on_battlefield(e, P0, card_index("46abaabe-7015-4d82-a5aa-2706b1f51bed"))
        .expect("the Thallid is out");
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("expected priority, got {:?}", e.pending())
    };
    legal.abilities.contains(&(id, 1))
}

/// Thallid (46abaabe-7015-4d82-a5aa-2706b1f51bed) — "At the beginning of your
/// upkeep, put a spore counter on this creature. Remove three spore counters
/// from this creature: Create a 1/1 green Saproling creature token."
#[test]
fn thallid_gains_a_spore_counter_on_each_of_your_upkeeps_and_not_the_opponents() {
    let mut e = ready();
    let start = spores(&e);

    pass_until(&mut e, |e| {
        e.state().turn.active == P1 && e.state().turn.step == Step::Upkeep
    });
    pass_until(&mut e, stack_is_empty);
    assert_eq!(spores(&e), start, "the opponent's upkeep gives none");

    next_own_upkeep(&mut e);
    assert_eq!(spores(&e), start + 1);
}

#[test]
fn thallid_trades_three_spore_counters_for_a_saproling() {
    let mut e = ready();
    while spores(&e) < 2 {
        next_own_upkeep(&mut e);
    }
    assert_eq!(spores(&e), 2);
    assert!(!offers_saproling(&e), "two counters cannot pay for it");
    next_own_upkeep(&mut e);
    assert_eq!(spores(&e), 3);
    assert!(offers_saproling(&e));

    let thallid = on_battlefield(&e, P0, card_index("46abaabe-7015-4d82-a5aa-2706b1f51bed"))
        .expect("the Thallid is out");
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source: thallid,
            ability_index: 1,
        },
    )
    .expect("three counters pay");
    pass_until(&mut e, stack_is_empty);

    assert_eq!(spores(&e), 0, "all three were removed");
    assert!(
        on_battlefield(&e, P0, card_index("46abaabe-7015-4d82-a5aa-2706b1f51bed")).is_some(),
        "the Thallid stays"
    );
    let tokens: Vec<ObjectId> = e
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            e.state()
                .object(*id)
                .is_some_and(|o| !o.is_card() && o.controller == P0)
        })
        .collect();
    assert_eq!(tokens.len(), 1, "one Saproling token: {tokens:?}");
    assert_eq!(pt(&e, tokens[0]), (1, 1));
    assert!(!offers_saproling(&e), "and no counters left for another");
}
