//! Rules-level source identities and retention, independent of card scripts.
use super::*;
use crate::event::Cause;
use crate::object::{AbilityLoc, PaidRecord, Status};
use crate::prevention::{ChosenSource, Shield};
use crate::state::{CardLookup, DelayedTrigger};
use crate::zone::ZonePosition;
use baylee_core::ids::CardIndex;
use baylee_core::preset::{FormatId, GamePreset, HouseRules, SeatController, SeatSpec};

const P: PlayerId = PlayerId::new(0);
struct NoCards;
impl CardLookup for NoCards {
    fn card(&self, _: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
        None
    }
}
fn game() -> GameState {
    GameState::from_preset(
        &GamePreset {
            format: FormatId::Freeform,
            seed: 15,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![],
            seats: (0..2)
                .map(|_| SeatSpec {
                    controller: SeatController::Open,
                    capabilities: baylee_core::preset::SeatCapabilities::default(),
                    deck: vec![],
                    sideboard: vec![],
                    commanders: vec![],
                    starting_life: Some(20),
                    starting_hand: None,
                    starting_battlefield: vec![],
                    emblems: vec![],
                    team: None,
                })
                .collect(),
        },
        &NoCards,
    )
    .unwrap()
}
fn object(state: &mut GameState, zone: ZoneLocation, kind: ObjectKind) -> ObjectId {
    let name = state.names.intern("source fixture");
    state.create_bare(P, kind, name, zone)
}
fn move_to(state: &mut GameState, id: ObjectId, zone: ZoneLocation) {
    state
        .move_object(id, zone, ZonePosition::Top, Cause::Effect)
        .unwrap();
}
fn refers(state: &mut GameState, source: DamageSourceRef) -> ObjectId {
    let id = object(state, ZoneLocation::Stack, ObjectKind::AbilityOnStack);
    let obj = state.object_mut(id).unwrap();
    obj.ability = Some(AbilityLoc {
        card: None,
        source: source.object,
        index: 0,
    });
    obj.riders.push(Rider::AbilitySourceVersion(source.version));
    id
}
fn offered(state: &mut GameState) -> Vec<DamageSourceRef> {
    options(state, &Filter::Any, P, ObjectId::NO_SOURCE)
}

#[test]
fn delayed_reference_retains_a_departed_source_across_cleanup_and_expires_with_the_trigger() {
    let mut state = game();
    let source = object(&mut state, ZoneLocation::Battlefield, ObjectKind::Permanent);
    let was = state.source_identity(source).unwrap();
    state.delayed.push(DelayedTrigger {
        controller: P,
        when: DelayedWhen::NextUpkeep,
        action: DelayedAction::Trigger {
            source,
            source_version: was.version,
            effects: &[],
        },
    });
    move_to(&mut state, source, ZoneLocation::Graveyard(P));
    move_to(&mut state, source, ZoneLocation::Hand(P));
    state.prune_damage_sources();
    assert!(offered(&mut state).contains(&was));
    assert_eq!(state.source_object(was).unwrap().zone, Zone::Battlefield);
    state.delayed.clear();
    state.prune_damage_sources();
    assert!(!offered(&mut state).contains(&was));
    assert!(state.source_object(was).is_none());
}

#[test]
fn ceased_source_and_stack_target_references_are_exact_and_copyable() {
    let mut state = game();
    let token = object(&mut state, ZoneLocation::Battlefield, ObjectKind::Permanent);
    let was = state.source_identity(token).unwrap();
    let ability = refers(&mut state, was);
    let target = object(&mut state, ZoneLocation::Battlefield, ObjectKind::Permanent);
    let old_target = state.source_identity(target).unwrap();
    state.object_mut(ability).unwrap().targets.push(target);
    move_to(&mut state, token, ZoneLocation::Graveyard(P));
    state.remember_damage_source(token);
    state.zones.remove(token, ZoneLocation::Graveyard(P));
    state.arena.remove(token);
    move_to(&mut state, target, ZoneLocation::Hand(P));
    move_to(&mut state, target, ZoneLocation::Battlefield);
    let copy = refers(&mut state, was);
    state.object_mut(copy).unwrap().targets.push(target);
    state.copy_source_references(state.source_identity(ability).unwrap(), copy);
    state.zones.remove(ability, ZoneLocation::Stack);
    state.arena.remove(ability);
    state.prune_damage_sources();
    assert!(offered(&mut state).contains(&was));
    assert!(offered(&mut state).contains(&old_target));
    assert_eq!(state.source_referenced_by(old_target), vec![copy]);
    state.replace_target_reference(copy, false, 0, state.source_identity(target).unwrap());
    state.prune_damage_sources();
    assert!(!offered(&mut state).contains(&old_target));
    assert!(offered(&mut state).contains(&state.source_identity(target).unwrap()));
}

#[test]
fn loop_signature_distinguishes_the_departed_spell_decisions_a_live_trigger_can_copy() {
    let mut state = game();
    let spell = object(&mut state, ZoneLocation::Stack, ObjectKind::Spell);
    let reference = state.source_identity(spell).unwrap();
    let target = object(&mut state, ZoneLocation::Battlefield, ObjectKind::Permanent);
    state.object_mut(spell).unwrap().targets.push(target);
    state.object_mut(spell).unwrap().x_value = 3;
    state.divided.push((spell, vec![(target, 3)]));
    refers(&mut state, reference);
    move_to(&mut state, spell, ZoneLocation::Graveyard(P));
    state.prune_damage_sources();
    let baseline = state.loop_signature();
    let mut changed_division = state.clone();
    changed_division
        .source_memory
        .divisions
        .get_mut(&reference)
        .unwrap()[0]
        .1 = 2;
    assert_ne!(baseline, changed_division.loop_signature());
    let mut changed_x = state.clone();
    changed_x
        .damage_sources
        .iter_mut()
        .find(|o| identity(o) == reference)
        .unwrap()
        .x_value = 2;
    assert_ne!(baseline, changed_x.loop_signature());
}

#[test]
fn only_an_actual_resolving_permanent_spell_extends_the_selected_source() {
    for resolves in [false, true] {
        let mut state = game();
        let spell = object(&mut state, ZoneLocation::Stack, ObjectKind::Spell);
        let chosen = ChosenSource::new(
            &state,
            state.source_identity(spell).unwrap(),
            &Filter::Any,
            P,
            spell,
        )
        .unwrap();
        if resolves {
            state.begin_permanent_resolution(spell);
        }
        state.object_mut(spell).unwrap().kind = ObjectKind::Permanent;
        move_to(&mut state, spell, ZoneLocation::Battlefield);
        assert_eq!(
            chosen.deals_as(&state, state.object(spell).unwrap()),
            resolves
        );
        move_to(&mut state, spell, ZoneLocation::Hand(P));
        move_to(&mut state, spell, ZoneLocation::Battlefield);
        assert!(!chosen.deals_as(&state, state.object(spell).unwrap()));
    }
}

#[test]
fn shield_references_and_face_up_command_objects_are_sources_but_unrelated_graveyards_are_not() {
    let mut state = game();
    let source = object(&mut state, ZoneLocation::Battlefield, ObjectKind::Permanent);
    let was = state.source_identity(source).unwrap();
    let selected = ChosenSource::new(&state, was, &Filter::Any, P, source).unwrap();
    state.shields.push(Shield {
        protects: Shielded::Player(P),
        controller: P,
        kind: ShieldKind::NextFrom {
            source: selected,
            all_but: 0,
            gain_life: false,
            combat_only: false,
        },
    });
    move_to(&mut state, source, ZoneLocation::Graveyard(P));
    let other = object(&mut state, ZoneLocation::Graveyard(P), ObjectKind::Card);
    let command = object(&mut state, ZoneLocation::Command(P), ObjectKind::Card);
    let command_ref = state.source_identity(command).unwrap();
    let choices = offered(&mut state);
    assert!(choices.contains(&was));
    assert!(choices.contains(&command_ref));
    assert!(!choices.iter().any(|r| r.object == other));
    state
        .object_mut(command)
        .unwrap()
        .status
        .insert(Status::FACE_DOWN);
    assert!(!offered(&mut state).contains(&command_ref));
    state.shields.clear();
    state.prune_damage_sources();
    assert!(!offered(&mut state).contains(&was));
}

#[test]
fn paid_sacrifice_and_linked_return_refer_to_exact_objects_not_all_cards_linked_to_a_source() {
    let mut state = game();
    let source = object(&mut state, ZoneLocation::Battlefield, ObjectKind::Permanent);
    let was = state.source_identity(source).unwrap();
    let ability = refers(&mut state, was);
    let victim = object(&mut state, ZoneLocation::Battlefield, ObjectKind::Permanent);
    let sacrificed = state.source_identity(victim).unwrap();
    state.object_mut(ability).unwrap().paid = Some(Box::new(PaidRecord {
        sacrificed: Some((victim, sacrificed.version)),
        ..PaidRecord::default()
    }));
    move_to(&mut state, victim, ZoneLocation::Graveyard(P));
    let held = object(&mut state, ZoneLocation::Exile(P), ObjectKind::Card);
    state.object_mut(held).unwrap().riders.push(Rider::Linked {
        host: source,
        until: None,
    });
    let held_ref = state.source_identity(held).unwrap();
    state.capture_linked_references(
        ability,
        &[baylee_cards_dsl::Effect::GainLife {
            amount: baylee_cards_dsl::Amount::Fixed(1),
        }],
    );
    assert!(offered(&mut state).contains(&sacrificed));
    assert!(!offered(&mut state).contains(&held_ref));
    state.capture_linked_references(
        ability,
        &[baylee_cards_dsl::Effect::ReturnLinkedToBattlefield],
    );
    move_to(&mut state, held, ZoneLocation::Hand(P));
    assert!(offered(&mut state).contains(&held_ref));
    assert_eq!(state.source_referenced_by(held_ref), vec![ability]);
}

#[test]
fn exact_source_decisions_are_deterministic_and_snapshots_include_reference_history() {
    let mut state = game();
    let source = object(&mut state, ZoneLocation::Battlefield, ObjectKind::Permanent);
    let old = state.source_identity(source).unwrap();
    refers(&mut state, old);
    move_to(&mut state, source, ZoneLocation::Hand(P));
    move_to(&mut state, source, ZoneLocation::Battlefield);
    let mut replay = state.clone();
    assert_eq!(offered(&mut state), offered(&mut replay));
    assert_eq!(state.next_source_choice(), replay.next_source_choice());
    assert_eq!(state.snapshot_hash(), replay.snapshot_hash());
    assert_eq!(state.loop_signature(), replay.loop_signature());
    replay.next_source_choice();
    assert_eq!(
        state.loop_signature(),
        replay.loop_signature(),
        "choice counters do not break rules loops"
    );
    replay.source_memory.stack.clear();
    assert_ne!(state.loop_signature(), replay.loop_signature());
    assert_ne!(state.snapshot_hash(), replay.snapshot_hash());
}

#[test]
fn event_context_requires_an_actual_reader_and_copy_preserves_that_entitlement() {
    use baylee_cards_dsl::{Effect, PlayerRel, TargetSpec};
    for effects in [
        &[Effect::gain_life(1)][..],
        &[Effect::GainLife {
            amount: baylee_cards_dsl::Amount::Plus {
                base: &baylee_cards_dsl::Amount::Negated(
                    &baylee_cards_dsl::Amount::SaturatingSub {
                        base: &baylee_cards_dsl::Amount::EventLastToughness,
                        subtract: 1,
                    },
                ),
                offset: 1,
            },
        }][..],
        &[Effect::BecomeMonarch(PlayerRel::ControllerOfEvent)][..],
        &[Effect::EventObjectDealsDamageEqualToPower {
            target: TargetSpec::Player(PlayerRel::You),
        }][..],
        &[Effect::ReturnToBattlefieldTapped {
            target: TargetSpec::EventObject,
        }][..],
    ] {
        let mut state = game();
        let source = object(&mut state, ZoneLocation::Battlefield, ObjectKind::Permanent);
        let victim = object(&mut state, ZoneLocation::Graveyard(P), ObjectKind::Card);
        let event = state.source_identity(victim).unwrap();
        let source_ref = state.source_identity(source).unwrap();
        let original = refers(&mut state, source_ref);
        state.object_mut(original).unwrap().event_object = Some(victim);
        let effects = Box::leak(effects.to_vec().into_boxed_slice());
        state.capture_linked_references(original, effects);
        let reads = !matches!(
            effects[0],
            Effect::GainLife {
                amount: baylee_cards_dsl::Amount::Fixed(1)
            }
        );
        assert_eq!(offered(&mut state).contains(&event), reads);
        assert_eq!(state.source_referenced_by(event).contains(&original), reads);
        let copy = refers(&mut state, source_ref);
        state.object_mut(copy).unwrap().event_object = Some(victim);
        state.copy_source_references(state.source_identity(original).unwrap(), copy);
        move_to(&mut state, original, ZoneLocation::Graveyard(P));
        assert_eq!(offered(&mut state).contains(&event), reads);
        assert_eq!(
            state.source_referenced_by(event),
            if reads { vec![copy] } else { vec![] }
        );
    }
}
