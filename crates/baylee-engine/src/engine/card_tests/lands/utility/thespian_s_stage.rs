//! `cards/lands/utility/thespian_s_stage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

#[test]
fn thespian_s_stage_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(112, forest())
        .battlefield(0, &[thespian_s_stage()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let stage = on_battlefield(&engine, p0, thespian_s_stage()).expect("Stage deployed");
    activate(&mut engine, p0, thespian_s_stage(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, stage));
}

#[test]
fn stage_keeps_its_copy_ability_and_does_not_copy_entry_tapped_state() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[thespian_s_stage(), forest(), island()])
        .hand(0, &[vesuva()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let stage = on_battlefield(&e, p0, thespian_s_stage()).unwrap();
    let model = on_battlefield(&e, p0, forest()).unwrap();
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 4);
    e.refresh_offer();
    activate(&mut e, p0, thespian_s_stage(), 1);
    aim_at(&mut e, p0, model);
    pass_until(&mut e, stack_is_empty);
    assert!(
        e.state()
            .object(stage)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::land::FOREST)
    );
    assert!(is_tapped(&e, stage));
    // A second copy receives the retained activation, but only Vesuva's own
    // replacement causes it to enter tapped.
    let card = in_hand(&e, p0, vesuva()).unwrap();
    e.apply(p0, PlayerAction::PlayLand { card }).unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![stage],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    let copy = on_battlefield(&e, p0, vesuva()).unwrap();
    assert!(is_tapped(&e, copy));
    e.dev_state_mut(p0).unwrap().set_tapped(copy, false);
    e.refresh_offer();
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority")
    };
    let ability_index = legal
        .abilities
        .iter()
        .find(|(id, index)| *id == copy && *index == 1)
        .unwrap()
        .1;
    e.apply(
        p0,
        PlayerAction::ActivateAbility {
            source: copy,
            ability_index,
        },
    )
    .unwrap();
    let island = on_battlefield(&e, p0, island()).unwrap();
    aim_at(&mut e, p0, island);
    pass_until(&mut e, stack_is_empty);
    assert!(
        e.state()
            .object(copy)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::land::ISLAND)
    );
    e.dev_state_mut(p0).unwrap().set_tapped(stage, false);
    e.refresh_offer();
    // The retained copy ability has the original 2,T price, not zero.
    let Pending::Priority { legal, .. } = e.pending() else {
        panic!("priority")
    };
    assert!(!legal.abilities.contains(&(stage, 1)));
}
