//! Result-stage regression tests independent of Ali's card definition.

use crate::damage::{Assignment, DamageWork};
use crate::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
use crate::event::{Cause, DamageTarget, GameEvent};
use crate::object::ObjectKind;
use crate::prevention::{ChosenSource, Shield, ShieldKind, Shielded};
use crate::state::{CardLookup, Commander, GameState};
use crate::zone::ZoneLocation;
use baylee_cards_dsl::{Duration, Filter, KeywordSet, Modifier, PlayerRel};
use baylee_core::ids::{CardIndex, EffectId, ObjectId, PlayerId};
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
fn state() -> GameState {
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
            rules: baylee_core::card_metadata::RulesMetadata::default(),
            format: FormatId::Freeform,
            seed: 4,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![],
            seats: vec![seat(), seat()],
        },
        &NoCards,
    )
    .unwrap()
}
fn creature(state: &mut GameState, owner: PlayerId, keywords: KeywordSet) -> ObjectId {
    let name = state.names.intern("Life-result fixture");
    let id = state.create_bare(
        owner,
        ObjectKind::Permanent,
        name,
        ZoneLocation::Battlefield,
    );
    let base = state.object_mut(id).unwrap().base_mut();
    base.types = TypeSet::CREATURE;
    base.power = Some(5);
    base.toughness = Some(1);
    base.keywords = keywords;
    id
}
fn effect(state: &mut GameState, modifier: Modifier) {
    state.effects.register(ContinuousEffect {
        id: EffectId::new(0),
        source: None,
        controller: A,
        origin: EffectOrigin::Resolution,
        layer: modifier.layer(),
        timestamp: 2,
        duration: Duration::UntilEndOfTurn,
        filter: EffectFilter::Dsl(&Filter::Any),
        modifier,
    });
}
fn protect(state: &mut GameState) {
    effect(
        state,
        Modifier::DamageLifeLossFloor {
            players: PlayerRel::You,
            minimum: 1,
        },
    );
}
fn assignment(source: ObjectId, recipient: DamageTarget, amount: u32) -> Assignment {
    Assignment {
        source,
        source_version: None,
        recipient,
        amount,
        is_combat: false,
    }
}
fn deal(state: &mut GameState, assignments: Vec<Assignment>) -> DamageWork {
    let mut work = DamageWork::new(state, assignments);
    assert!(work.advance(state).is_none());
    work
}
fn prevention_gain(state: &mut GameState, source: ObjectId) {
    let chosen = ChosenSource::new(
        state,
        state.source_identity(source).unwrap(),
        &Filter::Any,
        A,
        source,
    )
    .unwrap();
    state.shields.push(Shield {
        protects: Shielded::Player(A),
        controller: A,
        kind: ShieldKind::NextFrom {
            source: chosen,
            all_but: 0,
            gain_life: true,
            combat_only: false,
        },
    });
}

#[test]
fn floor_changes_only_life_loss_preserving_damage_lifelink_and_commander_tally() {
    let mut state = state();
    state.players[0].life = 2;
    protect(&mut state);
    let source = creature(&mut state, B, KeywordSet::LIFELINK);
    state.commanders[1].push(Commander {
        object: source,
        casts: 0,
        answered: 0,
    });
    let mut damage = assignment(source, DamageTarget::Player(A), 21);
    damage.is_combat = true;
    let work = deal(&mut state, vec![damage]);
    assert_eq!(work.dealt(), 21);
    assert_eq!(state.players[0].life, 1);
    assert_eq!(state.players[1].life, 41);
    assert_eq!(state.per_turn.life_lost_amount[0], 1);
    assert_eq!(state.per_turn.damage_dealt_to[0], 21);
    assert_eq!(state.players[0].commander_damage, vec![(source, 21)]);
    assert!(state.journal.entries().iter().any(|entry| matches!(
        entry.event,
        GameEvent::DamageDealt {
            target: DamageTarget::Player(A),
            amount: 21,
            is_combat: true,
            ..
        }
    )));
}

#[test]
fn floor_counts_all_simultaneous_prevention_gains_before_replacing_loss() {
    for incoming in [5, 6, 10] {
        let mut state = state();
        state.players[0].life = 2;
        protect(&mut state);
        let prevented = creature(&mut state, B, KeywordSet::EMPTY);
        let source = creature(&mut state, B, KeywordSet::EMPTY);
        prevention_gain(&mut state, prevented);
        deal(
            &mut state,
            vec![
                assignment(source, DamageTarget::Player(A), incoming),
                assignment(prevented, DamageTarget::Player(A), 5),
            ],
        );
        assert_eq!(state.players[0].life, if incoming == 5 { 2 } else { 1 });
        assert_eq!(state.per_turn.life_lost_amount[0], incoming.min(6));
        assert_eq!(state.per_turn.damage_dealt_to[0], incoming);
    }
}

#[test]
fn floor_counts_simultaneous_lifelink_even_when_damage_is_to_another_recipient() {
    for reverse in [false, true] {
        let mut state = state();
        state.players[0].life = 2;
        protect(&mut state);
        let source = creature(&mut state, B, KeywordSet::EMPTY);
        let healer = creature(&mut state, A, KeywordSet::LIFELINK);
        let mut assignments = vec![
            assignment(source, DamageTarget::Player(A), 5),
            assignment(healer, DamageTarget::Object(source), 5),
        ];
        if reverse {
            assignments.reverse();
        }
        deal(&mut state, assignments);
        assert_eq!(state.players[0].life, 2);
        assert_eq!(state.per_turn.life_lost_amount[0], 5);
        assert_eq!(state.object(source).unwrap().damage, 5);
    }
}

#[test]
fn replaced_prevention_and_lifelink_gains_do_not_enlarge_the_floor_allowance() {
    for lifelink in [false, true] {
        let mut state = state();
        state.players[0].life = 2;
        protect(&mut state);
        effect(
            &mut state,
            Modifier::LifeGainDrawsInstead {
                who: PlayerRel::You,
            },
        );
        let source = creature(&mut state, B, KeywordSet::EMPTY);
        let healer = creature(
            &mut state,
            if lifelink { A } else { B },
            if lifelink {
                KeywordSet::LIFELINK
            } else {
                KeywordSet::EMPTY
            },
        );
        if !lifelink {
            prevention_gain(&mut state, healer);
        }
        let recipient = if lifelink {
            DamageTarget::Player(B)
        } else {
            DamageTarget::Player(A)
        };
        deal(
            &mut state,
            vec![
                assignment(source, DamageTarget::Player(A), 5),
                assignment(healer, recipient, 5),
            ],
        );
        assert_eq!(state.players[0].life, 1);
        assert_eq!(state.per_turn.life_lost_amount[0], 1);
    }
}

#[test]
fn floor_is_shared_by_all_sources_but_does_not_replace_infect_or_ordinary_loss() {
    let mut state = state();
    state.players[0].life = 3;
    protect(&mut state);
    let source = creature(&mut state, B, KeywordSet::EMPTY);
    let infected = creature(
        &mut state,
        B,
        KeywordSet::INFECT.union(KeywordSet::LIFELINK),
    );
    deal(
        &mut state,
        vec![
            assignment(source, DamageTarget::Player(A), 1),
            assignment(infected, DamageTarget::Player(A), 5),
            assignment(source, DamageTarget::Player(A), 5),
        ],
    );
    assert_eq!(state.players[0].life, 1);
    assert_eq!(state.players[0].poison, 5);
    assert_eq!(state.players[1].life, 25);
    assert_eq!(state.per_turn.damage_dealt_to[0], 11);
    assert_eq!(state.per_turn.life_lost_amount[0], 2);
    state.change_life(A, -2, Cause::Effect);
    assert_eq!(state.players[0].life, -1);
}

#[test]
fn floor_boundaries_never_turn_damage_into_a_gain() {
    for life in [i32::MIN, -1, 0, 1, 2, i32::MAX] {
        let mut state = state();
        state.players[0].life = life;
        protect(&mut state);
        let source = creature(&mut state, B, KeywordSet::EMPTY);
        deal(
            &mut state,
            vec![assignment(source, DamageTarget::Player(A), u32::MAX)],
        );
        assert_eq!(state.players[0].life, life.min(1));
        assert_eq!(state.per_turn.damage_dealt_to[0], u32::MAX);
    }
}

#[test]
fn simultaneous_lethal_damage_to_the_floor_source_keeps_the_result_floor() {
    let mut state = state();
    state.players[0].life = 2;
    let protected_creature = creature(&mut state, A, KeywordSet::EMPTY);
    let source = creature(&mut state, B, KeywordSet::EMPTY);
    let modifier = Modifier::DamageLifeLossFloor {
        players: PlayerRel::You,
        minimum: 1,
    };
    state.effects.register(ContinuousEffect {
        id: EffectId::new(0),
        source: Some(protected_creature),
        controller: A,
        origin: EffectOrigin::Resolution,
        layer: modifier.layer(),
        timestamp: 2,
        duration: Duration::UntilEndOfTurn,
        filter: EffectFilter::Dsl(&Filter::Any),
        modifier,
    });
    deal(
        &mut state,
        vec![
            assignment(source, DamageTarget::Object(protected_creature), 5),
            assignment(source, DamageTarget::Player(A), 5),
        ],
    );
    assert_eq!(state.players[0].life, 1);
    assert_eq!(state.object(protected_creature).unwrap().damage, 5);
}

#[test]
fn fixed_life_total_still_records_full_damage() {
    let mut state = state();
    state.players[0].life = 2;
    protect(&mut state);
    effect(
        &mut state,
        Modifier::LifeTotalCantChange {
            who: PlayerRel::You,
        },
    );
    let source = creature(&mut state, A, KeywordSet::LIFELINK);
    deal(
        &mut state,
        vec![assignment(source, DamageTarget::Player(A), 5)],
    );
    assert_eq!(state.players[0].life, 2);
    assert_eq!(state.per_turn.life_lost_amount[0], 0);
    assert_eq!(state.per_turn.damage_dealt_to[0], 5);
}

fn result_counter_replacements(state: &mut GameState, additions: usize) {
    for _ in 0..additions {
        effect(state, Modifier::AdditionalCounterPlacement { n: 1 });
    }
    let source = creature(state, A, KeywordSet::EMPTY);
    state
        .replacement_rules
        .push(crate::state::ReplacementEntry {
            source,
            controller: A,
            rule: baylee_cards_dsl::ReplacementRule::DoubleCounterPlacement {
                object_filter: &Filter::Any,
            },
        });
}

#[test]
fn infect_result_orders_wait_for_every_recipient_before_any_damage_result() {
    use crate::choice::PlayerAction;
    use baylee_cards_dsl::CounterKind;
    for additive_first in [true, false] {
        let mut state = state();
        state.players[0].life = 2;
        state.turn.active = B;
        protect(&mut state);
        result_counter_replacements(&mut state, 1);
        let infected = creature(
            &mut state,
            B,
            KeywordSet::INFECT.union(KeywordSet::LIFELINK),
        );
        let normal = creature(&mut state, B, KeywordSet::EMPTY);
        let target_a = creature(&mut state, A, KeywordSet::EMPTY);
        let target_b = creature(&mut state, B, KeywordSet::EMPTY);
        let mut work = DamageWork::new(
            &mut state,
            vec![
                assignment(normal, DamageTarget::Player(A), 5),
                assignment(infected, DamageTarget::Object(target_a), 2),
                assignment(infected, DamageTarget::Object(target_b), 2),
            ],
        );
        let before = state.journal.len();
        let pending = work.advance(&mut state).unwrap();
        assert_eq!(pending.asked(), Some(B), "APNAP, not assignment order");
        assert_eq!(state.journal.len(), before);
        assert_eq!(state.players[0].life, 2);
        assert_eq!(state.players[1].life, 20);
        let answer = PlayerAction::YesNo(additive_first);
        assert_eq!(pending.answer_fault(&answer), None);
        let pending = work.answer(&mut state, &answer).unwrap();
        assert_eq!(pending.asked(), Some(A));
        assert_eq!(
            state.journal.len(),
            before,
            "first recipient is still uncommitted"
        );
        assert_eq!(
            state
                .object(target_b)
                .unwrap()
                .counters
                .get(CounterKind::M1M1),
            0
        );
        assert_eq!(state.per_turn.damage_dealt_to[0], 0);
        assert_eq!(state.per_turn.life_lost_amount[0], 0);
        assert_eq!(pending.answer_fault(&answer), None);
        assert!(work.answer(&mut state, &answer).is_none());
        let counters = if additive_first { 6 } else { 5 };
        for target in [target_a, target_b] {
            assert_eq!(
                state
                    .object(target)
                    .unwrap()
                    .counters
                    .get(CounterKind::M1M1),
                counters
            );
        }
        assert_eq!(state.players[0].life, 1);
        assert_eq!(
            state.players[1].life, 24,
            "lifelink reads damage, not counters"
        );
        assert_eq!(state.per_turn.damage_dealt_to[0], 5);
        assert_eq!(state.per_turn.life_lost_amount[0], 1);
        assert_eq!(work.dealt(), 9);
        assert!(!crate::replacement::counter_order::has_pending(&state));
    }
}

#[test]
fn combat_infect_addition_enables_doubling_then_allows_remaining_order_choice() {
    use crate::choice::PlayerAction;
    use baylee_cards_dsl::CounterKind;
    for additive_first in [true, false] {
        let mut state = state();
        result_counter_replacements(&mut state, 2);
        let source = creature(&mut state, B, KeywordSet::INFECT);
        let target = creature(&mut state, A, KeywordSet::EMPTY);
        let mut damage = assignment(source, DamageTarget::Object(target), 2);
        damage.is_combat = true;
        let mut work = DamageWork::new(&mut state, vec![damage]);
        let before = state.journal.len();
        // The first additive turns this turn-based result into an effect;
        // before that, Doubling Season is inapplicable (CR 614.16).
        let pending = work.advance(&mut state).unwrap();
        assert_eq!(pending.asked(), Some(A));
        let mut pending = work.answer(&mut state, &PlayerAction::YesNo(true));
        if pending.is_some() {
            pending = work.answer(&mut state, &PlayerAction::YesNo(additive_first));
        }
        assert!(pending.is_none());
        assert!(state.journal.len() > before);
        assert_eq!(
            state
                .object(target)
                .unwrap()
                .counters
                .get(CounterKind::M1M1),
            if additive_first { 8 } else { 7 }
        );
        assert_eq!(work.dealt(), 2);
    }
}

#[test]
fn simultaneous_infect_from_one_player_combines_before_additive_replacement() {
    use crate::choice::PlayerAction;
    use baylee_cards_dsl::CounterKind;
    let mut state = state();
    result_counter_replacements(&mut state, 1);
    let first = creature(&mut state, B, KeywordSet::INFECT);
    let second = creature(&mut state, B, KeywordSet::INFECT);
    let target = creature(&mut state, A, KeywordSet::EMPTY);
    let mut work = DamageWork::new(
        &mut state,
        vec![
            assignment(first, DamageTarget::Object(target), 2),
            assignment(second, DamageTarget::Object(target), 3),
        ],
    );
    assert!(work.advance(&mut state).is_some());
    assert!(
        work.answer(&mut state, &PlayerAction::YesNo(true))
            .is_none()
    );
    assert_eq!(
        state
            .object(target)
            .unwrap()
            .counters
            .get(CounterKind::M1M1),
        12
    );
    assert_eq!(work.dealt(), 5);
}

#[test]
fn forbidden_infect_counters_do_not_offer_result_replacement_order() {
    use baylee_cards_dsl::CounterKind;
    let mut state = state();
    result_counter_replacements(&mut state, 1);
    effect(
        &mut state,
        Modifier::CounterMaximum {
            kind: CounterKind::M1M1,
            maximum: 0,
        },
    );
    let source = creature(
        &mut state,
        B,
        KeywordSet::INFECT.union(KeywordSet::LIFELINK),
    );
    let target = creature(&mut state, A, KeywordSet::EMPTY);
    let work = deal(
        &mut state,
        vec![assignment(source, DamageTarget::Object(target), 2)],
    );
    assert_eq!(
        state
            .object(target)
            .unwrap()
            .counters
            .get(CounterKind::M1M1),
        0
    );
    assert_eq!(state.players[1].life, 22);
    assert_eq!(work.dealt(), 2);
}
