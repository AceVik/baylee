#[allow(clippy::wildcard_imports)]
use super::*;

fn divination_card() -> CardIndex {
    card_index("273b339c-964b-4a18-8eb5-ceb8abcdfd9e")
}

/// Checklist: no targets; draw exactly the top two in order; opponent hand
/// unchanged and the spell resolves at sorcery timing.
#[test]
fn divination_draws_the_top_two_cards_and_nothing_else() {
    let p0 = PlayerId::new(0);
    let mut game = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), island()])
        .hand(0, &[divination_card()])
        .hand(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);
    let library = game.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = library[library.len() - 1];
    let next = library[library.len() - 2];
    let opponent_hand = game
        .state()
        .zones
        .list(ZoneLocation::Hand(PlayerId::new(1)))
        .clone();
    cast_from_hand(&mut game, p0, divination_card());
    assert!(!matches!(game.pending(), Pending::ChooseTargets { .. }));
    pass_until(&mut game, stack_is_empty);
    let hand = game.state().zones.list(ZoneLocation::Hand(p0));
    assert!(hand.contains(&top) && hand.contains(&next));
    assert_eq!(
        hand.len(),
        2,
        "the spell was spent and exactly two cards were drawn"
    );
    assert_eq!(
        game.state()
            .zones
            .list(ZoneLocation::Hand(PlayerId::new(1))),
        &opponent_hand
    );
    assert_eq!(
        game.state().zones.list(ZoneLocation::Library(p0)).len(),
        library.len() - 2
    );
}
