//! `cards/creatures/mv_5/arctic_wolves.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn arctic_wolves() -> CardIndex {
    card_index("c9e7218f-07f2-46f3-bc63-eb5c0d32538b")
}

/// `Arctic Wolves` ({3}{G}{G}, 4/5 Creature — Wolf) draws a card when it enters
/// the battlefield and has cumulative upkeep `{2}`.
#[test]
fn arctic_wolves_draws_a_card_on_entry_and_has_cumulative_upkeep() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[arctic_wolves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_from_hand(&mut engine, p0, arctic_wolves());
    pass_until(&mut engine, stack_is_empty);

    let hand_after = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert_eq!(hand_after, hand_before);
    assert!(on_battlefield(&engine, p0, arctic_wolves()).is_some());
}

/// The cost the Wolves ask for at the beginning of their controller's upkeep
/// (the `PayTax` question), after asserting who is asked and when.
#[track_caller]
fn wolves_upkeep_question(engine: &mut Engine<RegistryLookup>, seat: PlayerId) -> u16 {
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { .. },
                ..
            }
        )
    });
    let Pending::YesNo {
        player,
        prompt: YesNoPrompt::PayTax { mana },
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, seat, "the upkeep cost is asked of the controller");
    assert_eq!(
        engine.state().turn.step,
        crate::turn::Step::Upkeep,
        "at the beginning of the upkeep"
    );
    mana
}

fn wolves_age_counters(engine: &Engine<RegistryLookup>, id: ObjectId) -> u16 {
    engine
        .state()
        .object(id)
        .map_or(0, |o| o.counters.get(baylee_cards_dsl::counters::AGE))
}

/// Cumulative upkeep {2} (CR 702.24a), played: the first upkeep puts one age
/// counter on and asks {2}, which two Forests pay; the next puts a second on
/// and asks {4}, and declined, the Wolves are sacrificed.
#[test]
fn arctic_wolves_cumulative_upkeep_grows_and_sacrifices_when_unpaid() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[arctic_wolves(), forest(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    let wolves = on_battlefield(&engine, p0, arctic_wolves()).expect("the Wolves are seated");
    let forests = all_on_battlefield(&engine, p0, forest());
    assert_eq!(forests.len(), 4);

    let mana = wolves_upkeep_question(&mut engine, p0);
    assert_eq!(
        wolves_age_counters(&engine, wolves),
        1,
        "the counter goes on first"
    );
    assert_eq!(mana, 2, "{{2}} for one age counter");
    engine
        .apply(p0, PlayerAction::YesNo(true))
        .expect("paying is one of the two answers");
    tap_mana_where(&mut engine, p0, |id| forests[..2].contains(&id));
    if engine.payment_window().is_some() {
        engine
            .apply(p0, PlayerAction::PassPriority)
            .expect("the window closes on a pool that covers the cost");
    }
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, arctic_wolves()).is_some(),
        "paid, the Wolves stay"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);

    let mana = wolves_upkeep_question(&mut engine, p0);
    assert_eq!(wolves_age_counters(&engine, wolves), 2, "a second counter");
    assert_eq!(mana, 4, "{{2}} for each of the two counters");
    engine
        .apply(p0, PlayerAction::YesNo(false))
        .expect("declining is the other answer");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p0, arctic_wolves()).is_some(),
        "unpaid, the Wolves are sacrificed"
    );
}

/// The upkeep belongs to the Wolves' controller alone: after the first one is
/// paid, the opponent's whole turn passes without a counter or a question
/// (`pass_until` panics on a `PayTax` it was not told to expect).
#[test]
fn arctic_wolves_ask_nothing_on_the_opponents_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[arctic_wolves(), forest(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    let wolves = on_battlefield(&engine, p0, arctic_wolves()).unwrap();
    let forests = all_on_battlefield(&engine, p0, forest());

    assert_eq!(wolves_upkeep_question(&mut engine, p0), 2);
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    tap_mana_where(&mut engine, p0, |id| forests[..2].contains(&id));
    if engine.payment_window().is_some() {
        engine.apply(p0, PlayerAction::PassPriority).unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        wolves_age_counters(&engine, wolves),
        1,
        "the opponent's upkeep put no age counter on the Wolves"
    );
    assert!(on_battlefield(&engine, p0, arctic_wolves()).is_some());
}
