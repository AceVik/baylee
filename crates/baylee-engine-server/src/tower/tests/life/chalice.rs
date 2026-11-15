#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn each_tier_gains_only_on_its_controllers_upkeep_through_the_stack() {
    for (rarity, life) in Rarity::ALL.into_iter().zip([1, 2, 3, 5]) {
        let mut game = prepared(&[item(Family::SpringChalice, rarity)], &[("Plains", 40)]);
        let first = game.state().turn.active;
        let other = PlayerId::new(1 - first.get());
        assert_eq!(
            game.state().players[first.get() as usize].life,
            20,
            "not granted during setup"
        );
        assert_eq!(game.state().zones.list(ZoneLocation::Stack).len(), 1);
        resolve(&mut game);
        assert_eq!(game.state().players[first.get() as usize].life, 20 + life);
        assert_eq!(game.state().players[other.get() as usize].life, 20);
        main(&mut game, 2, other);
        assert_eq!(game.state().players[first.get() as usize].life, 20 + life);
        assert_eq!(game.state().players[other.get() as usize].life, 20 + life);
    }
}

#[test]
fn ordinary_removal_stops_future_triggers_but_not_the_one_already_on_the_stack() {
    let mut game = prepared(
        &[item(Family::SpringChalice, Rarity::Common)],
        &[("Plains", 30), ("Disenchant", 10)],
    );
    let player = game.state().turn.active;
    for turn in [1, 3, 5] {
        main(&mut game, turn, player);
        land(&mut game, player);
    }
    let artifact = source(&game, player, LIFE_RELICS[0][0]);
    let card = baylee_cards::decks::by_name("Disenchant").unwrap();
    let spell = *game
        .state()
        .zones
        .list(ZoneLocation::Hand(player))
        .iter()
        .find(|id| {
            game.state()
                .arena
                .get(**id)
                .unwrap()
                .card
                .is_some_and(|reference| reference.index == card)
        })
        .expect("seeded removal in hand");
    let life = game.state().players[player.get() as usize].life;
    until(&mut game, |game| {
        game.state().turn.number == 7
            && game.state().turn.step == Step::Upkeep
            && matches!(game.pending(), Pending::Priority { player: asked, .. } if *asked == player)
    });
    assert_eq!(game.state().zones.list(ZoneLocation::Stack).len(), 1);
    mana(&mut game, player, 2);
    game.apply(player, PlayerAction::CastSpell { card: spell })
        .unwrap();
    game.apply(
        player,
        PlayerAction::ChooseTargets {
            objects: vec![artifact],
            players: vec![],
        },
    )
    .unwrap();
    resolve(&mut game);
    assert!(
        !game
            .state()
            .zones
            .list(ZoneLocation::Battlefield)
            .contains(&artifact)
    );
    assert_eq!(game.state().players[player.get() as usize].life, life + 1);
    main(&mut game, 9, player);
    assert_eq!(
        game.state().players[player.get() as usize].life,
        life + 1,
        "destroyed item grants no future life"
    );
}
