#[allow(clippy::wildcard_imports)]
use super::*;

fn disentomb_card() -> CardIndex {
    card_index("e4ab986d-662c-4879-b6f1-6887a2ec4d7c")
}

/// Checklist: return a creature card from your own graveyard to your hand;
/// opponent's creature and a noncreature card are absent from target choices.
#[test]
fn disentomb_returns_the_chosen_own_creature_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut game = Duel::new(SEED, forest())
        .hand(0, &[disentomb_card()])
        .battlefield(0, &[swamp(), forest(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);
    let own = on_battlefield(&game, p0, llanowar_elves()).unwrap();
    let theirs = on_battlefield(&game, p1, llanowar_elves()).unwrap();
    game.dev_state_mut(p0)
        .unwrap()
        .move_object(
            own,
            ZoneLocation::Graveyard(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    game.dev_state_mut(p1)
        .unwrap()
        .move_object(
            theirs,
            ZoneLocation::Graveyard(p1),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    let land = on_battlefield(&game, p0, forest()).unwrap();
    game.dev_state_mut(p0)
        .unwrap()
        .move_object(
            land,
            ZoneLocation::Graveyard(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    game.refresh_offer();

    cast_from_hand(&mut game, p0, disentomb_card());
    let Pending::ChooseTargets { options, .. } = game.pending().clone() else {
        panic!("graveyard target choice")
    };
    assert_eq!(options, vec![own]);
    assert!(!options.contains(&theirs));
    game.apply(p0, PlayerAction::ChooseObjects { objects: vec![own] })
        .unwrap();
    pass_until(&mut game, stack_is_empty);
    assert!(in_hand(&game, p0, llanowar_elves()).is_some());
    assert!(in_graveyard(&game, p0, llanowar_elves()).is_none());
    assert!(
        game.state()
            .zones
            .list(ZoneLocation::Graveyard(p1))
            .contains(&theirs)
    );
}
