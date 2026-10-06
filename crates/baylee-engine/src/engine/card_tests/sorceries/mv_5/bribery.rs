//! `cards/sorceries/mv_5/bribery.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Bribery — {3}{U}{U} sorcery: "Search target opponent's library for a
/// creature card and put that card onto the battlefield under your control.
/// Then that player shuffles."
///
/// The caster searches, the options come out of the *opponent's* library,
/// and the find is the caster's to control while its owner stays the
/// opponent.
#[test]
fn bribery_puts_a_creature_from_the_opponents_library_under_your_control() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, canopy_spider())
        .battlefield(0, &[island(), island(), island(), island(), island()])
        .hand(0, &[bribery()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let (mine_before, theirs_before) = (library_size(&engine, p0), library_size(&engine, p1));

    cast_from_hand(&mut engine, p0, bribery());
    let Pending::ChoosePlayer { player, options } = engine.pending().clone() else {
        panic!("\"target opponent\" is asked, got {:?}", engine.pending())
    };
    assert_eq!((player, options), (p0, vec![p1]), "only an opponent");
    engine.apply(p0, PlayerAction::ChoosePlayer(p1)).unwrap();
    let Pending::ChooseCards {
        player, options, ..
    } = pass_to_card_choice(&mut engine)
    else {
        unreachable!("the helper returns only a card choice")
    };
    assert_eq!(player, p0, "you search");
    let theirs = engine.state().zones.list(ZoneLocation::Library(p1)).clone();
    assert!(
        !options.is_empty() && options.iter().all(|o| theirs.contains(o)),
        "the search is of the targeted opponent's library"
    );
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

    let object = engine.state().object(found).expect("still an object");
    assert_eq!(object.controller, p0, "under your control");
    assert_eq!(object.owner, p1, "and still theirs to own");
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&found)
    );
    assert_eq!(library_size(&engine, p1), theirs_before - 1);
    assert_eq!(library_size(&engine, p0), mine_before);
}
