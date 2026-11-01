//! `cards/enchantments/mv_3/liability.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "65f1bebb-6441-4189-b6de-993ec9d24a59"
fn liability() -> CardIndex {
    card_index("65f1bebb-6441-4189-b6de-993ec9d24a59")
}

fn raise_the_alarm() -> CardIndex {
    card_index("5b2364d7-a811-4595-a1b4-224c70555ffa")
}

fn shock() -> CardIndex {
    card_index("a9d288b8-cdc1-4e55-a0c9-d6edfc95e65d")
}

/// `Liability` (`Coverage::Generated`):
/// "Whenever a nontoken permanent is put into a player's graveyard from the battlefield, that player loses 1 life."
///
/// Verifies that a nontoken permanent dying causes its owner to lose 1 life, while
/// a creature token dying does not trigger `Liability`.
#[test]
fn liability_causes_owner_to_lose_life_for_nontoken_permanent_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(9413, mountain())
        .battlefield(
            0,
            &[
                liability(),
                mountain(),
                mountain(),
                plains(),
                plains(),
                grizzly_bears(),
            ],
        )
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[raise_the_alarm(), shock(), shock()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Create two 1/1 Soldier tokens, then Shock one of them: token death does not trigger Liability.
    cast_from_hand(&mut engine, p0, raise_the_alarm());
    pass_until(&mut engine, stack_is_empty);
    let token = *tokens_of(&engine, p0).first().expect("Soldier token");
    cast_from_hand(&mut engine, p0, shock());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![token],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0].life,
        20,
        "token dying does not trigger Liability"
    );

    // Shock P1's nontoken Grizzly Bears -> P1 loses 1 life from Liability.
    let opp_bear = on_battlefield(&engine, p1, grizzly_bears()).expect("P1 bear");
    cast_from_hand(&mut engine, p0, shock());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![opp_bear],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[1].life,
        19,
        "nontoken permanent put into P1's graveyard causes P1 to lose 1 life"
    );
}
