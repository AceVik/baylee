//! Announcing damage and counter distributions before resolution (CR 601.2d).

use super::{CardLookup, Engine, ObjectId, Pending, PlanKind, PlayerId, SmallVec};
use crate::choice::{NumberPrompt, TargetRef};
use crate::state::GameState;
use crate::text_changes::RuleContext;
use baylee_cards_dsl::{Amount, CounterKind, Effect};

/// What an announced CR 601.2d distribution is dividing across targets.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub(crate) enum DividedKind {
    Damage,
    Counters(CounterKind),
}

impl DividedKind {
    pub(crate) fn prompt(
        self,
        target: TargetRef,
        index: usize,
        of: usize,
        left: u32,
    ) -> NumberPrompt {
        let index = u8::try_from(index).unwrap_or(u8::MAX);
        let of = u8::try_from(of).unwrap_or(u8::MAX);
        match self {
            Self::Damage => NumberPrompt::DivideDamage {
                target,
                index,
                of,
                left,
            },
            Self::Counters(kind) => NumberPrompt::DivideCounters {
                kind,
                target,
                index,
                of,
                left,
            },
        }
    }
}

/// Evaluates an announced counter-distribution [`Amount`], reading departed
/// source LKI (`CR 608.2h`, `CR 603.10a`) for `CountersOnSource` and `SourcePower`.
pub(crate) fn eval_division_amount(
    amount: &Amount,
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
    source_version: Option<u32>,
    x: u32,
) -> u32 {
    match amount {
        Amount::CountersOnSource(kind) => state
            .damage_source(context.source, source_version)
            .or_else(|| state.object(context.source))
            .map_or(0, |o| u32::from(o.counters.get(*kind))),
        Amount::SourcePower => state
            .damage_source(context.source, source_version)
            .or_else(|| state.object(context.source))
            .and_then(|o| o.characteristics().power)
            .map_or(0, |p| p.max(0) as u32),
        Amount::Plus { base, offset } => {
            eval_division_amount(base, state, you, context, source_version, x)
                .saturating_add(*offset)
        }
        Amount::SaturatingSub { base, subtract } => {
            eval_division_amount(base, state, you, context, source_version, x)
                .saturating_sub(*subtract)
        }
        _ => crate::eval::amount_with_context(amount, state, you, context, Some(x)),
    }
}

/// Inspects top-level `effects` for a CR 601.2d divided damage or counter instruction.
pub(crate) fn effects_divided_spec(
    effects: &[Effect],
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
    source_version: Option<u32>,
    x: u32,
) -> Option<(DividedKind, u32)> {
    effects.iter().find_map(|effect| match effect {
        Effect::DealDamageDivided { amount } => Some((DividedKind::Damage, amount.total(x))),
        Effect::PutCountersDivided { kind, amount } => Some((
            DividedKind::Counters(*kind),
            eval_division_amount(amount, state, you, context, source_version, x),
        )),
        _ => None,
    })
}

impl<L: CardLookup, D: crate::engine::LoopDetection> Engine<L, D> {
    /// Asks how the triggered ability just put on the stack divides its
    /// damage or counters (`CR 601.2d`, `CR 603.3d`).
    pub(crate) fn ask_trigger_division(&mut self) -> bool {
        let Some(top) = self
            .state
            .zones
            .list(crate::zone::ZoneLocation::Stack)
            .last()
            .copied()
        else {
            return false;
        };
        let Some((kind, total)) = self.stack_divided_spec(top) else {
            return false;
        };
        let Some(obj) = self.state.object(top) else {
            return false;
        };
        let (controller, targets) = (obj.controller, obj.targets.clone());
        let state = &mut self.state;
        let on_stack = state.zones.list(crate::zone::ZoneLocation::Stack);
        state.divided.retain(|(id, _)| on_stack.contains(id));
        if targets.is_empty() || total == 0 {
            return false;
        }
        self.ask_division(controller, top, kind, targets, Vec::new(), total)
    }

    /// Asks the next target's share of a trigger division, or assigns the
    /// remainder to the final target and records the division.
    pub(super) fn ask_division(
        &mut self,
        controller: PlayerId,
        on_stack: ObjectId,
        kind: DividedKind,
        targets: SmallVec<[ObjectId; 2]>,
        mut shares: Vec<u32>,
        total: u32,
    ) -> bool {
        let given: u32 = shares.iter().sum();
        let left = total.saturating_sub(given);
        let asked = shares.len();
        if asked + 1 >= targets.len() {
            shares.push(left);
            self.state.capture_source_references();
            let division = shares
                .into_iter()
                .enumerate()
                .map(|(index, share)| {
                    (
                        TargetRef::Object(
                            self.state
                                .recorded_target_reference(
                                    on_stack,
                                    false,
                                    u32::try_from(index).expect("target slot"),
                                )
                                .expect("announced target"),
                        ),
                        share,
                    )
                })
                .collect();
            self.state.divided.push((on_stack, division));
            return false;
        }
        let after = u32::try_from(targets.len() - asked - 1).unwrap_or(u32::MAX);
        let max = left.saturating_sub(after).max(1);
        let reason = kind.prompt(
            TargetRef::Object(
                self.state
                    .source_identity(targets[asked])
                    .expect("announced target"),
            ),
            asked,
            targets.len(),
            left,
        );
        self.pending_plan = Some(PlanKind::DivideDamage {
            kind,
            on_stack,
            targets,
            shares,
            total,
        });
        self.pending = Pending::ChooseNumber {
            player: controller,
            min: 1,
            max,
            reason,
        };
        self.awaiting_answer = true;
        true
    }

    /// Asks the next share of an activated or loyalty ability's CR 601.2d
    /// division before costs are paid, returning `true` when `Pending` is set.
    pub(crate) fn ask_activation_division(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        effects: &[Effect],
        targets: SmallVec<[ObjectId; 2]>,
        target_players: Vec<PlayerId>,
    ) -> bool {
        let context = self.ability_rule_context(source, ability_index);
        let source_version = self.state.object(source).map(|o| o.version);
        let x = self.activation_x.unwrap_or(0);
        let Some((kind, total)) =
            effects_divided_spec(effects, &self.state, player, context, source_version, x)
        else {
            return false;
        };
        let mut refs: Vec<TargetRef> = targets
            .iter()
            .filter_map(|&id| self.state.source_identity(id).map(TargetRef::Object))
            .collect();
        refs.extend(target_players.iter().copied().map(TargetRef::Player));
        if let Some(p) = self
            .loyalty_player_choice
            .filter(|p| !target_players.contains(p))
        {
            refs.push(TargetRef::Player(p));
        }
        if refs.is_empty() || total == 0 {
            return false;
        }
        let asked = self.activation_division.len();
        let left = total.saturating_sub(self.activation_division.iter().sum());
        if asked + 1 < refs.len() {
            let reserved = u32::try_from(refs.len() - asked - 1).unwrap_or(u32::MAX);
            let max = left.saturating_sub(reserved).max(1);
            let reason = kind.prompt(refs[asked], asked, refs.len(), left);
            self.pending_plan = Some(PlanKind::DivideActivation {
                source,
                ability_index,
                targets,
                target_players,
            });
            self.pending = Pending::ChooseNumber {
                player,
                min: 1,
                max,
                reason,
            };
            self.awaiting_answer = true;
            return true;
        }
        if asked < refs.len() {
            self.activation_division.push(left);
        }
        false
    }

    /// Writes any announced `activation_division` shares onto `state.divided`
    /// for the stack ability `on_stack`.
    pub(crate) fn record_activation_division(
        &mut self,
        on_stack: ObjectId,
        target_players: &[PlayerId],
    ) {
        let shares = std::mem::take(&mut self.activation_division);
        if shares.is_empty() {
            return;
        }
        let on_stack_list = self.state.zones.list(crate::zone::ZoneLocation::Stack);
        self.state
            .divided
            .retain(|(id, _)| on_stack_list.contains(id));
        self.state.capture_source_references();
        let obj_len = self.state.object(on_stack).map_or(0, |o| o.targets.len());
        let mut refs: Vec<TargetRef> = (0..obj_len)
            .filter_map(|i| {
                self.state
                    .recorded_target_reference(on_stack, false, u32::try_from(i).ok()?)
                    .map(TargetRef::Object)
            })
            .collect();
        refs.extend(target_players.iter().copied().map(TargetRef::Player));
        if let Some(chosen) = self
            .state
            .object(on_stack)
            .and_then(|o| o.chosen_player)
            .filter(|p| !target_players.contains(p))
        {
            refs.push(TargetRef::Player(chosen));
        }
        let division: Vec<(TargetRef, u32)> = refs.into_iter().zip(shares).collect();
        if !division.is_empty() {
            self.state.divided.push((on_stack, division));
        }
    }
}
