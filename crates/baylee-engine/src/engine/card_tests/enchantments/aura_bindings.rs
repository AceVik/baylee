//! Real-card author regressions for graveyard enchant and untargeted Aura moves.
#[allow(clippy::wildcard_imports)] // Shared real-card test vocabulary.
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn priority(engine: &mut Engine<RegistryLookup>, player: PlayerId) {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::Priority { player: holder, .. } if *holder == player),
    );
}

fn announce(
    engine: &mut Engine<RegistryLookup>,
    player: PlayerId,
    card: CardIndex,
    target: ObjectId,
) -> ObjectId {
    priority(engine, player);
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        engine.dev_state_mut(P0).unwrap().players[usize::from(player.get())]
            .mana_pool
            .add(color, 4);
    }
    engine.refresh_offer();
    let object = in_hand(engine, player, card).unwrap();
    cast_with_floating(engine, player, card);
    let Pending::ChooseTargets { options, .. } = engine.pending() else {
        panic!("Aura/spell target choice");
    };
    assert!(options.contains(&target));
    engine
        .apply(
            player,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .unwrap();
    object
}

fn animate_setup(creature: CardIndex) -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = Duel::new(3034, forest())
        .battlefield(0, &[swamp(), swamp(), plains(), plains()])
        .hand(0, &[index::ANIMATE_DEAD, index::DISENCHANT, ephemerate()])
        .battlefield(1, &[forest(), index::WHITE_KNIGHT])
        .hand(1, &[creature, index::CONTROL_MAGIC])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    // Initial graveyard arrangement only; Animate Dead itself is always
    // cast and resolved through its real target and trigger decisions.
    let host = in_hand(&engine, P1, creature).unwrap();
    engine
        .dev_state_mut(P0)
        .unwrap()
        .move_object(
            host,
            ZoneLocation::Graveyard(P1),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    engine.refresh_offer();
    (engine, host)
}

fn animate_on_stack(engine: &mut Engine<RegistryLookup>, host: ObjectId) -> ObjectId {
    announce(engine, P0, index::ANIMATE_DEAD, host)
}

fn aura_entered(engine: &mut Engine<RegistryLookup>, aura: ObjectId) {
    pass_until(engine, |e| {
        e.state()
            .object(aura)
            .is_some_and(|o| o.zone == Zone::Battlefield)
            && !stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { .. })
    });
}

#[test]
fn animate_dead_enchants_graveyard_then_returns_under_your_control_and_reduces_power() {
    let (mut engine, host) = animate_setup(index::GRIZZLY_BEARS);
    let old_version = engine.state().object(host).unwrap().version;
    let aura = animate_on_stack(&mut engine, host);
    aura_entered(&mut engine, aura);
    assert_eq!(engine.state().object(host).unwrap().zone, Zone::Graveyard);
    assert_eq!(engine.state().object(aura).unwrap().attached_to, Some(host));
    pass_until(&mut engine, stack_is_empty);
    let returned = engine.state().object(host).unwrap();
    assert_eq!(returned.zone, Zone::Battlefield);
    assert_eq!(returned.controller, P0);
    assert_eq!(returned.owner, P1);
    assert_ne!(returned.version, old_version);
    assert_eq!(pt(&engine, host), (1, 2));
    assert_eq!(engine.state().object(aura).unwrap().attached_to, Some(host));
    assert!(!engine.journal().entries().iter().any(|entry| {
        matches!(entry.event, crate::event::GameEvent::ZoneChanged { object, to: Zone::Graveyard, .. } if object == aura)
    }));
}

#[test]
fn animate_dead_removed_before_enter_trigger_does_not_return_the_card() {
    let (mut engine, host) = animate_setup(index::GRIZZLY_BEARS);
    let aura = animate_on_stack(&mut engine, host);
    aura_entered(&mut engine, aura);
    announce(&mut engine, P0, index::DISENCHANT, aura);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(host).unwrap().zone, Zone::Graveyard);
    assert_eq!(engine.state().object(aura).unwrap().zone, Zone::Graveyard);
}

#[test]
fn animate_dead_returns_and_attaches_to_a_creature_with_shroud() {
    let (mut engine, host) = animate_setup(index::HUMBLE_BUDOKA);
    let aura = animate_on_stack(&mut engine, host);
    pass_until(&mut engine, stack_is_empty);
    assert!(keywords(&engine, host).contains(KeywordSet::SHROUD));
    assert_eq!(engine.state().object(aura).unwrap().attached_to, Some(host));
    assert_eq!(engine.state().object(host).unwrap().zone, Zone::Battlefield);
    assert_eq!(pt(&engine, host), (1, 2));
}

#[test]
fn animate_dead_waits_for_entry_copy_and_its_protection_before_reattaching() {
    let (mut engine, host) = animate_setup(index::CLONE);
    let knight = on_battlefield(&engine, P1, index::WHITE_KNIGHT).unwrap();
    let aura = animate_on_stack(&mut engine, host);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    assert_eq!(engine.state().object(host).unwrap().zone, Zone::Battlefield);
    assert_eq!(engine.state().object(aura).unwrap().zone, Zone::Battlefield);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending()
    else {
        unreachable!();
    };
    assert!(options.contains(&knight));
    let player = *player;
    engine
        .apply(
            player,
            PlayerAction::ChooseObjects {
                objects: vec![knight],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(aura)
            .is_some_and(|o| o.zone == Zone::Graveyard)
            && e.state()
                .object(host)
                .is_some_and(|o| o.zone == Zone::Battlefield)
            && !stack_is_empty(e)
    });
    assert_eq!(pt(&engine, host), (2, 2));
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(host).unwrap().zone, Zone::Graveyard);
}

#[test]
fn animate_dead_protection_prevents_attachment_then_delayed_sacrifice_uses_the_stack() {
    let (mut engine, host) = animate_setup(index::WHITE_KNIGHT);
    let aura = animate_on_stack(&mut engine, host);
    pass_until(&mut engine, |e| {
        e.state()
            .object(aura)
            .is_some_and(|o| o.zone == Zone::Graveyard)
            && e.state()
                .object(host)
                .is_some_and(|o| o.zone == Zone::Battlefield)
            && !stack_is_empty(e)
    });
    assert_eq!(pt(&engine, host), (2, 2));
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(host).unwrap().zone, Zone::Graveyard);
}

#[test]
fn animate_dead_delayed_sacrifice_does_not_reach_a_blinked_creature() {
    let (mut engine, host) = animate_setup(index::WHITE_KNIGHT);
    let aura = animate_on_stack(&mut engine, host);
    pass_until(&mut engine, |e| {
        e.state()
            .object(aura)
            .is_some_and(|o| o.zone == Zone::Graveyard)
            && e.state()
                .object(host)
                .is_some_and(|o| o.zone == Zone::Battlefield)
            && !stack_is_empty(e)
    });
    let returned_version = engine.state().object(host).unwrap().version;
    announce(&mut engine, P0, ephemerate(), host);
    pass_until(&mut engine, stack_is_empty);
    let new_host = engine.state().object(host).unwrap();
    assert_ne!(new_host.version, returned_version);
    assert_eq!(new_host.zone, Zone::Battlefield);
}

#[test]
fn animate_dead_leave_trigger_sacrifices_the_creature_after_control_changes() {
    let (mut engine, host) = animate_setup(index::GRIZZLY_BEARS);
    let aura = animate_on_stack(&mut engine, host);
    pass_until(&mut engine, stack_is_empty);
    reach_their_main_phase(&mut engine, P1);
    announce(&mut engine, P1, index::CONTROL_MAGIC, host);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(host).unwrap().controller, P1);
    announce(&mut engine, P0, index::DISENCHANT, aura);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(host).unwrap().zone, Zone::Graveyard);
    assert!(in_graveyard(&engine, P1, index::GRIZZLY_BEARS).is_some());
}

fn kudzu_setup(land: CardIndex) -> (Engine<RegistryLookup>, ObjectId, ObjectId, ObjectId) {
    kudzu_fixture(land, false)
}

fn kudzu_fixture(
    land: CardIndex,
    with_shroud: bool,
) -> (Engine<RegistryLookup>, ObjectId, ObjectId, ObjectId) {
    let mut opposing_board = vec![land, mountain()];
    if with_shroud {
        opposing_board.extend([index::TREE_OF_TALES, index::FOUNTAIN_WATCH]);
    }
    let mut engine = Duel::new(7013, forest())
        .battlefield(0, &[forest(), plains(), plains()])
        .hand(0, &[index::KUDZU, index::DISENCHANT])
        .battlefield(1, &opposing_board)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let host = on_battlefield(&engine, P1, land).unwrap();
    let next = on_battlefield(&engine, P0, forest()).unwrap();
    let aura = announce(&mut engine, P0, index::KUDZU, host);
    pass_until(&mut engine, stack_is_empty);
    priority(&mut engine, P1);
    let Pending::Priority { legal, .. } = engine.pending() else {
        panic!("mana priority");
    };
    let action = if legal.mana_abilities.contains(&host) {
        PlayerAction::ActivateManaAbility { source: host }
    } else {
        let &(_, ability_index) = legal
            .abilities
            .iter()
            .find(|&&(source, _)| source == host)
            .unwrap();
        PlayerAction::ActivateAbility {
            source: host,
            ability_index,
        }
    };
    engine.apply(P1, action).unwrap();
    assert_eq!(engine.state().object(host).unwrap().zone, Zone::Battlefield);
    assert!(!stack_is_empty(&engine), "the tap trigger uses the stack");
    (engine, aura, host, next)
}

fn move_question(engine: &mut Engine<RegistryLookup>) -> Vec<ObjectId> {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending()
    else {
        panic!("untargeted optional attachment choice");
    };
    assert_eq!((*player, *min, *max), (P1, 0, 1));
    options.clone()
}

#[test]
fn kudzu_destroys_then_asks_the_lands_controller_to_move_it_to_any_players_land() {
    let (mut engine, aura, host, next) = kudzu_setup(forest());
    assert!(move_question(&mut engine).contains(&next));
    assert_eq!(engine.state().object(host).unwrap().zone, Zone::Graveyard);
    assert_eq!(engine.state().object(aura).unwrap().zone, Zone::Battlefield);
    let before = engine.snapshot_hash();
    assert!(
        engine
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: vec![next]
                }
            )
            .is_err()
    );
    assert_eq!(engine.snapshot_hash(), before);
    engine
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![next],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(aura).unwrap().controller, P0);
    assert_eq!(engine.state().object(aura).unwrap().attached_to, Some(next));
}

#[test]
fn kudzu_decline_after_destruction_puts_the_unattached_aura_in_the_graveyard() {
    let (mut engine, aura, host, _) = kudzu_setup(forest());
    move_question(&mut engine);
    engine
        .apply(P1, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    for object in [host, aura] {
        assert_eq!(engine.state().object(object).unwrap().zone, Zone::Graveyard);
    }
}

#[test]
fn kudzu_trigger_still_destroys_its_land_after_the_aura_is_destroyed() {
    let (mut engine, aura, host, _) = kudzu_setup(forest());
    announce(&mut engine, P0, index::DISENCHANT, aura);
    pass_until(&mut engine, stack_is_empty);
    for object in [host, aura] {
        assert_eq!(engine.state().object(object).unwrap().zone, Zone::Graveyard);
    }
}

#[test]
fn kudzu_can_move_to_a_shrouded_land_without_targeting_it() {
    let (mut engine, aura, _, _) = kudzu_fixture(forest(), true);
    let next = on_battlefield(&engine, P1, index::TREE_OF_TALES).unwrap();
    assert!(keywords(&engine, next).contains(KeywordSet::SHROUD));
    assert!(move_question(&mut engine).contains(&next));
    engine
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![next],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(aura).unwrap().attached_to, Some(next));
}

#[test]
fn kudzu_offers_relocation_even_when_its_land_cannot_be_destroyed() {
    let (mut engine, aura, host, next) = kudzu_setup(index::DARKSTEEL_CITADEL);
    let options = move_question(&mut engine);
    assert!(options.contains(&host) && options.contains(&next));
    assert_eq!(engine.state().object(host).unwrap().zone, Zone::Battlefield);
    engine
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![next],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(aura).unwrap().attached_to, Some(next));
    assert_eq!(engine.state().object(host).unwrap().zone, Zone::Battlefield);
}

#[test]
fn kudzu_decline_keeps_its_existing_indestructible_land_attached() {
    let (mut engine, aura, host, _) = kudzu_setup(index::DARKSTEEL_CITADEL);
    move_question(&mut engine);
    engine
        .apply(P1, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(aura).unwrap().attached_to, Some(host));
    assert_eq!(engine.state().object(host).unwrap().zone, Zone::Battlefield);
}
