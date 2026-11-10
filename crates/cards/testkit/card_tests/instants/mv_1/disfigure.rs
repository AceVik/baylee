#[allow(clippy::wildcard_imports)]
use super::*;

fn disfigure_card() -> CardIndex {
    card_index("77eafe49-b9c5-461d-89c5-cec217dd2974")
}

/// Checklist: exactly -2/-2 on a creature target; toughness three survives;
/// toughness one reaches zero and dies; duration is temporary.
#[test]
fn disfigure_reduces_power_and_toughness_by_two_until_cleanup() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut game = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[disfigure_card()])
        .battlefield(1, &[wild_elephant(), llanowar_elves()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);
    let three_toughness = on_battlefield(&game, p1, wild_elephant()).unwrap();
    let one_toughness = on_battlefield(&game, p1, llanowar_elves()).unwrap();
    cast_from_hand(&mut game, p0, disfigure_card());
    let Pending::ChooseTargets { options, .. } = game.pending().clone() else {
        panic!("creature target choice")
    };
    assert!(options.contains(&three_toughness) && options.contains(&one_toughness));
    game.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![three_toughness],
        },
    )
    .unwrap();
    pass_until(&mut game, stack_is_empty);
    assert_eq!(pt(&game, three_toughness), (1, 1));
    assert_eq!(pt(&game, one_toughness), (1, 1));
    pass_until(&mut game, |e| {
        e.state().turn.active == p1 && matches!(e.state().turn.phase, Phase::FirstMain)
    });
    assert_eq!(pt(&game, three_toughness), (3, 3));
}

#[test]
fn disfigure_kills_a_toughness_one_creature_by_zero_toughness() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut game = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp()])
        .hand(0, &[disfigure_card()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);
    let elf = on_battlefield(&game, p1, llanowar_elves()).unwrap();
    cast_from_hand(&mut game, p0, disfigure_card());
    game.apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    pass_until(&mut game, stack_is_empty);
    assert!(on_battlefield(&game, p1, llanowar_elves()).is_none());
    assert!(in_graveyard(&game, p1, llanowar_elves()).is_some());
}
