//! `cards/creatures/mv_1/maggot_carrier.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Maggot Carrier is a {B} 1/1 Zombie whose whole text is "When this creature
/// enters, each player loses 1 life." Three seats and two copies are the
/// scenario, because a duel cannot tell "each player" from "each opponent" —
/// the testkit's own reason for having `table` — and a single entry cannot
/// tell a per-entry trigger from a one-shot. The first carrier takes every
/// seat to 19, the second — cast off the black the first one left in the pool
/// — takes them all to 18.
#[test]
fn maggot_carrier_makes_each_player_lose_one_life_for_each_entry() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::table(SEED, forest(), 3)
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[maggot_carrier(), maggot_carrier()])
        .life(0, 20)
        .life(1, 20)
        .life(2, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, maggot_carrier());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, maggot_carrier()).is_some(),
        "the {{B}} 1/1 resolved onto the battlefield"
    );
    let lives = |e: &Engine<RegistryLookup>| -> (i32, i32, i32) {
        (
            e.state().players[0].life,
            e.state().players[1].life,
            e.state().players[2].life,
        )
    };
    assert_eq!(
        lives(&engine),
        (19, 19, 19),
        "\"each player loses 1 life\" — the two seats across the table and \
         the one that cast it"
    );

    // The second copy, off the {B} the first left floating: the loss is per
    // entry and not once per game.
    cast_with_floating(&mut engine, p0, maggot_carrier());
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        all_on_battlefield(&engine, p0, maggot_carrier()).len(),
        2,
        "two Maggot Carriers entered"
    );
    assert_eq!(
        lives(&engine),
        (18, 18, 18),
        "and the second entry took another point from every seat"
    );
}
