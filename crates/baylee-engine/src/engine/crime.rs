//! Commit a crime (`CR 700.13`) and Expend `N` (`CR 700.14`) hooks at stack-entry doors.

use super::{Engine, LoopDetection};
use crate::event::GameEvent;
use crate::object::ObjectKind;
use crate::state::{CardLookup, GameState};
use crate::zone::{Zone, ZoneLocation};
use baylee_cards_dsl::{AbilityDef, Modifier, TargetSpec, Trigger};
use baylee_core::ids::{ObjectId, PlayerId};

impl<L: CardLookup, D: LoopDetection> Engine<L, D> {
    /// Checks whether `stack_id` (just cast, activated, or put on the stack as a
    /// triggered ability by `actor`) commits a crime (`CR 700.13`), recording
    /// per-turn crime state and emitting [`GameEvent::CommittedCrime`] when
    /// observed.
    pub(super) fn note_crime_if_any(&mut self, actor: PlayerId, stack_id: ObjectId) {
        if !stack_object_commits_crime(&self.state, actor, stack_id) {
            return;
        }
        self.state.per_turn.note_crime(actor);
        if self.state.per_turn.crimes_committed.tracked || has_dynamic_crime_observer(&self.state) {
            self.state
                .journal
                .record(GameEvent::CommittedCrime { player: actor });
        }
    }

    /// Records mana `spent` to cast a spell by `player` (`CR 700.14`), updating
    /// per-turn expend totals and emitting [`GameEvent::ManaExpended`] when
    /// observed.
    pub(super) fn note_expend_if_any(&mut self, player: PlayerId, spent: u32) {
        if spent == 0 {
            return;
        }
        let (previous, total) = self.state.per_turn.record_mana_expended(player, spent);
        if self.state.per_turn.mana_expended_on_spells.tracked
            || has_dynamic_expend_observer(&self.state)
        {
            self.state.journal.record(GameEvent::ManaExpended {
                player,
                previous,
                total,
            });
        }
    }
}

fn stack_object_commits_crime(state: &GameState, actor: PlayerId, stack_id: ObjectId) -> bool {
    let Some(obj) = state.object(stack_id) else {
        return false;
    };
    let targets_opponent_player = obj
        .target_players
        .iter()
        .chain(obj.second_target_players().iter())
        .chain(
            obj.extra_target_instances()
                .iter()
                .flat_map(|inst| inst.players.iter()),
        )
        .chain(obj.chosen_player)
        .any(|p| state.is_opponent(actor, p));
    if targets_opponent_player {
        return true;
    }

    let first_is_real = obj.kind == ObjectKind::Spell
        || obj.target_req.is_some_and(|req| {
            !matches!(req.spec, TargetSpec::EventObject | TargetSpec::ThisObject)
        });
    let first_targets = if first_is_real {
        obj.targets.as_slice()
    } else {
        &[]
    };
    let second_targets = if obj.second_target_req().is_some() {
        obj.second_targets()
    } else {
        &[]
    };

    first_targets
        .iter()
        .chain(second_targets.iter())
        .chain(
            obj.extra_target_instances()
                .iter()
                .flat_map(|inst| inst.targets.iter()),
        )
        .any(|&tid| is_crime_object_target(state, actor, stack_id, tid))
}

fn is_crime_object_target(
    state: &GameState,
    actor: PlayerId,
    stack_id: ObjectId,
    tid: ObjectId,
) -> bool {
    let Some(target) = state.object(tid) else {
        return false;
    };
    if target.zone == Zone::Graveyard {
        return target.is_card()
            && target
                .zone_owner
                .is_some_and(|owner| state.is_opponent(actor, owner));
    }
    if !matches!(target.zone, Zone::Battlefield | Zone::Stack) {
        return false;
    }
    let _ = stack_id;
    state.is_opponent(actor, target.controller)
}

fn has_dynamic_crime_observer(state: &GameState) -> bool {
    state.delayed.iter().any(|w| {
        matches!(
            w.when,
            crate::state::DelayedWhen::OnEvent {
                trigger: Trigger::CommittedCrime { .. },
                ..
            }
        )
    }) || state.effects.iter().any(|fx| match fx.modifier {
        Modifier::GrantTriggered {
            trigger: Trigger::CommittedCrime { .. },
            ..
        } => true,
        Modifier::GrantTriggeredDefinition(def) => crate::state::reads::ability_reads_crime(def),
        _ => false,
    }) || any_own_ability(state, |a| {
        matches!(
            a,
            AbilityDef::Triggered {
                trigger: Trigger::CommittedCrime { .. },
                ..
            } | AbilityDef::ModalTriggered {
                trigger: Trigger::CommittedCrime { .. },
                ..
            }
        )
    })
}

fn has_dynamic_expend_observer(state: &GameState) -> bool {
    state.delayed.iter().any(|w| {
        matches!(
            w.when,
            crate::state::DelayedWhen::OnEvent {
                trigger: Trigger::Expended { .. },
                ..
            }
        )
    }) || state.effects.iter().any(|fx| match fx.modifier {
        Modifier::GrantTriggered {
            trigger: Trigger::Expended { .. },
            ..
        } => true,
        Modifier::GrantTriggeredDefinition(def) => crate::state::reads::ability_reads_expend(def),
        _ => false,
    }) || any_own_ability(state, |a| {
        matches!(
            a,
            AbilityDef::Triggered {
                trigger: Trigger::Expended { .. },
                ..
            } | AbilityDef::ModalTriggered {
                trigger: Trigger::Expended { .. },
                ..
            }
        )
    })
}

fn any_own_ability(state: &GameState, pred: impl Fn(&AbilityDef) -> bool) -> bool {
    state
        .battlefield_seen()
        .chain(state.players.iter().flat_map(|p| {
            state
                .zones
                .list(ZoneLocation::Command(p.id))
                .iter()
                .copied()
        }))
        .filter_map(|id| state.object(id))
        .filter_map(|o| o.own_abilities.as_ref())
        .any(|bundle| bundle.entries().iter().any(|entry| pred(entry.definition)))
}
