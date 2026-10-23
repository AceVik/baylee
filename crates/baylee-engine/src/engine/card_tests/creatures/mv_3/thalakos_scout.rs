#[allow(clippy::wildcard_imports)]
use super::*;

fn thalakos_scout() -> CardIndex {
    card_index("c8877da5-7008-4b47-955e-d205220c91e6")
}

#[test]
fn thalakos_scout_discards_a_card_to_return_itself_to_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[thalakos_scout(), island()])
        .battlefield(0, &[island(), island(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, thalakos_scout());
    pass_until(&mut engine, stack_is_empty);
    let scout = on_battlefield(&engine, p0, thalakos_scout()).expect("seated");
    let card_in_hand = in_hand(&engine, p0, island()).expect("in hand");

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: scout,
                ability_index: 0,
            },
        )
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseCards { .. }) {
        engine
            .apply(
                p0,
                PlayerAction::ChooseObjects {
                    objects: vec![card_in_hand],
                },
            )
            .unwrap();
    }
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, thalakos_scout()).is_none());
    assert!(in_hand(&engine, p0, thalakos_scout()).is_some());
    assert!(in_graveyard(&engine, p0, island()).is_some());
}
