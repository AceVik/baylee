//! `cards/lands/artifacts/slagwoods_bridge.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Slagwoods Bridge prints three lines and one board reads all three. It is
/// *played* rather than seeded, so "This land enters tapped" is the game's own
/// entry replacement and not a placement, and the `{T}: Add {R} or {G}` is read
/// only on the following turn, because a land that arrived tapped has nothing
/// to untap until then. Indestructible is the line a keyword read cannot settle
/// by itself, so an opponent's Vindicate — the one spell in the pool that says
/// "destroy target permanent" — is aimed at the artifact land whose type line
/// invites exactly that.
#[test]
fn slagwoods_bridge_enters_tapped_survives_destruction_and_taps_for_red_or_green() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[slagwoods_bridge()])
        .battlefield(1, &[plains(), swamp(), plains()])
        .hand(1, &[vindicate()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Played, not seeded: `starting_battlefield` places a permanent with
    // `Cause::Setup`, which runs no entry replacement — the one reading this
    // card cannot be tested under.
    let bridge = play_land(&mut engine, p0, slagwoods_bridge());
    assert!(
        entered_tapped(&engine, bridge),
        "\"This land enters tapped\""
    );
    let kinds = types(&engine, bridge);
    assert!(
        kinds.contains(TypeSet::ARTIFACT) && kinds.contains(TypeSet::LAND),
        "an artifact land, which is why \"destroy target permanent\" wants \
         it at all: {kinds:?}"
    );
    assert!(
        keywords(&engine, bridge).contains(KeywordSet::INDESTRUCTIBLE),
        "\"Indestructible\""
    );

    // The keyword has to do something, so the other seat pays {1}{W}{B} for
    // the one spell in the pool that destroys a permanent.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, vindicate());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        unreachable!("the predicate just matched")
    };
    assert!(
        options.contains(&bridge),
        "the Bridge is a permanent Vindicate may name: {options:?}"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![bridge],
            },
        )
        .expect("the Bridge was one of the targets it offered");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, slagwoods_bridge()).is_some(),
        "CR 702.12b: an indestructible permanent is not destroyed"
    );
    assert!(
        in_graveyard(&engine, p0, slagwoods_bridge()).is_none(),
        "and a destroy that is refused never puts the card in a graveyard"
    );

    // A turn later, which is the first moment the land is untapped: an entry
    // that tapped it gives nothing in the turn it arrived, and the untap step
    // running is what says the mana below belongs to the Bridge itself.
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, bridge),
        "the untap step ran, so the Bridge is a live source again"
    );
    activate(&mut engine, p0, slagwoods_bridge(), 0);
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        panic!(
            "\"Add {{R}} or {{G}}\" is a question, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that tapped chooses");
    assert!(
        options.contains(&ManaColor::Red) && options.contains(&ManaColor::Green),
        "both of the printed colours are on offer: {options:?}"
    );
    assert_eq!(
        options.len(),
        2,
        "red or green and nothing else — no blue, and colourless is no colour \
         at all (CR 105.4): {options:?}"
    );
    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Red))
        .expect("red was one of the two it offered");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Red),
        1,
        "the colour that was named, and not a default"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "the other half of the choice bought nothing"
    );
    assert_eq!(pool.total(), 1, "one mana off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack"
    );
    assert!(is_tapped(&engine, bridge), "the Bridge paid its own {{T}}");
}
