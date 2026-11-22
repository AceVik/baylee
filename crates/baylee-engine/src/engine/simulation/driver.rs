//! The driver half of a fork: which continuations it may carry, the copy
//! itself, and the seat settings it does not.
//!
//! Both functions that read every [`Engine`] field destructure it whole, as
//! the decision checkpoint does (`engine/checkpoint.rs`): a field added to
//! the engine does not compile here until somebody has said whether a fork
//! may carry it.

use super::ForkRefusal;
use crate::choice::Pending;
use crate::engine::Engine;
use crate::state::CardLookup;
use baylee_core::ids::PlayerId;

/// Whether the engine stands at an ordinary priority decision, or at a
/// combat declaration (declare attackers, declare blockers), whose
/// continuation names no hidden card.
///
/// The latches of the priority round and of the turn (passes, priority
/// holder, a regrant owed, combat declaration progress, the cleanup stage,
/// loyalty used this turn) are carried as they stand: they are the reason a
/// fork copies the engine instead of rebuilding a position. Everything that
/// suspends an operation half done is refused, because its continuation
/// may hold a hidden card's id, a library top, or a choice offered from a
/// hand, and a deal under it would leave it pointing at a different card.
///
/// A declaration is a turn-based action asked at the start of its step
/// (CR 508.1, CR 509.1) with nothing suspended: what it offers — the
/// creatures that may attack, the defenders, the blocks each blocker may
/// make — is the battlefield's and the players', all public, and names no
/// hidden card. An attack cost or a "can't attack unless" reads public
/// permanents too (CR 508.1c, 508.1d).
///
/// # Errors
/// [`ForkRefusal::OpeningHands`], [`ForkRefusal::Continuation`],
/// [`ForkRefusal::LoopWatch`] or [`ForkRefusal::NotPriority`].
#[allow(clippy::too_many_lines)] // one line per engine field: the list is the guard
pub(super) fn check_settled<L: CardLookup, D: crate::engine::LoopDetection>(
    engine: &Engine<L, D>,
) -> Result<(), ForkRefusal> {
    let Engine {
        lookup: _,
        state,
        pending,
        // Configuration, not position.
        house_rules: _,
        // The priority round's latches: carried, never reset.
        passes: _,
        priority_holder: _,
        resolve_next: _,
        regrant_priority: _,
        mulligans,
        combat_declared: _,
        cleanup: _,
        // Planeswalkers on the battlefield, public.
        loyalty_used_this_turn: _,
        awaiting_answer,
        resolution,
        effect_plays,
        // Who controls whose turn (CR 724), public.
        player_control: _,
        combat_damage,
        mana_window,
        attack_sacrifice,
        trigger_scan_seq,
        pending_plan,
        library_action_tops,
        loyalty_player_choice,
        loyalty_paid,
        activation_target_choice,
        activation_payment,
        activation_mode,
        activation_target_players,
        activation_cost_choices,
        activation_second_targets,
        activation_target_references,
        activation_targets_answered,
        activation_x,
        activation_graveyard,
        activation_phyrexian,
        activation_either_branches,
        activation_counter_removals,
        activation_pending_counter_object,
        activation_division,
        activating_abilities,
        activating_grant,
        // Replaced by `forget_settings`.
        capabilities: _,
        // Arrivals onto the battlefield, public; see below.
        entry_scan_seq: _,
        entry_questions,
        delayed_queue,
        // Keyed by stack objects, public.
        synthetic_fx: _,
        cast_wizard,
        trigger_queue,
        agreed_draw: _,
        // Replaced for every other seat by `forget_settings`.
        automation: _,
        loop_control,
    } = engine;
    if mulligans.is_some() {
        return Err(ForkRefusal::OpeningHands);
    }
    let suspended = [
        (resolution.is_some(), "resolution"),
        (mana_window.is_some(), "mana_window"),
        (attack_sacrifice.is_some(), "attack_sacrifice"),
        (cast_wizard.is_some(), "cast_wizard"),
        (pending_plan.is_some(), "pending_plan"),
        (combat_damage.is_some(), "combat_damage"),
        (!effect_plays.is_empty(), "effect_plays"),
        (!trigger_queue.is_empty(), "trigger_queue"),
        (!entry_questions.is_empty(), "entry_questions"),
        (!delayed_queue.is_empty(), "delayed_queue"),
        (library_action_tops.is_some(), "library_action_tops"),
        (loyalty_player_choice.is_some(), "loyalty_player_choice"),
        (loyalty_paid.is_some(), "loyalty_paid"),
        (activation_payment.is_some(), "activation_payment"),
        (activation_mode.is_some(), "activation_mode"),
        (
            !activation_target_players.is_empty(),
            "activation_target_players",
        ),
        (
            !activation_cost_choices.is_empty(),
            "activation_cost_choices",
        ),
        (
            activation_second_targets.is_some(),
            "activation_second_targets",
        ),
        (
            activation_target_choice.controller.is_some()
                || activation_target_choice.opponent.is_some()
                || !activation_target_choice.second_players.is_empty()
                || activation_target_choice.per_player.is_some()
                || !activation_target_choice.first_groups.is_empty()
                || !activation_target_choice.second_groups.is_empty(),
            "activation_target_choice",
        ),
        (
            !activation_target_references.first.is_empty()
                || !activation_target_references.second.is_empty(),
            "activation_target_references",
        ),
        (*activation_targets_answered, "activation_targets_answered"),
        (activation_x.is_some(), "activation_x"),
        (activation_graveyard.is_some(), "activation_graveyard"),
        (!activation_phyrexian.is_empty(), "activation_phyrexian"),
        (
            !activation_either_branches.is_empty(),
            "activation_either_branches",
        ),
        (
            !activation_counter_removals.is_empty() || activation_pending_counter_object.is_some(),
            "activation_counter_removals",
        ),
        (!activation_division.is_empty(), "activation_division"),
        (activating_abilities.is_some(), "activating_abilities"),
        (activating_grant.is_some(), "activating_grant"),
        // Every event up to now has been read for triggers: the journal's
        // past is rewritten in the fork, and an unread entry would be read
        // there as the rewritten one. The entry scan needs no such check:
        // it reads arrivals onto the battlefield, which are public and
        // which the rewrite leaves alone.
        (
            *trigger_scan_seq != state.journal.last_seq(),
            "trigger_scan_seq",
        ),
    ];
    if let Some(&(_, field)) = suspended.iter().find(|(held, _)| *held) {
        return Err(ForkRefusal::Continuation(field));
    }
    // The state's own continuations are `GameState::audit_fork`'s.
    if !loop_control.allows_fork() {
        return Err(ForkRefusal::LoopWatch);
    }
    if !*awaiting_answer
        || !matches!(
            pending,
            Pending::Priority { .. }
                | Pending::ChooseAttackers { .. }
                | Pending::ChooseBlockers { .. }
        )
    {
        return Err(ForkRefusal::NotPriority);
    }
    Ok(())
}

/// A field-for-field copy of the engine.
///
/// Deliberately not `impl Clone for Engine`: an undisguised copy of a real
/// game is the real game's hidden cards and random stream, and the one
/// thing that may be done with it is to disguise it before anybody sees it
/// (`Engine::fork_for`).
#[allow(clippy::too_many_lines)] // one line per engine field: the list is the guard
pub(super) fn duplicate<L: CardLookup + Clone, D: crate::engine::LoopDetection>(
    engine: &Engine<L, D>,
) -> Engine<L, D> {
    let Engine {
        lookup,
        state,
        pending,
        house_rules,
        passes,
        priority_holder,
        resolve_next,
        regrant_priority,
        mulligans,
        combat_declared,
        cleanup,
        loyalty_used_this_turn,
        awaiting_answer,
        resolution,
        effect_plays,
        player_control,
        combat_damage,
        mana_window,
        attack_sacrifice,
        trigger_scan_seq,
        pending_plan,
        library_action_tops,
        loyalty_player_choice,
        loyalty_paid,
        activation_target_choice,
        activation_payment,
        activation_mode,
        activation_target_players,
        activation_cost_choices,
        activation_second_targets,
        activation_target_references,
        activation_targets_answered,
        activation_x,
        activation_graveyard,
        activation_phyrexian,
        activation_either_branches,
        activation_counter_removals,
        activation_pending_counter_object,
        activation_division,
        activating_abilities,
        activating_grant,
        capabilities,
        entry_scan_seq,
        entry_questions,
        delayed_queue,
        synthetic_fx,
        cast_wizard,
        trigger_queue,
        agreed_draw,
        automation,
        loop_control,
    } = engine;
    Engine {
        lookup: lookup.clone(),
        state: state.clone(),
        pending: pending.clone(),
        house_rules: house_rules.clone(),
        passes: *passes,
        priority_holder: *priority_holder,
        resolve_next: *resolve_next,
        regrant_priority: *regrant_priority,
        mulligans: mulligans.clone(),
        combat_declared: *combat_declared,
        cleanup: *cleanup,
        loyalty_used_this_turn: loyalty_used_this_turn.clone(),
        awaiting_answer: *awaiting_answer,
        resolution: resolution.clone(),
        effect_plays: effect_plays.clone(),
        player_control: player_control.clone(),
        combat_damage: combat_damage.clone(),
        mana_window: mana_window.clone(),
        attack_sacrifice: attack_sacrifice.clone(),
        trigger_scan_seq: *trigger_scan_seq,
        pending_plan: pending_plan.clone(),
        library_action_tops: library_action_tops.clone(),
        loyalty_player_choice: *loyalty_player_choice,
        loyalty_paid: loyalty_paid.clone(),
        activation_target_choice: activation_target_choice.clone(),
        activation_payment: activation_payment.clone(),
        activation_mode: *activation_mode,
        activation_target_players: activation_target_players.clone(),
        activation_cost_choices: activation_cost_choices.clone(),
        activation_second_targets: activation_second_targets.clone(),
        activation_target_references: activation_target_references.clone(),
        activation_targets_answered: *activation_targets_answered,
        activation_x: *activation_x,
        activation_graveyard: *activation_graveyard,
        activation_phyrexian: activation_phyrexian.clone(),
        activation_either_branches: activation_either_branches.clone(),
        activation_counter_removals: activation_counter_removals.clone(),
        activation_pending_counter_object: *activation_pending_counter_object,
        activation_division: activation_division.clone(),
        activating_abilities: activating_abilities.clone(),
        activating_grant: *activating_grant,
        capabilities: capabilities.clone(),
        entry_scan_seq: *entry_scan_seq,
        entry_questions: entry_questions.clone(),
        delayed_queue: delayed_queue.clone(),
        synthetic_fx: synthetic_fx.clone(),
        cast_wizard: cast_wizard.clone(),
        trigger_queue: trigger_queue.clone(),
        agreed_draw: *agreed_draw,
        automation: automation.clone(),
        loop_control: loop_control.clone(),
    }
}

/// Drops what a seat's chair says rather than what its board says.
///
/// Capabilities are the host's trust in a seat (developer commands, seeing
/// hidden cards) and a simulation has no host. Another seat's standing
/// answers and priority holds are that seat's private setting, which the
/// forking seat has no business simulating it by (`docs/proposals/
/// lookahead-design.md` §Q1); its own are its own.
pub(super) fn forget_settings<L: CardLookup, D: crate::engine::LoopDetection>(
    fork: &mut Engine<L, D>,
    seat: PlayerId,
) {
    for capabilities in &mut fork.capabilities {
        *capabilities = baylee_core::preset::SeatCapabilities::default();
    }
    for (index, automation) in fork.automation.iter_mut().enumerate() {
        if index != usize::from(seat.get()) {
            *automation = crate::choice::SeatAutomation::default();
        }
    }
}

/// A fair search assumes no unobserved opponent consumables. Concealed supplies
/// always create the same two source objects, including empty stock: resetting
/// only their counters therefore preserves public IDs and reveals no capacity.
/// Effects already paid for remain on the stack, with their ordinary outcomes.
pub(super) fn forget_inventory<L: CardLookup, D: crate::engine::LoopDetection>(
    fork: &mut Engine<L, D>,
    seat: PlayerId,
) {
    use crate::{event::GameEvent, object::CounterKind, zone::ZoneLocation};
    if !fork
        .house_rules
        .supplies
        .iter()
        .any(|supply| supply.concealed)
    {
        return;
    }
    let mut hidden = Vec::new();
    for index in 0..fork.state.players.len() {
        let owner = PlayerId::new(u8::try_from(index).expect("validated seat count"));
        if fork.may_inspect_private(seat, owner) {
            continue;
        }
        let zone = ZoneLocation::Command(owner);
        for position in 0..fork.state.zones.list(zone).len() {
            let id = fork.state.zones.list(zone)[position];
            if fork
                .state
                .object(id)
                .is_some_and(crate::object::GameObject::private_inventory)
            {
                hidden.push(id);
                fork.state
                    .object_mut(id)
                    .expect("listed source")
                    .counters
                    .set(CounterKind::Charge, 0);
            }
        }
        for supply in &mut fork.house_rules.supplies {
            if supply.seat == owner.get() && supply.concealed {
                supply.healing = 0;
                supply.mana = 0;
            }
        }
    }
    if hidden.is_empty() {
        return;
    }
    fork.state.journal.rewrite(|entry| {
        if let GameEvent::CounterChanged {
            object, old, new, ..
        } = &mut entry.event
            && hidden.contains(object)
        {
            *old = 0;
            *new = 0;
        }
    });
}
