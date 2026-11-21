//! `cards/creatures/mv_6/necrosavant.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn necrosavant() -> CardIndex {
    card_index("6732a587-e3cc-4c30-9267-f8949dd8724a")
}

/// During your upkeep, paying `{3}{B}{B}` and sacrificing a creature you control
/// returns Necrosavant from your graveyard to the battlefield; it cannot be
/// activated in main phases or during an opponent's upkeep.
#[test]
fn necrosavant_returns_from_graveyard_only_during_your_upkeep_by_sacrificing_your_creature() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                necrosavant(),
                grizzly_bears(),
                llanowar_elves(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
            ],
        )
        .battlefield(1, &[craw_wurm()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Put Necrosavant into p0's graveyard.
    let savant_bf = on_battlefield(&engine, p0, necrosavant()).unwrap();
    bury(&mut engine, &[savant_bf]);
    engine.refresh_offer();
    let savant_gy = in_graveyard(&engine, p0, necrosavant()).unwrap();
    let swamps = all_on_battlefield(&engine, p0, swamp());

    // Main phase (even with 5 mana floating and creatures to sacrifice): not offered.
    tap_mana_where(&mut engine, p0, |id| swamps.contains(&id));
    let Pending::Priority { legal, .. } = engine.pending() else {
        unreachable!()
    };
    assert!(
        !legal.abilities.contains(&(savant_gy, 0)),
        "not legal during FirstMain"
    );

    // Opponent's upkeep: not offered even if p0 holds priority.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { .. })
    });
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    let Pending::Priority { player, legal } = engine.pending() else {
        unreachable!()
    };
    assert_eq!(*player, p0);
    assert!(
        !legal.abilities.contains(&(savant_gy, 0)),
        "not legal during opponent's upkeep"
    );

    // Advance to p0's next upkeep: tap 5 Swamps and activate from graveyard.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.step == Step::Upkeep
            && matches!(e.pending(), Pending::Priority { .. })
    });
    tap_mana_where(&mut engine, p0, |id| swamps.contains(&id));
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let elves = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    let opp_wurm = on_battlefield(&engine, p1, craw_wurm()).unwrap();

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: savant_gy,
                ability_index: 0,
            },
        )
        .unwrap();
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!(
            "expected sacrifice creature choice, got {:?}",
            engine.pending()
        );
    };
    assert!(options.contains(&bears) && options.contains(&elves));
    assert!(
        !options.contains(&opp_wurm),
        "cannot sacrifice opponent's creature"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, grizzly_bears()).is_some());
    let returned = on_battlefield(&engine, p0, necrosavant()).expect("returned to battlefield");
    assert_eq!(pt(&engine, returned), (5, 5));
}
