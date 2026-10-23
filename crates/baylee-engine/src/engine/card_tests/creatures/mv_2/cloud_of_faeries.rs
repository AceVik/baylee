#[allow(clippy::wildcard_imports)]
use super::*;

fn cloud_of_faeries() -> CardIndex {
    card_index("adc1f3b4-cce7-42c9-a208-dbab1d1924f5")
}

#[test]
fn cloud_of_faeries_untaps_up_to_two_lands_on_etb() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[cloud_of_faeries()])
        .battlefield(0, &[island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, cloud_of_faeries());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let lands: Vec<_> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            engine
                .state()
                .object(id)
                .and_then(|o| o.card)
                .map(|c| c.index)
                == Some(island())
        })
        .collect();
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: lands.clone(),
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    for l in lands {
        assert!(
            !engine
                .state()
                .object(l)
                .unwrap()
                .status
                .contains(Status::TAPPED)
        );
    }
}
