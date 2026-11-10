//! `cards/lands/utility/oakhollow_village.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Oakhollow Village prints `{{T}}: Add {{C}}.`, `{{T}}: Add {{G}}. Spend this mana only to cast a creature spell.`, and `{{G}}, {{T}}: Put a +1/+1 counter on each Frog, Rabbit, Raccoon, or Squirrel you control that entered the battlefield this turn.`
///
/// With Oakhollow Village and a `forest()` on the battlefield under `PlayerId::new(0)`, floating `{{G}}` while keeping Oakhollow Village untapped shows that all three abilities are offered; the counter ability is `oakhollow_village_counters_only_this_turns_arrivals`.
/// Activating ability 1 produces green mana restricted to creature spells, which appears in `pool.restricted()` rather than general available mana, and leaves the land tapped.
#[test]
fn oakhollow_village_produces_creature_restricted_green_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[oakhollow_village(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let village = on_battlefield(&engine, p0, oakhollow_village()).expect("village on battlefield");

    // Float {{G}} from Forest while keeping Oakhollow Village untapped.
    tap_mana_except(&mut engine, p0, village);
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
    assert!(!is_tapped(&engine, village));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        legal.abilities.contains(&(village, 0)),
        "ability 0 ({{T}}: Add {{C}}) is offered"
    );
    assert!(
        legal.abilities.contains(&(village, 1)),
        "ability 1 ({{T}}: Add {{G}} restricted) is offered"
    );
    assert!(
        legal.abilities.contains(&(village, 2)),
        "ability 2 ({{G}}, {{T}}: the counters) is offered with {{G}} floating"
    );

    activate(&mut engine, p0, oakhollow_village(), 1);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "only the forest's unrestricted green mana appears in the available pool"
    );
    assert_eq!(pool.restricted().len(), 1);
    assert_eq!(pool.restricted()[0].amount, 1);
    assert_eq!(pool.restricted()[0].color, ManaColor::Green);
    assert_eq!(pool.total(), 2);
    assert!(is_tapped(&engine, village));
}

/// Oakhollow Village's third ability: "{G}, {T}: Put a +1/+1 counter on each
/// Frog, Rabbit, Raccoon, or Squirrel you control that entered the
/// battlefield this turn."
///
/// Two Frogs (Wretched Anurid, a Zombie Frog Beast) on one board: one was
/// there when the turn began and one is cast this turn. Only the new one is
/// counted, which is the whole of "that entered the battlefield this turn";
/// a filter without it would put a counter on both.
#[test]
fn oakhollow_village_counters_only_this_turns_arrivals() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                oakhollow_village(),
                forest(),
                swamp(),
                swamp(),
                wretched_anurid(),
            ],
        )
        .hand(0, &[wretched_anurid()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    let old = on_battlefield(&engine, p0, wretched_anurid()).expect("the Frog already here");

    let swamps = all_on_battlefield(&engine, p0, swamp());
    tap_mana_where(&mut engine, p0, |id| swamps.contains(&id));
    cast_with_floating(&mut engine, p0, wretched_anurid());
    pass_until(&mut engine, |e| at_rest(e, p0));
    let new = all_on_battlefield(&engine, p0, wretched_anurid())
        .into_iter()
        .find(|id| *id != old)
        .expect("the Frog cast this turn");

    let the_forest = on_battlefield(&engine, p0, forest()).expect("p0's Forest");
    tap_mana_where(&mut engine, p0, |id| id == the_forest);
    activate(&mut engine, p0, oakhollow_village(), 2);
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert_eq!(
        counters_on(&engine, new, CounterKind::P1P1),
        1,
        "the Frog that entered this turn"
    );
    assert_eq!(
        counters_on(&engine, old, CounterKind::P1P1),
        0,
        "and not the one that was already here"
    );
    let village = on_battlefield(&engine, p0, oakhollow_village()).expect("the village");
    assert!(is_tapped(&engine, village), "{{T}} was part of the cost");
}
