#[allow(clippy::wildcard_imports)]
use super::*;

fn electryte() -> CardIndex {
    card_index("5ff0c25f-4593-460b-9d7f-d9c37f135a18")
}

#[test]
fn electryte_partial_keeps_its_body_but_does_not_use_an_overbroad_damage_trigger() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .hand(0, &[electryte()])
        .battlefield(
            0,
            &[mountain(), mountain(), mountain(), mountain(), mountain()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, electryte());
    pass_until(&mut engine, stack_is_empty);
    let creature = on_battlefield(&engine, p0, electryte()).unwrap();
    assert_eq!(pt(&engine, creature), (3, 3));
}

#[test]
fn electryte_does_not_sweep_blockers_when_combat_damage_is_redirected_to_a_nondefender() {
    use crate::engine::card_tests::legends_kit as lk;
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = lk::game(
        SEED,
        &[electryte(), grizzly_bears(), index::PERSONAL_INCARNATION],
        &[index::PHYREXIAN_WALKER],
        &[index::MENDING_TOUCH],
        &[index::CONTROL_MAGIC],
    );
    let elec = on_battlefield(&engine, p0, electryte()).unwrap();
    let bear = on_battlefield(&engine, p0, grizzly_bears()).unwrap();
    let incarnation = on_battlefield(&engine, p0, index::PERSONAL_INCARNATION).unwrap();
    // Transfer control through the real Aura. A raw controller mutation is
    // overwritten when the layer system projects the original control state.
    reach_their_main_phase(&mut engine, p1);
    lk::float(&mut engine, p1, &[(ManaColor::Blue, 4)]);
    lk::cast_at(&mut engine, p1, index::CONTROL_MAGIC, &[incarnation], &[]);
    reach_their_main_phase(&mut engine, p0);
    assert_eq!(engine.state().object(incarnation).unwrap().owner, p0);
    assert_eq!(engine.state().object(incarnation).unwrap().controller, p1);
    let elf = on_battlefield(&engine, p1, index::PHYREXIAN_WALKER).unwrap();
    lk::attack(&mut engine, &[elec, bear]);
    lk::block(&mut engine, &[(incarnation, elec), (elf, bear)]);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    // The owner redirects one point of Electryte's combat damage to p0.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: incarnation,
                ability_index: 0,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p0),
    );
    lk::float(&mut engine, p0, &[(ManaColor::Green, 1)]);
    lk::cast_at(&mut engine, p0, index::MENDING_TOUCH, &[elec], &[]);
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::End
    });
    assert_eq!(engine.state().players[0].life, 19);
    assert!(on_battlefield(&engine, p0, electryte()).is_some());
    assert!(
        on_battlefield(&engine, p1, index::PHYREXIAN_WALKER).is_some(),
        "a nondefender damage event must not sweep a different blocker"
    );
    assert_eq!(engine.state().object(elf).unwrap().damage, 2);
}
