//! `cards/lands/artifacts/goldmire_bridge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Goldmire Bridge prints three lines, and each is a different kind of claim,
/// so one game reads all three. "This land enters tapped" is only visible
/// through a real land drop — a seeded battlefield places a permanent without
/// running its replacement effects — and it costs the mana line a turn, since
/// an untapped-`{T}` ability is not even offered while the land lies down.
/// "Indestructible" needs something that would otherwise destroy it, so the
/// opponent aims a Vindicate at the land, and it is still standing when the
/// sorcery is in a graveyard. Then `{T}: Add {W} or {B}` is played on the next
/// turn: the colour question has to name both printed colours and no others,
/// and the black it produces is the Bridge's own tap and nothing else.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn goldmire_bridge_enters_tapped_survives_vindicate_and_taps_for_either_colour() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[goldmire_bridge()])
        .battlefield(1, &[plains(), plains(), swamp(), swamp()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let bridge = play_land(&mut engine, p0, goldmire_bridge());
    assert!(
        entered_tapped(&engine, bridge),
        "\"This land enters tapped\" — and the land drop is the only way to \
         see it, since `starting_battlefield` would have placed it upright"
    );
    let kinds = types(&engine, bridge);
    assert!(
        kinds.contains(TypeSet::ARTIFACT) && kinds.contains(TypeSet::LAND),
        "an artifact land, which is why \"target permanent\" reaches it: {kinds:?}"
    );
    assert!(
        keywords(&engine, bridge).contains(KeywordSet::INDESTRUCTIBLE),
        "the printed keyword is on the permanent, not only in the card file"
    );

    // p1's turn. The Bridge is still down — a permanent played this turn
    // untaps in its controller's next untap step and not before — so the
    // Vindicate below is the keyword's test and nothing else.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but a target choice")
    };
    assert!(
        options.contains(&bridge),
        "\"target permanent\" is every permanent on the table: {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![bridge],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, goldmire_bridge()).is_some(),
        "a destroy effect cannot destroy an indestructible permanent (CR 702.12b)"
    );
    assert!(
        in_graveyard(&engine, p0, goldmire_bridge()).is_none(),
        "and the land was never put into a graveyard"
    );
    assert!(
        in_graveyard(&engine, p1, vindicate()).is_some(),
        "Vindicate resolved, which is what says the destroy effect happened \
         and was prevented"
    );

    // Back around to p0, whose untap step is what stands the Bridge back up.
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, bridge),
        "the untap step untapped the land that had entered tapped"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "nothing is floating, so the mana below is this tap's alone"
    );

    // Ability 0 is the printed "{T}: Add {W} or {B}."
    activate(&mut engine, p0, goldmire_bridge(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{W}} or {{B}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat is the one that names it");
    assert_eq!(
        options.len(),
        2,
        "the two colours the card prints, and not the five of \"any colour\": \
         {options:?}"
    );
    assert!(
        options.contains(&ManaColor::White) && options.contains(&ManaColor::Black),
        "white and black, either way round: {options:?}"
    );

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the two it offered");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::White),
        0,
        "and the other half of the choice was not produced alongside it"
    );
    assert_eq!(pool.total(), 1, "one mana off one tap");
    assert!(is_tapped(&engine, bridge), "the tap is the whole price");
}
