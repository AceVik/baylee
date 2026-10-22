//! `cards/creatures/mv_2/soldevi_sage.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn soldevi_sage() -> CardIndex {
    card_index("1f612df3-53b6-4317-9d63-1f903ee3f0c4")
}

/// Soldevi Sage ({1}{U} 1/1 Creature — Human Wizard):
/// - "{T}, Sacrifice two lands: Draw three cards, then discard one of them."
#[test]
fn soldevi_sage_sacrifices_two_lands_to_draw_three_and_discard_one() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[soldevi_sage(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let sage = on_battlefield(&engine, p0, soldevi_sage()).unwrap();

    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    let lands: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| id != sage)
        .collect();
    assert_eq!(lands.len(), 2);

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: sage,
                ability_index: 0,
            },
        )
        .unwrap();
    for land in lands {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![land],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("expected discard choice");
    };
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![options[0]],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 2
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Graveyard(p0)).len(),
        3
    );
}
