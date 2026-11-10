//! `cards/sorceries/mv_2/demonic_tutor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn demonic_tutor() -> CardIndex {
    card_index("82004860-e589-4e38-8d61-8c0210e4ea39")
}

/// Demonic Tutor: "Search your library for a card, put that card into your
/// hand, then shuffle." Nothing narrows the search, so the whole library is
/// the menu and a Lightning Bolt buried in a deck of Forests can be named;
/// it lands in hand, and the rest of the library is shuffled (a library left
/// in its old order is the unshuffled one) while the opponent's library is
/// not touched.
#[test]
fn demonic_tutor_fetches_the_named_card_and_shuffles_the_library() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[demonic_tutor(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bolt = hand_to_library_top(&mut engine, p0, lightning_bolt());
    let before: Vec<ObjectId> = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    let theirs_before: Vec<ObjectId> = engine.state().zones.list(ZoneLocation::Library(p1)).clone();
    assert_eq!(before.last(), Some(&bolt), "the Bolt is on top of the deck");

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, demonic_tutor());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the search")
    };
    assert_eq!(player, p0);
    assert_eq!(
        (min, max),
        (1, 1),
        "\"a card\": exactly one, not an \"up to\""
    );
    assert_eq!(
        options.len(),
        before.len(),
        "no filter: every card of the library is on the menu"
    );
    assert!(options.contains(&bolt));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bolt],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_hand(&engine, p0, lightning_bolt()).is_some(),
        "into the hand"
    );
    let after: Vec<ObjectId> = engine.state().zones.list(ZoneLocation::Library(p0)).clone();
    assert_eq!(after.len(), before.len() - 1, "one card left the library");
    assert!(after.iter().all(|id| before.contains(id)) && !after.contains(&bolt));
    assert_ne!(
        after,
        before[..before.len() - 1].to_vec(),
        "\"then shuffle\": the 59 that remain are no longer in their old order"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Library(p1)).clone(),
        theirs_before,
        "the opponent's library is not shuffled"
    );
}
