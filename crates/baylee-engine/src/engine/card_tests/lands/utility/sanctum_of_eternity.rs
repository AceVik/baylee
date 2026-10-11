//! `cards/lands/utility/sanctum_of_eternity.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Sanctum of Eternity: "{T}: Add {C}." / "{2}, {T}: Return target commander you own from the battlefield to your hand."
/// Activating Sanctum of Eternity produces {C} and leaves the land tapped.
#[test]
fn sanctum_of_eternity_taps_for_colorless_mana() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(130, forest())
        .battlefield(0, &[sanctum_of_eternity()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p0, sanctum_of_eternity()).expect("Sanctum deployed");
    activate(&mut engine, p0, sanctum_of_eternity(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1);
    assert_eq!(pool.available(ManaColor::Colorless), 1);
    assert!(is_tapped(&engine, land));
}

#[test]
fn sanctum_bounces_your_commander_during_combat_but_not_an_opponents_turn() {
    use baylee_core::generated::index;
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .commander(0, &[index::SAVANNAH_LIONS])
        .battlefield(0, &[sanctum_of_eternity(), plains()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let commander = e.state().commanders[0][0].object;
    tap_all_mana(&mut e, p0);
    e.apply(p0, PlayerAction::CastSpell { card: commander })
        .unwrap();
    pass_until(&mut e, stack_is_empty);
    // The target is ours even while an opponent controls it.
    let state = e.dev_state_mut(p0).unwrap();
    state.object_mut(commander).unwrap().set_controller(p1);
    state.invalidate_projections();
    let sanctum = on_battlefield(&e, p0, sanctum_of_eternity()).unwrap();
    e.dev_state_mut(p0).unwrap().set_tapped(sanctum, false);
    pass_until(&mut e, |e| {
        e.state().turn.step == crate::turn::Step::CombatBegin
    });
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 2);
    e.refresh_offer();
    activate(&mut e, p0, sanctum_of_eternity(), 1);
    aim_at(&mut e, p0, commander);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(commander).unwrap().zone, Zone::Hand);
    reach_their_main_phase(&mut e, p1);
    // A valid target is on the battlefield again; only the turn bars this activation.
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
    state.set_tapped(sanctum, false);
    state.players[0].mana_pool.add(ManaColor::Colorless, 2);
    e.refresh_offer();
    e.apply(p1, PlayerAction::PassPriority).unwrap();
    assert!(
        e.apply(
            p0,
            PlayerAction::ActivateAbility {
                source: sanctum,
                ability_index: 1
            }
        )
        .is_err()
    );
}
