//! Independent card audit: Earthbind is cast and answered with real spells.
#[allow(clippy::wildcard_imports)] // Shared card test vocabulary.
use super::*;
use crate::event::{DamageTarget, GameEvent};
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn board(host_card: CardIndex, spells: &[CardIndex]) -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = Duel::new(1207, forest())
        .battlefield(
            0,
            &[mountain(), island(), island(), island(), plains(), plains()],
        )
        .battlefield(1, &[host_card, index::AIR_ELEMENTAL])
        .hand(0, spells)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let host = on_battlefield(&engine, P1, host_card).expect("host seated");
    tap_all_mana(&mut engine, P0);
    (engine, host)
}

fn cast_at(engine: &mut Engine<RegistryLookup>, card: CardIndex, target: ObjectId) -> ObjectId {
    pass_until(
        engine,
        |engine| matches!(engine.pending(), Pending::Priority { player, .. } if *player == P0),
    );
    let source = in_hand(engine, P0, card).expect("spell in hand");
    cast_with_floating(engine, P0, card);
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("spell must ask for its target")
    };
    assert!(options.contains(&target), "target must be legally offered");
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .expect("offered target accepted");
    source
}

fn enter_earthbind(engine: &mut Engine<RegistryLookup>, host: ObjectId) -> ObjectId {
    let aura = cast_at(engine, earthbind(), host);
    pass_until(engine, |engine| {
        engine
            .state()
            .object(aura)
            .is_some_and(|object| object.zone == Zone::Battlefield)
    });
    aura
}

fn flies(engine: &Engine<RegistryLookup>, host: ObjectId) -> bool {
    keywords(engine, host).contains(KeywordSet::FLYING)
}

fn hits(engine: &Engine<RegistryLookup>, aura: ObjectId) -> Vec<(ObjectId, u16)> {
    engine
        .journal()
        .entries()
        .iter()
        .filter_map(|entry| match entry.event {
            GameEvent::DamageDealt {
                source: Some(source),
                target: DamageTarget::Object(host),
                amount,
                is_combat: false,
            } if source == aura => Some((host, amount)),
            _ => None,
        })
        .collect()
}

#[test]
fn earthbind_independent_later_jump_flies_until_cleanup_then_loss_resumes() {
    let (mut engine, host) = board(index::WALL_OF_AIR, &[earthbind(), index::JUMP]);
    let aura = enter_earthbind(&mut engine, host);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(hits(&engine, aura), vec![(host, 2)]);
    assert!(!flies(&engine, host));
    cast_at(&mut engine, index::JUMP, host);
    pass_until(&mut engine, stack_is_empty);
    assert!(flies(&engine, host), "later timestamp wins");
    reach_their_main_phase(&mut engine, P1);
    assert!(
        !flies(&engine, host),
        "Earthbind lasts beyond Jump's cleanup"
    );
    assert_eq!(hits(&engine, aura), vec![(host, 2)], "no repeated damage");
}

#[test]
fn earthbind_independent_nonflier_gains_flight_without_retroactive_trigger() {
    let (mut engine, host) = board(index::WALL_OF_WOOD, &[earthbind(), flight()]);
    let aura = enter_earthbind(&mut engine, host);
    assert!(
        stack_is_empty(&engine),
        "intervening condition prevents the trigger"
    );
    cast_at(&mut engine, flight(), host);
    pass_until(&mut engine, stack_is_empty);
    assert!(flies(&engine, host));
    assert!(hits(&engine, aura).is_empty());
    reach_their_main_phase(&mut engine, P1);
    assert!(
        flies(&engine, host),
        "Earthbind never acquired its static ability"
    );
}

#[test]
fn earthbind_independent_unsummon_in_response_leaves_no_host_to_damage() {
    let (mut engine, host) = board(index::WALL_OF_AIR, &[earthbind(), index::UNSUMMON]);
    let aura = enter_earthbind(&mut engine, host);
    assert_eq!(engine.state().zones.list(ZoneLocation::Stack).len(), 1);
    cast_at(&mut engine, index::UNSUMMON, host);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(in_hand(&engine, P1, index::WALL_OF_AIR), Some(host));
    assert_eq!(in_graveyard(&engine, P0, earthbind()), Some(aura));
    assert!(hits(&engine, aura).is_empty());
}

#[test]
fn earthbind_independent_disenchant_in_response_keeps_damage_but_cannot_ground_host() {
    let (mut engine, host) = board(index::WALL_OF_AIR, &[earthbind(), index::DISENCHANT]);
    let aura = enter_earthbind(&mut engine, host);
    cast_at(&mut engine, index::DISENCHANT, aura);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(in_graveyard(&engine, P0, earthbind()), Some(aura));
    assert_eq!(
        hits(&engine, aura),
        vec![(host, 2)],
        "departed source uses last attachment"
    );
    assert!(
        flies(&engine, host),
        "departed Aura cannot gain a battlefield ability"
    );
}

#[test]
fn earthbind_independent_disenchant_after_resolution_restores_only_its_host() {
    let (mut engine, host) = board(index::WALL_OF_AIR, &[earthbind(), index::DISENCHANT]);
    let other = on_battlefield(&engine, P1, index::AIR_ELEMENTAL).expect("bystander seated");
    let aura = enter_earthbind(&mut engine, host);
    pass_until(&mut engine, stack_is_empty);
    assert!(!flies(&engine, host));
    assert!(flies(&engine, other));
    cast_at(&mut engine, index::DISENCHANT, aura);
    pass_until(&mut engine, stack_is_empty);
    assert!(flies(&engine, host));
    assert!(flies(&engine, other));
    assert_eq!(hits(&engine, aura), vec![(host, 2)]);
}

#[test]
fn earthbind_independent_losing_flight_in_response_fails_the_intervening_condition() {
    let (mut engine, host) = board(
        index::WALL_OF_WOOD,
        &[flight(), earthbind(), index::DISENCHANT, flight()],
    );
    let first_flight = cast_at(&mut engine, flight(), host);
    pass_until(&mut engine, stack_is_empty);
    assert!(flies(&engine, host));
    let aura = enter_earthbind(&mut engine, host);
    assert_eq!(engine.state().zones.list(ZoneLocation::Stack).len(), 1);
    cast_at(&mut engine, index::DISENCHANT, first_flight);
    pass_until(&mut engine, stack_is_empty);
    assert!(!flies(&engine, host));
    assert!(
        hits(&engine, aura).is_empty(),
        "flying is checked again at resolution"
    );
    cast_at(&mut engine, flight(), host);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        flies(&engine, host),
        "failed trigger gained no lasting static ability"
    );
    assert!(hits(&engine, aura).is_empty());
}

#[test]
fn earthbind_independent_tidebinder_counters_the_trigger_without_grounding_the_host() {
    let (mut engine, host) = board(
        index::WALL_OF_AIR,
        &[earthbind(), index::TISHANA_S_TIDEBINDER],
    );
    let aura = enter_earthbind(&mut engine, host);
    let trigger = engine.state().zones.list(ZoneLocation::Stack)[0];
    pass_until(
        &mut engine,
        |engine| matches!(engine.pending(), Pending::Priority { player, .. } if *player == P0),
    );
    cast_with_floating(&mut engine, P0, index::TISHANA_S_TIDEBINDER);
    pass_until(&mut engine, |engine| {
        matches!(engine.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("Tidebinder must offer the waiting Earthbind ability")
    };
    assert!(options.contains(&trigger));
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![trigger],
            },
        )
        .expect("Tidebinder counters Earthbind's ability");
    pass_until(&mut engine, stack_is_empty);
    assert!(hits(&engine, aura).is_empty());
    assert!(flies(&engine, host));
    assert_eq!(
        engine
            .state()
            .object(aura)
            .and_then(|object| object.attached_to),
        Some(host)
    );
    reach_their_main_phase(&mut engine, P1);
    assert!(
        flies(&engine, host),
        "countered ability created no lasting static"
    );
}

#[test]
fn earthbind_independent_ephemerate_does_not_reattach_to_the_returning_creature() {
    let mut engine = Duel::new(1208, forest())
        .battlefield(0, &[mountain(), plains(), index::WALL_OF_AIR])
        .hand(0, &[earthbind(), index::EPHEMERATE])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let host = on_battlefield(&engine, P0, index::WALL_OF_AIR).expect("own host seated");
    let old_version = engine.state().object(host).expect("host").version;
    tap_all_mana(&mut engine, P0);
    let aura = enter_earthbind(&mut engine, host);
    cast_at(&mut engine, index::EPHEMERATE, host);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(on_battlefield(&engine, P0, index::WALL_OF_AIR), Some(host));
    assert_ne!(
        engine.state().object(host).expect("returned host").version,
        old_version
    );
    assert_eq!(in_graveyard(&engine, P0, earthbind()), Some(aura));
    assert!(
        hits(&engine, aura).is_empty(),
        "old trigger cannot damage a new incarnation"
    );
    assert!(
        flies(&engine, host),
        "old Aura never enchanted the returning creature"
    );
}
