//! `cards/creatures/mv_6/primeval_titan.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Primeval Titan: the enter trigger finds two lands and they arrive tapped.
///
/// "…put them onto the battlefield tapped" is the half a search that found
/// the cards would still get wrong, and `Find::BATTLEFIELD_TAPPED` twice is
/// what the card writes for it.
#[test]
fn primeval_titan_fetches_two_lands_and_both_arrive_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(388, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .hand(0, &[primeval_titan()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let before = library_size(&engine, p0);
    let seated: Vec<ObjectId> = engine.state().zones.list(ZoneLocation::Battlefield).clone();
    cast_from_hand(&mut engine, p0, primeval_titan());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, max, .. } = engine.pending().clone() else {
        unreachable!("pass_until only stops on the search")
    };
    assert_eq!(
        max, 2,
        "\"up to two land cards\" is the offer the card makes"
    );
    let found: Vec<ObjectId> = options.into_iter().take(2).collect();
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: found })
        .expect("two lands are a legal answer");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        library_size(&engine, p0),
        before - 2,
        "two land cards left the library"
    );
    // The six Forests the board started with were tapped for the Titan's own
    // cost, so "tapped" alone says nothing: what is asked is the two objects
    // that were not on the battlefield before.
    let arrived: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| !seated.contains(id))
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.characteristics().types.intersects(TypeSet::LAND))
        })
        .collect();
    assert_eq!(arrived.len(), 2, "two lands arrived from the library");
    assert!(
        arrived.iter().all(|id| is_tapped(&engine, *id)),
        "and both of them arrived tapped"
    );
}

/// Primeval Titan: "Whenever this creature enters or attacks, you may search
/// your library for up to two land cards, put them onto the battlefield
/// tapped". The attack half, with the Titan seated and never cast.
#[test]
fn primeval_titan_fetches_two_tapped_lands_when_it_attacks() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(2204, forest())
        .battlefield(0, &[primeval_titan()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let titan = on_battlefield(&engine, p0, primeval_titan()).expect("titan");
    let before = library_size(&engine, p0);

    unf_attack(&mut engine, p0, &[titan]);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, max, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the search")
    };
    assert_eq!(max, 2);
    let found: Vec<ObjectId> = options.into_iter().take(2).collect();
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: found })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(library_size(&engine, p0), before - 2);
    let lands = lands_of(&engine, p0);
    assert_eq!(lands.len(), 2, "two lands arrived");
    assert!(lands.iter().all(|l| is_tapped(&engine, *l)), "tapped");
}
