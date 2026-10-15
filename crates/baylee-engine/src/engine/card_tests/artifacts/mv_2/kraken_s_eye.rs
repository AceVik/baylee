#[allow(clippy::wildcard_imports)]
use super::*;

fn krakens_eye_card() -> CardIndex {
    card_index("14a21d8a-2097-4150-8af5-8c155049477a")
}
fn divination_for_eye() -> CardIndex {
    card_index("273b339c-964b-4a18-8eb5-ceb8abcdfd9e")
}
fn beetle_for_eye() -> CardIndex {
    card_index("9353f6dd-9a08-4e55-9b07-3e68d7d97ef4")
}

/// Checklist: own blue spell triggers an optional point of life; declining
/// gains none; a green cast does not trigger this blue-spell ability.
#[test]
fn krakens_eye_may_gain_life_for_a_blue_spell_and_may_decline() {
    let p0 = PlayerId::new(0);
    let mut game = Duel::new(SEED, forest())
        .life(0, 18)
        .battlefield(0, &[island(), island(), island(), krakens_eye_card()])
        .hand(0, &[divination_for_eye(), divination_for_eye()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);
    cast_from_hand(&mut game, p0, divination_for_eye());
    pass_until(
        &mut game,
        |e| matches!(e.pending(), Pending::YesNo { player, prompt: YesNoPrompt::MayDo, .. } if *player == p0),
    );
    game.apply(p0, PlayerAction::YesNo(false))
        .expect("the life gain is optional");
    assert_eq!(game.state().players[0].life, 18);
    pass_until(&mut game, stack_is_empty);

    // The same Eye does not trigger for a green spell.
    let mut other = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), krakens_eye_card()])
        .hand(0, &[beetle_for_eye()])
        .start();
    keep_mulligans(&mut other);
    reach_main_phase(&mut other, p0);
    cast_from_hand(&mut other, p0, beetle_for_eye());
    pass_until(&mut other, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    assert_eq!(other.state().players[0].life, 20);
    let beetle = on_battlefield(&other, p0, beetle_for_eye()).unwrap();
    other
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![beetle],
            },
        )
        .unwrap();
    pass_until(&mut other, stack_is_empty);
    assert_eq!(other.state().players[0].life, 20);
}

#[test]
fn krakens_eye_gains_one_when_its_controller_accepts() {
    let p0 = PlayerId::new(0);
    let mut game = Duel::new(SEED, forest())
        .life(0, 19)
        .battlefield(0, &[island(), island(), island(), krakens_eye_card()])
        .hand(0, &[divination_for_eye()])
        .start();
    keep_mulligans(&mut game);
    reach_main_phase(&mut game, p0);
    cast_from_hand(&mut game, p0, divination_for_eye());
    pass_until(
        &mut game,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == p0),
    );
    game.apply(p0, PlayerAction::YesNo(true))
        .expect("the controller accepts");
    pass_until(&mut game, stack_is_empty);
    assert_eq!(game.state().players[0].life, 20);
}

#[test]
fn krakens_eye_triggers_for_an_opponents_blue_spell_too() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut game = Duel::new(SEED, forest())
        .life(0, 16)
        .battlefield(0, &[krakens_eye_card()])
        .battlefield(1, &[island(), island(), island()])
        .hand(1, &[divination_for_eye()])
        .start();
    keep_mulligans(&mut game);
    reach_their_main_phase(&mut game, p1);
    cast_from_hand(&mut game, p1, divination_for_eye());
    pass_until(
        &mut game,
        |e| matches!(e.pending(), Pending::YesNo { player, .. } if *player == p0),
    );
    game.apply(p0, PlayerAction::YesNo(true))
        .expect("the Eye's controller accepts");
    pass_until(&mut game, stack_is_empty);
    assert_eq!(game.state().players[0].life, 17);
}
