#[allow(clippy::wildcard_imports)]
use super::*;

fn tithebearer_giant_card() -> CardIndex {
    card_index("475aaf7c-b0f8-40f3-ad91-3e7d414de933")
}

/// Checklist: ETB trigger is mandatory; draw one then lose one life; casting
/// alone has no result; source enters as 4/5 and opponent life is unchanged.
#[test]
fn tithebearer_giant_draws_then_costs_one_life_when_it_enters() {
    let p0 = PlayerId::new(0);
    let mut game = Duel::new(SEED, forest())
        .life(0, 20)
        .life(1, 22)
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[tithebearer_giant_card()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);
    let library = game.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library.last().unwrap();
    cast_from_hand(&mut game, p0, tithebearer_giant_card());
    assert_eq!(
        game.state().players[0].life,
        20,
        "no ETB effect while the creature is still a spell"
    );
    assert!(on_stack(&game, tithebearer_giant_card()).is_some());
    pass_until(&mut game, stack_is_empty);
    let giant = on_battlefield(&game, p0, tithebearer_giant_card()).unwrap();
    assert_eq!(pt(&game, giant), (4, 5));
    assert_eq!(game.state().players[0].life, 19);
    assert_eq!(game.state().players[1].life, 22);
    assert!(
        game.state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&top)
    );
    assert_eq!(
        game.state().zones.list(ZoneLocation::Library(p0)).len(),
        library.len() - 1
    );
}
