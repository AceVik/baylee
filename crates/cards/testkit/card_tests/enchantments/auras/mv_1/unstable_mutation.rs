//! `cards/enchantments/auras/mv_1/unstable_mutation.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Unstable Mutation prints three sentences: "Enchant creature", "Enchanted
/// creature gets +3/+3", and "At the beginning of the upkeep of enchanted
/// creature's controller, put a -1/-1 counter on that creature."
///
/// The pump is read on the host and on an Elf beside it, so the static is
/// shown to reach the creature the Aura holds and no other. The trigger is
/// then walked to on its controller's own upkeep and read as a counter
/// (CR 122.1) beside the projected body, so the +3/+3 and the -1/-1 are two
/// separate facts rather than a net +2/+2. Disenchant removes the Aura, and
/// what is left is the distinguishing half of the card's official ruling:
/// the +3/+3 belongs to the continuous effect and goes away with its source
/// (CR 611.3, 613.4c), while the counter is a marker on the permanent and
/// stays (CR 122.1).
#[test]
fn unstable_mutation_pumps_the_host_counts_it_down_at_upkeep_and_leaves_the_counters() {
    let p0 = PlayerId::new(0);
    let disenchant = card_index("a7e97fa9-4b72-4548-b854-5be5f18a6f1a");
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[island(), plains(), plains(), gray_ogre(), llanowar_elves()],
        )
        .hand(0, &[unstable_mutation(), disenchant])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let host = on_battlefield(&engine, p0, gray_ogre()).expect("the Ogre is out");
    let bystander = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    assert_eq!(pt(&engine, host), (2, 2), "a printed 2/2 before the Aura");
    assert_eq!(pt(&engine, bystander), (1, 1), "and a printed 1/1");

    cast_from_hand(&mut engine, p0, unstable_mutation());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the Ogre is a legal host");
    pass_until(&mut engine, stack_is_empty);
    let mutation = on_battlefield(&engine, p0, unstable_mutation()).expect("the Aura resolved");
    assert_eq!(
        engine
            .state()
            .object(mutation)
            .expect("the Aura is an object")
            .attached_to,
        Some(host),
        "the Aura enters attached to the creature it was cast on"
    );
    assert_eq!(pt(&engine, host), (5, 5), "enchanted creature gets +3/+3");
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "and the static reaches the host and no other creature"
    );

    // The trigger belongs to the upkeep of the enchanted creature's
    // controller, so p0's own next upkeep — one turn cycle away.
    pass_until(&mut engine, |e| {
        counters_on(e, host, CounterKind::M1M1) >= 1
    });
    assert_eq!(
        counters_on(&engine, host, CounterKind::M1M1),
        1,
        "\"put a -1/-1 counter on that creature\""
    );
    assert_eq!(
        counters_on(&engine, bystander, CounterKind::M1M1),
        0,
        "and on that creature, not on creatures in general"
    );
    assert_eq!(
        pt(&engine, host),
        (4, 4),
        "a 2/2 with +3/+3 and one -1/-1 is a 4/4, so the counter is real and \
         the pump is still standing"
    );

    // Back in p0's first main, the real removal spell: the +3/+3 is the
    // Aura's continuous effect and ends with it, the counter is on the
    // permanent and does not.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain && at_rest(e, p0)
    });
    cast_from_hand(&mut engine, p0, disenchant);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mutation],
            },
        )
        .expect("the Aura is a legal target");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, unstable_mutation()).is_none(),
        "the Aura is off the battlefield"
    );
    assert!(
        in_graveyard(&engine, p0, unstable_mutation()).is_some(),
        "and in its owner's graveyard"
    );
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "the +3/+3 went away with the Aura"
    );
    assert_eq!(
        counters_on(&engine, host, CounterKind::M1M1),
        1,
        "the -1/-1 counter stayed: \"The -1/-1 counters stay even if the Aura \
         is removed\""
    );
}
