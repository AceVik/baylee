#[allow(clippy::wildcard_imports)]
use super::*;

fn wurms_tooth_card() -> CardIndex {
    card_index("69476326-fda5-48c8-a506-e71eb7c81c2f")
}
fn beetle_for_tooth() -> CardIndex {
    card_index("9353f6dd-9a08-4e55-9b07-3e68d7d97ef4")
}
fn divination_for_tooth() -> CardIndex {
    card_index("273b339c-964b-4a18-8eb5-ceb8abcdfd9e")
}

/// Checklist: green spell cast triggers optional life gain once; controller
/// may accept or decline; a non-green spell is outside the condition.
#[test]
fn wurms_tooth_gains_exactly_one_when_green_spell_is_accepted() {
    let p0 = PlayerId::new(0);
    let mut game = Duel::new(SEED, forest())
        .life(0, 19)
        .battlefield(0, &[forest(), wurms_tooth_card()])
        .hand(0, &[beetle_for_tooth()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);
    cast_from_hand(&mut game, p0, beetle_for_tooth());
    pass_until(
        &mut game,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == p0),
    );
    game.apply(p0, PlayerAction::YesNo(true))
        .expect("controller accepts one life");
    pass_until(&mut game, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let beetle = on_battlefield(&game, p0, beetle_for_tooth()).unwrap();
    game.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![beetle],
        },
    )
    .expect("the Beetle's own trigger may target it");
    pass_until(&mut game, stack_is_empty);
    assert_eq!(game.state().players[0].life, 20);
    assert!(on_battlefield(&game, p0, beetle_for_tooth()).is_some());
}

#[test]
fn wurms_tooth_optional_trigger_can_be_declined() {
    let p0 = PlayerId::new(0);
    let mut game = Duel::new(SEED, forest())
        .life(0, 18)
        .battlefield(0, &[forest(), wurms_tooth_card()])
        .hand(0, &[beetle_for_tooth()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);
    cast_from_hand(&mut game, p0, beetle_for_tooth());
    pass_until(
        &mut game,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == p0),
    );
    game.apply(p0, PlayerAction::YesNo(false))
        .expect("controller declines");
    pass_until(&mut game, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let beetle = on_battlefield(&game, p0, beetle_for_tooth()).unwrap();
    game.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![beetle],
        },
    )
    .expect("the Beetle's own trigger may target it");
    pass_until(&mut game, stack_is_empty);
    assert_eq!(game.state().players[0].life, 18);
}

#[test]
fn wurms_tooth_triggers_for_an_opponents_green_spell() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut game = Duel::new(SEED, forest())
        .life(0, 15)
        .battlefield(0, &[wurms_tooth_card()])
        .battlefield(1, &[forest()])
        .hand(1, &[beetle_for_tooth()])
        .start();
    keep_mulligans(&mut game);
    reach_their_main_phase(&mut game, p1);
    cast_from_hand(&mut game, p1, beetle_for_tooth());
    pass_until(
        &mut game,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == p0),
    );
    game.apply(p0, PlayerAction::YesNo(true))
        .expect("the Tooth's controller accepts");
    pass_until(&mut game, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let beetle = on_battlefield(&game, p1, beetle_for_tooth()).unwrap();
    game.apply(
        p1,
        PlayerAction::ChooseObjects {
            objects: vec![beetle],
        },
    )
    .expect("the Beetle's own trigger may target it");
    pass_until(&mut game, stack_is_empty);
    assert_eq!(game.state().players[0].life, 16);
}

#[test]
fn wurms_tooth_does_not_trigger_for_a_blue_spell() {
    let p0 = PlayerId::new(0);
    let mut game = Duel::new(SEED, forest())
        .life(0, 17)
        .battlefield(0, &[island(), island(), island(), wurms_tooth_card()])
        .hand(0, &[divination_for_tooth()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);
    cast_from_hand(&mut game, p0, divination_for_tooth());
    pass_until(&mut game, stack_is_empty);
    assert_eq!(game.state().players[0].life, 17);
}
