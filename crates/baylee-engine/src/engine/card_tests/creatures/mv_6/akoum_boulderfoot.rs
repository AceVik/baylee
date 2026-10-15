#[allow(clippy::wildcard_imports)]
use super::*;

fn akoum_boulderfoot_card() -> CardIndex {
    card_index("c16367eb-1c0a-4197-8e2f-2d74e4381024")
}
fn teferi_card() -> CardIndex {
    card_index("ae7604bb-4818-45a3-960c-cf3d83f15964")
}

/// Checklist: ETB trigger targets either a player or creature; one damage is
/// dealt on resolution; a 1-toughness creature dies while the source enters 4/5.
#[test]
fn akoum_boulderfoot_trigger_can_deal_one_to_a_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut game = Duel::new(SEED, forest())
        .life(1, 21)
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[akoum_boulderfoot_card()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);
    cast_from_hand(&mut game, p0, akoum_boulderfoot_card());
    pass_until(&mut game, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = game.pending().clone()
    else {
        panic!("any-target ETB choice")
    };
    let elf = on_battlefield(&game, p1, llanowar_elves()).unwrap();
    assert!(options.contains(&elf));
    assert!(player_options.contains(&p1));
    game.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![p1],
        },
    )
    .unwrap();
    pass_until(&mut game, stack_is_empty);
    assert_eq!(game.state().players[1].life, 20);
    assert_eq!(
        pt(
            &game,
            on_battlefield(&game, p0, akoum_boulderfoot_card()).unwrap()
        ),
        (4, 5)
    );
}

#[test]
fn akoum_boulderfoot_trigger_removes_one_loyalty_from_a_planeswalker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut game = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[akoum_boulderfoot_card()])
        .battlefield(1, &[teferi_card()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);
    let walker = on_battlefield(&game, p1, teferi_card()).unwrap();
    cast_from_hand(&mut game, p0, akoum_boulderfoot_card());
    pass_until(&mut game, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = game.pending().clone() else {
        panic!("any-target ETB choice")
    };
    assert!(options.contains(&walker));
    game.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![walker],
        },
    )
    .unwrap();
    pass_until(&mut game, stack_is_empty);
    assert_eq!(
        game.state()
            .object(walker)
            .unwrap()
            .counters
            .get(CounterKind::Loyalty),
        3
    );
}

#[test]
fn akoum_boulderfoot_trigger_deals_one_to_a_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut game = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[akoum_boulderfoot_card()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);
    let elf = on_battlefield(&game, p1, llanowar_elves()).unwrap();
    cast_from_hand(&mut game, p0, akoum_boulderfoot_card());
    pass_until(&mut game, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    game.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![elf],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut game, stack_is_empty);
    assert!(on_battlefield(&game, p1, llanowar_elves()).is_none());
    assert!(in_graveyard(&game, p1, llanowar_elves()).is_some());
}
