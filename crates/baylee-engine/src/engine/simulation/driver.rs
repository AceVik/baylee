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

/// Whether the engine stands at an ordinary priority decision whose
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
/// # Errors
/// [`ForkRefusal::OpeningHands`], [`ForkRefusal::Continuation`],
/// [`ForkRefusal::LoopWatch`] or [`ForkRefusal::NotPriority`].
#[allow(clippy::too_many_lines)] // one line per engine field: the list is the guard
pub(super) fn check_settled<L: CardLookup>(engine: &Engine<L>) -> Result<(), ForkRefusal> {
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
        // Who controls whose turn (CR 722), public.
        player_control: _,
        combat_damage,
        mana_window,
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
        activating_abilities,
        activating_grant,
        // Replaced by `forget_settings`.
        capabilities: _,
        // Arrivals onto the battlefield, public; see below.
        entry_scan_seq: _,
        entry_questions,
        delayed_queue,
        upkeep_payments,
        // Keyed by stack objects, public.
        synthetic_fx: _,
        cast_wizard,
        trigger_queue,
        agreed_draw: _,
        // Replaced for every other seat by `forget_settings`.
        automation: _,
        breaking_loop,
        action_loops,
        loops_broken: _,
    } = engine;
    if mulligans.is_some() {
        return Err(ForkRefusal::OpeningHands);
    }
    let suspended = [
        (resolution.is_some(), "resolution"),
        (mana_window.is_some(), "mana_window"),
        (cast_wizard.is_some(), "cast_wizard"),
        (pending_plan.is_some(), "pending_plan"),
        (combat_damage.is_some(), "combat_damage"),
        (!effect_plays.is_empty(), "effect_plays"),
        (!trigger_queue.is_empty(), "trigger_queue"),
        (!entry_questions.is_empty(), "entry_questions"),
        (!delayed_queue.is_empty(), "delayed_queue"),
        (!upkeep_payments.is_empty(), "upkeep_payments"),
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
                || !activation_target_choice.second_players.is_empty(),
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
    if *breaking_loop || action_loops.holds_signature() {
        return Err(ForkRefusal::LoopWatch);
    }
    if !*awaiting_answer || !matches!(pending, Pending::Priority { .. }) {
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
pub(super) fn duplicate<L: CardLookup + Clone>(engine: &Engine<L>) -> Engine<L> {
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
        activating_abilities,
        activating_grant,
        capabilities,
        entry_scan_seq,
        entry_questions,
        delayed_queue,
        upkeep_payments,
        synthetic_fx,
        cast_wizard,
        trigger_queue,
        agreed_draw,
        automation,
        breaking_loop,
        action_loops,
        loops_broken,
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
        activating_abilities: activating_abilities.clone(),
        activating_grant: *activating_grant,
        capabilities: capabilities.clone(),
        entry_scan_seq: *entry_scan_seq,
        entry_questions: entry_questions.clone(),
        delayed_queue: delayed_queue.clone(),
        upkeep_payments: upkeep_payments.clone(),
        synthetic_fx: synthetic_fx.clone(),
        cast_wizard: cast_wizard.clone(),
        trigger_queue: trigger_queue.clone(),
        agreed_draw: *agreed_draw,
        automation: automation.clone(),
        breaking_loop: *breaking_loop,
        action_loops: action_loops.clone(),
        loops_broken: *loops_broken,
    }
}

/// Drops what a seat's chair says rather than what its board says.
///
/// Capabilities are the host's trust in a seat (developer commands, seeing
/// hidden cards) and a simulation has no host. Another seat's standing
/// answers and priority holds are that seat's private setting, which the
/// forking seat has no business simulating it by (`docs/proposals/
/// lookahead-design.md` §Q1); its own are its own.
pub(super) fn forget_settings<L: CardLookup>(fork: &mut Engine<L>, seat: PlayerId) {
    for capabilities in &mut fork.capabilities {
        *capabilities = baylee_core::preset::SeatCapabilities::default();
    }
    for (index, automation) in fork.automation.iter_mut().enumerate() {
        if index != usize::from(seat.get()) {
            *automation = crate::choice::SeatAutomation::default();
        }
    }
}
