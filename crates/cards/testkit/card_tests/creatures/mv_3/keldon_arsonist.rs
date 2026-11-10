//! `cards/creatures/mv_3/keldon_arsonist.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn keldon_arsonist() -> CardIndex {
    card_index("d32f973e-38db-4f74-aec6-258d25c1fb7d")
}

/// Keldon Arsonist ({2}{R} 1/1 Creature — Human Soldier):
/// "{1}, Sacrifice two lands: Destroy target land."
#[test]
fn keldon_arsonist_sacrifices_two_lands_to_destroy_target_land() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[keldon_arsonist(), mountain(), mountain(), mountain()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let opp_land = on_battlefield(&engine, p1, forest()).unwrap();
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, keldon_arsonist(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![opp_land],
            },
        )
        .unwrap();
    for _ in 0..2 {
        let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
            panic!(
                "expected ChooseCards for land sacrifice, got {:?}",
                engine.pending()
            );
        };
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![options[0]],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(all_on_battlefield(&engine, p0, mountain()).len(), 1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        2
    );
    assert!(on_battlefield(&engine, p1, forest()).is_none());
    assert!(in_graveyard(&engine, p1, forest()).is_some());
}

/// Keldon Arsonist cannot activate its ability when its controller controls
/// only one land (since the cost requires sacrificing two lands).
#[test]
fn keldon_arsonist_cannot_activate_with_only_one_land() {
    let (p0, _p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[keldon_arsonist(), mountain()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    let arsonist = on_battlefield(&engine, p0, keldon_arsonist()).unwrap();
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: arsonist,
                    ability_index: 0,
                },
            )
            .is_err(),
        "Keldon Arsonist should not be activatable with only one land on the battlefield"
    );
}
