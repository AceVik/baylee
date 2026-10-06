//! `cards/lands/filter/desolate_mire.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Desolate Mire prints one line and no other: "{1}, {T}: Add {W}{B}." Both
/// halves of that price are what this scenario reads. The `{1}` is why the
/// ability is *not* in the offer while the Mire stands untapped and nothing
/// floats — `legal.abilities` is filtered through `can_afford`, which reads
/// the pool and not the untapped board — and it is why `tap_all_mana` is the
/// wrong helper here: the printed price is more than the land's own tap
/// symbol, so the Mire has to be named as the thing kept back while the
/// Forest pays. The two named colours then arrive together off one
/// activation with no colour question asked, because the card prints
/// {W}{B} rather than "one mana of any colour".
#[test]
fn desolate_mire_charges_one_mana_for_its_two_named_colours() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[desolate_mire(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mire = on_battlefield(&engine, p0, desolate_mire()).expect("the Mire is on the table");
    let payer = on_battlefield(&engine, p0, forest()).expect("a Forest stands beside it");
    let offered = |engine: &Engine<RegistryLookup>| -> bool {
        matches!(
            engine.pending(),
            Pending::Priority { legal, .. } if legal.abilities.contains(&(mire, 0))
        )
    };
    assert!(
        !offered(&engine),
        "the Mire is untapped, but {{1}} is half its price and `can_afford` \
         reads the pool rather than the board: {:?}",
        engine.pending()
    );
    assert!(!is_tapped(&engine, mire), "and no price has been paid yet");

    // The Forest pays the {1}; the Mire is kept back because its price is not
    // its own tap alone, so `tap_all_mana` would have spent the ability this
    // test is about to press.
    tap_mana_except(&mut engine, p0, mire);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one green from the Forest, and the Mire itself made nothing"
    );
    assert!(
        is_tapped(&engine, payer),
        "the Forest is the source that paid"
    );
    assert!(!is_tapped(&engine, mire), "and the Mire is still standing");
    assert!(offered(&engine), "so {{1}}, {{T}} is affordable at last");

    activate(&mut engine, p0, desolate_mire(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "the {{W}} the card prints"
    );
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "and the {{B}} printed beside it"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "the green went to the {{1}}: two mana in the pool and not three"
    );
    assert_eq!(pool.total(), 2, "one activation, two mana");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability never uses the stack"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "and the seat holds priority again, got {:?}",
        engine.pending()
    );
    assert!(
        is_tapped(&engine, mire),
        "the tap was the other half of the printed price"
    );
}
