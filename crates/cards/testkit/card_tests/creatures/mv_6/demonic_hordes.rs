//! `cards/creatures/mv_6/demonic_hordes.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Demonic Hordes pays its upkeep before using `{T}: Destroy target land`.
/// `Filter::LAND` names no side, so the menu holds a land from either seat
/// and excludes the creature standing beside them.
#[test]
fn demonic_hordes_taps_to_destroy_a_targeted_land_of_either_seat() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[demonic_hordes(), swamp(), swamp(), swamp()])
        .battlefield(1, &[forest(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    assert!(
        matches!(engine.pending(), Pending::YesNo { player, prompt: YesNoPrompt::PayMana { cost }, .. }
        if *player == p0 && *cost == baylee_core::mana!("{B}{B}{B}"))
    );
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(tap_all_mana(&mut engine, p0), 3);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    reach_main_phase(&mut engine, p0);

    let hordes = on_battlefield(&engine, p0, demonic_hordes()).expect("seated");
    assert_eq!(pt(&engine, hordes), (5, 5), "the body the card prints");
    let my_swamp = on_battlefield(&engine, p0, swamp()).expect("seated");
    let their_forest = on_battlefield(&engine, p1, forest()).expect("seated");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("seated");

    activate(&mut engine, p0, demonic_hordes(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target land\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (1, 1), "one land, and the ability asks once");
    assert!(
        options.contains(&my_swamp) && options.contains(&their_forest),
        "\"target land\" is any land, on either side of the table: {options:?}"
    );
    assert!(
        !options.contains(&their_elf),
        "a creature is no land, whatever side it stands on: {options:?}"
    );
    assert!(
        !is_tapped(&engine, hordes),
        "targets are chosen before the {{T}} cost is paid"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![their_forest],
                players: vec![],
            },
        )
        .expect("the Forest was among the options the ability enumerated");
    assert!(is_tapped(&engine, hordes), "the {{T}} was the price");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, forest()).is_some(),
        "\"destroy target land\": the targeted Forest was destroyed"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_none(),
        "and left the battlefield, which is what destroy means"
    );
    assert!(
        on_battlefield(&engine, p0, swamp()).is_some(),
        "the Swamp nobody targeted stands untouched"
    );
    assert!(
        on_battlefield(&engine, p0, demonic_hordes()).is_some(),
        "the activation cost Demonic Hordes nothing but its tap"
    );
}
