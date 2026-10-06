//! `cards/creatures/mv_4/vedalken_archmage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Vedalken Archmage — {2}{U}{U}, a 0/2 Vedalken Wizard printing "Whenever you
/// cast an artifact spell, draw a card." The Archmage's own arrival is the
/// control: it is a creature spell and no artifact, so the library must not
/// move for it, which is what tells the printed filter from a trigger that
/// fires on any spell its controller casts. The Sol Ring cast afterwards, off
/// the mana left over from the same main phase, is the trigger; the Sol Ring
/// the opponent then casts is the other half of "you", and an artifact spell
/// cast across the table has to draw the Archmage's controller nothing.
#[test]
fn vedalken_archmage_draws_for_your_artifact_spell_and_for_nothing_else() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(); 6])
        .hand(0, &[vedalken_archmage(), quiet_artifact()])
        .battlefield(1, &[island(); 3])
        .hand(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Six Islands into the pool, so both spells below are paid out of mana the
    // board really made: the Archmage's {2}{U}{U} and the Sol Ring's {1} are
    // one payment inside this single main phase (CR 500.5).
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Islands, six blue"
    );

    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_with_floating(&mut engine, p0, vedalken_archmage());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let archmage = on_battlefield(&engine, p0, vedalken_archmage()).expect("the Archmage resolved");
    assert_eq!(pt(&engine, archmage), (0, 2), "the body the card prints");
    assert!(
        !types(&engine, archmage).contains(TypeSet::ARTIFACT),
        "a Vedalken Wizard is a creature and no artifact, so its own cast is \
         the control and not the trigger"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before,
        "the card waits for an *artifact* spell, and this was not one"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "one card left the hand and none came back"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{2}}{{U}}{{U}} came out of the pool"
    );

    // The Sol Ring: an artifact spell, which is the whole of what the card is
    // about.
    let library_before = library_size(&engine, p0);
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_with_floating(&mut engine, p0, quiet_artifact());
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "the Sol Ring resolved"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "\"whenever you cast an artifact spell, draw a card\": one card off the top"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before,
        "one card cast and one card drawn, so the hand is the size it was"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the Sol Ring's {{1}} came out of the same pool"
    );

    // And the "you" half. The same spell cast by the opponent is nobody's
    // trigger, and the readings are taken on p0, whose Archmage is the only
    // one on the table.
    reach_their_main_phase(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    let my_library = library_size(&engine, p0);
    let my_hand = engine.state().zones.list(ZoneLocation::Hand(p0)).len();

    cast_with_floating(&mut engine, p1, quiet_artifact());
    pass_until(&mut engine, |e| at_rest(e, p1));

    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "the opponent's Sol Ring resolved on their side of the table"
    );
    assert_eq!(
        library_size(&engine, p0),
        my_library,
        "\"whenever *you* cast\": an artifact spell cast across the table draws \
         the Archmage's controller nothing"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        my_hand,
        "and the Archmage's controller's hand never moved"
    );
}
