//! `cards/creatures/mv_5/fire_snake.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Fire Snake is a {4}{R} 3/1 whose entire text is a dies trigger: "When this
/// creature dies, destroy target land." The Snake is really cast — five
/// Mountains pay `{4}{R}` — and it dies through the harness' own door,
/// because what the test reads is the sentence that follows the death.
/// `Filter::LAND` carries no "you control", so the question has to reach both
/// sides of the table: the opponent's Forest is the target that proves that
/// half, and the five Mountains beside the Snake are the control saying a land
/// is a legal target whoever owns it — one land dies, the rest do not.
#[test]
fn fire_snake_destroys_a_land_of_either_player_when_it_dies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .hand(0, &[fire_snake()])
        .battlefield(1, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // The card arrives the way the card arrives: five Mountains, all of which
    // tap for the whole `{{4}}{{R}}`.
    cast_from_hand(&mut engine, p0, fire_snake());
    pass_until(&mut engine, stack_is_empty);
    let snake = on_battlefield(&engine, p0, fire_snake()).expect("the Snake resolved");
    assert_eq!(pt(&engine, snake), (3, 1), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "five Mountains paid five mana and nothing is left floating"
    );

    let my_land = on_battlefield(&engine, p0, mountain()).expect("my Mountains are out");
    let their_land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    assert_eq!(
        all_on_battlefield(&engine, p0, mountain()).len(),
        5,
        "five Mountains on the board before the Snake dies"
    );

    // The harness is the door the Snake dies through (CR 704.5f); what the
    // card is, is what happens next.
    let state = engine
        .dev_state_mut(p0)
        .expect("the harness may set boards up");
    crate::sba::destroy(state, snake);
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    engine.apply(player, PlayerAction::PassPriority).unwrap();

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the dies trigger asks which land to destroy, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(player, p0, "the Snake's controller aims the trigger");
    assert_eq!((min, max), (1, 1), "one land, and the trigger asks once");
    assert!(
        options.contains(&their_land) && options.contains(&my_land),
        "\"target land\" is any land, on either side of the table: {options:?}"
    );
    assert_eq!(
        options.len(),
        7,
        "five Mountains and two Forests are the whole menu, and every entry on \
         it is a land: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_land],
            },
        )
        .expect("their Forest was one of the options the question enumerated");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, fire_snake()).is_some(),
        "the Snake is dead, which is the trigger's own condition"
    );
    assert!(
        in_graveyard(&engine, p1, forest()).is_some(),
        "and the land the trigger named is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "the Forest it did not name is still standing: the effect targets, it \
         does not sweep the board"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, mountain()).len(),
        5,
        "and the lands on the Snake's own side never moved"
    );
}
