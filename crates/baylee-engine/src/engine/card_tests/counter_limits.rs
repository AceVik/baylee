//! Counter limits and beginning-of-turn history, exercised with Rasputin.
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
use baylee_cards_dsl::{Duration, Modifier, counters};
use baylee_core::generated::index;
use baylee_core::ids::EffectId;

fn setup(doubled: bool) -> (Engine<RegistryLookup>, ObjectId) {
    let mut cards = vec![index::RASPUTIN_DREAMWEAVER];
    if doubled {
        cards.push(index::DOUBLING_SEASON);
    }
    let mut e = Duel::new(SEED, forest()).battlefield(0, &cards).start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, PlayerId::new(0));
    let source = on_battlefield(&e, PlayerId::new(0), index::RASPUTIN_DREAMWEAVER).unwrap();
    (e, source)
}

#[test]
fn rasputin_caps_doubled_entry_and_placement_but_counter_costs_are_not_doubled() {
    for doubled in [false, true] {
        let p0 = PlayerId::new(0);
        let (mut e, source) = setup(doubled);
        assert_eq!(counters_on(&e, source, counters::DREAM), 7);
        for remaining in (0..7).rev() {
            activate(&mut e, p0, index::RASPUTIN_DREAMWEAVER, 0);
            assert_eq!(counters_on(&e, source, counters::DREAM), remaining);
        }
        assert_eq!(
            e.state().players[0]
                .mana_pool
                .available(ManaColor::Colorless),
            7
        );
        let Pending::Priority { legal, .. } = e.pending() else {
            panic!("priority")
        };
        assert!(!legal.abilities.contains(&(source, 0)));
        let state = e.dev_state_mut(p0).unwrap();
        crate::replacement::put_counters(state, source, counters::DREAM, 10);
        assert_eq!(
            state.object(source).unwrap().counters.get(counters::DREAM),
            7
        );
    }
}

#[test]
fn rasputin_checks_before_untapping_and_history_is_hidden_from_the_public_status() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut e, source) = setup(false);
    let state = e.dev_state_mut(p0).unwrap();
    crate::replacement::remove_counters(state, source, counters::DREAM, 2);
    state.set_tapped(source, true);
    reach_their_main_phase(&mut e, p1);
    assert!(walk_to_own_main(&mut e, p0));
    assert!(!is_tapped(&e, source), "the untap step still happens");
    assert_eq!(
        counters_on(&e, source, counters::DREAM),
        5,
        "it started tapped"
    );
    reach_their_main_phase(&mut e, p1);
    assert!(walk_to_own_main(&mut e, p0));
    assert_eq!(counters_on(&e, source, counters::DREAM), 6);
    let status = e.state().object(source).unwrap().status;
    assert!(status.contains(Status::UNTAPPED_AT_TURN_START));
    assert!(!status.public().contains(Status::UNTAPPED_AT_TURN_START));
    let state = e.dev_state_mut(p0).unwrap();
    state
        .move_object(
            source,
            ZoneLocation::Hand(p0),
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Setup,
        )
        .unwrap();
    state
        .move_object(
            source,
            ZoneLocation::Battlefield,
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Setup,
        )
        .unwrap();
    assert!(
        !state
            .object(source)
            .unwrap()
            .status
            .contains(Status::UNTAPPED_AT_TURN_START)
    );
}

#[test]
fn gaining_rasputins_counter_limit_removes_excess_and_overlapping_caps_do_not_add() {
    let p0 = PlayerId::new(0);
    let (mut e, source) = setup(false);
    let state = e.dev_state_mut(p0).unwrap();
    let lost = super::keyword_counters::remove_abilities(state, source);
    state.refresh_characteristics();
    let Pending::Priority { player, .. } = e.pending().clone() else {
        panic!("priority")
    };
    e.apply(player, PlayerAction::PassPriority).unwrap();
    let state = e.dev_state_mut(p0).unwrap();
    crate::replacement::put_counters(state, source, counters::DREAM, 18);
    assert_eq!(
        state.object(source).unwrap().counters.get(counters::DREAM),
        25
    );
    state.effects.remove_where(|effect| effect.id == lost);
    let timestamp = state.next_timestamp();
    state.effects.register(ContinuousEffect {
        id: EffectId::new(0),
        source: None,
        controller: p0,
        origin: EffectOrigin::Resolution,
        layer: baylee_cards_dsl::Layer::Text,
        timestamp,
        duration: Duration::Indefinitely,
        filter: EffectFilter::object(state, source),
        modifier: Modifier::CounterMaximum {
            kind: counters::DREAM,
            maximum: 3,
        },
    });
    state.refresh_characteristics();
    let Pending::Priority { player, .. } = e.pending().clone() else {
        panic!("priority")
    };
    e.apply(player, PlayerAction::PassPriority).unwrap();
    assert_eq!(counters_on(&e, source, counters::DREAM), 3);
}

/// `amount` noncombat damage from `source` to `victim`, dealt at once through
/// the damage pipeline (prevention included) and without a state-based-action
/// check, so a one-toughness Rasputin is still there to be read afterwards.
/// Returns what was actually dealt.
fn hit(e: &mut Engine<RegistryLookup>, source: ObjectId, victim: ObjectId, amount: u32) -> u32 {
    let state = e.dev_state_mut(PlayerId::new(0)).unwrap();
    let source_version = Some(state.object(source).unwrap().version);
    let mut work = crate::damage::DamageWork::new(
        state,
        vec![crate::damage::Assignment {
            source,
            source_version,
            recipient: crate::event::DamageTarget::Object(victim),
            amount,
            is_combat: false,
        }],
    );
    assert!(work.advance(state).is_none(), "no question is asked");
    work.dealt()
}

fn rasputin_against_a_bear() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[index::RASPUTIN_DREAMWEAVER])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, PlayerId::new(0));
    let rasputin = on_battlefield(&e, PlayerId::new(0), index::RASPUTIN_DREAMWEAVER).unwrap();
    let bear = on_battlefield(&e, PlayerId::new(1), index::GRIZZLY_BEARS).unwrap();
    (e, rasputin, bear)
}

#[test]
fn rasputin_dream_shield_costs_a_counter_and_prevents_only_the_next_one_damage_to_itself() {
    let p0 = PlayerId::new(0);
    let (mut e, rasputin, bear) = rasputin_against_a_bear();
    assert_eq!(counters_on(&e, rasputin, counters::DREAM), 7);
    activate(&mut e, p0, index::RASPUTIN_DREAMWEAVER, 1);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        counters_on(&e, rasputin, counters::DREAM),
        6,
        "the cost removes one dream counter from Rasputin"
    );
    assert_eq!(
        hit(&mut e, rasputin, bear, 3),
        3,
        "the shield names Rasputin: damage to another creature is not prevented"
    );
    assert_eq!(
        hit(&mut e, bear, rasputin, 3),
        2,
        "prevent the next 1 damage: three damage to Rasputin becomes two"
    );
    assert_eq!(u32::from(e.state().object(rasputin).unwrap().damage), 2);
    assert_eq!(
        hit(&mut e, bear, rasputin, 3),
        3,
        "the shield was only 1 damage: a second hit is not reduced"
    );
    assert_eq!(u32::from(e.state().object(rasputin).unwrap().damage), 5);
    assert_eq!(
        counters_on(&e, rasputin, counters::DREAM),
        6,
        "the shield is paid once, not per hit"
    );
}

#[test]
fn rasputin_unused_dream_shield_ends_with_the_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut e, rasputin, bear) = rasputin_against_a_bear();
    activate(&mut e, p0, index::RASPUTIN_DREAMWEAVER, 1);
    pass_until(&mut e, stack_is_empty);
    reach_their_main_phase(&mut e, p1);
    assert_eq!(
        hit(&mut e, bear, rasputin, 3),
        3,
        "this turn: the unused shield has ended with the turn it was made in"
    );
}
