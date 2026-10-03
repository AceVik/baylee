//! Generic damage procedure tests, independent of card scripts.

use super::*;
use crate::choice::AnswerFault;
use crate::object::ObjectKind;
use crate::prevention::{ChosenSource, Shield, Shielded};
use crate::state::CardLookup;
use crate::zone::ZoneLocation;
use baylee_core::ids::CardIndex;
use baylee_core::preset::{FormatId, GamePreset, HouseRules, SeatController, SeatSpec};
use baylee_core::types::TypeSet;

const A: PlayerId = PlayerId::new(0);
const B: PlayerId = PlayerId::new(1);
struct NoCards;
impl CardLookup for NoCards {
    fn card(&self, _: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
        None
    }
}
fn state(seats: usize) -> GameState {
    let seat = || SeatSpec {
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
    };
    GameState::from_preset(
        &GamePreset {
            format: FormatId::Freeform,
            seed: 4,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![],
            seats: (0..seats).map(|_| seat()).collect(),
        },
        &NoCards,
    )
    .unwrap()
}
fn creature(state: &mut GameState, owner: PlayerId, power: i16, toughness: i16) -> ObjectId {
    let name = state.names.intern("Damage fixture");
    let id = state.create_bare(
        owner,
        ObjectKind::Permanent,
        name,
        ZoneLocation::Battlefield,
    );
    let base = state.object_mut(id).unwrap().base_mut();
    base.types = TypeSet::CREATURE;
    base.power = Some(power);
    base.toughness = Some(toughness);
    id
}
fn assigned(source: ObjectId, recipient: DamageTarget, amount: u32) -> Assignment {
    Assignment {
        source,
        source_version: None,
        recipient,
        amount,
        is_combat: false,
    }
}
fn choose(pending: &Pending, kind: impl Fn(DamageEffectKind) -> bool) -> PlayerAction {
    let Pending::ChooseDamageEffect {
        choice, options, ..
    } = pending
    else {
        panic!("effect choice: {pending:?}")
    };
    PlayerAction::ChooseDamageEffect {
        choice: *choice,
        effect: options.iter().find(|o| kind(o.kind)).unwrap().id,
    }
}
fn shield(state: &mut GameState, player: PlayerId, kind: ShieldKind) {
    state.shields.push(Shield {
        protects: Shielded::Player(player),
        kind,
        controller: player,
    });
}

#[test]
fn prevention_life_results_wait_for_the_entire_simultaneous_event() {
    let mut state = state(3);
    state.players[0].life = 2;
    let first = creature(&mut state, B, 5, 5);
    let second = creature(&mut state, B, 5, 5);
    let chosen = ChosenSource::new(
        &state,
        state.source_identity(first).unwrap(),
        &baylee_cards_dsl::Filter::Any,
        A,
        first,
    )
    .unwrap();
    shield(
        &mut state,
        A,
        ShieldKind::NextFrom {
            source: chosen,
            all_but: 0,
            gain_life: true,
            combat_only: false,
        },
    );
    shield(&mut state, B, ShieldKind::Next(1));
    shield(&mut state, B, ShieldKind::Next(2));
    let mut work = DamageWork::new(
        &mut state,
        vec![
            assigned(first, DamageTarget::Player(A), 5),
            assigned(second, DamageTarget::Player(A), 5),
            assigned(first, DamageTarget::Player(B), 1),
        ],
    );
    let pending = work.advance(&mut state).unwrap();
    assert_eq!(pending.asked(), Some(B));
    assert_eq!(
        state.players[0].life, 2,
        "prevention gain is still a pending result"
    );
    assert_eq!(work.life_gains, vec![(A, 5)]);
    let action = choose(&pending, |kind| {
        matches!(kind, DamageEffectKind::PreventNext { remaining: 1 })
    });
    assert!(work.answer(&mut state, &action).is_none());
    assert_eq!(
        state.players[0].life, 2,
        "CR 120.4d's loss-five/gain-five event has net zero"
    );
}

#[test]
fn old_and_returned_source_incarnations_keep_distinct_characteristics() {
    use crate::event::Cause;
    use crate::zone::ZonePosition;
    use baylee_core::color::{Color, ColorSet};
    const RED: baylee_cards_dsl::Filter =
        baylee_cards_dsl::Filter::HasColor(ColorSet::of(Color::Red));
    let mut state = state(2);
    let source = creature(&mut state, A, 2, 2);
    state.object_mut(source).unwrap().base_mut().colors = ColorSet::of(Color::Red);
    state.object_mut(source).unwrap().base_mut().keywords = KeywordSet::LIFELINK;
    state.refresh_characteristics();
    let old_version = state.object(source).unwrap().version;
    let chosen = ChosenSource::new(
        &state,
        state.source_identity(source).unwrap(),
        &RED,
        B,
        source,
    )
    .unwrap();
    state
        .move_object(
            source,
            ZoneLocation::Exile(A),
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    state
        .move_object(
            source,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    state.object_mut(source).unwrap().base_mut().colors = ColorSet::of(Color::Blue);
    state.object_mut(source).unwrap().base_mut().keywords = KeywordSet::EMPTY;
    state.refresh_characteristics();
    assert!(chosen.deals_as(
        &state,
        state.damage_source(source, Some(old_version)).unwrap()
    ));
    assert!(!chosen.deals_as(&state, state.object(source).unwrap()));
    let mut old = assigned(source, DamageTarget::Player(B), 2);
    old.source_version = Some(old_version);
    assert!(
        DamageWork::new(&mut state, vec![old])
            .advance(&mut state)
            .is_none()
    );
    assert_eq!(
        state.players[0].life, 22,
        "the departed source still has lifelink"
    );
    assert!(
        DamageWork::new(
            &mut state,
            vec![assigned(source, DamageTarget::Player(B), 2)]
        )
        .advance(&mut state)
        .is_none()
    );
    assert_eq!(
        state.players[0].life, 22,
        "the returned source has no lifelink"
    );
    assert_eq!(state.players[1].life, 16);
}

#[test]
fn competing_paid_and_life_prevention_permit_both_orders() {
    for paid_first in [true, false] {
        let mut state = state(2);
        let source = creature(&mut state, A, 2, 2);
        let chosen = ChosenSource::new(
            &state,
            state.source_identity(source).unwrap(),
            &baylee_cards_dsl::Filter::Any,
            B,
            source,
        )
        .unwrap();
        shield(
            &mut state,
            B,
            ShieldKind::NextFrom {
                source: chosen,
                all_but: 0,
                gain_life: true,
                combat_only: false,
            },
        );
        let mut work = DamageWork::new(
            &mut state,
            vec![assigned(source, DamageTarget::Player(B), 2)],
        )
        .with_paid_prevention(
            1,
            ShieldOrigin {
                source,
                ability: None,
            },
            B,
        );
        let pending = work.advance(&mut state).unwrap();
        assert_eq!(
            state.players[1].life, 20,
            "no damage result before the choice"
        );
        let action = choose(&pending, |kind| {
            matches!(kind, DamageEffectKind::PreventThisEvent { .. }) == paid_first
        });
        assert_eq!(pending.answer_fault(&action), None);
        assert!(work.answer(&mut state, &action).is_none());
        assert_eq!(state.players[1].life, if paid_first { 21 } else { 22 });
        assert_eq!(work.dealt(), 0);
    }
}

#[test]
fn finite_shield_can_be_split_across_simultaneous_sources() {
    let mut state = state(2);
    let first = creature(&mut state, A, 2, 2);
    let second = creature(&mut state, A, 3, 3);
    state.object_mut(second).unwrap().base_mut().keywords = KeywordSet::LIFELINK;
    shield(&mut state, B, ShieldKind::Next(3));
    let mut work = DamageWork::new(
        &mut state,
        vec![
            assigned(first, DamageTarget::Player(B), 2),
            assigned(second, DamageTarget::Player(B), 3),
        ],
    );
    let pending = work.advance(&mut state).unwrap();
    let Pending::AllocatePrevention {
        choice,
        ref damage,
        total,
        ..
    } = pending
    else {
        panic!("allocation")
    };
    assert_eq!(total, 3);
    assert_eq!(state.players[1].life, 20);
    let action = PlayerAction::AllocatePrevention {
        choice,
        allocation: vec![(damage[0].id, 1), (damage[1].id, 2)],
    };
    assert_eq!(pending.answer_fault(&action), None);
    assert!(work.answer(&mut state, &action).is_none());
    assert_eq!(state.players[1].life, 18);
    assert_eq!(
        state.players[0].life, 21,
        "lifelink counts only damage that gets through"
    );
    assert!(state.shields.is_empty());
}

#[test]
fn simultaneous_decisions_follow_active_player_order() {
    let mut state = state(3);
    state.turn.active = B;
    let source = creature(&mut state, A, 2, 2);
    for p in [A, B, PlayerId::new(2)] {
        shield(&mut state, p, ShieldKind::Next(3));
        shield(&mut state, p, ShieldKind::Next(4));
    }
    let assignments = [A, B, PlayerId::new(2)]
        .into_iter()
        .map(|p| assigned(source, DamageTarget::Player(p), 2))
        .collect();
    let mut work = DamageWork::new(&mut state, assignments);
    let mut pending = work.advance(&mut state);
    for player in [B, PlayerId::new(2), A] {
        let question = pending.take().unwrap();
        assert_eq!(question.asked(), Some(player));
        let action = choose(&question, |_| true);
        pending = work.answer(&mut state, &action);
    }
    assert!(pending.is_none());
    assert!(state.players.iter().all(|p| p.life == 20));
}

#[test]
fn allocations_reject_stale_duplicate_unknown_and_wrong_totals() {
    let mut state = state(2);
    let source = creature(&mut state, A, 2, 2);
    let other = creature(&mut state, A, 2, 2);
    shield(&mut state, B, ShieldKind::Next(3));
    let mut work = DamageWork::new(
        &mut state,
        vec![
            assigned(source, DamageTarget::Player(B), 2),
            assigned(other, DamageTarget::Player(B), 2),
        ],
    );
    let pending = work.advance(&mut state).unwrap();
    let Pending::AllocatePrevention { choice, .. } = pending else {
        panic!("allocation")
    };
    for (allocation, fault) in [
        (vec![(0, 1), (0, 2)], AnswerFault::Repeated),
        (vec![(99, 3)], AnswerFault::NotOffered),
        (vec![(0, 3)], AnswerFault::OutOfRange),
        (vec![(0, 1)], AnswerFault::TotalTooLow),
        (vec![(0, 2), (1, 2)], AnswerFault::TotalTooHigh),
    ] {
        assert_eq!(
            pending.answer_fault(&PlayerAction::AllocatePrevention { choice, allocation }),
            Some(fault)
        );
    }
    assert_eq!(
        pending.answer_fault(&PlayerAction::AllocatePrevention {
            choice: DamageChoiceId {
                step: choice.step + 1,
                ..choice
            },
            allocation: vec![(0, 1), (1, 2)]
        }),
        Some(AnswerFault::NotOffered)
    );
    assert_eq!(state.players[1].life, 20);
    assert_eq!(state.shields[0].kind, ShieldKind::Next(3));
}

#[test]
fn allocation_sums_use_the_full_unsigned_range() {
    let mut state = state(2);
    let one = creature(&mut state, A, 1, 1);
    let two = creature(&mut state, A, 1, 1);
    shield(&mut state, B, ShieldKind::Next(u32::MAX));
    let mut work = DamageWork::new(
        &mut state,
        vec![
            assigned(one, DamageTarget::Player(B), u32::MAX),
            assigned(two, DamageTarget::Player(B), u32::MAX),
        ],
    );
    let pending = work.advance(&mut state).unwrap();
    let Pending::AllocatePrevention { choice, total, .. } = pending else {
        panic!("allocation")
    };
    assert_eq!(total, u32::MAX);
    let action = PlayerAction::AllocatePrevention {
        choice,
        allocation: vec![(0, u32::MAX - 1), (1, 1)],
    };
    assert_eq!(pending.answer_fault(&action), None);
    assert!(work.answer(&mut state, &action).is_none());
    assert_eq!(work.dealt(), u32::MAX);
    assert!(state.shields.is_empty());
}

#[test]
fn shield_identity_survives_removing_an_earlier_shield() {
    let mut store = crate::prevention::ShieldStore::default();
    let plain = Shield {
        protects: Shielded::Player(A),
        kind: ShieldKind::Next(1),
        controller: A,
    };
    let first = store.push_from(plain, None);
    let second = store.push_from(plain, None);
    store.remove(0);
    assert_eq!(store.position(first), None);
    assert_eq!(store.position(second), Some(0));
    store.clear();
    assert!(store.push_from(plain, None) > second);
}

#[test]
fn finite_redirection_allocates_unpreventable_damage_by_recipient_controller() {
    for unpreventable in [false, true] {
        let mut state = state(2);
        let first = creature(&mut state, B, 1, 1);
        let second = creature(&mut state, B, 3, 3);
        let target = creature(&mut state, B, 6, 6);
        state.object_mut(second).unwrap().base_mut().keywords = KeywordSet::LIFELINK;
        if unpreventable {
            let modifier = baylee_cards_dsl::Modifier::CombatDamageCantBePrevented;
            state.effects.register(crate::effects::ContinuousEffect {
                id: baylee_core::ids::EffectId::new(0),
                source: None,
                controller: A,
                origin: crate::effects::EffectOrigin::Resolution,
                layer: modifier.layer(),
                timestamp: 2,
                duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
                filter: crate::effects::EffectFilter::Dsl(&baylee_cards_dsl::Filter::Any),
                modifier,
            });
        }
        let version = state.object(target).unwrap().version;
        state.shields.push(Shield {
            protects: Shielded::Object(target, version),
            kind: ShieldKind::RedirectNext {
                remaining: 1,
                to: A,
            },
            controller: A,
        });
        let mut assignments = vec![
            assigned(first, DamageTarget::Object(target), 1),
            assigned(second, DamageTarget::Object(target), 3),
        ];
        for part in &mut assignments {
            part.is_combat = true;
        }
        let mut work = DamageWork::new(&mut state, assignments);
        let pending = work.advance(&mut state).unwrap();
        let Pending::AllocatePrevention {
            player,
            choice,
            ref damage,
            total,
            ref effect,
        } = pending
        else {
            panic!("allocation: {pending:?}")
        };
        assert_eq!(player, B, "affected controller, not shield's controller");
        assert_eq!(total, 1);
        assert_eq!(
            effect.kind,
            DamageEffectKind::RedirectNext {
                remaining: 1,
                to: DamageTarget::Player(A)
            }
        );
        assert!(damage.iter().all(|part| part.preventable != unpreventable));
        let before = work.fingerprint();
        let invalid = PlayerAction::AllocatePrevention {
            choice,
            allocation: vec![(damage[0].id, 2)],
        };
        assert_eq!(
            pending.answer_fault(&invalid),
            Some(AnswerFault::OutOfRange)
        );
        assert_eq!(before, work.fingerprint());
        let action = PlayerAction::AllocatePrevention {
            choice,
            allocation: vec![(damage[0].id, 1)],
        };
        let mut replay_state = state.clone();
        let mut replay = work.clone();
        assert!(work.answer(&mut state, &action).is_none());
        assert!(replay.answer(&mut replay_state, &action).is_none());
        assert_eq!(work.fingerprint(), replay.fingerprint());
        assert_eq!(state.players[0].life, 19);
        assert_eq!(state.players[1].life, 23, "lifelink source is unchanged");
        assert_eq!(state.object(target).unwrap().damage, 3);
        assert!(state.shields.is_empty());
        assert_eq!(work.parts[2].view.source, first);
        assert_ne!(work.parts[2].view.id, work.parts[0].view.id);
    }
}

#[test]
fn finite_redirection_retains_capacity_across_events_and_never_prevents_damage() {
    let mut state = state(2);
    let source = creature(&mut state, B, 2, 2);
    let target = creature(&mut state, A, 6, 6);
    state.object_mut(source).unwrap().base_mut().keywords = KeywordSet::LIFELINK;
    state.shields.push(Shield {
        protects: Shielded::Object(target, state.object(target).unwrap().version),
        kind: ShieldKind::RedirectNext {
            remaining: 3,
            to: A,
        },
        controller: A,
    });
    for (damage, life, left) in [(2, 18, 1), (3, 17, 0)] {
        let mut work = DamageWork::new(
            &mut state,
            vec![assigned(source, DamageTarget::Object(target), damage)],
        );
        assert!(work.advance(&mut state).is_none());
        assert_eq!(work.dealt(), damage);
        assert_eq!(state.players[0].life, life);
        if left > 0 {
            assert_eq!(
                state.shields[0].kind,
                ShieldKind::RedirectNext {
                    remaining: left,
                    to: A
                }
            );
        } else {
            assert!(state.shields.is_empty());
        }
    }
    assert_eq!(state.object(target).unwrap().damage, 2);
    assert_eq!(state.players[1].life, 25);
}

#[test]
fn finite_redirection_follows_only_damageable_type_changes() {
    for types in [TypeSet::PLANESWALKER, TypeSet::ARTIFACT] {
        let mut state = state(2);
        let source = creature(&mut state, B, 2, 2);
        let target = creature(&mut state, A, 6, 6);
        let version = state.object(target).unwrap().version;
        state.shields.push(Shield {
            protects: Shielded::Object(target, version),
            kind: ShieldKind::RedirectNext {
                remaining: 1,
                to: A,
            },
            controller: A,
        });
        state.object_mut(target).unwrap().base_mut().types = types;
        state
            .object_mut(target)
            .unwrap()
            .counters
            .set(CounterKind::Loyalty, 5);
        state.invalidate_projections();
        state.refresh_characteristics();
        let mut work = DamageWork::new(
            &mut state,
            vec![assigned(source, DamageTarget::Object(target), 2)],
        );
        assert!(work.advance(&mut state).is_none());
        if types == TypeSet::PLANESWALKER {
            assert_eq!(state.players[0].life, 19);
            assert_eq!(
                state
                    .object(target)
                    .unwrap()
                    .counters
                    .get(CounterKind::Loyalty),
                4
            );
        } else {
            assert_eq!(state.players[0].life, 20);
            assert_eq!(state.shields.len(), 1);
        }
    }
}

#[test]
fn redirection_existing_shields_and_standing_effects_follow_damageable_types() {
    for types in [TypeSet::PLANESWALKER, TypeSet::BATTLE, TypeSet::ARTIFACT] {
        for standing in [false, true] {
            let mut state = state(2);
            let source = creature(&mut state, B, 2, 2);
            let target = creature(&mut state, A, 6, 6);
            let version = state.object(target).unwrap().version;
            if standing {
                let modifier =
                    baylee_cards_dsl::Modifier::RedirectDamageToYou(&baylee_cards_dsl::Filter::Any);
                state.effects.register(crate::effects::ContinuousEffect {
                    id: baylee_core::ids::EffectId::new(0),
                    source: Some(target),
                    controller: A,
                    origin: crate::effects::EffectOrigin::Resolution,
                    layer: modifier.layer(),
                    timestamp: 2,
                    duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
                    filter: crate::effects::EffectFilter::ObjectIs(target, version),
                    modifier,
                });
            } else {
                let chosen = ChosenSource::new(
                    &state,
                    state.source_identity(source).unwrap(),
                    &baylee_cards_dsl::Filter::Any,
                    A,
                    target,
                )
                .unwrap();
                state.shields.push(Shield {
                    protects: Shielded::Object(target, version),
                    kind: ShieldKind::RedirectNextFrom {
                        source: chosen,
                        to: A,
                    },
                    controller: A,
                });
            }
            state.object_mut(target).unwrap().base_mut().types = types;
            state
                .object_mut(target)
                .unwrap()
                .counters
                .set(CounterKind::Loyalty, 5);
            state.invalidate_projections();
            state.refresh_characteristics();
            let recipient = if standing {
                DamageTarget::Player(A)
            } else {
                DamageTarget::Object(target)
            };
            let mut work = DamageWork::new(&mut state, vec![assigned(source, recipient, 2)]);
            assert!(work.advance(&mut state).is_none());
            let eligible = types != TypeSet::ARTIFACT;
            let redirected = work.parts[0].view.recipient != recipient;
            assert_eq!(redirected, eligible);
            let life_lost = if standing { !eligible } else { eligible };
            assert_eq!(state.players[0].life, if life_lost { 18 } else { 20 });
            if standing && types == TypeSet::PLANESWALKER {
                assert_eq!(
                    state
                        .object(target)
                        .unwrap()
                        .counters
                        .get(CounterKind::Loyalty),
                    3
                );
            }
        }
    }
}
