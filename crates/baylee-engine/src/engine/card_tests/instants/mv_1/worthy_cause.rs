//! `cards/instants/mv_1/worthy_cause.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::HILL_GIANT;

fn worthy_cause() -> CardIndex {
    card_index("b24063f7-157f-47c0-919a-33856d53b902")
}

/// `Worthy Cause` ({W}, Instant, `Coverage::Partial`):
/// `Buyback {2}`
/// `As an additional cost to cast this spell, sacrifice a creature.`
/// `You gain life equal to the sacrificed creature's toughness.`
#[test]
fn worthy_cause_sacrifices_a_creature_and_gains_life_equal_to_its_toughness() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), HILL_GIANT])
        .hand(0, &[worthy_cause()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p0, HILL_GIANT).unwrap();
    cast_from_hand(&mut engine, p0, worthy_cause());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![giant],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, HILL_GIANT).is_none());
    assert_eq!(engine.state().players[0].life, 23);
}

#[test]
fn no_creature_means_the_additional_cost_cannot_be_paid() {
    use crate::engine::card_tests::legends_kit as lk;
    let p0 = PlayerId::new(0);
    let mut e = lk::game(SEED, &[], &[], &[worthy_cause()], &[]);
    lk::float(&mut e, p0, &[(ManaColor::White, 5)]);
    let card = in_hand(&e, p0, worthy_cause()).unwrap();
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority");
    };
    assert!(
        !legal.castable.contains(&card) && !legal.payable.contains(&card),
        "mana alone cannot pay the sacrifice"
    );
}
