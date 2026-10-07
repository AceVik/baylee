//! `cards/enchantments/sagas/mv_3/welcome_to.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Welcome to . . .` // `Jurassic Park` (`Coverage::Partial`):
/// "I — For each opponent, up to one target noncreature artifact they control becomes a 0/4
/// Wall artifact creature with defender for as long as you control this Saga.
/// II — Create a 3/3 green Dinosaur creature token with trample. It gains haste until end of turn.
/// III — Destroy all Walls. Exile this Saga, then return it to the battlefield transformed under
/// your control. // `{{T}}`: Add `{{G}}` for each Dinosaur you control."
///
/// Under `Coverage::Partial`, chapters I and II and the graveyard escape grant are omitted.
/// Chapter III and the back face's Dinosaur-scaled mana ability are implemented.
/// The test casts `Welcome to . . .` with a Dinosaur (`Carnage Tyrant`) on the battlefield,
/// advances lore counters to chapter III, resolves the transformation to `Jurassic Park` on face 1,
/// and taps `Jurassic Park` to produce one green mana for the controlled Dinosaur.
#[test]
fn welcome_to_advances_to_chapter_three_and_taps_for_dinosaur_mana() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(102, forest())
        .battlefield(0, &[forest(), forest(), forest(), carnage_tyrant()])
        .hand(0, &[welcome_to()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, welcome_to());
    pass_until(&mut engine, stack_is_empty);

    let saga = on_battlefield(&engine, p0, welcome_to()).expect("Welcome to . . . on battlefield");
    assert_eq!(
        counters_on(&engine, saga, CounterKind::Lore),
        1,
        "enters with one lore counter"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    let saga = on_battlefield(&engine, p0, welcome_to())
        .expect("Welcome to . . . on battlefield in turn 2");
    assert_eq!(
        counters_on(&engine, saga, CounterKind::Lore),
        2,
        "second lore counter added in precombat main phase"
    );

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);

    // Chapter III triggered at the start of turn 3's precombat main phase; resolve it.
    pass_until(&mut engine, stack_is_empty);

    let jp = on_battlefield(&engine, p0, welcome_to()).expect("Jurassic Park on battlefield");
    assert_eq!(
        engine.state().object(jp).expect("object exists").face_index,
        1,
        "transformed to face 1"
    );
    assert!(
        types(&engine, jp).contains(TypeSet::LAND),
        "Jurassic Park is a land"
    );

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        0,
        "mana pool is empty before activating Jurassic Park"
    );

    activate(&mut engine, p0, welcome_to(), 0);

    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        1,
        "produced 1 green mana for 1 controlled Dinosaur (Carnage Tyrant)"
    );
}
