//! `cards/creatures/mv_3/yavimaya_elder.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn yavimaya_elder() -> CardIndex {
    card_index("7fd4c452-07f2-492c-9c78-d1c6362d9eec")
}

#[test]
fn yavimaya_elder_sacrifices_to_draw_and_dies_trigger_fetches_up_to_two_basic_lands() {
    let (p0, _p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[yavimaya_elder()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Cast Yavimaya Elder for {1}{G}{G}, leaving two Forests untapped.
    let lands = lands_of(&engine, p0);
    tap_mana_where(&mut engine, p0, |id| lands[..3].contains(&id));
    cast_with_floating(&mut engine, p0, yavimaya_elder());
    pass_until(&mut engine, stack_is_empty);
    let elder = on_battlefield(&engine, p0, yavimaya_elder()).expect("Yavimaya Elder resolved");
    assert_eq!(pt(&engine, elder), (2, 1));

    // Activate {2}, Sacrifice Yavimaya Elder: Draw a card.
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    tap_mana_where(&mut engine, p0, |id| lands[3..5].contains(&id));
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: elder,
                ability_index: 1,
            },
        )
        .expect("activates {2}, Sacrifice Yavimaya Elder");

    // Sacrificing Yavimaya Elder as cost triggers its dies ability on top of
    // the activated draw ability.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine
        .apply(p0, PlayerAction::YesNo(true))
        .expect("accept the optional dies search");

    // Pick two basic lands (Forests) from the library; `optional: true` gives
    // `min == 0` and `max == 2` ("up to two basic land cards").
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = engine.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(min, 0, "up to two basic land cards allows finding fewer");
    assert_eq!(max, 2, "up to two basic land cards");
    let first_land = options[0];
    let second_land = options[1];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![first_land, second_land],
            },
        )
        .unwrap();

    // Now the activated ability ({2}, Sacrifice: Draw a card) resolves too.
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, yavimaya_elder()).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 3,
        "two basic lands fetched from the dies trigger plus one card drawn from the activated ability"
    );
}
