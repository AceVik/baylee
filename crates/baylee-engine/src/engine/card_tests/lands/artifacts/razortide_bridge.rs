//! `cards/lands/artifacts/razortide_bridge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Razortide Bridge prints three sentences and this board holds all three: an
/// artifact land that enters tapped, is indestructible, and taps for {W} *or*
/// {U} — a real question, where a basic's single colour would ask nothing at
/// all. The land is played for real rather than seeded onto the battlefield,
/// because `starting_battlefield` places a permanent without an entry and a
/// bridge that never entered would arrive untapped, proving nothing about the
/// sentence it prints. The indestructible half is read off a Vindicate aimed
/// at it: the spell resolves (it lies in its owner's graveyard afterwards) and
/// destroys nothing (CR 702.12b), which is not a target that fizzled.
#[test]
fn razortide_bridge_enters_tapped_survives_vindicate_and_taps_for_either_colour() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(7, forest())
        .battlefield(1, &[plains(), swamp(), plains()])
        .hand(1, &[vindicate()])
        .hand(0, &[razortide_bridge()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let bridge = play_land(&mut engine, p0, razortide_bridge());
    assert!(
        entered_tapped(&engine, bridge),
        "\"This land enters tapped\""
    );
    assert!(
        keywords(&engine, bridge).contains(KeywordSet::INDESTRUCTIBLE),
        "and the keyword reaches the permanent, not only the card file"
    );

    // The other side of the table tries to destroy it, which is the only
    // reading of indestructible that is not a re-read of the card text.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p1, "the seat that cast the spell names its target");
    assert!(
        options.contains(&bridge),
        "\"target permanent\" reaches an indestructible artifact land: {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![bridge],
            },
        )
        .expect("the permanent the offer named is a legal answer");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, razortide_bridge()).is_some(),
        "CR 702.12b: a permanent with indestructible can't be destroyed"
    );
    assert!(
        in_graveyard(&engine, p1, vindicate()).is_some(),
        "and the spell really resolved — one still on the stack would leave \
         the land standing for the wrong reason"
    );

    // Back into p0's own turn: the untap step is what stands the land up
    // again, and until it does the printed {T} is not even offered (CR 502.3).
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, bridge),
        "the untap step ran, so the tap below is a choice and not a refusal"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the bridge is this seat's only mana source and nothing floats yet"
    );

    // Ability 0 is the printed "{T}: Add {W} or {U}".
    activate(&mut engine, p0, razortide_bridge(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{W}} or {{U}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped the land names the colour");
    assert!(
        options.contains(&ManaColor::White) && options.contains(&ManaColor::Blue),
        "both colours the card prints: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "and exactly those two — a Mox Diamond's \"any color\" would offer \
         five here: {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Blue))
        .expect("blue was one of the colours it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Blue),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::White),
        0,
        "one mana of one colour: the other arm of the choice was not paid too"
    );
    assert_eq!(pool.total(), 1, "one tap, one mana");
    assert!(is_tapped(&engine, bridge), "which tapped the land itself");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting"
    );
}
