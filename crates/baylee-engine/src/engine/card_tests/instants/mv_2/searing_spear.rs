#[allow(clippy::wildcard_imports)]
use super::*;

fn searing_spear_card() -> CardIndex {
    card_index("aafd44c1-74a9-4aa7-a4a2-67eb39a07478")
}
fn spear_target_card() -> CardIndex {
    card_index("68954295-54e3-4303-a6bc-fc4547a4e3a3")
}
fn teferi_card() -> CardIndex {
    card_index("ae7604bb-4818-45a3-960c-cf3d83f15964")
}

/// Checklist: any-target includes a player and a creature; exact three damage;
/// unrelated artifact is illegal; a 1-toughness creature dies.
#[test]
fn searing_spear_can_target_a_player_for_exactly_three() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut game = Duel::new(SEED, forest())
        .life(1, 21)
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[searing_spear_card()])
        .battlefield(1, &[darksteel_pendant()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);
    let artifact = on_battlefield(&game, p1, darksteel_pendant()).unwrap();
    cast_from_hand(&mut game, p0, searing_spear_card());
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = game.pending().clone()
    else {
        panic!("any-target choice")
    };
    assert!(!options.contains(&artifact));
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
    assert_eq!(game.state().players[1].life, 18);
    assert_eq!(game.state().players[0].life, 20);
    assert!(on_battlefield(&game, p1, darksteel_pendant()).is_some());
}

#[test]
fn searing_spear_reduces_a_planeswalkers_loyalty_by_three() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut game = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[searing_spear_card()])
        .battlefield(1, &[teferi_card()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);
    let walker = on_battlefield(&game, p1, teferi_card()).unwrap();
    cast_from_hand(&mut game, p0, searing_spear_card());
    let Pending::ChooseTargets { options, .. } = game.pending().clone() else {
        panic!("any-target choice")
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
        1
    );
}

#[test]
fn searing_spear_deals_three_to_a_creature_and_a_toughness_one_dies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut game = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[searing_spear_card()])
        .battlefield(1, &[spear_target_card(), fangren_hunter()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);
    let elf = on_battlefield(&game, p1, spear_target_card()).unwrap();
    let bystander = on_battlefield(&game, p1, fangren_hunter()).unwrap();
    cast_from_hand(&mut game, p0, searing_spear_card());
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = game.pending().clone()
    else {
        panic!("any-target choice")
    };
    assert!(options.contains(&elf) && options.contains(&bystander));
    assert!(player_options.contains(&p0) && player_options.contains(&p1));
    game.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![elf],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut game, stack_is_empty);
    assert!(on_battlefield(&game, p1, spear_target_card()).is_none());
    assert!(in_graveyard(&game, p1, spear_target_card()).is_some());
    assert_eq!(game.state().object(bystander).unwrap().damage, 0);
    assert_eq!(pt(&game, bystander), (4, 4));
}

#[test]
fn searing_spear_marks_exactly_three_damage_on_a_surviving_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut game = Duel::new(SEED, forest())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[searing_spear_card()])
        .battlefield(1, &[fangren_hunter()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);
    let hunter = on_battlefield(&game, p1, fangren_hunter()).unwrap();
    cast_from_hand(&mut game, p0, searing_spear_card());
    game.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![hunter],
        },
    )
    .unwrap();
    pass_until(&mut game, stack_is_empty);
    assert_eq!(game.state().object(hunter).unwrap().damage, 3);
    assert_eq!(pt(&game, hunter), (4, 4));
}
