//! `cards/creatures/mv_3/azimaet_drake.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Azimaet Drake — {2}{U}, a 1/3 Drake with flying and "{U}: This creature gets
/// +1/+0 until end of turn. Activate only once each turn."
///
/// One scenario reads all three printed clauses. The body and the keyword are the
/// board it lands on; the pump is the {U} really spent out of a pool three Islands
/// filled, which leaves a 2/3 rather than a 2/1; and the limit is the same offer
/// read twice — *absent* at the end of the turn with blue still floating, so the
/// price cannot be what withheld it, and *present* again on the next turn, so what
/// barred it was "each turn" and not a permanent exhaustion.
#[test]
fn azimaet_drake_pumps_once_a_turn_and_pumps_again_on_the_next() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(71, island())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[azimaet_drake()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Three Islands pay the {2}{U} and two are left standing: the pump's {U}
    // needs a source in the *same* turn, because a pool empties when the step
    // ends (CR 500.5) and everything below plays inside this one main phase.
    let islands = all_on_battlefield(&engine, p0, island());
    assert_eq!(islands.len(), 5, "five Islands were dealt");
    let held = [islands[3], islands[4]];
    tap_mana_where(&mut engine, p0, |id| !held.contains(&id));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Islands, three blue, and the other two still standing"
    );
    cast_with_floating(&mut engine, p0, azimaet_drake());
    pass_until(&mut engine, stack_is_empty);
    let drake = on_battlefield(&engine, p0, azimaet_drake()).expect("the Drake resolved");
    assert_eq!(pt(&engine, drake), (1, 3), "the printed 1/3 body");
    assert!(
        keywords(&engine, drake).contains(KeywordSet::FLYING),
        "and the flying it prints"
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the two Islands held back: the whole price and one spare"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.iter().any(|(src, _)| *src == drake),
        "with {{U}} floating the one line the Drake prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, azimaet_drake(), 0);
    assert!(
        !stack_is_empty(&engine),
        "a pump is no mana ability, so it goes on the stack"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, drake),
        (2, 3),
        "+1/+0: power up and toughness untouched — a (2, 4) would be a \
         toughness the card does not print"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{U}} came out of the pool"
    );

    // The limit, read where only the limit can be the reason: one blue is
    // still floating, so `can_afford` has nothing to withhold the line for.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.abilities.iter().any(|(src, _)| *src == drake),
        "\"Activate only once each turn\" — the {{U}} is still there, so the \
         limit and not the price keeps the line out of the offer: {:?}",
        legal.abilities
    );

    // And it is per *turn*, not once and for all: a turn later the same board
    // offers the same line, with the pump it bought already worn off.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        pt(&engine, drake),
        (1, 3),
        "\"until end of turn\": the pump did not survive the turn"
    );
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "every Island untapped across the turn"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.iter().any(|(src, _)| *src == drake),
        "on the next turn the same line is offered again: {:?}",
        legal.abilities
    );
    activate(&mut engine, p0, azimaet_drake(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, drake), (2, 3), "and it pumps the very same way");
}
