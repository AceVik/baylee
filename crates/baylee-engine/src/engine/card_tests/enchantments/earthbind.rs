//! Earthbind: the intervening condition, ordered instructions, and an ability
//! gained by the Aura rather than a permanent change to its first creature.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
use crate::event::{Cause, DamageTarget, GameEvent};
use baylee_cards_dsl::{Duration, Filter, KeywordSet, Modifier};
use baylee_core::generated::index;
use baylee_core::ids::EffectId;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn setup(card: CardIndex) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut e = Duel::new(1200, forest())
        .battlefield(0, &[mountain(), island(), island()])
        .battlefield(1, &[card, index::AIR_ELEMENTAL])
        .hand(0, &[earthbind(), index::UNSUMMON])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let host = on_battlefield(&e, P1, card).unwrap();
    let aura = in_hand(&e, P0, earthbind()).unwrap();
    cast_from_hand(&mut e, P0, earthbind());
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![host],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        e.state().object(aura).unwrap().zone == Zone::Battlefield
    });
    (e, host, aura)
}

fn flying(e: &Engine<RegistryLookup>, object: ObjectId) -> bool {
    e.state()
        .object(object)
        .unwrap()
        .characteristics()
        .keywords
        .contains(KeywordSet::FLYING)
}

fn damage(e: &Engine<RegistryLookup>, aura: ObjectId) -> Vec<(ObjectId, u32)> {
    e.journal()
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

fn modify(e: &mut Engine<RegistryLookup>, object: ObjectId, modifier: Modifier) -> EffectId {
    let state = e.dev_state_mut(P0).unwrap();
    let filter = EffectFilter::object(state, object);
    let timestamp = state.next_timestamp();
    let id = state.effects.register(ContinuousEffect {
        id: EffectId::new(0),
        source: None,
        controller: P0,
        origin: EffectOrigin::Resolution,
        layer: modifier.layer(),
        timestamp,
        duration: Duration::Indefinitely,
        filter,
        modifier,
    });
    state.refresh_characteristics();
    id
}

fn settle(e: &mut Engine<RegistryLookup>) {
    e.sync_static_effects();
    e.dev_state_mut(P0).unwrap().refresh_characteristics();
}

fn move_to(e: &mut Engine<RegistryLookup>, object: ObjectId, to: ZoneLocation) {
    e.dev_state_mut(P0)
        .unwrap()
        .move_object(object, to, ZonePosition::Top, Cause::Effect)
        .unwrap();
}

#[test]
fn earthbind_triggers_without_targeting_then_deals_two_and_removes_only_flying() {
    let (mut e, host, aura) = setup(index::WALL_OF_AIR);
    assert!(flying(&e, host), "the trigger has not resolved");
    assert!(damage(&e, aura).is_empty());
    let stack = e.state().zones.list(ZoneLocation::Stack);
    assert_eq!(
        stack.len(),
        1,
        "the enter trigger is independently respondable"
    );
    assert!(e.state().object(stack[0]).unwrap().targets.is_empty());
    modify(&mut e, host, Modifier::AddKeyword(KeywordSet::HEXPROOF));
    pass_until(&mut e, stack_is_empty);
    assert_eq!(damage(&e, aura), vec![(host, 2)]);
    assert!(!flying(&e, host));
    let keywords = e.state().object(host).unwrap().characteristics().keywords;
    assert!(keywords.contains(KeywordSet::DEFENDER.union(KeywordSet::HEXPROOF)));
    assert!(flying(
        &e,
        on_battlefield(&e, P1, index::AIR_ELEMENTAL).unwrap()
    ));
}

#[test]
fn earthbind_on_a_nonflier_never_triggers_or_removes_later_flying() {
    let (mut e, host, aura) = setup(index::WALL_OF_WOOD);
    assert!(stack_is_empty(&e));
    modify(&mut e, host, Modifier::AddKeyword(KeywordSet::FLYING));
    settle(&mut e);
    assert!(flying(&e, host));
    assert!(damage(&e, aura).is_empty());
}

#[test]
fn earthbind_rechecks_flying_on_resolution_and_does_not_gain_the_ability_if_false() {
    let (mut e, host, aura) = setup(index::WALL_OF_AIR);
    let removal = modify(&mut e, host, Modifier::RemoveKeyword(KeywordSet::FLYING));
    pass_until(&mut e, stack_is_empty);
    assert!(damage(&e, aura).is_empty());
    e.dev_state_mut(P0)
        .unwrap()
        .effects
        .remove_where(|fx| fx.id == removal);
    settle(&mut e);
    assert!(flying(&e, host), "no ability was gained by the Aura");
}

#[test]
fn earthbind_allows_flying_to_disappear_and_return_before_resolution() {
    let (mut e, host, aura) = setup(index::WALL_OF_AIR);
    modify(&mut e, host, Modifier::RemoveKeyword(KeywordSet::FLYING));
    modify(&mut e, host, Modifier::AddKeyword(KeywordSet::FLYING));
    pass_until(&mut e, stack_is_empty);
    assert_eq!(damage(&e, aura), vec![(host, 2)]);
    assert!(
        !flying(&e, host),
        "the gained static starts at resolution's timestamp"
    );
}

#[test]
fn earthbind_deals_damage_before_removing_flying_even_when_that_prevents_the_damage() {
    let (mut e, host, aura) = setup(index::WALL_OF_AIR);
    let state = e.dev_state_mut(P0).unwrap();
    let timestamp = state.next_timestamp();
    crate::replacement::put_counters(state, host, CounterKind::Charge, 2);
    state.effects.register(ContinuousEffect {
        id: EffectId::new(0),
        source: None,
        controller: P0,
        origin: EffectOrigin::Resolution,
        layer: Modifier::CountersPreventDamage(CounterKind::Charge).layer(),
        timestamp,
        duration: Duration::Indefinitely,
        filter: EffectFilter::Dsl(&Filter::HasKeyword(KeywordSet::FLYING)),
        modifier: Modifier::CountersPreventDamage(CounterKind::Charge),
    });
    pass_until(&mut e, stack_is_empty);
    assert!(
        damage(&e, aura).is_empty(),
        "the creature still had flying during damage"
    );
    assert!(
        !flying(&e, host),
        "prevention does not stop the next instruction"
    );
}

#[test]
fn earthbind_later_flying_wins_and_removing_that_effect_restores_the_loss() {
    let (mut e, host, _) = setup(index::WALL_OF_AIR);
    pass_until(&mut e, stack_is_empty);
    let grant = modify(&mut e, host, Modifier::AddKeyword(KeywordSet::FLYING));
    settle(&mut e);
    assert!(flying(&e, host));
    e.dev_state_mut(P0)
        .unwrap()
        .effects
        .remove_where(|fx| fx.id == grant);
    settle(&mut e);
    assert!(!flying(&e, host));
}

#[test]
fn earthbind_loss_ends_when_aura_leaves_and_does_not_follow_its_return() {
    let (mut e, host, aura) = setup(index::WALL_OF_AIR);
    pass_until(&mut e, stack_is_empty);
    move_to(&mut e, aura, ZoneLocation::Hand(P0));
    settle(&mut e);
    assert!(flying(&e, host));
    e.dev_state_mut(P0)
        .unwrap()
        .object_mut(aura)
        .unwrap()
        .attached_to = Some(host);
    move_to(&mut e, aura, ZoneLocation::Battlefield);
    settle(&mut e);
    assert!(
        flying(&e, host),
        "the new Aura has not resolved its new trigger"
    );
}

#[test]
fn earthbind_gained_ability_is_lost_when_aura_loses_abilities_and_returns_afterward() {
    let (mut e, host, aura) = setup(index::WALL_OF_AIR);
    pass_until(&mut e, stack_is_empty);
    let removal = modify(&mut e, aura, Modifier::LoseAllAbilities);
    settle(&mut e);
    assert!(flying(&e, host));
    e.dev_state_mut(P0)
        .unwrap()
        .effects
        .remove_where(|fx| fx.id == removal);
    settle(&mut e);
    assert!(!flying(&e, host));
}

#[test]
fn earthbind_trigger_survives_aura_removal_using_its_last_attachment() {
    let (mut e, host, aura) = setup(index::WALL_OF_AIR);
    move_to(&mut e, aura, ZoneLocation::Hand(P0));
    pass_until(&mut e, stack_is_empty);
    assert_eq!(damage(&e, aura), vec![(host, 2)]);
    assert!(flying(&e, host), "a departed Aura cannot gain the ability");
}

#[test]
fn earthbind_old_trigger_does_not_grant_an_ability_to_a_returned_aura() {
    let (mut e, host, aura) = setup(index::WALL_OF_AIR);
    move_to(&mut e, aura, ZoneLocation::Hand(P0));
    e.dev_state_mut(P0)
        .unwrap()
        .object_mut(aura)
        .unwrap()
        .attached_to = Some(host);
    move_to(&mut e, aura, ZoneLocation::Battlefield);
    // Remove the new incarnation's printed ETB before its event is scanned;
    // only the old stack ability is being exercised by this identity probe.
    e.dev_state_mut(P0)
        .unwrap()
        .object_mut(aura)
        .unwrap()
        .own_abilities =
        Some(crate::object::AbilityList::from_static(&[], None, None).into_bundle());
    pass_until(&mut e, stack_is_empty);
    assert_eq!(damage(&e, aura), vec![(host, 2)]);
    assert!(flying(&e, host));
}

#[test]
fn earthbind_old_trigger_cannot_damage_a_creature_that_left_and_returned() {
    let (mut e, host, aura) = setup(index::WALL_OF_AIR);
    move_to(&mut e, aura, ZoneLocation::Hand(P0));
    move_to(&mut e, host, ZoneLocation::Hand(P1));
    move_to(&mut e, host, ZoneLocation::Battlefield);
    pass_until(&mut e, stack_is_empty);
    assert!(damage(&e, aura).is_empty());
    assert!(flying(&e, host));
}

#[test]
fn earthbind_gained_ability_follows_reattachment_without_dealing_damage_again() {
    let (mut e, host, aura) = setup(index::WALL_OF_AIR);
    pass_until(&mut e, stack_is_empty);
    let other = on_battlefield(&e, P1, index::AIR_ELEMENTAL).unwrap();
    e.dev_state_mut(P0).unwrap().attach(aura, other);
    settle(&mut e);
    assert!(flying(&e, host));
    assert!(!flying(&e, other));
    assert_eq!(damage(&e, aura), vec![(host, 2)]);
}

#[test]
fn earthbind_reattachment_restamps_the_gained_static_but_same_host_does_not() {
    let (mut e, host, aura) = setup(index::WALL_OF_AIR);
    pass_until(&mut e, stack_is_empty);
    modify(&mut e, host, Modifier::AddKeyword(KeywordSet::FLYING));
    e.dev_state_mut(P0).unwrap().attach(aura, host);
    settle(&mut e);
    assert!(
        flying(&e, host),
        "attaching to its existing host is no change"
    );
    let other = on_battlefield(&e, P1, index::AIR_ELEMENTAL).unwrap();
    e.dev_state_mut(P0).unwrap().attach(aura, other);
    e.dev_state_mut(P0).unwrap().attach(aura, host);
    settle(&mut e);
    assert!(
        !flying(&e, host),
        "the new attachment is newer than the flying grant"
    );
    assert_eq!(damage(&e, aura), vec![(host, 2)]);
}

#[test]
fn earthbind_phasing_out_and_in_keeps_its_gained_ability_and_timestamp() {
    let (mut e, host, aura) = setup(index::WALL_OF_AIR);
    pass_until(&mut e, stack_is_empty);
    let timestamp = e.state().object(aura).unwrap().timestamp;
    e.dev_state_mut(P0).unwrap().phase_out(&[aura]);
    settle(&mut e);
    assert!(flying(&e, host));
    e.dev_state_mut(P0).unwrap().phase_in(aura);
    settle(&mut e);
    assert!(!flying(&e, host));
    assert_eq!(e.state().object(aura).unwrap().timestamp, timestamp);
    assert_eq!(damage(&e, aura), vec![(host, 2)]);
}

#[test]
fn earthbind_phased_out_before_trigger_resolves_cannot_gain_an_ability() {
    let (mut e, host, aura) = setup(index::WALL_OF_AIR);
    e.dev_state_mut(P0).unwrap().phase_out(&[aura]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        damage(&e, aura),
        vec![(host, 2)],
        "the source uses its last known attachment"
    );
    e.dev_state_mut(P0).unwrap().phase_in(aura);
    settle(&mut e);
    assert!(flying(&e, host));
}

#[test]
fn earthbind_can_gain_its_static_after_an_older_effect_removed_its_abilities() {
    let (mut e, host, aura) = setup(index::WALL_OF_AIR);
    modify(&mut e, aura, Modifier::LoseAllAbilities);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(damage(&e, aura), vec![(host, 2)]);
    assert!(
        !flying(&e, host),
        "the newer grant survives the older ability removal"
    );
}

#[test]
fn earthbind_lethal_damage_puts_the_creature_and_aura_in_the_graveyard() {
    let (mut e, host, aura) = setup(index::BIRDS_OF_PARADISE);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(damage(&e, aura), vec![(host, 2)]);
    assert_eq!(e.state().object(host).unwrap().zone, Zone::Graveyard);
    assert_eq!(e.state().object(aura).unwrap().zone, Zone::Graveyard);
}

#[test]
fn earthbind_rechecks_its_current_attachment_when_moved_before_resolution() {
    let (mut e, host, aura) = setup(index::WALL_OF_AIR);
    let other = on_battlefield(&e, P1, index::AIR_ELEMENTAL).unwrap();
    e.dev_state_mut(P0).unwrap().attach(aura, other);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(damage(&e, aura), vec![(other, 2)]);
    assert!(flying(&e, host));
    assert!(!flying(&e, other));
}

#[test]
fn earthbind_rechecks_the_new_hosts_flying_instead_of_the_original_hosts() {
    let (mut e, host, aura) = setup(index::WALL_OF_AIR);
    let other = on_battlefield(&e, P1, index::AIR_ELEMENTAL).unwrap();
    modify(&mut e, other, Modifier::RemoveKeyword(KeywordSet::FLYING));
    e.dev_state_mut(P0).unwrap().attach(aura, other);
    pass_until(&mut e, stack_is_empty);
    assert!(damage(&e, aura).is_empty());
    e.dev_state_mut(P0).unwrap().attach(aura, host);
    settle(&mut e);
    assert!(
        flying(&e, host),
        "the false condition granted no static ability"
    );
}

#[test]
fn earthbind_a_copy_takes_printed_rules_but_not_the_originals_gained_static() {
    let (mut e, host, aura) = setup(index::WALL_OF_AIR);
    pass_until(&mut e, stack_is_empty);
    let copy = on_battlefield(&e, P1, index::AIR_ELEMENTAL).unwrap();
    modify(&mut e, copy, Modifier::BecomeCopyOf(aura));
    e.dev_state_mut(P0).unwrap().attach(copy, host);
    let Pending::Priority { player, .. } = e.pending() else {
        panic!("priority")
    };
    let player = *player;
    e.apply(player, PlayerAction::PassPriority).unwrap();
    assert_eq!(
        e.state().object(copy).unwrap().characteristics().types,
        TypeSet::ENCHANTMENT
    );
    assert!(
        !flying(&e, host),
        "the original Aura still supplies its static"
    );
    move_to(&mut e, aura, ZoneLocation::Hand(P0));
    settle(&mut e);
    assert!(
        flying(&e, host),
        "copy effects exclude abilities gained by other effects"
    );
    assert_eq!(damage(&e, aura), vec![(host, 2)]);
    assert!(
        damage(&e, copy).is_empty(),
        "becoming a copy is not entering"
    );
}

#[test]
fn earthbind_host_blink_before_trigger_resolution_does_not_enchant_the_returning_creature() {
    let (mut e, host, aura) = setup(index::WALL_OF_AIR);
    move_to(&mut e, host, ZoneLocation::Exile(P1));
    move_to(&mut e, host, ZoneLocation::Battlefield);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(aura).unwrap().zone, Zone::Graveyard);
    assert!(damage(&e, aura).is_empty());
    assert!(flying(&e, host));
}

#[test]
fn earthbind_phase_out_uses_the_most_recent_attachment_of_the_same_incarnation() {
    let (mut e, host, aura) = setup(index::WALL_OF_AIR);
    let other = on_battlefield(&e, P1, index::AIR_ELEMENTAL).unwrap();
    e.dev_state_mut(P0).unwrap().phase_out(&[aura]);
    e.dev_state_mut(P0).unwrap().phase_in(aura);
    e.dev_state_mut(P0).unwrap().attach(aura, other);
    e.dev_state_mut(P0).unwrap().phase_out(&[aura]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(damage(&e, aura), vec![(other, 2)]);
    assert!(flying(&e, host));
    assert!(flying(&e, other));
}

#[test]
fn earthbind_phased_out_aura_cannot_reattach_to_a_blinked_host_on_phasing_in() {
    let (mut e, host, aura) = setup(index::WALL_OF_AIR);
    e.dev_state_mut(P0).unwrap().phase_out(&[aura]);
    move_to(&mut e, host, ZoneLocation::Exile(P1));
    move_to(&mut e, host, ZoneLocation::Battlefield);
    e.dev_state_mut(P0).unwrap().phase_in(aura);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(aura).unwrap().zone, Zone::Graveyard);
    assert!(damage(&e, aura).is_empty());
    assert!(flying(&e, host));
}
