//! Stun replacement, exact tapped recipients, and an energy choice without a 255-object cap.
use super::counter_resources::prepared;
#[allow(clippy::wildcard_imports)]
use super::*;
use crate::event::GameEvent;
use baylee_core::generated::index;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn tapped(e: &Engine<RegistryLookup>, object: ObjectId) -> bool {
    e.state()
        .object(object)
        .unwrap()
        .status
        .contains(Status::TAPPED)
}

fn ray(e: &mut Engine<RegistryLookup>, target: PlayerId) {
    cast_with_floating(e, P0, index::SUPPRESSION_RAY);
    e.apply(P0, PlayerAction::ChoosePlayer(target)).unwrap();
}

#[test]
fn stun_ray_only_counts_actual_taps_and_allows_overpayment_without_targeting_creatures() {
    let mut e = prepared(
        &[index::SUPPRESSION_RAY],
        &[index::GRIZZLY_BEARS],
        &[index::GRIZZLY_BEARS, index::SERRA_ANGEL, index::FOREST],
    );
    let fresh = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    let already = on_battlefield(&e, P1, index::SERRA_ANGEL).unwrap();
    let mine = on_battlefield(&e, P0, index::GRIZZLY_BEARS).unwrap();
    let land = on_battlefield(&e, P1, index::FOREST).unwrap();
    let s = e.dev_state_mut(P0).unwrap();
    s.players[0].energy = 9;
    s.set_tapped(already, true);
    let o = s.object_mut(fresh).unwrap();
    let keywords = o.base.keywords.union(KeywordSet::HEXPROOF);
    o.base_mut().keywords = keywords;
    s.invalidate_projections();
    e.refresh_offer();
    ray(&mut e, P1);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    assert!(matches!(
        e.pending(),
        Pending::ChooseNumber {
            min: 0,
            max: 9,
            reason: crate::choice::NumberPrompt::EnergyPayment { candidates: 1 },
            ..
        }
    ));
    assert!(tapped(&e, fresh));
    assert!(!tapped(&e, mine));
    assert!(!tapped(&e, land));
    let hash = e.state().snapshot_hash();
    assert!(e.apply(P0, PlayerAction::ChooseNumber(10)).is_err());
    assert_eq!(e.state().snapshot_hash(), hash);
    e.apply(P0, PlayerAction::ChooseNumber(7)).unwrap();
    let Pending::ChooseCards {
        options, min, max, ..
    } = e.pending()
    else {
        panic!("counter recipients")
    };
    assert_eq!(options, &[fresh]);
    assert_eq!((*min, *max), (0, 1));
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: vec![fresh],
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].energy, 2);
    assert_eq!(counters_on(&e, fresh, CounterKind::Stun), 1);
    assert_eq!(counters_on(&e, already, CounterKind::Stun), 0);
    assert_eq!(
        e.state()
            .journal
            .entries()
            .iter()
            .filter(|entry| matches!(entry.event,
                GameEvent::ObjectTapped { object, cause: Cause::Effect } if object == fresh
            ))
            .count(),
        1
    );
    reach_their_main_phase(&mut e, P1);
    assert!(tapped(&e, fresh), "the normal untap step is replaced");
    assert_eq!(counters_on(&e, fresh, CounterKind::Stun), 0);
    assert!(!tapped(&e, already));
}

#[test]
fn stun_ray_selects_three_hundred_recipients_before_placing_any_counters() {
    let mut e = prepared(
        &[index::SUPPRESSION_RAY],
        &[index::DOUBLING_SEASON],
        &vec![index::GRIZZLY_BEARS; 300],
    );
    e.dev_state_mut(P0).unwrap().players[0].energy = 300;
    ray(&mut e, P1);
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseNumber { .. })
    });
    e.apply(P0, PlayerAction::ChooseNumber(300)).unwrap();
    let first = match e.pending() {
        Pending::ChooseCards {
            options, max: 255, ..
        } => options.clone(),
        other => panic!("first page: {other:?}"),
    };
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: first.clone(),
        },
    )
    .unwrap();
    assert!(
        first
            .iter()
            .all(|id| counters_on(&e, *id, CounterKind::Stun) == 0)
    );
    let last = match e.pending() {
        Pending::ChooseCards {
            options, max: 45, ..
        } => options.clone(),
        other => panic!("second page: {other:?}"),
    };
    e.apply(
        P0,
        PlayerAction::ChooseObjects {
            objects: last.clone(),
        },
    )
    .unwrap();
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].energy, 0);
    for id in first.into_iter().chain(last) {
        assert_eq!(
            counters_on(&e, id, CounterKind::Stun),
            1,
            "the caster's Doubling Season does not double counters on opposing permanents"
        );
    }
}

#[test]
fn stun_ray_declining_or_spending_without_recipients_is_legal() {
    for (count, energy, payment) in [(1, 0, 0), (1, 4, 0), (0, 4, 4)] {
        let mut e = prepared(
            &[index::SUPPRESSION_RAY],
            &[],
            &vec![index::GRIZZLY_BEARS; count],
        );
        e.dev_state_mut(P0).unwrap().players[0].energy = energy;
        ray(&mut e, P1);
        if energy > 0 {
            pass_until(&mut e, |e| {
                matches!(e.pending(), Pending::ChooseNumber { .. })
            });
            e.apply(P0, PlayerAction::ChooseNumber(payment)).unwrap();
        }
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            u32::from(e.state().players[0].energy),
            u32::from(energy) - payment
        );
        for id in e.state().battlefield_seen() {
            assert_eq!(counters_on(&e, id, CounterKind::Stun), 0);
            assert!(tapped(&e, id));
        }
    }
}

#[test]
fn stun_replaces_each_effect_untap_but_leaves_untapped_and_reversed_actions_alone() {
    let mut e = prepared(
        &[index::TWIDDLE, index::TWIDDLE, index::TWIDDLE],
        &[index::FOREST],
        &[],
    );
    let land = on_battlefield(&e, P0, index::FOREST).unwrap();
    let s = e.dev_state_mut(P0).unwrap();
    s.object_mut(land)
        .unwrap()
        .counters
        .set(CounterKind::Stun, 2);
    assert!(!s.set_tapped(land, false));
    assert_eq!(s.object(land).unwrap().counters.get(CounterKind::Stun), 2);
    s.set_tapped(land, true);
    for expected in [1, 0, 0] {
        e.refresh_offer();
        cast_with_floating(&mut e, P0, index::TWIDDLE);
        e.apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![land],
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(counters_on(&e, land, CounterKind::Stun), expected);
    }
    assert!(!tapped(&e, land));
    assert_eq!(
        e.state()
            .journal
            .entries()
            .iter()
            .filter(|entry| matches!(entry.event,
                GameEvent::ObjectUntapped { object, cause: Cause::Effect } if object == land
            ))
            .count(),
        1,
        "replaced untaps do not trigger untap abilities"
    );
    let s = e.dev_state_mut(P0).unwrap();
    s.object_mut(land)
        .unwrap()
        .counters
        .set(CounterKind::Stun, 2);
    s.set_tapped(land, true);
    assert!(s.restore_tapped(land, false));
    assert_eq!(s.object(land).unwrap().counters.get(CounterKind::Stun), 2);
}

#[test]
fn stun_untap_symbol_needs_tapped_state_but_a_replaced_payment_still_pays_the_cost() {
    use crate::engine::synthetic::{self, SyntheticLookup};
    use baylee_cards_dsl::{AbilityDef, Amount, Effect};
    static ABILITIES: &[AbilityDef] = &[baylee_cards_dsl::activated!(
        baylee_cards_dsl::cost!("", UntapSelf),
        &[Effect::GainLife {
            amount: Amount::Fixed(1)
        }]
    )];
    let definition = synthetic::land(98_042, "Untap cost probe", ABILITIES);
    let mut e = Engine::new(
        &synthetic::preset(SEED, &[98_042]),
        SyntheticLookup::new(vec![definition]),
    )
    .unwrap();
    synthetic::keep_mulligans(&mut e);
    for _ in 0..50 {
        if matches!(e.pending(), Pending::Priority { player: P0, .. }) {
            break;
        }
        let pending = e.pending().clone();
        assert!(synthetic::walk_past(&mut e, &pending));
    }
    let source = synthetic::permanents(&e, 98_042)[0];
    let action = PlayerAction::ActivateAbility {
        source,
        ability_index: 0,
    };
    let hash = e.state().snapshot_hash();
    assert!(e.apply(P0, action.clone()).is_err());
    assert_eq!(e.state().snapshot_hash(), hash);
    let s = e.dev_state_mut(P0).unwrap();
    s.set_tapped(source, true);
    s.object_mut(source)
        .unwrap()
        .counters
        .set(CounterKind::Stun, 1);
    e.refresh_offer();
    let life = e.state().players[0].life;
    e.apply(P0, action).unwrap();
    assert!(synthetic::tapped(&e, source));
    assert_eq!(
        e.state()
            .object(source)
            .unwrap()
            .counters
            .get(CounterKind::Stun),
        0
    );
    for _ in 0..50 {
        if e.state().players[0].life == life + 1 {
            break;
        }
        let pending = e.pending().clone();
        assert!(synthetic::walk_past(&mut e, &pending));
    }
    assert_eq!(e.state().players[0].life, life + 1);
}
