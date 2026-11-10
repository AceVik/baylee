//! `cards/instants/mv_1/battlegrowth.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Battlegrowth is `{G}` for one sentence — "Put a +1/+1 counter on target
/// creature" — and the board makes each of its words answerable somewhere
/// different. "Target creature" is any creature, so the menu holds both of my
/// Elves *and* the one across the table while the Forest beside them is no
/// target at all; the named creature is the only one that changes, with the
/// second Elf and the Elf across the table as the two controls that say so;
/// and the counter is read twice — as a projected (2, 2) and as the `+1/+1`
/// counter on the permanent — because a pump and a counter are different
/// claims. The activation order is the card's too: CR 601.2c names the target
/// and CR 601.2h pays the `{G}` afterwards, so the mana is spent by the pool
/// before the question is answered rather than after it.
#[test]
fn battlegrowth_puts_its_counter_on_the_creature_it_targets_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), llanowar_elves(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[battlegrowth()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(
        elves.len(),
        2,
        "two Elves, one of which stays a printed 1/1"
    );
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    let land = on_battlefield(&engine, p0, forest()).expect("the Forest is out");
    assert_eq!(pt(&engine, host), (1, 1), "a printed 1/1 before the spell");

    // One Forest into the pool, and both Elves named as kept back: the {@G}
    // is read off the pool rather than off the untapped lands, and a creature
    // that tapped for it would leave one fewer 1/1 to read the filter against.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one Forest, one green"
    );
    cast_with_floating(&mut engine, p0, battlegrowth());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the caster is the one that aims it");
    assert_eq!((min, max), (1, 1), "exactly one creature");
    assert!(
        options.contains(&host) && options.contains(&bystander),
        "\"target creature\" reaches either of my own: {options:?}"
    );
    assert!(
        options.contains(&theirs),
        "and any creature across the table: {options:?}"
    );
    assert!(
        !options.contains(&land),
        "a land is no creature, so it is no target: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the Elf the question offered is the one it is aimed at");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        counters_on(&engine, host, baylee_cards_dsl::CounterKind::P1P1),
        1,
        "one +1/+1 counter, on the permanent that was named"
    );
    assert_eq!(
        pt(&engine, host),
        (2, 2),
        "and the counter is a body: 1/1 plus one of each"
    );
    assert_eq!(
        counters_on(&engine, bystander, baylee_cards_dsl::CounterKind::P1P1),
        0,
        "the Elf nobody named never got one"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "and is still the 1/1 it was printed as"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "the counter lands only where the spell was aimed, never across the table"
    );
}
