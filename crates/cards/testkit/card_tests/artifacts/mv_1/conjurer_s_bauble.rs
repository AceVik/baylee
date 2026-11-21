//! `cards/artifacts/mv_1/conjurer_s_bauble.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn conjurer_s_bauble() -> CardIndex {
    card_index("4fd493b3-63f9-4633-8f88-8755e489da32")
}

/// Conjurer's Bauble (4fd493b3-63f9-4633-8f88-8755e489da32) — {1} Artifact:
/// "{T}, Sacrifice this artifact: Put up to one target card from your
/// graveyard on the bottom of your library. Draw a card."
///
/// With a card in each player's graveyard, targeting offers only your
/// graveyard (and never the Bauble itself, since targets are chosen before
/// costs are paid), puts the chosen card on the bottom of your library, and
/// draws from the top.
#[test]
fn conjurer_s_bauble_tucks_one_own_graveyard_card_to_bottom_and_draws() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[conjurer_s_bauble(), mountain()])
        .hand(0, &[lightning_bolt()])
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Put one card in p0's graveyard and one in p1's graveyard through real play.
    let my_bolt = in_hand(&engine, p0, lightning_bolt()).unwrap();
    cast_from_hand(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    reach_their_main_phase(&mut engine, p1);
    let their_bolt = in_hand(&engine, p1, lightning_bolt()).unwrap();
    cast_from_hand(&mut engine, p1, lightning_bolt());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p0],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    pass_until(&mut engine, |e| e.state().turn.active == p0);
    reach_main_phase(&mut engine, p0);
    let bauble = on_battlefield(&engine, p0, conjurer_s_bauble()).unwrap();
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let lib_before = library_size(&engine, p0);

    activate(&mut engine, p0, conjurer_s_bauble(), 0);
    let Pending::ChooseTargets {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        panic!("expected ChooseTargets, got {:?}", engine.pending())
    };
    assert_eq!((min, max), (0, 1), "up to one target card");
    assert!(options.contains(&my_bolt), "own graveyard card is offered");
    assert!(
        !options.contains(&their_bolt),
        "opponent's graveyard card is excluded"
    );
    assert!(
        !options.contains(&bauble),
        "Bauble is still on the battlefield when targets are chosen"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![my_bolt],
            },
        )
        .unwrap();
    assert_eq!(
        engine.state().object(bauble).unwrap().zone,
        Zone::Graveyard,
        "Bauble is sacrificed as part of the activation cost"
    );
    pass_until(&mut engine, stack_is_empty);

    let lib = engine.state().zones.list(ZoneLocation::Library(p0));
    assert_eq!(
        lib.first().copied(),
        Some(my_bolt),
        "chosen card sits on the bottom of your library"
    );
    assert_eq!(
        lib.len(),
        lib_before,
        "+1 tucked to bottom, -1 drawn from top"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "drew a card from the top of the library"
    );
    assert_eq!(
        engine.state().object(their_bolt).unwrap().zone,
        Zone::Graveyard,
        "opponent's graveyard is untouched"
    );
}

/// Choosing zero targets ("up to one") still sacrifices the Bauble and draws a
/// card while leaving your graveyard untouched.
#[test]
fn conjurer_s_bauble_can_choose_zero_targets_and_still_draw() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[conjurer_s_bauble()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 1);
    let gy_card = engine.state().zones.list(ZoneLocation::Graveyard(p0))[0];
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let lib_before = library_size(&engine, p0);

    activate(&mut engine, p0, conjurer_s_bauble(), 0);
    assert!(matches!(
        engine.pending(),
        Pending::ChooseTargets { min: 0, max: 1, .. }
    ));
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(gy_card).unwrap().zone,
        Zone::Graveyard,
        "choosing zero targets leaves the graveyard card in place"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "still draws a card when zero targets are chosen"
    );
    assert_eq!(library_size(&engine, p0), lib_before - 1);
}
