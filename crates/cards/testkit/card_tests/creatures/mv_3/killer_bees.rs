//! `cards/creatures/mv_3/killer_bees.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Killer Bees is a `{1}{G}{G}` 0/1 Insect with flying and one activated
/// line: "{G}: This creature gets +1/+1 until end of turn."
///
/// The word the card turns on is *repeatable*: the pump has no `{T}` and no
/// once-a-turn clause, so one pool of green has to buy two counters rather
/// than one — a one-shot pump would leave a 1/2 after the first activation
/// and pass a test that stopped there. Five Forests pay the cast and leave
/// exactly the two `{G}` the scenario then spends, which is also why the
/// pool is read after each step: three green gone on the cast, one per
/// activation, and nothing left at the end. The empty pool is the control
/// for the last claim — with no mana floating the ability is not offered at
/// all, because `can_afford` reads the pool and not the Forests beside it.
#[test]
fn killer_bees_pumps_itself_once_per_green_paid() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[killer_bees()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Five Forests into the pool first: a cast is only claimable against
    // what is actually floating (rule 6), and the same mana is what the two
    // activations below are read against.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Forests, five green — the Bees are a card in hand, not a source"
    );
    cast_with_floating(&mut engine, p0, killer_bees());
    pass_until(&mut engine, stack_is_empty);

    let bees = on_battlefield(&engine, p0, killer_bees()).expect("the Bees resolved");
    assert_eq!(
        pt(&engine, bees),
        (0, 1),
        "the printed body, before anything pumps it"
    );
    assert!(
        keywords(&engine, bees).contains(KeywordSet::FLYING),
        "the printed flying line reaches the permanent"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{1}}{{G}}{{G}} came out of the pool and left two green"
    );

    // Ability 0 is the printed "{G}: This creature gets +1/+1 until end of
    // turn." Its price is mana and not its own tap, so `tap_all_mana` never
    // pressed it and the creature is still standing to be pumped.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(bees, 0)),
        "with two green floating the pump is affordable, so it is offered: {:?}",
        legal.abilities
    );
    assert!(
        !is_tapped(&engine, bees),
        "the price is not the tap symbol; nothing has tapped the creature"
    );

    // The first payment, and the order CR 601.2h puts on it: the ability is
    // on the stack and the green is already spent, while the body is still
    // the printed 0/1 because nothing has resolved yet.
    activate(&mut engine, p0, killer_bees(), 0);
    assert!(
        !stack_is_empty(&engine),
        "the pump is no mana ability, so it waits on the stack"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{G}} is paid as the ability is activated"
    );
    assert_eq!(
        pt(&engine, bees),
        (0, 1),
        "and the pump has not happened yet"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, bees),
        (1, 2),
        "+1/+1 on the creature the ability names, and no other"
    );

    // The second payment is the whole card: the line is repeatable, so the
    // counter stacks instead of replacing itself.
    activate(&mut engine, p0, killer_bees(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, bees),
        (2, 3),
        "a second {{G}} buys a second +1/+1 — a one-shot pump would read 1/2"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "both green are spent and the pool is empty"
    );
    assert!(
        keywords(&engine, bees).contains(KeywordSet::FLYING),
        "and the printed flying survives every layer of the pump"
    );

    // The emptied pool is the control: an unaffordable ability is absent
    // from the offer rather than refused, so an empty pool means no pump.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.contains(&(bees, 0)),
        "with nothing floating the {{G}} cannot be paid, so the pump is not \
         offered at all — the five tapped Forests beside it are not mana the \
         engine may spend: {:?}",
        legal.abilities
    );
}
