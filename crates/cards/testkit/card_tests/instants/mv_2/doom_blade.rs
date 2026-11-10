#[allow(clippy::wildcard_imports)]
use super::*;

fn doom_blade_card() -> CardIndex {
    card_index("59e7f2ae-4535-4191-98be-3e65b6b2befa")
}
fn black_knight_card() -> CardIndex {
    card_index("9456c5b6-946d-403a-8ed0-dff9f921d98c")
}
fn darksteel_sentinel_card() -> CardIndex {
    card_index("b91c8946-591b-4c0d-a37e-36803df40db7")
}

/// Checklist: nonblack own/opposing/colorless creatures are legal; black is
/// excluded; destroy is attempted against an indestructible legal target.
#[test]
fn doom_blade_destroys_a_nonblack_creature_but_black_is_not_a_target() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut game = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[swamp(), swamp(), llanowar_elves(), darksteel_pendant()],
        )
        .hand(0, &[doom_blade_card()])
        .battlefield(
            1,
            &[
                llanowar_elves(),
                black_knight_card(),
                darksteel_sentinel_card(),
            ],
        )
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);
    let elf = on_battlefield(&game, p1, llanowar_elves()).unwrap();
    let black = on_battlefield(&game, p1, black_knight_card()).unwrap();
    let sentinel = on_battlefield(&game, p1, darksteel_sentinel_card()).unwrap();
    let own = on_battlefield(&game, p0, llanowar_elves()).unwrap();
    let pendant = on_battlefield(&game, p0, darksteel_pendant()).unwrap();
    cast_from_hand(&mut game, p0, doom_blade_card());
    let Pending::ChooseTargets { options, .. } = game.pending().clone() else {
        panic!("creature target choice")
    };
    assert!(options.contains(&elf) && options.contains(&sentinel) && options.contains(&own));
    assert!(!options.contains(&black));
    assert!(
        !options.contains(&pendant),
        "an artifact without creature type is not a target"
    );
    game.apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    pass_until(&mut game, stack_is_empty);
    assert!(on_battlefield(&game, p1, llanowar_elves()).is_none());
    assert!(on_battlefield(&game, p1, black_knight_card()).is_some());
    assert!(on_battlefield(&game, p1, darksteel_sentinel_card()).is_some());
}

#[test]
fn doom_blade_is_legal_on_but_cannot_destroy_an_indestructible_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut game = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[doom_blade_card()])
        .battlefield(1, &[darksteel_sentinel_card()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);
    let sentinel = on_battlefield(&game, p1, darksteel_sentinel_card()).unwrap();
    cast_from_hand(&mut game, p0, doom_blade_card());
    game.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![sentinel],
        },
    )
    .unwrap();
    pass_until(&mut game, stack_is_empty);
    assert_eq!(pt(&game, sentinel), (3, 3));
    assert!(on_battlefield(&game, p1, darksteel_sentinel_card()).is_some());
}
