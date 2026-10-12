//! Real Wall block triggers and exact next-controller-turn restrictions.
//! Dynamic waiting-controller binding is inferred from the official Breaching
//! Leviathan ruling; no Wall-specific controller-change ruling is claimed.
use super::counter_resources::prepared;
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::event::Cause;
use crate::zone::ZonePosition;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn priority(e: &mut Engine<RegistryLookup>, p: PlayerId) {
    pass_until(
        e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p),
    );
}
fn cast(e: &mut Engine<RegistryLookup>, card: CardIndex, target: Option<ObjectId>) {
    priority(e, P0);
    for color in ManaColor::ALL {
        e.state.players[0].mana_pool.add(color, 30);
    }
    e.refresh_offer();
    cast_with_floating(e, P0, card);
    if let Some(target) = target {
        let player = e.pending().asked().unwrap();
        e.apply(
            player,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .unwrap();
    }
    pass_until(e, stack_is_empty);
}
fn next_main(e: &mut Engine<RegistryLookup>, player: PlayerId) {
    let turn = e.state.turn.number;
    pass_until(e, |e| {
        e.state.turn.number > turn
            && e.state.turn.active == player
            && e.state.turn.phase == crate::turn::Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player: p, .. } if *p == player)
    });
}
fn block(e: &mut Engine<RegistryLookup>) -> (ObjectId, ObjectId) {
    let wall = on_battlefield(e, P0, index::WALL_OF_DUST).unwrap();
    let bear = on_battlefield(e, P1, index::GRIZZLY_BEARS).unwrap();
    next_main(e, P1);
    pass_until(e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { player: P1, .. })
    });
    e.apply(
        P1,
        PlayerAction::DeclareAttackers {
            attackers: vec![(bear, Defender::Player(P0))],
        },
    )
    .unwrap();
    pass_until(e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { player: P0, .. })
    });
    e.apply(
        P0,
        PlayerAction::DeclareBlockers {
            blockers: vec![(wall, bear)],
        },
    )
    .unwrap();
    pass_until(e, |e| {
        !stack_is_empty(e) && matches!(e.pending(), Pending::Priority { .. })
    });
    (wall, bear)
}
fn attack_offer(e: &mut Engine<RegistryLookup>, player: PlayerId, bear: ObjectId, expected: bool) {
    pass_until(
        e,
        |e| matches!(e.pending(), Pending::ChooseAttackers { player: p, .. } if *p == player),
    );
    let Pending::ChooseAttackers { attackers, .. } = e.pending() else {
        unreachable!()
    };
    assert_eq!(attackers.contains(&bear), expected);
    let defender = e
        .state
        .players
        .iter()
        .find(|p| p.id != player && !p.has_lost())
        .unwrap()
        .id;
    if !expected {
        let before = e.snapshot_hash();
        assert!(
            e.apply(
                player,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(bear, Defender::Player(defender))]
                }
            )
            .is_err()
        );
        assert_eq!(e.snapshot_hash(), before);
    }
    let attackers = if expected {
        vec![(bear, Defender::Player(defender))]
    } else {
        vec![]
    };
    e.apply(player, PlayerAction::DeclareAttackers { attackers })
        .unwrap();
}
fn blink(e: &mut Engine<RegistryLookup>, bear: ObjectId) {
    // Transparent incarnation fixture; trigger and turn actions remain real.
    e.state
        .move_object(
            bear,
            ZoneLocation::Exile(P1),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.state
        .move_object(
            bear,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
}

#[test]
fn wall_next_controller_turn_restriction_survives_source_departure_then_expires() {
    let mut e = prepared(
        &[index::UNSUMMON],
        &[index::WALL_OF_DUST],
        &[index::GRIZZLY_BEARS],
    );
    let (wall, bear) = block(&mut e);
    pass_until(&mut e, stack_is_empty);
    cast(&mut e, index::UNSUMMON, Some(wall));
    assert_eq!(e.state.object(wall).unwrap().zone, Zone::Hand);
    next_main(&mut e, P1);
    attack_offer(&mut e, P1, bear, false);
    next_main(&mut e, P1);
    attack_offer(&mut e, P1, bear, true);
}

#[test]
fn wall_waiting_restriction_follows_new_controller_into_earlier_extra_turn() {
    let mut e = prepared(
        &[index::CONTROL_MAGIC, index::TIME_WALK],
        &[index::WALL_OF_DUST],
        &[index::GRIZZLY_BEARS],
    );
    let (_, bear) = block(&mut e);
    pass_until(&mut e, stack_is_empty);
    next_main(&mut e, P0);
    cast(&mut e, index::CONTROL_MAGIC, Some(bear));
    cast(&mut e, index::TIME_WALK, None);
    next_main(&mut e, P0);
    attack_offer(&mut e, P0, bear, false);
    next_main(&mut e, P0);
    attack_offer(&mut e, P0, bear, true);
}

#[test]
fn wall_old_trigger_cannot_restrict_blinked_attacker_incarnation() {
    let mut e = prepared(&[], &[index::WALL_OF_DUST], &[index::GRIZZLY_BEARS]);
    let (_, bear) = block(&mut e);
    let old = e.state.object(bear).unwrap().version;
    blink(&mut e, bear);
    assert_ne!(e.state.object(bear).unwrap().version, old);
    pass_until(&mut e, stack_is_empty);
    next_main(&mut e, P1);
    attack_offer(&mut e, P1, bear, true);
}

#[test]
fn wall_skipped_affected_controller_turn_does_not_consume_waiting_restriction() {
    let mut e = prepared(
        &[],
        &[index::WALL_OF_DUST],
        &[index::GRIZZLY_BEARS, index::TIME_VAULT],
    );
    let vault = on_battlefield(&e, P1, index::TIME_VAULT).unwrap();
    // Avoid an unrelated skip offer before the initial real block.
    e.state.set_tapped(vault, false);
    e.refresh_offer();
    let (_, bear) = block(&mut e);
    pass_until(&mut e, stack_is_empty);
    // Set only the Vault's tapped fixture state; the turn replacement is
    // offered and accepted through the real optional skip action.
    e.state.set_tapped(vault, true);
    e.refresh_offer();
    next_main(&mut e, P0);
    pass_until(&mut e, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::SkipTurn { .. },
                ..
            }
        )
    });
    assert_eq!(e.pending().asked(), Some(P1));
    e.apply(P1, PlayerAction::YesNo(true)).unwrap();
    assert!(!is_tapped(&e, vault));
    // P1's skipped occurrence never activates/expires the restriction.
    next_main(&mut e, P1);
    attack_offer(&mut e, P1, bear, false);
    next_main(&mut e, P1);
    attack_offer(&mut e, P1, bear, true);
}

#[test]
fn wall_does_not_restrict_a_second_combat_in_the_current_turn() {
    let mut e = prepared(&[], &[index::WALL_OF_DUST], &[index::GRIZZLY_BEARS]);
    let (_, bear) = block(&mut e);
    pass_until(&mut e, stack_is_empty);
    pass_until(&mut e, |e| {
        e.state.turn.phase == crate::turn::Phase::SecondMain
            && matches!(e.pending(), Pending::Priority { player: P1, .. })
    });
    let turn = e.state.turn.number;
    // Transparent extra-combat schedule, matching existing batch49_turtle
    // fixture: normal CombatEnd already cleared combat state. No Implemented
    // extra-combat helper exists. Actual attack declarations are still used.
    e.state.set_tapped(bear, false);
    e.state.turn.phase = crate::turn::Phase::Combat;
    e.state.turn.step = crate::turn::Step::CombatBegin;
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
    attack_offer(&mut e, P1, bear, true);
    assert_eq!(e.state.turn.number, turn);
    next_main(&mut e, P1);
    attack_offer(&mut e, P1, bear, false);
}

#[test]
fn wall_active_restriction_stays_for_whole_bound_turn_after_control_change() {
    let mut e = prepared(&[], &[index::WALL_OF_DUST], &[index::GRIZZLY_BEARS]);
    let (_, bear) = block(&mut e);
    pass_until(&mut e, stack_is_empty);
    next_main(&mut e, P1);
    // Isolate mid-bound-turn control change and summoning-sickness exemption.
    // can_attack deliberately tests attack permission, not whose turn it is.
    let object = e.state.object_mut(bear).unwrap();
    object.set_controller(P0);
    let keywords = object
        .base
        .keywords
        .union(baylee_cards_dsl::KeywordSet::HASTE);
    object.base_mut().keywords = keywords;
    e.state.set_tapped(bear, false);
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
    assert!(!crate::combat::can_attack(&e.state, P0, bear));
    next_main(&mut e, P0);
    assert!(crate::combat::can_attack(&e.state, P0, bear));
}

mod boundary;

mod blocks_creature;
