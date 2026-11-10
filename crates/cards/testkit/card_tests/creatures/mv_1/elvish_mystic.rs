//! `cards/creatures/mv_1/elvish_mystic.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Elvish Mystic prints one line — `{T}: Add {G}` — on a 1/1 Elf Druid that
/// costs {G}, so the whole card is a price and a colour, and both are read off
/// one board. A copy is cast off a single Forest and the pool goes from one
/// green to none, which is the printed {G} and nothing cheaper; then the other
/// copy's tap symbol is pressed and exactly one green arrives with no stack
/// entry (CR 605.3b). The Elf that taps is the one already standing when the
/// turn began, because a creature cast this turn may not pay a `{T}` cost
/// (CR 302.6) — so the arrival proves the cost and the body, the standing Elf
/// proves the mana, and neither can stand in for the other.
#[test]
fn elvish_mystic_costs_one_green_and_taps_for_one_green() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[elvish_mystic(), forest()])
        .hand(0, &[elvish_mystic()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // The Forest pays, and both Elves are kept out of it: the one standing is
    // the source the second half presses, and tapping it here would leave
    // nothing to tap there.
    tap_all_mana_but(&mut engine, p0, Some(elvish_mystic()));
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "one Forest is one green, and it is the whole of the pool"
    );

    let arrival = in_hand(&engine, p0, elvish_mystic()).expect("a second Elf is in hand");
    cast_with_floating(&mut engine, p0, elvish_mystic());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine
            .state()
            .object(arrival)
            .expect("the cast Elf arrived somewhere")
            .zone,
        Zone::Battlefield,
        "{{G}} was paid, so the Elf is on the battlefield"
    );
    assert_eq!(pt(&engine, arrival), (1, 1), "a printed 1/1");
    assert!(
        types(&engine, arrival).contains(TypeSet::CREATURE),
        "and it is a creature"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the green that floated is the {{G}}, and it is the whole of it: a \
         cheaper Elf would have left mana behind"
    );

    // `{T}: Add {G}` — the only ability the card prints, so index 0.
    activate(&mut engine, p0, elvish_mystic(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Green),
        1,
        "one green, and the color the card names"
    );
    assert_eq!(pool.total(), 1, "and nothing else came with it");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    let tapped = all_on_battlefield(&engine, p0, elvish_mystic())
        .into_iter()
        .filter(|id| is_tapped(&engine, *id))
        .count();
    assert_eq!(
        tapped, 1,
        "{{T}} is the whole price, so exactly one of the two Elves is tapped"
    );
}
