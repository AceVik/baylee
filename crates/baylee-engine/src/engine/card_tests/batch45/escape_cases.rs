//! Escape and flashback keep additional costs separate from alternative costs.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn batch45_escape_can_kick_a_spell_whose_ordinary_target_is_unavailable() {
    let mut e = prepared(
        &[index::TEAR_ASUNDER],
        &[index::UNDERWORLD_BREACH],
        &[index::GRIZZLY_BEARS],
    );
    let spell = hand_to_graveyard(&mut e, P0, index::TEAR_ASUNDER);
    let bear = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    seed_graveyard(&mut e, P0, 3);
    pool(&mut e, ManaColor::Green, 1);
    e.state.players[0].mana_pool.add(ManaColor::Black, 1);
    e.state.players[0].mana_pool.add(ManaColor::Colorless, 2);
    assert!(castable(&e, spell));
    cast(&mut e, spell);
    mode(
        &mut e,
        CastModeKind::EscapeGranted {
            face: 0,
            modes: 0,
            exile: 3,
            kicked: true,
            prototype: false,
        },
    );
    target(&mut e, vec![bear], vec![]);
    let fodder = cards_offered(&e);
    choose(&mut e, fodder);
    assert!(e.state.object(spell).unwrap().kicked);
    assert_eq!(
        e.state.players[0].mana_pool.total(),
        0,
        "kicker is paid exactly once"
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(bear).unwrap().zone, Zone::Exile);
    assert_eq!(e.state.object(spell).unwrap().zone, Zone::Graveyard);
}

#[test]
fn batch45_escape_uses_a_graveyard_dinosaurs_adventure_cost_and_return_permission() {
    let mut e = prepared(
        &[index::BRAZEN_BORROWER, index::WELCOME_TO],
        &[],
        &[index::SOL_RING],
    );
    cast_with_floating(&mut e, P0, index::WELCOME_TO);
    pass_until(&mut e, |engine| {
        matches!(engine.pending(), Pending::ChooseTargets { .. })
    });
    target(&mut e, vec![], vec![]);
    pass_until(&mut e, stack_is_empty);
    let park = on_battlefield(&e, P0, index::WELCOME_TO).unwrap();
    let def = RegistryLookup.card(index::WELCOME_TO).unwrap();
    e.state.switch_face(park, def, 1);
    e.sync_static_effects();
    let adventure = hand_to_graveyard(&mut e, P0, index::BRAZEN_BORROWER);
    // A Dinosaur Adventure is the relevant rules case; the pool currently has
    // none, so change only this fixture's front characteristics in the graveyard.
    e.state
        .object_mut(adventure)
        .unwrap()
        .base_mut()
        .subtypes
        .insert(subtypes::creature::DINOSAUR);
    e.state.object_mut(adventure).unwrap().cache.clear();
    e.state.refresh_characteristics();
    let ring = on_battlefield(&e, P1, index::SOL_RING).unwrap();
    seed_graveyard(&mut e, P0, 3);
    pool(&mut e, ManaColor::Blue, 2);
    assert!(
        castable(&e, adventure),
        "the Adventure costs 1U, the creature costs 1UU"
    );
    cast(&mut e, adventure);
    mode(
        &mut e,
        CastModeKind::EscapeGranted {
            face: 1,
            modes: 0,
            exile: 3,
            kicked: false,
            prototype: false,
        },
    );
    target(&mut e, vec![ring], vec![]);
    let fodder = cards_offered(&e);
    choose(&mut e, fodder);
    assert_eq!(e.state.object(adventure).unwrap().face_index, 1);
    assert_eq!(e.state.players[0].mana_pool.total(), 0);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(ring).unwrap().zone, Zone::Hand);
    assert_eq!(e.state.object(adventure).unwrap().zone, Zone::Exile);
    assert!(crate::casting::adventure_permission(
        &e.state, P0, adventure
    ));
}

fn grant_snapcaster(e: &mut Engine<RegistryLookup>, spell: ObjectId) {
    cast_with_floating(e, P0, index::SNAPCASTER_MAGE);
    pass_until(e, |engine| {
        matches!(engine.pending(), Pending::ChooseTargets { .. })
    });
    target(e, vec![spell], vec![]);
    pass_until(e, stack_is_empty);
    assert!(crate::casting::flashback_granted(&e.state, spell));
}

#[test]
fn batch45_granted_flashback_allows_kicker_and_exiles_the_finished_spell() {
    let mut e = prepared(
        &[index::SNAPCASTER_MAGE, index::TEAR_ASUNDER],
        &[],
        &[index::GRIZZLY_BEARS],
    );
    let spell = hand_to_graveyard(&mut e, P0, index::TEAR_ASUNDER);
    grant_snapcaster(&mut e, spell);
    let bear = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    cast(&mut e, spell);
    mode(&mut e, CastModeKind::Kicked);
    target(&mut e, vec![bear], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(bear).unwrap().zone, Zone::Exile);
    assert_eq!(e.state.object(spell).unwrap().zone, Zone::Exile);
}

#[test]
fn batch45_granted_flashback_rejects_overload_in_both_offer_and_wizard() {
    let mut e = prepared(&[index::SNAPCASTER_MAGE, index::DAMN], &[], &[]);
    let spell = hand_to_graveyard(&mut e, P0, index::DAMN);
    grant_snapcaster(&mut e, spell);
    pool(&mut e, ManaColor::White, 4);
    assert!(
        !castable(&e, spell),
        "overload cannot be combined with flashback"
    );
    pool(&mut e, ManaColor::Black, 2);
    e.state.players[0].mana_pool.add(ManaColor::White, 4);
    cast(&mut e, spell);
    if let Pending::ChooseCastMode { options, .. } = e.pending() {
        assert!(
            options
                .iter()
                .all(|option| option.kind != CastModeKind::Mode(1))
        );
        mode(&mut e, CastModeKind::Mode(0));
    }
    let snapcaster = on_battlefield(&e, P0, index::SNAPCASTER_MAGE).unwrap();
    target(&mut e, vec![snapcaster], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(spell).unwrap().zone, Zone::Exile);
}

#[test]
fn batch45_kess_can_pay_overload_without_using_an_available_flashback_grant() {
    let mut e = prepared(
        &[index::SNAPCASTER_MAGE, index::DAMN],
        &[index::KESS_DISSIDENT_MAGE],
        &[],
    );
    let spell = hand_to_graveyard(&mut e, P0, index::DAMN);
    grant_snapcaster(&mut e, spell);
    pool(&mut e, ManaColor::White, 4);
    assert!(castable(&e, spell));
    cast(&mut e, spell);
    mode(&mut e, CastModeKind::Mode(1));
    assert!(
        matches!(e.pending(), Pending::Priority { .. }),
        "overload has only the Kess permission"
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state.object(spell).unwrap().zone, Zone::Exile);
    assert!(on_battlefield(&e, P0, index::KESS_DISSIDENT_MAGE).is_none());
}

#[test]
fn batch45_breach_escapes_a_prototype_for_its_smaller_cost_and_characteristics() {
    let mut e = prepared(
        &[index::PHYREXIAN_FLESHGORGER],
        &[index::UNDERWORLD_BREACH],
        &[],
    );
    let fleshgorger = hand_to_graveyard(&mut e, P0, index::PHYREXIAN_FLESHGORGER);
    seed_graveyard(&mut e, P0, 3);
    pool(&mut e, ManaColor::Black, 3);
    assert!(castable(&e, fleshgorger));
    cast(&mut e, fleshgorger);
    mode(
        &mut e,
        CastModeKind::EscapeGranted {
            face: 0,
            modes: 0,
            exile: 3,
            kicked: false,
            prototype: true,
        },
    );
    let fodder = cards_offered(&e);
    choose(&mut e, fodder);
    assert_eq!(e.state.players[0].mana_pool.total(), 0);
    assert!(e.state.object(fleshgorger).unwrap().prototyped);
    assert!(
        e.state
            .object(fleshgorger)
            .unwrap()
            .riders
            .contains(&Rider::Escaped)
    );
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, fleshgorger), (3, 3));
    kill(&mut e, fleshgorger);
    assert_eq!(e.state.object(fleshgorger).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        e.state
            .object(fleshgorger)
            .unwrap()
            .characteristics()
            .mana_value(),
        7
    );
}
