#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;

fn fortitude() -> CardIndex {
    card_index("5017c07d-8ad5-4ee2-91d3-974811cdff75")
}

#[test]
fn fortitude_sacrifices_only_your_forest_to_regenerate_its_host_and_returns_on_death() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = lk::game(
        SEED,
        &[forest(), forest(), llanowar_elves()],
        &[forest()],
        &[fortitude(), lightning_bolt()],
        &[],
    );
    let elves = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let forest_id = on_battlefield(&engine, p0, forest()).unwrap();
    let their_forest = on_battlefield(&engine, p1, super::forest()).unwrap();
    lk::float(&mut engine, p0, &[(ManaColor::Green, 2)]);
    lk::cast_at(&mut engine, p0, fortitude(), &[elves], &[]);
    activate(&mut engine, p0, fortitude(), 1);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("Forest cost");
    };
    assert!(options.contains(&forest_id));
    assert!(!options.contains(&elves));
    assert!(!options.contains(&their_forest));
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![their_forest]
                }
            )
            .is_err()
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![forest_id],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, super::forest()).is_some());
    assert_eq!(
        engine.state().object(elves).unwrap().regeneration_shields,
        1,
        "sacrificing the Forest creates a regeneration shield on the enchanted creature"
    );
    assert_eq!(
        pt(&engine, elves),
        (1, 1),
        "the test's creature dies to three damage without regeneration"
    );
    // Fund the Bolt after Fortitude has paid its generic cost, so that cost
    // cannot consume the red mana reserved for this separate spell.
    lk::float(&mut engine, p0, &[(ManaColor::Red, 1)]);
    assert_eq!(lk::pool_of(&engine, p0, ManaColor::Red), 1);
    assert!(
        lk::castable(&engine, p0, lightning_bolt()),
        "the fully funded Bolt is offered before casting"
    );
    lk::cast_at(&mut engine, p0, lightning_bolt(), &[elves], &[]);
    assert!(
        in_graveyard(&engine, p0, lightning_bolt()).is_some(),
        "the Bolt actually resolved"
    );
    assert!(on_battlefield(&engine, p0, llanowar_elves()).is_some());
    assert!(
        lk::tapped(&engine, elves),
        "regeneration taps its creature; remaining shields: {}, marked damage: {}",
        engine.state().object(elves).unwrap().regeneration_shields,
        engine.state().object(elves).unwrap().damage
    );
    assert_eq!(engine.state().object(elves).unwrap().damage, 0);
    let aura = on_battlefield(&engine, p0, fortitude()).unwrap();
    kill(&mut engine, aura);
    assert!(in_hand(&engine, p0, fortitude()).is_some());
}

#[test]
fn fortitude_does_not_return_when_discarded_from_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest()).hand(0, &[fortitude()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    hand_to_graveyard(&mut engine, p0, fortitude());
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, fortitude()).is_some());
    assert!(in_hand(&engine, p0, fortitude()).is_none());
}
