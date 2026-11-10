// Oracle checklist: Whenever a creature you control dies, you draw a card and you lose 1 life.
// Exercise the named effect, target/trigger boundary, exact recipient and outcome.
#[allow(clippy::wildcard_imports)] // Shared card-test vocabulary.
use super::*;

fn subject() -> CardIndex {
    card_index("f47659b5-d834-4153-b9fc-6a0702c503d9")
}

// A controlled creature dying draws one and costs one life; an opponent's death does neither.
#[test]
fn own_creature_death_draws_one_and_loses_one() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut game = Duel::new(SEED, forest())
        .life(0, 15)
        .battlefield(0, &[subject(), gray_ogre()])
        .battlefield(1, &[wild_elephant()])
        .start();
    keep_mulligans(&mut game);
    assert!(walk_to_own_main(&mut game, p0));
    let own = on_battlefield(&game, p0, gray_ogre()).unwrap();
    let theirs = on_battlefield(&game, p1, wild_elephant()).unwrap();
    let hand_before = game.state().zones.list(ZoneLocation::Hand(p0)).len();
    kill(&mut game, own);
    assert!(in_graveyard(&game, p0, gray_ogre()).is_some());
    assert_eq!(game.state().players[0].life, 14);
    assert_eq!(
        game.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
    kill(&mut game, theirs);
    assert_eq!(game.state().players[0].life, 14);
    assert_eq!(
        game.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
}

#[test]
fn death_uses_controller_at_death_and_returns_cards_to_owners() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut game = Duel::new(SEED, forest())
        .life(0, 15)
        .battlefield(0, &[subject(), gray_ogre()])
        .battlefield(1, &[wild_elephant()])
        .start();
    keep_mulligans(&mut game);
    assert!(walk_to_own_main(&mut game, p0));
    let owned_by_us = on_battlefield(&game, p0, gray_ogre()).unwrap();
    let owned_by_them = on_battlefield(&game, p1, wild_elephant()).unwrap();
    {
        let state = game.dev_state_mut(p0).unwrap();
        state.object_mut(owned_by_us).unwrap().set_controller(p1);
        state.object_mut(owned_by_them).unwrap().set_controller(p0);
    }
    game.refresh_offer();
    assert_eq!(game.state().object(owned_by_us).unwrap().owner, p0);
    assert_eq!(game.state().object(owned_by_us).unwrap().controller, p1);
    assert_eq!(game.state().object(owned_by_them).unwrap().owner, p1);
    assert_eq!(game.state().object(owned_by_them).unwrap().controller, p0);
    let hand_before = game.state().zones.list(ZoneLocation::Hand(p0)).len();

    kill(&mut game, owned_by_them);
    assert!(in_graveyard(&game, p1, wild_elephant()).is_some());
    assert_eq!(game.state().players[0].life, 14);
    assert_eq!(
        game.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );

    kill(&mut game, owned_by_us);
    assert!(in_graveyard(&game, p0, gray_ogre()).is_some());
    assert_eq!(game.state().players[0].life, 14);
    assert_eq!(
        game.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
}
