//! `cards/lands/fetch/riveteers_overlook.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Riveteers Overlook's enter trigger sacrifices the land. Its "When you do"
/// is a reflexive triggered ability (CR 603.12), which searches for a basic
/// Swamp, Mountain or Forest, puts it onto the battlefield tapped, and gains
/// 1 life.
///
/// Playing the land and answering the search proves all three clauses: the
/// Overlook leaves, the chosen land arrives tapped, and life goes up by one.
/// `reflexive_tests` plays the bounce in response that stops the search.
#[test]
fn riveteers_overlook_sacrifices_itself_fetches_a_basic_and_gains_one_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(73, swamp())
        .hand(0, &[riveteers_overlook()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let life_before = engine.state().players[0].life;
    let library_before = library_size(&engine, p0);

    let land = in_hand(&engine, p0, riveteers_overlook()).expect("the Overlook is in hand");
    engine
        .apply(p0, PlayerAction::PlayLand { card: land })
        .unwrap();

    // The ETB trigger is on the stack; pass until the search question arrives.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected the library search, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the controller searches");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::SearchLibrary,
        "this is a library search"
    );
    // Filler deck is Swamps — there will be hits.
    assert!(!options.is_empty(), "the library holds fetchable basics");
    assert_eq!((min, max), (1, 1), "exactly one basic is fetched");

    let found = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![found],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // The Overlook sacrificed itself.
    assert!(
        on_battlefield(&engine, p0, riveteers_overlook()).is_none(),
        "the Overlook sacrificed itself"
    );
    assert!(
        in_graveyard(&engine, p0, riveteers_overlook()).is_some(),
        "a sacrificed permanent goes to its owner's graveyard"
    );
    // The fetched land is on the battlefield, tapped.
    let obj = engine
        .state()
        .object(found)
        .expect("the found land is an object");
    assert_eq!(
        obj.zone,
        crate::zone::Zone::Battlefield,
        "the fetched basic entered the battlefield"
    );
    assert!(
        obj.status.contains(crate::object::Status::TAPPED),
        "the card enters tapped"
    );
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "one card left the library"
    );
    assert_eq!(
        engine.state().players[0].life,
        life_before + 1,
        "gain 1 life"
    );
}
