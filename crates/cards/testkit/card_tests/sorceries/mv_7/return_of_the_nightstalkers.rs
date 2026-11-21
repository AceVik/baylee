//! `cards/sorceries/mv_7/return_of_the_nightstalkers.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn return_of_the_nightstalkers() -> CardIndex {
    card_index("6082f66e-3641-4b09-bb04-86bf29ee1c6f")
}

fn lurking_nightstalker() -> CardIndex {
    card_index("8c17f48b-467e-4953-85d0-221cef5b3bff")
}

fn prowling_nightstalker() -> CardIndex {
    card_index("6279078a-5c77-4a9e-8e9b-e99310363b6f")
}

/// Return of the Nightstalkers (6082f66e-3641-4b09-bb04-86bf29ee1c6f) —
/// {5}{B}{B} Sorcery: "Return all Nightstalker permanent cards from your
/// graveyard to the battlefield. Then destroy all Swamps you control."
#[test]
fn return_of_the_nightstalkers_reanimates_your_nightstalkers_and_destroys_your_swamps() {
    let mut engine = lk::game(
        SEED,
        &[
            swamp(),
            swamp(),
            swamp(),
            swamp(),
            swamp(),
            forest(),
            forest(),
        ],
        &[swamp(), swamp()],
        &[
            return_of_the_nightstalkers(),
            lurking_nightstalker(),
            prowling_nightstalker(),
            aurochs(),
        ],
        &[lurking_nightstalker()],
    );
    let my_swamps: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.controller == P0 && o.card.is_some_and(|c| c.index == swamp()))
        })
        .collect();
    let my_forest = lk::obj(&engine, P0, forest());
    let their_swamp = lk::obj(&engine, P1, swamp());

    let my_lurking = hand_to_graveyard(&mut engine, P0, lurking_nightstalker());
    let my_prowling = hand_to_graveyard(&mut engine, P0, prowling_nightstalker());
    let my_non_nightstalker = hand_to_graveyard(&mut engine, P0, aurochs());
    let their_lurking = hand_to_graveyard(&mut engine, P1, lurking_nightstalker());

    cast_from_hand(&mut engine, P0, return_of_the_nightstalkers());
    lk::settle(&mut engine);

    assert_eq!(lk::zone_of(&engine, my_lurking), Zone::Battlefield);
    assert_eq!(lk::zone_of(&engine, my_prowling), Zone::Battlefield);
    assert_eq!(engine.state().object(my_lurking).unwrap().controller, P0);
    assert_eq!(engine.state().object(my_prowling).unwrap().controller, P0);
    assert_eq!(
        lk::zone_of(&engine, my_non_nightstalker),
        Zone::Graveyard,
        "non-Nightstalker card stays in your graveyard"
    );
    assert_eq!(
        lk::zone_of(&engine, their_lurking),
        Zone::Graveyard,
        "Nightstalkers in opponent's graveyard are not returned"
    );

    for s in my_swamps {
        assert_eq!(
            lk::zone_of(&engine, s),
            Zone::Graveyard,
            "all Swamps you control are destroyed"
        );
    }
    assert_eq!(
        lk::zone_of(&engine, my_forest),
        Zone::Battlefield,
        "non-Swamp lands you control survive"
    );
    assert_eq!(
        lk::zone_of(&engine, their_swamp),
        Zone::Battlefield,
        "opponent's Swamps survive"
    );
}
