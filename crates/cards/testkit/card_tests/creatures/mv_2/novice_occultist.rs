// Oracle checklist: entry has no trigger; dies draws exactly one and loses one; last controller rather than owner receives both; exile is not death; opponent unaffected.
#[allow(clippy::wildcard_imports)] // Shared card-test harness.
use super::*;

fn subject() -> CardIndex {
    card_index("e51f5b41-5465-4b1d-b96a-544762657ca5")
}

#[test]
fn entry_does_nothing_but_death_draws_and_loses_one_for_last_controller() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for controller in [p0, p1] {
        let mut game = Duel::new(SEED, forest())
            .battlefield(0, &[swamp(), swamp()])
            .hand(0, &[subject()])
            .start();
        keep_mulligans(&mut game);
        reach_main_phase(&mut game, p0);
        let library = [
            game.state().zones.list(ZoneLocation::Library(p0)).len(),
            game.state().zones.list(ZoneLocation::Library(p1)).len(),
        ];
        let life = [game.state().players[0].life, game.state().players[1].life];
        cast_from_hand(&mut game, p0, subject());
        pass_until(&mut game, stack_is_empty);
        let source = on_battlefield(&game, p0, subject()).unwrap();
        assert_eq!(
            game.state().zones.list(ZoneLocation::Library(p0)).len(),
            library[0]
        );
        assert_eq!(game.state().players[0].life, life[0]);
        game.dev_state_mut(p0)
            .unwrap()
            .object_mut(source)
            .unwrap()
            .set_controller(controller);
        game.refresh_offer();
        assert_eq!(game.state().object(source).unwrap().controller, controller);
        assert_eq!(game.state().object(source).unwrap().owner, p0);
        let hands = [
            game.state().zones.list(ZoneLocation::Hand(p0)).len(),
            game.state().zones.list(ZoneLocation::Hand(p1)).len(),
        ];
        kill(&mut game, source);
        for (seat, i) in [(p0, 0), (p1, 1)] {
            let n = usize::from(seat == controller);
            assert_eq!(
                game.state().zones.list(ZoneLocation::Library(seat)).len(),
                library[i] - n
            );
            assert_eq!(
                game.state().zones.list(ZoneLocation::Hand(seat)).len(),
                hands[i] + n
            );
            assert_eq!(
                game.state().players[i].life,
                life[i] - i32::from(seat == controller)
            );
        }
        assert!(in_graveyard(&game, p0, subject()).is_some());
    }
}

#[test]
fn exile_does_not_trigger_death() {
    let p0 = PlayerId::new(0);
    let mut game = Duel::new(SEED, forest())
        .battlefield(0, &[subject(), plains()])
        .hand(0, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);
    let source = on_battlefield(&game, p0, subject()).unwrap();
    let library = game.state().zones.list(ZoneLocation::Library(p0)).len();
    let life = game.state().players[0].life;
    cast_from_hand(&mut game, p0, swords_to_plowshares());
    aim_at(&mut game, p0, source);
    pass_until(&mut game, stack_is_empty);
    assert_eq!(game.state().object(source).unwrap().zone, Zone::Exile);
    assert_eq!(
        game.state().zones.list(ZoneLocation::Library(p0)).len(),
        library
    );
    assert_eq!(game.state().players[0].life, life + 1);
}
