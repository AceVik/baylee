//! `cards/creatures/mv_7/krosan_tusker.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::legends_kit as lk;

fn krosan_tusker() -> CardIndex {
    card_index("82406175-63b3-4972-be8b-db0e70c908cb")
}

fn wasteland() -> CardIndex {
    card_index("09a70ae8-3859-4a09-901d-dce063fa3b5f")
}

fn yawgmoth_s_will() -> CardIndex {
    card_index("322f0459-f394-44f0-977b-55fd0cbe0712")
}

/// Cycling Krosan Tusker searches for a basic land (excluding nonbasic lands)
/// BEFORE the cycling draw resolves (CR 702.29c), and declining `MayDo` skips the
/// search while still drawing 1 card.
#[test]
fn krosan_tusker_searches_basic_land_before_draw_and_may_be_declined() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), forest(), forest()],
        )
        .hand(0, &[krosan_tusker(), krosan_tusker(), wasteland()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Put a nonbasic Wasteland into P0's library so we verify it is excluded.
    let nonbasic = hand_to_library_top(&mut engine, p0, wasteland());
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let lands = all_on_battlefield(&engine, p0, forest());

    // 1. Cycle first Tusker and accept search.
    tap_mana_where(&mut engine, p0, |id| lands[..3].contains(&id));
    activate(&mut engine, p0, krosan_tusker(), 0);
    assert_eq!(engine.state().zones.list(ZoneLocation::Stack).len(), 2);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    lk::yes_no(&mut engine, true);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });

    // At the search prompt, the cycling draw has NOT resolved yet.
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before - 1,
        "search runs before cycling draw"
    );
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("matched ChooseCards");
    };
    assert!(!options.is_empty());
    assert!(
        !options.contains(&nonbasic),
        "nonbasic Wasteland must not be offered for basic land search"
    );
    let picked = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![picked],
            },
        )
        .expect("pick basic Forest");

    // Still 1 item left on stack (cycling draw); hand gained the searched land.
    assert_eq!(engine.state().zones.list(ZoneLocation::Stack).len(), 1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "net +1 card: discarded 1, searched 1 basic land, drew 1"
    );

    // 2. Cycle second Tusker and decline MayDo -> no ChooseCards prompt, still draws 1.
    let hand_before2 = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_mana_where(&mut engine, p0, |id| lands[3..6].contains(&id));
    activate(&mut engine, p0, krosan_tusker(), 0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    lk::yes_no(&mut engine, false);
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before2,
        "declining search still draws 1 card from cycling"
    );
}

/// CR 702.29c: when a replacement effect (Yawgmoth's Will) sends the cycled
/// Krosan Tusker to exile instead of the graveyard, its cycling trigger still
/// fires from exile.
#[test]
fn krosan_tusker_triggers_when_discard_is_replaced_into_exile() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[swamp(), swamp(), swamp(), forest(), forest(), forest()],
        )
        .hand(0, &[yawgmoth_s_will(), krosan_tusker()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let swamps = all_on_battlefield(&engine, p0, swamp());
    tap_mana_where(&mut engine, p0, |id| swamps.contains(&id));
    cast_with_floating(&mut engine, p0, yawgmoth_s_will());
    pass_until(&mut engine, stack_is_empty);

    let tusker = in_hand(&engine, p0, krosan_tusker()).expect("Tusker in hand");
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, krosan_tusker(), 0);

    assert_eq!(
        lk::zone_of(&engine, tusker),
        Zone::Exile,
        "Yawgmoth's Will exiled the cycled card instead of graveyard"
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    lk::yes_no(&mut engine, true);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        unreachable!("matched ChooseCards");
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .expect("put basic Forest into hand");
    pass_until(&mut engine, |e| at_rest(e, p0));
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
}
