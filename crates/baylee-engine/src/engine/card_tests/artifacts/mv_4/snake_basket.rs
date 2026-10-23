//! `cards/artifacts/mv_4/snake_basket.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use crate::turn::Step;
use baylee_cards_dsl::{Color, ColorSet};
use baylee_core::generated::subtypes;

fn snake_basket() -> CardIndex {
    card_index("12236231-7c6e-4edc-87dc-737115843185")
}

/// `Snake Basket` ({4}, Artifact, `Coverage::Implemented`) sacrifices itself
/// for `{X}` at sorcery speed to create X 1/1 green Snake creature tokens.
#[test]
fn snake_basket_sacrifices_at_sorcery_speed_to_create_x_snake_tokens() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[snake_basket()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, snake_basket());
    pass_until(&mut engine, stack_is_empty);

    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, snake_basket(), 0);
    engine
        .apply(p0, PlayerAction::ChooseNumber(3))
        .expect("choose X=3");
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, snake_basket()).is_some());
    let snakes: Vec<_> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            let obj = engine.state().object(id).unwrap();
            obj.controller == p0 && obj.card.is_none()
        })
        .collect();
    assert_eq!(snakes.len(), 3);
    for &id in &snakes {
        assert_eq!(pt(&engine, id), (1, 1));
        let snake = engine.state().object(id).unwrap().characteristics();
        assert_eq!(snake.colors, ColorSet::of(Color::Green), "green");
        assert!(snake.subtypes.contains(subtypes::creature::SNAKE), "Snake");
    }
}

/// "Activate only as a sorcery": offered in P0's main phase with an empty
/// stack, not at instant speed in their end step.
#[test]
fn snake_basket_is_activated_only_at_sorcery_speed() {
    let p0 = PlayerId::new(0);
    let mut engine = lk::game(
        SEED,
        &[snake_basket(), forest(), forest(), forest()],
        &[],
        &[],
        &[],
    );
    assert!(
        lk::offered(&engine, snake_basket(), p0, 0),
        "offered in the main phase with an empty stack"
    );

    lk::to_step(&mut engine, p0, Step::End);
    assert!(
        !lk::offered(&engine, snake_basket(), p0, 0),
        "not offered in the end step"
    );
}
