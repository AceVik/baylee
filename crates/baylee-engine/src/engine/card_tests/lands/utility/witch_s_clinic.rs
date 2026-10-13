//! `cards/lands/utility/witch_s_clinic.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Witch's Clinic: "{T}: Add {C}." / "{2}, {T}: Target commander gains lifelink until end of turn."
/// Under `Coverage::Partial`, targeting a commander is unsupported and the lifelink ability is omitted.
/// Witch's Clinic is played as a land and taps for colorless mana, offering no lifelink activation.
#[test]
fn witch_s_clinic_taps_for_colorless_and_omits_unsupported_lifelink() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(99, forest())
        .hand(0, &[witch_s_clinic()])
        .battlefield(0, &[forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let clinic = play_land(&mut engine, p0, witch_s_clinic());
    assert!(!is_tapped(&engine, clinic));

    // Everything but the land under test: `tap_all_mana` takes printed mana
    // abilities as well as the CR 305.6 shortcut (#159), so tapping it would
    // remove the very offer this asserts on.
    tap_mana_except(&mut engine, p0, clinic);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(clinic, 0)),
        "colorless mana ability is offered"
    );
    assert!(
        !legal.abilities.iter().any(|(s, i)| *s == clinic && *i == 1),
        "unsupported commander lifelink ability is omitted"
    );

    activate(&mut engine, p0, witch_s_clinic(), 0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        1
    );
    assert!(is_tapped(&engine, clinic));
}

#[test]
fn clinic_can_give_an_opposing_commander_lifelink_but_not_an_ordinary_creature() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .commander(1, &[index::SAVANNAH_LIONS])
        .battlefield(0, &[index::WITCH_S_CLINIC, index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let commander = e.state().commanders[1][0].object;
    let state = e.dev_state_mut(p0).unwrap();
    state.object_mut(commander).unwrap().kind = crate::object::ObjectKind::Permanent;
    state
        .move_object(
            commander,
            ZoneLocation::Battlefield,
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    state.players[0].mana_pool.add(ManaColor::Colorless, 2);
    e.refresh_offer();
    activate(&mut e, p0, index::WITCH_S_CLINIC, 1);
    let Pending::ChooseTargets { options, .. } = e.pending() else {
        panic!("targets")
    };
    assert_eq!(options, &vec![commander]);
    aim_at(&mut e, p0, commander);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(commander).unwrap().controller, p1);
    assert!(keywords(&e, commander).contains(KeywordSet::LIFELINK));
}

/// "gains lifelink until end of turn": the opposing commander has lifelink
/// once the ability has resolved, and has lost it again by the opponent's
/// own main phase, after p0's turn has ended.
#[test]
fn clinic_lifelink_lasts_only_until_end_of_turn() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .commander(1, &[index::SAVANNAH_LIONS])
        .battlefield(0, &[index::WITCH_S_CLINIC])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let commander = e.state().commanders[1][0].object;
    let state = e.dev_state_mut(p0).unwrap();
    state.object_mut(commander).unwrap().kind = crate::object::ObjectKind::Permanent;
    state
        .move_object(
            commander,
            ZoneLocation::Battlefield,
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    state.players[0].mana_pool.add(ManaColor::Colorless, 2);
    e.refresh_offer();
    activate(&mut e, p0, index::WITCH_S_CLINIC, 1);
    aim_at(&mut e, p0, commander);
    pass_until(&mut e, stack_is_empty);
    assert!(
        keywords(&e, commander).contains(KeywordSet::LIFELINK),
        "control: the commander has lifelink this turn"
    );
    reach_their_main_phase(&mut e, p1);
    assert_eq!(e.state().turn.active, p1);
    assert!(
        !keywords(&e, commander).contains(KeywordSet::LIFELINK),
        "\"until end of turn\": gone when p0's turn is over"
    );
}
