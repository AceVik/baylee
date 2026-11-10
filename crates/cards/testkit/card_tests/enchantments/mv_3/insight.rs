#[allow(clippy::wildcard_imports)]
use super::*;

fn insight_card() -> CardIndex {
    card_index("c77afbeb-cc9b-4770-9a48-e7460562895f")
}
fn green_spell_for_insight() -> CardIndex {
    card_index("9353f6dd-9a08-4e55-9b07-3e68d7d97ef4")
}
fn white_spell_for_insight() -> CardIndex {
    card_index("b1385b03-cb4b-4812-857f-7421f1df39af")
}

/// Checklist: only an opponent's green spell cast draws for the Insight
/// controller; draw occurs while the creature spell remains on the stack.
#[test]
fn insight_draws_for_an_opponents_green_spell_before_it_resolves() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut game = Duel::new(SEED, forest())
        .battlefield(0, &[insight_card()])
        .battlefield(1, &[forest()])
        .hand(1, &[green_spell_for_insight()])
        .start();
    keep_mulligans(&mut game);
    reach_their_main_phase(&mut game, p1);
    let library = game.state().zones.list(ZoneLocation::Library(p0)).clone();
    let top = *library.last().unwrap();
    let hand_before = game.state().zones.list(ZoneLocation::Hand(p0)).len();
    cast_from_hand(&mut game, p1, green_spell_for_insight());
    pass_until(&mut game, |e| {
        e.state().zones.list(ZoneLocation::Library(p0)).len() == library.len() - 1
    });
    assert!(
        on_stack(&game, green_spell_for_insight()).is_some(),
        "the cast spell has not resolved yet"
    );
    assert!(
        game.state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&top)
    );
    assert_eq!(
        game.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1
    );
    pass_until(&mut game, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let beetle = on_battlefield(&game, p1, green_spell_for_insight()).unwrap();
    game.apply(
        p1,
        PlayerAction::ChooseObjects {
            objects: vec![beetle],
        },
    )
    .expect("the Beetle's own trigger may target it");
    pass_until(&mut game, stack_is_empty);
    assert!(on_battlefield(&game, p1, green_spell_for_insight()).is_some());
}

#[test]
fn insight_does_not_draw_for_its_controllers_green_spell_or_an_opponents_white_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut own_green = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), insight_card()])
        .hand(0, &[green_spell_for_insight()])
        .start();
    keep_mulligans(&mut own_green);
    reach_main_phase(&mut own_green, p0);
    let initial_library = own_green
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .len();
    cast_from_hand(&mut own_green, p0, green_spell_for_insight());
    pass_until(&mut own_green, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    assert_eq!(
        own_green
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .len(),
        initial_library
    );
    let beetle = on_battlefield(&own_green, p0, green_spell_for_insight()).unwrap();
    own_green
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![beetle],
            },
        )
        .unwrap();
    pass_until(&mut own_green, stack_is_empty);
    assert_eq!(
        own_green
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .len(),
        initial_library
    );

    let mut enemy_white = Duel::new(SEED, forest())
        .battlefield(0, &[insight_card()])
        .battlefield(1, &[plains(), plains(), plains()])
        .hand(1, &[white_spell_for_insight()])
        .start();
    keep_mulligans(&mut enemy_white);
    reach_their_main_phase(&mut enemy_white, p1);
    let initial_library = enemy_white
        .state()
        .zones
        .list(ZoneLocation::Library(p0))
        .len();
    cast_from_hand(&mut enemy_white, p1, white_spell_for_insight());
    pass_until(&mut enemy_white, stack_is_empty);
    assert_eq!(
        enemy_white
            .state()
            .zones
            .list(ZoneLocation::Library(p0))
            .len(),
        initial_library
    );
}
