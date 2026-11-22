//! Engine Wave 14f resolution operations:
//! - [`Effect::ChooseNumberEx`] (`CR 107.1`, `CR 107.14`, `CR 118.3`, `CR 119.4`,
//!   `CR 119.8`, `CR 608.2d`) + [`Amount::ResolutionChosenNumber`]
//! - [`Effect::MoveAttachment`] (`CR 301.5c`, `CR 301.6`, `CR 303.4d`, `CR 701.3a–b`)

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::object::Status;
use crate::zone::Zone;
use baylee_cards_dsl::{AbilityDef, Filter, NumberChoiceKind, VariablePayKind};
use baylee_core::{generated::subtypes, ids::DamageSourceRef, types::TypeSet};

/// Start [`Effect::ChooseNumberEx`]: evaluate dynamic `min` and `max`, clamp `max`
/// to affordable resources when `pay_cost` is `Some`, and suspend on
/// `Pending::ChooseNumber` whenever `min <= max`.
#[allow(clippy::too_many_arguments)]
pub(super) fn choose_number_ex_start(
    state: &mut GameState,
    res: &mut Resolution,
    chooser: PlayerRel,
    min: Amount,
    max: Amount,
    prompt: NumberChoiceKind,
    pay_cost: Option<VariablePayKind>,
    then: &'static [Effect],
) -> Option<Pending> {
    let you = res.controller;
    let chooser_seat = super::wave12a::apnap_seats_of(state, players_of(chooser, state, you, res))
        .into_iter()
        .find(|&p| !state.has_left(p))?;
    let raw_min = super::amount_for_player(&min, state, you, res, chooser_seat);
    let raw_max = super::amount_for_player(&max, state, you, res, chooser_seat);
    if raw_min > raw_max {
        return None;
    }
    // A resolving mana ability may already be running inside a payment
    // window. Preserve that window and offer payment from its current pool.
    if pay_cost == Some(VariablePayKind::GenericMana) && raw_max > 0 && !res.mana_ability {
        res.awaiting = Some(AwaitingOp::ManaForNumber {
            player: chooser_seat,
            min: raw_min,
            max: raw_max,
            then,
        });
        // The driver fills in the mana-only options before exposing this.
        return Some(Pending::Priority {
            player: chooser_seat,
            legal: Box::default(),
        });
    }
    let effective_max = match pay_cost {
        None => raw_max,
        Some(VariablePayKind::Energy) => {
            let avail = u32::from(state.players[usize::from(chooser_seat.get())].energy);
            raw_max.min(avail)
        }
        Some(VariablePayKind::GenericMana) => {
            generic_payment_max(state, res, chooser_seat, raw_max)
        }
        Some(VariablePayKind::Life) => {
            let avail = u32::try_from(state.life_payable(chooser_seat).max(0)).unwrap_or(0);
            raw_max.min(avail)
        }
    };
    if raw_min > effective_max {
        return None;
    }
    let reason = match prompt {
        NumberChoiceKind::Choice => crate::choice::NumberPrompt::Choice,
        NumberChoiceKind::EnergyPayment => crate::choice::NumberPrompt::EnergyPayment {
            candidates: effective_max,
        },
        NumberChoiceKind::ManaPayment => crate::choice::NumberPrompt::VariableManaPayment,
    };
    res.awaiting = Some(AwaitingOp::ChooseNumberEx {
        chooser: chooser_seat,
        pay_cost,
        then,
    });
    Some(Pending::ChooseNumber {
        player: chooser_seat,
        min: raw_min,
        max: effective_max,
        reason,
    })
}

/// Close the mana opportunity, then offer only prices the floating pool can pay.
pub(crate) fn finish_variable_mana(
    state: &mut GameState,
    res: &mut Resolution,
    player: PlayerId,
    min: u32,
    max: u32,
    then: &'static [Effect],
) -> (Flow, bool) {
    let max = generic_payment_max(state, res, player, max);
    if min > max || state.has_left(player) {
        res.awaiting = None;
        super::advance_program(res);
        return (super::run(state, res), false);
    }
    res.awaiting = Some(AwaitingOp::ChooseNumberEx {
        chooser: player,
        pay_cost: Some(VariablePayKind::GenericMana),
        then,
    });
    (
        Flow::Wait(Pending::ChooseNumber {
            player,
            min,
            max,
            reason: crate::choice::NumberPrompt::VariableManaPayment,
        }),
        true,
    )
}

fn generic_payment_max(state: &GameState, res: &Resolution, player: PlayerId, max: u32) -> u32 {
    use crate::casting;
    use baylee_core::mana::ManaCost;

    let what = super::payment_for(state, res);
    let upper = max.min(casting::spendable_units(state, player, what));
    let allowance = casting::live_allowance(state, player, what);
    let can_pay = |number| {
        number == 0
            || casting::plan_mana_scoped(
                state,
                player,
                what,
                &ManaCost::from_symbol_generic(number),
                None,
                allowance,
            )
            .is_some()
    };
    if can_pay(upper) {
        upper
    } else {
        casting::greatest_affordable(upper, can_pay)
    }
}

/// Resume [`Effect::ChooseNumberEx`] with `number`: pay `pay_cost` when `Some`,
/// bind [`Amount::ResolutionChosenNumber`] on `res.subject`, and splice `then`.
pub(super) fn choose_number_ex_resume(
    state: &mut GameState,
    res: &mut Resolution,
    chooser: PlayerId,
    pay_cost: Option<VariablePayKind>,
    then: &'static [Effect],
    number: u32,
) -> Flow {
    if !state.has_left(chooser) {
        let paid = match pay_cost {
            None => true,
            Some(VariablePayKind::Energy) => number == 0 || state.pay_energy(chooser, number),
            Some(VariablePayKind::GenericMana) => {
                if number == 0 {
                    true
                } else {
                    let cost = baylee_core::mana::ManaCost::from_symbol_generic(number);
                    let what = super::payment_for(state, res);
                    crate::casting::pay_mana_for(state, chooser, what, &cost).is_some()
                }
            }
            Some(VariablePayKind::Life) => {
                if number == 0 {
                    true
                } else if let Ok(life) = i32::try_from(number)
                    && state.can_pay_life(chooser, life)
                {
                    state.change_life(chooser, -life, Cause::Cost);
                    true
                } else {
                    false
                }
            }
        };
        if paid {
            res.subject.chosen_number = Some(number);
            if !then.is_empty() {
                let next = res.pc + 1;
                res.effects.splice(next..next, super::flatten(then));
            }
        }
    }
    super::advance_program(res);
    super::run(state, res)
}

/// Resolve `attachment` to its exact live, unphased battlefield incarnation.
fn resolve_attachment_on_battlefield(
    state: &GameState,
    res: &Resolution,
    spec: TargetSpec,
) -> Option<DamageSourceRef> {
    let reference = match spec {
        TargetSpec::ThisObject => subjects::source(state, res)?,
        TargetSpec::EventObject => subjects::event(state, res)?,
        _ => {
            let id = zones::spec_object(state, res, spec)?;
            subjects::target(state, res, id).or_else(|| state.source_identity(id))?
        }
    };
    state
        .object(reference.object)
        .filter(|o| {
            o.version == reference.version
                && o.zone == Zone::Battlefield
                && !o.status.contains(Status::PHASED_OUT)
        })
        .map(|_| reference)
}

/// Whether `attachment` can legally become attached to `host` on the battlefield
/// (`CR 301.5c`, `CR 301.6`, `CR 303.4d`, `CR 701.3a–b`, `CR 702.16c–d`, `CR 702.151b`).
pub(super) fn can_attach_object(state: &GameState, attachment: ObjectId, host: ObjectId) -> bool {
    if attachment == host {
        return false;
    }
    let (Some(att_obj), Some(host_obj)) = (state.object(attachment), state.object(host)) else {
        return false;
    };
    if att_obj.zone != Zone::Battlefield
        || host_obj.zone != Zone::Battlefield
        || att_obj.status.contains(Status::PHASED_OUT)
        || host_obj.status.contains(Status::PHASED_OUT)
    {
        return false;
    }
    let att_chars = att_obj.characteristics();
    let has_reconfigure =
        att_obj
            .abilities(state.bases.as_ref())
            .iter()
            .any(|ability| match ability {
                AbilityDef::Activated { effects, .. }
                | AbilityDef::ActivatedConditional { effects, .. } => {
                    effects.contains(&Effect::ReconfigureAttach)
                }
                _ => false,
            });
    if att_chars.types.contains(TypeSet::BATTLE)
        || (att_chars.types.contains(TypeSet::CREATURE) && !has_reconfigure)
    {
        return false;
    }
    if att_chars.subtypes.contains(subtypes::enchantment::AURA) {
        return crate::aura_bindings::can_attach(state, attachment, host)
            && crate::permanent_guards::permits_new_enchantment(state, host, attachment);
    }
    let required = if att_chars.subtypes.contains(subtypes::artifact::EQUIPMENT) {
        TypeSet::CREATURE
    } else if att_chars
        .subtypes
        .contains(subtypes::artifact::FORTIFICATION)
    {
        TypeSet::LAND
    } else {
        return false;
    };
    host_obj.characteristics().types.contains(required)
        && !crate::eval::protected_from(state, host, attachment)
}

/// Start [`Effect::MoveAttachment`]: enumerate legal permanent recipients on the
/// battlefield and prompt the resolving controller via `Pending::ChooseCards`.
pub(super) fn move_attachment_start(
    state: &mut GameState,
    res: &mut Resolution,
    attachment: TargetSpec,
    recipient_filter: &'static Filter,
    must_change_recipient: bool,
    optional: bool,
    then_if_moved: &'static [Effect],
) -> Option<Pending> {
    let you = res.controller;
    if state.has_left(you) {
        return None;
    }
    let att_ref = resolve_attachment_on_battlefield(state, res, attachment)?;
    let (current_host, is_aura) = {
        let obj = state.object(att_ref.object)?;
        (
            obj.attached_to,
            obj.characteristics()
                .subtypes
                .contains(subtypes::enchantment::AURA),
        )
    };
    let options: Vec<ObjectId> = state
        .battlefield_seen()
        .filter(|&id| {
            if must_change_recipient && current_host == Some(id) {
                return false;
            }
            can_attach_object(state, att_ref.object, id)
                && state.object(id).is_some_and(|host_obj| {
                    super::matches_in_resolution(
                        recipient_filter,
                        state,
                        host_obj,
                        you,
                        res,
                        Some(you),
                    )
                })
        })
        .collect();
    if options.is_empty() {
        return None;
    }
    let prompt = if is_aura {
        ChoicePrompt::NewHost {
            aura: att_ref.object,
        }
    } else {
        ChoicePrompt::Generic
    };
    res.awaiting = Some(AwaitingOp::MoveAttachment {
        attachment: att_ref,
        then_if_moved,
    });
    Some(Pending::ChooseCards {
        player: you,
        options,
        min: u8::from(!optional),
        max: 1,
        prompt,
        total: None,
    })
}

/// Resume [`Effect::MoveAttachment`] with `chosen`: attach `attachment` to the
/// selected permanent and splice `then_if_moved` only when it actually attached
/// to a different object (`CR 701.3b`).
pub(super) fn move_attachment_resume(
    state: &mut GameState,
    res: &mut Resolution,
    attachment: DamageSourceRef,
    then_if_moved: &'static [Effect],
    chosen: &[ObjectId],
) {
    let Some(&host) = chosen.first() else {
        return;
    };
    if !subjects::on_battlefield(state, attachment)
        || state
            .object(attachment.object)
            .is_none_or(|o| o.status.contains(Status::PHASED_OUT))
        || !can_attach_object(state, attachment.object, host)
    {
        return;
    }
    let prev_host = state.object(attachment.object).and_then(|o| o.attached_to);
    if prev_host == Some(host) {
        return;
    }
    state.attach(attachment.object, host);
    if state.object(attachment.object).and_then(|o| o.attached_to) == Some(host)
        && !then_if_moved.is_empty()
    {
        let next = res.pc + 1;
        res.effects
            .splice(next..next, super::flatten(then_if_moved));
    }
}
