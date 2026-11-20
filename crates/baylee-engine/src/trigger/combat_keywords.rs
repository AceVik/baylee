//! Synthetic attack triggers for combat keywords:
//! - Battle cry (CR 702.91a–b)
//! - Dethrone (CR 702.105a–b)
//! - Melee (CR 702.121a–b)
//! - Mentor (CR 702.134a–b)
//! - Training (CR 702.149a–b)

use super::events::{ObjectMove, object_moves};
use super::{PendingTrigger, source_at_event_with_moves, times_triggered};
use crate::{
    event::{GameEvent, JournalEntry},
    object::GameObject,
    state::GameState,
    text_changes::TextChangeMap,
};
use baylee_cards_dsl::{
    Amount, CounterKind, Duration, Effect, Filter, KeywordSet, Layer, Modifier, TargetReq,
    TargetSpec, TriggerEventKind,
};
use baylee_core::ids::{AbilityRef, Defender};

const COMBAT_ATTACK_KEYWORDS: KeywordSet = KeywordSet::BATTLE_CRY
    .union(KeywordSet::DETHRONE)
    .union(KeywordSet::MELEE)
    .union(KeywordSet::MENTOR)
    .union(KeywordSet::TRAINING)
    .union(KeywordSet::MYRIAD);

static MYRIAD_EFFECTS: &[Effect] = &[Effect::Myriad];

static ENLIST_EFFECTS: &[Effect] = &[Effect::PumpFilter {
    filter: &Filter::This,
    controlled_by: None,
    power: Amount::EventPower,
    toughness: Amount::Fixed(0),
    keywords: KeywordSet::EMPTY,
    duration: Duration::UntilEndOfTurn,
}];

static OTHER_ATTACKING_CREATURES: Filter =
    Filter::And(&[Filter::Another, Filter::ATTACKING_CREATURE]);

static BATTLE_CRY_EFFECTS: &[Effect] = &[Effect::CreateContinuousEffect {
    layer: Layer::PtModify,
    filter: &OTHER_ATTACKING_CREATURES,
    modifier: Modifier::ModifyPT(1, 0),
    duration: Duration::UntilEndOfTurn,
}];

static PLUS_ONE_COUNTER_SOURCE: &[Effect] = &[Effect::AddCounterSource {
    kind: CounterKind::P1P1,
    amount: Amount::Fixed(1),
}];

static MELEE_EFFECTS: &[Effect] = &[Effect::PumpFilter {
    filter: &Filter::This,
    controlled_by: None,
    power: Amount::OpponentsYouAttackedThisCombat,
    toughness: Amount::OpponentsYouAttackedThisCombat,
    keywords: KeywordSet::EMPTY,
    duration: Duration::UntilEndOfTurn,
}];

static MENTOR_TARGET: Filter =
    Filter::And(&[Filter::ATTACKING_CREATURE, Filter::PowerLessThanSourcePower]);

static MENTOR_EFFECTS: &[Effect] = &[Effect::AddCounter {
    kind: CounterKind::P1P1,
    amount: Amount::Fixed(1),
}];

pub(super) fn collect(
    state: &GameState,
    obj: &GameObject,
    events: &[JournalEntry],
    moves: &[ObjectMove],
    batch_has_attacks: bool,
    batch_has_zone_changes: bool,
    triggers: &mut Vec<PendingTrigger>,
) {
    if !batch_has_attacks {
        return;
    }
    let permanent = obj.id;
    for entry in events {
        let GameEvent::BecameAttacker { object, defending } = &entry.event else {
            continue;
        };
        if *object != permanent {
            continue;
        }
        let source = source_at_event_with_moves(state, obj, moves, entry.seq, false);
        let kws = source.characteristics().keywords;
        if (kws.bits() & COMBAT_ATTACK_KEYWORDS.bits()) == 0 {
            continue;
        }
        let multiplier = times_triggered(state, TriggerEventKind::Any, permanent);
        if multiplier == 0 {
            continue;
        }

        // CR 702.91a–b: Battle cry — "Whenever this creature attacks, each other
        // attacking creature gets +1/+0 until end of turn."
        if kws.contains(KeywordSet::BATTLE_CRY) {
            let n = crate::layers::keyword_instances(state, source, KeywordSet::BATTLE_CRY)
                .saturating_mul(multiplier);
            push_synthetic(
                state,
                source,
                entry.seq,
                n,
                BATTLE_CRY_EFFECTS,
                None,
                None,
                triggers,
            );
        }

        // CR 702.105a–b: Dethrone — "Whenever this creature attacks the player
        // with the most life or tied for most life, put a +1/+1 counter on this
        // creature." Attacking a planeswalker or battle never triggers dethrone.
        if kws.contains(KeywordSet::DETHRONE)
            && let Defender::Player(attacked) = *defending
            && has_most_or_tied_life(state, attacked)
        {
            let n = crate::layers::keyword_instances(state, source, KeywordSet::DETHRONE)
                .saturating_mul(multiplier);
            push_synthetic(
                state,
                source,
                entry.seq,
                n,
                PLUS_ONE_COUNTER_SOURCE,
                None,
                None,
                triggers,
            );
        }

        // CR 702.121a–b: Melee — "Whenever this creature attacks, it gets +1/+1
        // until end of turn for each opponent you attacked with a creature this
        // combat."
        if kws.contains(KeywordSet::MELEE) {
            let n = crate::layers::keyword_instances(state, source, KeywordSet::MELEE)
                .saturating_mul(multiplier);
            push_synthetic(
                state,
                source,
                entry.seq,
                n,
                MELEE_EFFECTS,
                None,
                None,
                triggers,
            );
        }

        // CR 702.134a–b: Mentor — "Whenever this creature attacks, put a +1/+1
        // counter on target attacking creature with power less than this
        // creature's power."
        if kws.contains(KeywordSet::MENTOR) {
            let n = crate::layers::keyword_instances(state, source, KeywordSet::MENTOR)
                .saturating_mul(multiplier);
            push_synthetic(
                state,
                source,
                entry.seq,
                n,
                MENTOR_EFFECTS,
                Some(TargetReq::one(TargetSpec::Object(&MENTOR_TARGET))),
                None,
                triggers,
            );
        }

        // CR 702.149a–b: Training — "Whenever this creature and at least one
        // other creature with power greater than this creature's power attack,
        // put a +1/+1 counter on this creature."
        if kws.contains(KeywordSet::TRAINING)
            && attacked_with_greater_power(state, source, events, entry.seq, batch_has_zone_changes)
        {
            let n = crate::layers::keyword_instances(state, source, KeywordSet::TRAINING)
                .saturating_mul(multiplier);
            push_synthetic(
                state,
                source,
                entry.seq,
                n,
                PLUS_ONE_COUNTER_SOURCE,
                None,
                None,
                triggers,
            );
        }

        if kws.contains(KeywordSet::MYRIAD) {
            collect_myriad(state, source, *defending, entry.seq, multiplier, triggers);
        }
    }
}

/// CR 702.154a–d: Enlist's linked triggered ability — "When you do, this
/// creature gets +X/+0 until end of turn, where X is the tapped creature's
/// power." Fires once per paid enlist cost (`GameEvent::CreatureEnlisted`),
/// independent of how many instances of enlist the attacker has (CR 702.154d).
pub(super) fn collect_enlist(
    state: &GameState,
    obj: &GameObject,
    events: &[JournalEntry],
    moves: &[ObjectMove],
    batch_has_enlists: bool,
    triggers: &mut Vec<PendingTrigger>,
) {
    if !batch_has_enlists {
        return;
    }
    let permanent = obj.id;
    for entry in events {
        let GameEvent::CreatureEnlisted {
            attacker,
            enlisted,
            player,
        } = &entry.event
        else {
            continue;
        };
        if *attacker != permanent {
            continue;
        }
        let source = source_at_event_with_moves(state, obj, moves, entry.seq, false);
        let n = times_triggered(state, TriggerEventKind::Any, permanent);
        if n == 0 {
            continue;
        }
        let enlisted_identity = state.event_object_identity(*enlisted, entry.seq);
        for _ in 0..n {
            triggers.push(PendingTrigger {
                text: TextChangeMap::IDENTITY,
                source_version: Some(source.version),
                related_objects: None,
                grantor: None,
                event_subject: None,
                event_ability: None,
                event_object_identity: enlisted_identity,
                counter_source_version: None,
                event_mana: None,
                event_mana_value: None,
                event_departure: None,
                event_amount: None,
                event_player: Some(*player),
                source: source.id,
                ability_index: AbilityRef::SYNTHETIC,
                controller: source.controller,
                timestamp: source.timestamp,
                event_object: Some(*enlisted),
                implicit_target: None,
                abilities: None,
                synthetic_effects: Some(ENLIST_EFFECTS),
                once_per_turn: false,
                synthetic_target: None,
                synthetic_condition: None,
                chosen_mode: None,
                in_chosen_order: false,
            });
        }
    }
}

/// CR 702.116a–b: Myriad — "Whenever this creature attacks, for each opponent
/// other than defending player, you may create a token that's a copy of this
/// creature that's tapped and attacking that player or a planeswalker they
/// control. If one or more tokens are created this way, exile the tokens at end
/// of combat."
fn collect_myriad(
    state: &GameState,
    source: &GameObject,
    defending: Defender,
    seq: u64,
    multiplier: u32,
    triggers: &mut Vec<PendingTrigger>,
) {
    let defending_player = crate::combat::blocking_player(state, defending);
    let has_other_opponent = state.players.iter().any(|p| {
        !p.has_lost()
            && state.is_opponent(p.id, source.controller)
            && Some(p.id) != defending_player
    });
    if has_other_opponent {
        let n = crate::layers::keyword_instances(state, source, KeywordSet::MYRIAD)
            .saturating_mul(multiplier);
        push_synthetic(
            state,
            source,
            seq,
            n,
            MYRIAD_EFFECTS,
            None,
            defending_player,
            triggers,
        );
    }
}

fn has_most_or_tied_life(state: &GameState, attacked: baylee_core::ids::PlayerId) -> bool {
    let Some(target_player) = state
        .players
        .get(usize::from(attacked.get()))
        .filter(|p| !p.has_lost())
    else {
        return false;
    };
    state
        .players
        .iter()
        .filter(|p| !p.has_lost())
        .map(|p| p.life)
        .max()
        .is_some_and(|best| target_player.life == best)
}

fn attacked_with_greater_power(
    state: &GameState,
    source: &GameObject,
    events: &[JournalEntry],
    seq: u64,
    batch_has_zone_changes: bool,
) -> bool {
    let Some(this_power) = source.characteristics().power else {
        return false;
    };
    events.iter().any(|other_entry| {
        let GameEvent::BecameAttacker { object: other, .. } = &other_entry.event else {
            return false;
        };
        if *other == source.id {
            return false;
        }
        let Some(current) = state.object_or_departed(*other) else {
            return false;
        };
        let other_moves = object_moves(events, batch_has_zone_changes, *other);
        let at_attack = source_at_event_with_moves(state, current, &other_moves, seq, false);
        at_attack
            .characteristics()
            .power
            .is_some_and(|power| power > this_power)
    })
}

#[allow(clippy::too_many_arguments)]
fn push_synthetic(
    state: &GameState,
    source: &GameObject,
    seq: u64,
    count: u32,
    effects: &'static [Effect],
    target: Option<TargetReq>,
    event_player: Option<baylee_core::ids::PlayerId>,
    triggers: &mut Vec<PendingTrigger>,
) {
    let identity = state.event_object_identity(source.id, seq);
    for _ in 0..count {
        triggers.push(PendingTrigger {
            text: TextChangeMap::IDENTITY,
            source_version: Some(source.version),
            related_objects: None,
            grantor: None,
            event_subject: None,
            event_ability: None,
            event_object_identity: identity,
            counter_source_version: None,
            event_mana: None,
            event_mana_value: None,
            event_departure: None,
            event_amount: None,
            event_player,
            source: source.id,
            ability_index: AbilityRef::SYNTHETIC,
            controller: source.controller,
            timestamp: source.timestamp,
            event_object: Some(source.id),
            implicit_target: None,
            abilities: None,
            synthetic_effects: Some(effects),
            once_per_turn: false,
            synthetic_target: target,
            synthetic_condition: None,
            chosen_mode: None,
            in_chosen_order: false,
        });
    }
}
