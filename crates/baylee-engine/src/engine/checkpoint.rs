//! Full decision transaction for failures discovered during automatic resolution.
//!
//! Unlike an illegal UI answer, a numerical capacity failure can be discovered
//! after costs and nested effects ran. Restore the whole driver, not just the
//! payment pool. Exhaustive destructuring makes every new Engine field join it.
#[allow(clippy::wildcard_imports)]
use super::*;

#[allow(clippy::struct_excessive_bools)] // Exact snapshot of the driver's independent latches.
pub(super) struct Checkpoint {
    state: GameState,
    pending: Pending,
    house_rules: HouseRules,
    passes: u8,
    priority_holder: Option<PlayerId>,
    resolve_next: bool,
    regrant_priority: Option<PlayerId>,
    mulligans: Option<Vec<mulligan::SeatMulligan>>,
    combat_declared: CombatDeclared,
    cleanup: Cleanup,
    loyalty_used_this_turn: Vec<ObjectId>,
    awaiting_answer: bool,
    resolution: Option<Resolution>,
    effect_plays: Vec<effect_play::EffectPlay>,
    player_control: player_control::PlayerControl,
    combat_damage: Option<crate::damage::DamageWork>,
    mana_window: Option<PaymentWindow>,
    trigger_scan_seq: u64,
    pending_plan: Option<PlanKind>,
    library_action_tops: Option<Vec<Option<ObjectId>>>,
    loyalty_player_choice: Option<PlayerId>,
    activation_target_players: Vec<PlayerId>,
    activation_cost_choices: Vec<ObjectId>,
    activation_second_targets: Option<SmallVec<[ObjectId; 1]>>,
    activation_target_references: crate::sources::TargetReferences,
    activation_targets_answered: bool,
    activation_x: Option<u32>,
    activation_graveyard: Option<PlayerId>,
    activation_phyrexian: Vec<bool>,
    activating_abilities: Option<(ObjectId, crate::object::AbilityList)>,
    capabilities: Vec<baylee_core::preset::SeatCapabilities>,
    entry_scan_seq: u64,
    entry_questions: VecDeque<(ObjectId, PlayerId, progress::EntryAsk)>,
    delayed_queue: VecDeque<(PlayerId, crate::state::DelayedAction)>,
    upkeep_payments: VecDeque<crate::state::DelayedAction>,
    synthetic_fx: rustc_hash::FxHashMap<ObjectId, &'static [baylee_cards_dsl::Effect]>,
    cast_wizard: Option<cast_wizard::CastWizard>,
    trigger_queue: VecDeque<trigger::PendingTrigger>,
    agreed_draw: bool,
    automation: Vec<crate::choice::SeatAutomation>,
    breaking_loop: bool,
    action_loops: crate::loops::LoopWatch,
    loops_broken: u32,
}

impl Checkpoint {
    pub(super) fn capture<L: CardLookup>(engine: &Engine<L>) -> Self {
        let Engine {
            lookup: _,
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
            activation_target_players,
            activation_cost_choices,
            activation_second_targets,
            activation_target_references,
            activation_targets_answered,
            activation_x,
            activation_graveyard,
            activation_phyrexian,
            activating_abilities,
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
        Self {
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
            activation_target_players: activation_target_players.clone(),
            activation_cost_choices: activation_cost_choices.clone(),
            activation_second_targets: activation_second_targets.clone(),
            activation_target_references: activation_target_references.clone(),
            activation_targets_answered: *activation_targets_answered,
            activation_x: *activation_x,
            activation_graveyard: *activation_graveyard,
            activation_phyrexian: activation_phyrexian.clone(),
            activating_abilities: activating_abilities.clone(),
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

    pub(super) fn restore<L: CardLookup>(self, engine: &mut Engine<L>) {
        let Engine {
            lookup: _,
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
            activation_target_players,
            activation_cost_choices,
            activation_second_targets,
            activation_target_references,
            activation_targets_answered,
            activation_x,
            activation_graveyard,
            activation_phyrexian,
            activating_abilities,
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
        *state = self.state;
        *pending = self.pending;
        *house_rules = self.house_rules;
        *passes = self.passes;
        *priority_holder = self.priority_holder;
        *resolve_next = self.resolve_next;
        *regrant_priority = self.regrant_priority;
        *mulligans = self.mulligans;
        *combat_declared = self.combat_declared;
        *cleanup = self.cleanup;
        *loyalty_used_this_turn = self.loyalty_used_this_turn;
        *awaiting_answer = self.awaiting_answer;
        *resolution = self.resolution;
        *effect_plays = self.effect_plays;
        *player_control = self.player_control;
        *combat_damage = self.combat_damage;
        *mana_window = self.mana_window;
        *trigger_scan_seq = self.trigger_scan_seq;
        *pending_plan = self.pending_plan;
        *library_action_tops = self.library_action_tops;
        *loyalty_player_choice = self.loyalty_player_choice;
        *activation_target_players = self.activation_target_players;
        *activation_cost_choices = self.activation_cost_choices;
        *activation_second_targets = self.activation_second_targets;
        *activation_target_references = self.activation_target_references;
        *activation_targets_answered = self.activation_targets_answered;
        *activation_x = self.activation_x;
        *activation_graveyard = self.activation_graveyard;
        *activation_phyrexian = self.activation_phyrexian;
        *activating_abilities = self.activating_abilities;
        *capabilities = self.capabilities;
        *entry_scan_seq = self.entry_scan_seq;
        *entry_questions = self.entry_questions;
        *delayed_queue = self.delayed_queue;
        *upkeep_payments = self.upkeep_payments;
        *synthetic_fx = self.synthetic_fx;
        *cast_wizard = self.cast_wizard;
        *trigger_queue = self.trigger_queue;
        *agreed_draw = self.agreed_draw;
        *automation = self.automation;
        *breaking_loop = self.breaking_loop;
        *action_loops = self.action_loops;
        *loops_broken = self.loops_broken;
    }
}
