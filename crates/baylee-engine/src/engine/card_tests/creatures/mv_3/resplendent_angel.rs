//! `cards/creatures/mv_3/resplendent_angel.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn resplendent_angel() -> CardIndex {
    card_index("2ac93092-1a57-4cf7-8f03-1dca86d5c476")
}

#[test]
fn resplendent_angel_rewards_five_life_and_temporarily_pumps_with_lifelink() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let reunion = baylee_core::generated::index::HEROES_REUNION;
    let mut board = vec![resplendent_angel(), forest()];
    board.extend_from_slice(&[plains(); 7]);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, &[reunion])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let angel = on_battlefield(&e, p0, resplendent_angel()).expect("the Angel");
    assert!(keywords(&e, angel).contains(KeywordSet::FLYING));
    cast_from_hand(&mut e, p0, reunion);
    e.apply(p0, PlayerAction::ChoosePlayer(p0))
        .expect("target self for seven life");
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].life, 27);

    activate(&mut e, p0, resplendent_angel(), 1);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, angel), (5, 5));
    assert!(keywords(&e, angel).contains(KeywordSet::LIFELINK));
    pass_until(&mut e, |e| e.state().turn.step == crate::turn::Step::End);
    pass_until(&mut e, stack_is_empty);
    let tokens = tokens_of(&e, p0);
    assert_eq!(
        tokens.len(),
        1,
        "seven gained life exceeds the five-life threshold"
    );
    let token = tokens[0];
    let chars = e
        .state()
        .object(token)
        .expect("Angel token")
        .characteristics();
    assert_eq!(pt(&e, token), (4, 4));
    assert_eq!(
        chars.colors,
        baylee_core::color::ColorSet::from_slice(&[baylee_core::color::Color::White])
    );
    assert!(
        chars
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::ANGEL)
    );
    assert!(keywords(&e, token).contains(KeywordSet::FLYING));
    assert!(keywords(&e, token).contains(KeywordSet::VIGILANCE));

    reach_their_main_phase(&mut e, p1);
    reach_their_main_phase(&mut e, p0);
    assert_eq!(
        pt(&e, angel),
        (3, 3),
        "the pump lasts only until end of turn"
    );
    assert!(!keywords(&e, angel).contains(KeywordSet::LIFELINK));
    pass_until(&mut e, |e| e.state().turn.step == crate::turn::Step::End);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        tokens_of(&e, p0).len(),
        1,
        "no life gained this turn, no token"
    );
}
