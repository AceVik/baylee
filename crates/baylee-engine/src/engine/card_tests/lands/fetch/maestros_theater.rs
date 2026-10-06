//! `cards/lands/fetch/maestros_theater.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Maestros Theater sacrifices itself when it enters. Its "When you do" is a
/// reflexive triggered ability (CR 603.12), which searches for a basic
/// Island, Swamp or Mountain, puts it onto the battlefield tapped, and gains
/// its controller 1 life. It used to search inside the enter trigger's own
/// resolution, so a Theater bounced in response still fetched.
///
/// Playing the land, letting its trigger resolve, and answering the search
/// proves all three clauses: the Theater leaves, the chosen land arrives, and
/// the life goes up by exactly one.
#[test]
fn maestros_theater_sacrifices_itself_fetches_a_basic_and_gains_one_life() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(71, swamp())
        .hand(0, &[maestros_theater()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let life_before = engine.state().players[0].life;
    let library_before = library_size(&engine, p0);

    let land = in_hand(&engine, p0, maestros_theater()).expect("the Theater is in hand");
    engine
        .apply(p0, PlayerAction::PlayLand { card: land })
        .unwrap();

    // After PlayLand the trigger is on the stack; pass until the search arrives.
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
        "a library search, not a cost"
    );
    // The filler deck is Swamps, so there must be options.
    assert!(!options.is_empty(), "the library holds fetchable basics");
    // The search finds exactly one card (mandatory, not optional).
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

    // The Theater sacrificed itself on the way.
    assert!(
        on_battlefield(&engine, p0, maestros_theater()).is_none(),
        "the Theater sacrificed itself as part of the trigger"
    );
    assert!(
        in_graveyard(&engine, p0, maestros_theater()).is_some(),
        "a sacrificed permanent goes to its owner's graveyard"
    );
    // The fetched land is on the battlefield, tapped.
    assert_eq!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Battlefield)
            .len(),
        1,
        "exactly the fetched basic entered"
    );
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
        "it enters tapped"
    );
    // Library shrank by one.
    assert_eq!(
        library_size(&engine, p0),
        library_before - 1,
        "one card left the library"
    );
    // Life went up by one.
    assert_eq!(
        engine.state().players[0].life,
        life_before + 1,
        "gain 1 life"
    );
}
