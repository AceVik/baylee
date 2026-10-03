//! Independent Chromatic Sphere stack and payment-window behavior.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;
const USER: PlayerId = PlayerId::new(0);
const OTHER: PlayerId = PlayerId::new(1);

fn sphere_fixture() -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[index::CHROMATIC_SPHERE, forest(), plains(), plains()])
        .hand(0, &[index::GUARDIAN_ANGEL])
        .battlefield(1, &[island(), island(), island()])
        .hand(1, &[counterspell(), index::TISHANA_S_TIDEBINDER])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, USER);
    let sphere = on_battlefield(&engine, USER, index::CHROMATIC_SPHERE).unwrap();
    (engine, sphere)
}

fn sphere_priority(engine: &mut Engine<RegistryLookup>, player: PlayerId) {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::Priority { player: p, .. } if *p == player),
    );
}

fn sphere_activate(engine: &mut Engine<RegistryLookup>, sphere: ObjectId) -> ObjectId {
    let forest = on_battlefield(engine, USER, forest()).unwrap();
    engine
        .apply(USER, PlayerAction::ActivateManaAbility { source: forest })
        .unwrap();
    engine
        .apply(
            USER,
            PlayerAction::ActivateAbility {
                source: sphere,
                ability_index: 0,
            },
        )
        .unwrap();
    assert_eq!(engine.state().object(sphere).unwrap().zone, Zone::Graveyard);
    assert_eq!(engine.state().zones.list(ZoneLocation::Stack).len(), 1);
    assert!(matches!(engine.pending(), Pending::Priority { .. }));
    *engine
        .state()
        .zones
        .list(ZoneLocation::Stack)
        .last()
        .unwrap()
}

#[test]
fn sphere_review_color_and_draw_wait_for_resolving_the_respondable_ability() {
    let (mut engine, sphere) = sphere_fixture();
    let hand = engine.state().zones.list(ZoneLocation::Hand(USER)).len();
    sphere_activate(&mut engine, sphere);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(USER)).len(),
        hand
    );
    sphere_priority(&mut engine, OTHER);
    assert!(
        !stack_is_empty(&engine),
        "opponent receives a real response opportunity"
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    let Pending::ChooseColor { player, options } = engine.pending() else {
        panic!("color");
    };
    assert_eq!(*player, USER);
    assert_eq!(options.len(), 5);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(USER)).len(),
        hand
    );
    engine
        .apply(USER, PlayerAction::ChooseColor(ManaColor::Blue))
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Blue),
        1
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 1);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(USER)).len(),
        hand + 1
    );
}

#[test]
fn sphere_review_counterspell_cannot_counter_it_but_real_tidebinder_can() {
    let (mut engine, sphere) = sphere_fixture();
    let hand = engine.state().zones.list(ZoneLocation::Hand(USER)).len();
    let ability = sphere_activate(&mut engine, sphere);
    sphere_priority(&mut engine, OTHER);
    let counter = in_hand(&engine, OTHER, counterspell()).unwrap();
    let before = engine.fingerprint();
    assert!(
        engine
            .apply(OTHER, PlayerAction::CastSpell { card: counter })
            .is_err()
    );
    assert_eq!(
        engine.fingerprint(),
        before,
        "Counterspell cannot target an ability"
    );
    cast_from_hand(&mut engine, OTHER, index::TISHANA_S_TIDEBINDER);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("ability target");
    };
    assert!(options.contains(&ability));
    aim(&mut engine, vec![ability], vec![]);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(USER)).len(),
        hand
    );
    assert_eq!(engine.state().object(sphere).unwrap().zone, Zone::Graveyard);
    assert!(on_battlefield(&engine, OTHER, index::TISHANA_S_TIDEBINDER).is_some());
}

#[test]
fn sphere_review_real_mana_payment_omits_and_atomically_refuses_activation() {
    let (mut engine, sphere) = sphere_fixture();
    let plains = on_battlefield(&engine, USER, plains()).unwrap();
    engine
        .apply(USER, PlayerAction::ActivateManaAbility { source: plains })
        .unwrap();
    cast_with_floating(&mut engine, USER, index::GUARDIAN_ANGEL);
    engine.apply(USER, PlayerAction::ChooseNumber(0)).unwrap();
    aim(&mut engine, vec![], vec![USER]);
    pass_until(&mut engine, stack_is_empty);
    sphere_priority(&mut engine, USER);
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("priority");
    };
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert!(
        legal
            .unpaid_abilities
            .iter()
            .any(|(source, index, _)| *source == sphere && *index == 0),
        "normal priority offers the ability with its unpaid cost"
    );
    let action = legal
        .granted_actions
        .iter()
        .find(|offer| {
            matches!(
                offer.effect,
                crate::choice::GrantedActionKind::PreventNextDamage { .. }
            )
        })
        .unwrap()
        .id;
    engine
        .apply(USER, PlayerAction::TakeGrantedAction { id: action })
        .unwrap();
    assert_eq!(
        engine.payment_window().map(|(player, _)| player),
        Some(USER)
    );
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("payment");
    };
    assert!(!legal.abilities.contains(&(sphere, 0)));
    assert!(!legal.mana_abilities.contains(&sphere));
    let before = engine.fingerprint();
    assert!(
        engine
            .apply(
                USER,
                PlayerAction::ActivateAbility {
                    source: sphere,
                    ability_index: 0
                }
            )
            .is_err()
    );
    assert_eq!(engine.fingerprint(), before);
    assert!(!is_tapped(&engine, sphere));
    let forest = on_battlefield(&engine, USER, forest()).unwrap();
    engine
        .apply(USER, PlayerAction::ActivateManaAbility { source: forest })
        .unwrap();
    engine.apply(USER, PlayerAction::PassPriority).unwrap();
    assert_eq!(engine.payment_window(), None);
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert_eq!(
        engine.state().object(sphere).unwrap().zone,
        Zone::Battlefield
    );
    assert!(stack_is_empty(&engine));
}
