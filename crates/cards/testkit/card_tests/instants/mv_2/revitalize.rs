#[allow(clippy::wildcard_imports)]
use super::*;

fn revitalize_card() -> CardIndex {
    card_index("b1385b03-cb4b-4812-857f-7421f1df39af")
}

/// Checklist: you gain exactly 3, then draw exactly one library-top card;
/// opponent and all other cards remain unchanged.
#[test]
fn revitalize_gains_three_then_draws_the_top_card() {
    let p0 = PlayerId::new(0);
    let mut game = Duel::new(SEED, forest())
        .life(0, 17)
        .life(1, 23)
        .battlefield(0, &[plains(), plains()])
        .hand(0, &[revitalize_card()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);
    let library = game.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library.last().unwrap();
    let hand_before = game.state().zones.list(ZoneLocation::Hand(p0)).len();
    cast_from_hand(&mut game, p0, revitalize_card());
    assert_eq!(
        game.state().players[0].life,
        17,
        "no effect before resolution"
    );
    pass_until(&mut game, stack_is_empty);
    assert_eq!(game.state().players[0].life, 20);
    assert_eq!(game.state().players[1].life, 23);
    assert!(
        game.state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&top)
    );
    assert_eq!(
        game.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before
    );
    assert_eq!(
        game.state().zones.list(ZoneLocation::Library(p0)).len(),
        library.len() - 1
    );
}
