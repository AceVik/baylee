#[allow(clippy::wildcard_imports)]
use super::*;

fn puffer_extract() -> CardIndex {
    card_index("4f427de6-b551-4699-b78a-44ff668198d8")
}

#[test]
fn puffer_extract_pumps_controlled_creature_by_x_and_destroys_it_at_next_end_step() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let bears_card = baylee_core::generated::index::GRIZZLY_BEARS;
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[puffer_extract()])
        .battlefield(
            0,
            &[
                llanowar_elves(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, &[bears_card])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, puffer_extract());
    pass_until(&mut engine, stack_is_empty);

    let extract = on_battlefield(&engine, p0, puffer_extract()).expect("Puffer Extract seated");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("elves seated");
    let bears = on_battlefield(&engine, p1, bears_card).expect("opponent bears seated");

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: extract,
                ability_index: 0,
            },
        )
        .unwrap();
    if matches!(engine.pending(), Pending::ChooseNumber { .. }) {
        engine.apply(p0, PlayerAction::ChooseNumber(3)).unwrap();
    }
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![bears],
                    players: vec![],
                },
            )
            .is_err(),
        "opponent creature must not be a legal target for Puffer Extract"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elves],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, elves), (4, 4));

    pass_until(&mut engine, |e| e.state().turn.step == Step::End);
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "creature pumped by Puffer Extract must be destroyed at the next end step"
    );
    assert!(in_graveyard(&engine, p0, llanowar_elves()).is_some());
}
