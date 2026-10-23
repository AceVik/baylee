//! `cards/creatures/mv_1/mwonvuli_ooze.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::counters;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mwonvuli_ooze() -> CardIndex {
    card_index("42b2c6d0-e0da-41b0-8876-f17dbe5db79a")
}

/// A game on turn 1 with the Ooze and four Forests out, run to the first
/// question of P0's upkeep.
fn ooze_with_forests() -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[mwonvuli_ooze(), forest(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut e);
    e
}

/// The tax question of P0's upkeep on `turn`, and the generic mana it names.
fn upkeep_price(e: &mut Engine<RegistryLookup>, turn: u32) -> u16 {
    pass_until(e, |e| {
        e.state().turn.number == turn && matches!(e.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayTax { mana },
        ..
    } = e.pending().clone()
    else {
        panic!("the upkeep asks to pay mana: {:?}", e.pending())
    };
    assert_eq!(player, P0);
    mana
}

/// Says yes and pays by tapping `n` of P0's untapped Forests.
#[track_caller]
fn pay_with_forests(e: &mut Engine<RegistryLookup>, n: usize) {
    let forests: Vec<ObjectId> = e
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            e.state().object(*id).is_some_and(|o| {
                o.controller == P0
                    && o.card.is_some_and(|c| c.index == forest())
                    && !o.status.contains(Status::TAPPED)
            })
        })
        .take(n)
        .collect();
    assert_eq!(forests.len(), n, "enough untapped Forests to pay with");
    e.apply(P0, PlayerAction::YesNo(true)).unwrap();
    e.apply(
        P0,
        PlayerAction::PayWindow {
            steps: forests
                .into_iter()
                .map(|source| PlayerAction::ActivateManaAbility { source })
                .collect(),
        },
    )
    .expect("the Forests pay");
}

/// Mwonvuli Ooze (42b2c6d0-e0da-41b0-8876-f17dbe5db79a) — "Cumulative upkeep
/// {2}": the first upkeep puts an age counter and asks {2}, paid by two
/// Forests (the other two stay untapped) and the Ooze lives; the opponent's
/// upkeep adds nothing; the next of its controller's upkeeps asks {4} (two
/// counters, {2} each) and, declined, sacrifices it.
#[test]
fn mwonvuli_ooze_asks_two_more_each_of_its_controllers_upkeeps() {
    let mut e = ooze_with_forests();
    let ooze = on_battlefield(&e, P0, mwonvuli_ooze()).expect("the Ooze is out");

    assert_eq!(upkeep_price(&mut e, 1), 2, "one age counter, {{2}}");
    assert_eq!(counters_on(&e, ooze, counters::AGE), 1);
    pay_with_forests(&mut e, 2);
    pass_until(&mut e, stack_is_empty);
    assert!(
        on_battlefield(&e, P0, mwonvuli_ooze()).is_some(),
        "paid, so it stays"
    );

    reach_their_main_phase(&mut e, P1);
    assert_eq!(
        counters_on(&e, ooze, counters::AGE),
        1,
        "P1's upkeep is not yours"
    );

    assert_eq!(upkeep_price(&mut e, 3), 4, "two age counters, {{2}} each");
    assert_eq!(counters_on(&e, ooze, counters::AGE), 2);
    e.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert!(
        in_graveyard(&e, P0, mwonvuli_ooze()).is_some(),
        "unpaid: sacrificed"
    );
}

/// Mwonvuli Ooze — "sacrifice it unless you pay": declined at the very first
/// upkeep it is sacrificed at once and the Forests stay untapped.
#[test]
fn mwonvuli_ooze_is_sacrificed_when_its_first_upkeep_goes_unpaid() {
    let mut e = ooze_with_forests();

    assert_eq!(upkeep_price(&mut e, 1), 2);
    e.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut e, stack_is_empty);

    assert!(in_graveyard(&e, P0, mwonvuli_ooze()).is_some());
    assert!(on_battlefield(&e, P0, mwonvuli_ooze()).is_none());
}
