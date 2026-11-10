#[allow(clippy::wildcard_imports)]
use super::*;

fn bond_beetle_card() -> CardIndex {
    card_index("9353f6dd-9a08-4e55-9b07-3e68d7d97ef4")
}

/// Checklist: entering makes a mandatory one-creature target trigger; another
/// player's creature may be chosen; exactly one persistent +1/+1 counter lands.
#[test]
fn bond_beetle_puts_a_counter_on_the_creature_its_trigger_targets() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut game = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), llanowar_elves()])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[bond_beetle_card()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);
    let mine = on_battlefield(&game, p0, llanowar_elves()).unwrap();
    let theirs = on_battlefield(&game, p1, llanowar_elves()).unwrap();
    cast_from_hand(&mut game, p0, bond_beetle_card());
    pass_until(&mut game, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = game.pending().clone()
    else {
        panic!("ETB target choice")
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (1, 1));
    let beetle = on_battlefield(&game, p0, bond_beetle_card()).unwrap();
    assert!(options.contains(&mine) && options.contains(&theirs) && options.contains(&beetle));
    game.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![theirs],
        },
    )
    .unwrap();
    pass_until(&mut game, stack_is_empty);
    assert_eq!(pt(&game, theirs), (2, 2));
    assert_eq!(
        game.state()
            .object(theirs)
            .unwrap()
            .counters
            .get(CounterKind::P1P1),
        1
    );
    assert_eq!(pt(&game, mine), (1, 1));
    assert!(on_battlefield(&game, p0, bond_beetle_card()).is_some());
}
