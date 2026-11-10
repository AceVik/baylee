//! `cards/creatures/mv_3/breathstealer.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Breathstealer is a 2/2 Nightstalker for {2}{B} printing one line: "{B}:
/// This creature gets +1/-1 until end of turn."
///
/// Both halves of that pump have to be read off one board, so five Swamps pay
/// the cast and leave exactly the two {B} the ability charges in the pool: the
/// first activation has to leave the creature a 3/1 — power up *and* toughness
/// down, which a test that only looked at power would miss — on mana that was
/// really spent. The second activation is what says the toughness is being
/// subtracted rather than clamped, because a 2/2 pumped twice is a 4/0 and
/// CR 704.5f cannot leave that standing.
#[test]
fn breathstealer_trades_toughness_for_power_until_it_dies() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[breathstealer()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {2}{B} out of the five Swamps, leaving the two {B} the ability charges
    // beside it: CR 500.5 empties a pool at the end of a step, and the whole
    // scenario plays inside this one main phase.
    cast_from_hand(&mut engine, p0, breathstealer());
    pass_until(&mut engine, stack_is_empty);
    let creature = on_battlefield(&engine, p0, breathstealer()).expect("it resolved");
    assert_eq!(pt(&engine, creature), (2, 2), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "five Swamps less the {{2}}{{B}} the cast cost"
    );

    // The pump is no mana ability, so it only lands when it resolves.
    activate(&mut engine, p0, breathstealer(), 0);
    assert!(
        !stack_is_empty(&engine),
        "getting +1/-1 is an activated ability like any other"
    );
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, creature),
        (3, 1),
        "+1/-1 read as both numbers: a (3, 2) would mean the minus was dropped"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and the {{B}} came out of the pool"
    );

    // A second activation is lethal, which is the only reading that says the
    // toughness is really being subtracted rather than floored at one.
    activate(&mut engine, p0, breathstealer(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, breathstealer()).is_none(),
        "a 2/2 pumped twice is a 4/0, which CR 704.5f cannot leave standing"
    );
    assert!(
        in_graveyard(&engine, p0, breathstealer()).is_some(),
        "and a creature that dies goes to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the second {{B}} was paid, and nothing else was left in the pool"
    );
}
