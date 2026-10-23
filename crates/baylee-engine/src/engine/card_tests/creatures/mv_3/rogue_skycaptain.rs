//! `cards/creatures/mv_3/rogue_skycaptain.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn rogue_skycaptain() -> CardIndex {
    card_index("e600136a-8b4f-4725-a29e-021e0cfc7d0a")
}

/// The price the pending "pay?" question names, as a mana value.
fn price(engine: &Engine<RegistryLookup>) -> u32 {
    let Pending::YesNo {
        prompt: crate::choice::YesNoPrompt::PayMana { cost },
        ..
    } = engine.pending()
    else {
        panic!("a payment question, got {:?}", engine.pending())
    };
    cost.cmc()
}

/// Rogue Skycaptain ({2}{R} 3/4 Creature — Human Rogue Mercenary):
/// - Flying
/// - "At the beginning of your upkeep, put a wage counter on this creature. You may pay {2} for each wage counter on it. If you don't, remove all wage counters from this creature and an opponent gains control of it."
#[test]
fn rogue_skycaptain_accumulates_wage_counters_and_changes_control_when_unpaid() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[rogue_skycaptain(), mountain(), mountain()])
        .start();
    keep_mulligans(&mut engine);

    let captain = on_battlefield(&engine, p0, rogue_skycaptain()).unwrap();
    assert!(keywords(&engine, captain).contains(KeywordSet::FLYING));

    // Turn 1 upkeep: pay {2} for the first wage counter.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert_eq!(price(&engine), 2, "{{2}} for the one wage counter");
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    reach_main_phase(&mut engine, p0);

    assert_eq!(engine.state().object(captain).unwrap().controller, p0);
    assert_eq!(
        engine
            .state()
            .object(captain)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::WAGE),
        1
    );

    // On P0's following upkeep, decline to pay {4} -> wage counters removed and P1 gains control.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain && stack_is_empty(e)
    });
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && matches!(e.pending(), Pending::YesNo { .. })
    });
    assert_eq!(price(&engine), 4, "{{2}} for each of two wage counters");
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(engine.state().object(captain).unwrap().controller, p1);
    assert_eq!(
        engine
            .state()
            .object(captain)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::WAGE),
        0
    );
}
