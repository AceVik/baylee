//! Evaluation of DSL data: filters, amounts, target options.
//!
//! All evaluation is pure read access to [`GameState`]; `you` is the
//! ability/spell controller, `this` its source object.

mod amounts;
mod conditions;
mod defender_conditions;
#[cfg(test)]
mod ice_condition_tests;
mod names;
mod protection;
mod relations;
mod stack_filters;
pub(crate) mod target_domain;
pub(crate) mod target_pair;
mod targets;
mod upkeep_history;

use crate::object::{GameObject, ObjectKind, Status};
use crate::state::GameState;
use crate::text_changes::{RuleContext, TextChangeMap};
use crate::zone::Zone;
use baylee_cards_dsl::{Filter, TargetSpec};
use baylee_core::ids::{ObjectId, PlayerId};

pub use amounts::{amount, amount_with_context, public_linked_exile_cards, tapped_power};
pub(crate) use amounts::{
    beheld_or_revealed_mana_value, beheld_or_revealed_power, cards_discarded_for_seats,
    colors_among, colors_in_zone, colors_of_mana_spent_on, convoked_count_on, count_controlled_by,
    creatures_attacked_for_seats, devotion_for_players, distinct_card_types_cast_for_seats,
    distinct_sacrificed_card_types_for_seats, energy_removed_for_seats,
    entered_this_turn_for_seats, entry_sacrifices_on, highest_life_lost_for_seats,
    highest_opponent_hand_size, left_zone_this_turn_for_seats, life_lost_for_seats,
    linked_exile_cards, mana_color_spent_on, mana_spent_on, object_colors_of_mana_spent,
    object_convoked_count, object_mana_color_spent, object_snow_mana_spent,
    permanents_sacrificed_for_seats, snow_mana_spent_on, spells_cast_matching_for_seats,
    tokens_created_this_turn_for_seats, turn_history_seats,
};
pub(crate) use conditions::{
    attached_for_ability, condition_reads_event, condition_with_event,
    counters_placed_on_source_this_turn, discarded_for_cost, evolve_holds,
    intervening_if_for_incarnation, sacrificed_for_cost,
};
pub use conditions::{
    condition_holds, condition_holds_with_context, controlled_matching,
    controlled_matching_with_context, intervening_if,
};
pub use protection::{
    permits_enchantment, player_protected_from, protected_from, untargetable_by,
    untargetable_by_source,
};
pub(crate) use protection::{
    player_protected_from_object, protected_from_object, protection_from_exiled_types,
};
pub use relations::{
    controller_of_attached, defending_player_of, enchanted_player, most_life, players,
    players_for_source,
};
pub(crate) use targets::{
    DistinctGroupPool, can_complete_target_groups, defenders_target_filter,
    retain_distinct_group_options,
};
pub use targets::{
    choice_options, choice_options_with_context, eligible_same_controller_groups,
    first_target_players, living_players_in_turn_order, one_per_player_satisfied,
    one_per_player_seats, same_controller_group, stack_target_options,
    stack_target_options_relative, target_options, target_options_with_context,
    target_player_options, target_player_options_from, targeted_players,
};
#[cfg(feature = "test-support")]
pub use targets::{
    spell_target_options_for, spell_target_options_relative, stack_retarget_options,
    target_options_in_domain,
};
#[cfg(not(feature = "test-support"))]
pub(crate) use targets::{
    spell_target_options_for, spell_target_options_relative, stack_retarget_options,
    target_options_in_domain,
};

/// Printed rules use the live source's words; captured stack rules use an
/// explicit [`RuleContext`] instead.
#[must_use]
#[inline]
pub fn live_context(state: &GameState, source: ObjectId) -> RuleContext {
    if state.text_changes.is_empty() {
        return RuleContext {
            source,
            text: TextChangeMap::IDENTITY,
            resolved_name: None,
            resolved_subtype: None,
            reference: None,
        };
    }
    RuleContext {
        source,
        text: state
            .object(source)
            .map_or(TextChangeMap::IDENTITY, |object| {
                state.text_changes.get(baylee_core::ids::DamageSourceRef {
                    object: source,
                    version: object.version,
                })
            }),
        resolved_name: None,
        resolved_subtype: None,
        reference: None,
    }
}

/// Evaluates a [`Filter`] against an object.
#[must_use]
pub fn matches(
    filter: &Filter,
    state: &GameState,
    obj: &GameObject,
    you: PlayerId,
    this: ObjectId,
) -> bool {
    matches_projected(filter, state, obj, obj.characteristics(), you, this)
}

/// Evaluate a filter with the words of a specific captured or granted ability.
#[must_use]
pub fn matches_with_context(
    filter: &Filter,
    state: &GameState,
    obj: &GameObject,
    you: PlayerId,
    context: RuleContext,
) -> bool {
    matches_projected_with_context(filter, state, obj, obj.characteristics(), you, context)
}

/// Evaluate a source filter specifically for a damage event. Effects such as
/// Ghostly Flame change only the source's color for this question; targeting,
/// blocking and every non-damage use still see its actual characteristics
/// (only damage is affected in protection's DEBT, CR 702.16b).
#[must_use]
pub(crate) fn matches_damage_source_with_context(
    filter: &Filter,
    state: &GameState,
    obj: &GameObject,
    you: PlayerId,
    context: RuleContext,
) -> bool {
    matches_damage_source_with_choices(filter, state, obj, you, context, None, None)
}

/// Same as [`matches_damage_source_with_context`], with frozen source choices
/// (`chosen_color` and `chosen_subtype`) captured when a shield or instruction
/// resolved (CR 113.7a, CR 609.7b).
#[must_use]
pub(crate) fn matches_damage_source_with_choices(
    filter: &Filter,
    state: &GameState,
    obj: &GameObject,
    you: PlayerId,
    context: RuleContext,
    chosen_color: Option<baylee_core::mana::ManaColor>,
    chosen_subtype: Option<baylee_core::ids::SubtypeId>,
) -> bool {
    let characteristics = obj.characteristics();
    if characteristics.colors.is_empty() || !state.effects.may_change_damage_source_color() {
        return matches_projected_with_choices(
            filter,
            state,
            obj,
            characteristics,
            you,
            context,
            chosen_color,
            chosen_subtype,
        );
    }
    let colorless = state.effects.iter().any(|fx| {
        crate::effects::applies_to(state, fx, obj)
            && match fx.modifier {
                baylee_cards_dsl::Modifier::DamageFromColoredSourcesIsColorless(colors) => {
                    characteristics.colors.intersects(colors)
                }
                _ => false,
            }
    });
    if !colorless {
        return matches_projected_with_choices(
            filter,
            state,
            obj,
            characteristics,
            you,
            context,
            chosen_color,
            chosen_subtype,
        );
    }
    let mut chars = characteristics.clone();
    chars.colors = baylee_core::color::ColorSet::EMPTY;
    matches_projected_with_choices(
        filter,
        state,
        obj,
        &chars,
        you,
        context,
        chosen_color,
        chosen_subtype,
    )
}

/// Whether `filter` directly or recursively reads [`Filter::OfSourceChosenColor`]
/// or [`Filter::ChosenColor`].
#[must_use]
pub(crate) fn filter_reads_chosen_color(filter: &Filter) -> bool {
    match filter {
        Filter::OfSourceChosenColor | Filter::ChosenColor => true,
        Filter::And(parts) | Filter::Or(parts) => parts.iter().any(filter_reads_chosen_color),
        Filter::Not(inner) => filter_reads_chosen_color(inner),
        _ => false,
    }
}

/// Whether `filter` directly or recursively reads [`Filter::MatchesChosenTypeOfSource`].
#[must_use]
pub(crate) fn filter_reads_chosen_subtype(filter: &Filter) -> bool {
    match filter {
        Filter::MatchesChosenTypeOfSource => true,
        Filter::And(parts) | Filter::Or(parts) => parts.iter().any(filter_reads_chosen_subtype),
        Filter::Not(inner) => filter_reads_chosen_subtype(inner),
        _ => false,
    }
}

/// Evaluates a [`Filter`] against an object whose characteristics are
/// supplied separately.
///
/// The layer system needs this: CR 613.1 evaluates each layer against the
/// characteristics as modified by every *earlier* layer — a value that
/// exists only mid-projection and is not yet in the object's cache.
#[must_use]
pub fn matches_projected(
    filter: &Filter,
    state: &GameState,
    obj: &GameObject,
    chars: &crate::object::Characteristics,
    you: PlayerId,
    this: ObjectId,
) -> bool {
    matches_projected_with_context(filter, state, obj, chars, you, live_context(state, this))
}

/// Projected matching with explicit rules text; recursion keeps that same
/// snapshot even when the original source has changed or left the battlefield.
#[must_use]
pub fn matches_projected_with_context(
    filter: &Filter,
    state: &GameState,
    obj: &GameObject,
    chars: &crate::object::Characteristics,
    you: PlayerId,
    context: RuleContext,
) -> bool {
    matches_projected_with_choices(filter, state, obj, chars, you, context, None, None)
}

#[must_use]
#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
pub(crate) fn matches_projected_with_choices(
    filter: &Filter,
    state: &GameState,
    obj: &GameObject,
    chars: &crate::object::Characteristics,
    you: PlayerId,
    context: RuleContext,
    chosen_color: Option<baylee_core::mana::ManaColor>,
    chosen_subtype: Option<baylee_core::ids::SubtypeId>,
) -> bool {
    if obj.is_merged_component() {
        return false;
    }
    let this = context.source;
    let matches = |f: &Filter| {
        matches_projected_with_choices(
            f,
            state,
            obj,
            chars,
            you,
            context,
            chosen_color,
            chosen_subtype,
        )
    };
    // Unchanged wording is nearly every read, and a leaf is copied only to be
    // rewritten.
    let rewritten;
    let filter = if context.text == crate::text_changes::TextChangeMap::IDENTITY {
        filter
    } else {
        rewritten = context.text.filter_leaf(*filter);
        &rewritten
    };
    match filter {
        Filter::Any => true,
        Filter::This => obj.id == this,
        Filter::Another => obj.id != this,
        Filter::And(parts) => parts.iter().all(&matches),
        Filter::Or(parts) => parts.iter().any(&matches),
        Filter::Not(f) => !matches(f),
        Filter::HasType(t) => chars.types.intersects(*t),
        Filter::LacksType(t) => !chars.types.intersects(*t),
        Filter::HasSupertype(t) => chars.supertypes.contains(*t),
        Filter::HasSubtype(s) => chars.subtypes.contains(*s),
        Filter::HasWatermark => obj.watermark().is_some(),
        Filter::SharesWatermarkWithSource => obj.watermark().is_some_and(|mark| {
            state
                .object(this)
                .is_some_and(|source| source.watermark() == Some(mark))
        }),
        Filter::UnCard => obj.is_un_card(),
        Filter::HasColor(c) => chars.colors.intersects(*c),
        Filter::IsColorless => chars.colors.is_colorless(),
        Filter::Monocolored => chars.colors.len() == 1,
        // CR 201.2: the name an object has *now*, read off its projected
        // characteristics, so a clone answers to what it copied.
        Filter::Named(name) => state.names.get(chars.name) == *name,
        Filter::NameOriginallyPrintedIn(expansion) => {
            expansion.contains_name(state.names.get(chars.name))
        }
        Filter::NotTargetedByAnotherNamed(name) => !state
            .zones
            .list(crate::zone::ZoneLocation::Stack)
            .iter()
            .filter_map(|&id| state.object(id))
            .filter(|ability| ability.kind == crate::object::ObjectKind::AbilityOnStack)
            .any(|ability| {
                ability.targets.contains(&obj.id)
                    && ability.ability.is_some_and(|loc| {
                        loc.source != this
                            && state.object(loc.source).is_some_and(|source| {
                                state.names.get(source.characteristics().name) == *name
                            })
                    })
            }),
        Filter::NamedBySource => names::named_by_source(state, context, chars.name),
        Filter::IsToken => !obj.is_card(),
        Filter::ActivatedAbilityFrom(filter) => {
            stack_filters::activated_source_matches(filter, state, obj, you, context)
        }
        Filter::TargetsPermanentYouControl => {
            let pre_stack_projection = state
                .object(obj.id)
                .is_some_and(|live| live.zone != Zone::Stack);
            obj.targets
                .iter()
                .chain(obj.second_targets())
                .chain(
                    obj.extra_target_instances()
                        .iter()
                        .flat_map(|inst| inst.targets.iter()),
                )
                .any(|id| {
                    state.object(*id).is_some_and(|target| {
                        target.zone == Zone::Battlefield && target.controller == you
                    }) && (pre_stack_projection || state.targets_current_object(obj.id, *id))
                })
        }
        Filter::TargetsYouControl(filter) => {
            let pre_stack_projection = state
                .object(obj.id)
                .is_some_and(|live| live.zone != Zone::Stack);
            obj.targets
                .iter()
                .chain(obj.second_targets())
                .chain(
                    obj.extra_target_instances()
                        .iter()
                        .flat_map(|inst| inst.targets.iter()),
                )
                .any(|id| {
                    state.object(*id).is_some_and(|target| {
                        target.zone == Zone::Battlefield
                            && target.controller == you
                            && matches_with_context(filter, state, target, you, context)
                    }) && (pre_stack_projection || state.targets_current_object(obj.id, *id))
                })
        }
        Filter::WithSingleTarget => {
            let players = obj.target_players.len()
                + usize::from(
                    obj.chosen_player
                        .is_some_and(|p| !obj.target_players.contains(p)),
                );
            let extra_count: usize = obj
                .extra_target_instances()
                .iter()
                .map(|inst| inst.targets.len() + inst.players.len())
                .sum();
            obj.targets.len()
                + obj.second_targets().len()
                + players
                + obj.second_target_players().len()
                + extra_count
                == 1
        }
        Filter::HasManaAbility => {
            (chars.has_mana_ability && chars.abilities_lost.is_none() && !chars.rules_text_lost)
                || !crate::casting::intrinsic_mana_colors(state, obj.id).is_empty()
                || crate::effects::granted_activated(state, obj.id).any(|a| a.mana_ability)
        }
        Filter::HasNonManaActivatedAbility => {
            obj.abilities(state.bases.as_ref()).iter().any(|ability| {
                matches!(
                    ability,
                    baylee_cards_dsl::AbilityDef::Activated { .. }
                        | baylee_cards_dsl::AbilityDef::ActivatedConditional { .. }
                        | baylee_cards_dsl::AbilityDef::ModalActivated { .. }
                        | baylee_cards_dsl::AbilityDef::Loyalty { .. }
                        | baylee_cards_dsl::AbilityDef::Unearth { .. }
                        | baylee_cards_dsl::AbilityDef::Encore { .. }
                ) && !ability.is_mana_ability()
            }) || crate::effects::granted_activated(state, obj.id).any(|a| !a.mana_ability)
        }
        Filter::ControlledByYou => obj.controller == you,
        Filter::ControlledByYourTeam => !state.is_opponent(obj.controller, you),
        Filter::ControlledByOpponent => state.is_opponent(obj.controller, you),
        Filter::ControlledByActivePlayer => obj.controller == state.turn.active,
        // CR 508.5: an attack trigger can belong to the attacker itself or
        // to its Equipment. The latter gets the attacker from the event.
        Filter::ControlledByDefendingPlayer => {
            if let Some(attacker) = [Some(this), context.reference]
                .into_iter()
                .flatten()
                .find(|&id| last_defender(state, id).is_some())
            {
                crate::combat::historical_defending_player(state, attacker) == Some(obj.controller)
            } else {
                state.turn.phase == crate::turn::Phase::Combat
                    && state.is_opponent(obj.controller, state.turn.active)
            }
        }
        Filter::OwnedByYou => obj.owner == you,
        Filter::OwnedBySourceOwner => state
            .object_or_departed(this)
            .is_some_and(|source| source.owner == obj.owner),
        Filter::Tapped => obj.status.contains(Status::TAPPED),
        Filter::Untapped => !obj.status.contains(Status::TAPPED),
        // A lookup and not a scan: this arm runs once per object whenever a
        // filter is walked over the battlefield (`CombatState`'s doc).
        Filter::Attacking => {
            if state
                .object(obj.id)
                .is_some_and(|live| live.version == obj.version)
            {
                state.combat.is_attacking(obj.id)
            } else {
                state
                    .journal
                    .was_attacking_on_departure(obj.id, obj.version)
            }
        }
        Filter::AttackingYou => state.combat.attackers().iter().any(|attacker| {
            attacker.creature == obj.id
                && attacker.defending == baylee_core::ids::Defender::Player(you)
        }),
        Filter::AttackingYouOrYourPlaneswalker => state.combat.attackers().iter().any(|attacker| {
            attacker.creature == obj.id
                && match attacker.defending {
                    baylee_core::ids::Defender::Player(p) => p == you,
                    baylee_core::ids::Defender::Planeswalker(pw) => {
                        state.last_known_controller(pw) == Some(you)
                    }
                    baylee_core::ids::Defender::Battle(_) => false,
                }
        }),
        Filter::DefendingPlayerIsYou => state.combat.attackers().iter().any(|attacker| {
            attacker.creature == obj.id
                && match attacker.defending {
                    baylee_core::ids::Defender::Player(p) => p == you,
                    baylee_core::ids::Defender::Planeswalker(pw) => {
                        state.last_known_controller(pw) == Some(you)
                    }
                    baylee_core::ids::Defender::Battle(b) => {
                        crate::battle::protector(state, b) == Some(you)
                    }
                }
        }),
        // CR 702.22: in the source's band, which the attack declared.
        Filter::BandedWithSource => {
            obj.id != this
                && state
                    .combat
                    .band_of(this)
                    .is_some_and(|band| state.combat.band_of(obj.id) == Some(band))
        }
        Filter::Blocking => state.combat.blockers.iter().any(|b| b.blocker == obj.id),
        Filter::Blocked => {
            state.object(obj.id).is_some_and(|current| {
                current.version == obj.version && current.zone == Zone::Battlefield
            }) && state.combat.is_attacking(obj.id)
                && state.combat.is_blocked(obj.id)
                && matches!(
                    state.turn.step,
                    crate::turn::Step::DeclareBlockers
                        | crate::turn::Step::CombatDamageFirst
                        | crate::turn::Step::CombatDamage
                        | crate::turn::Step::CombatEnd
                )
        }
        // Blocked or unblocked is settled as blockers are declared (CR
        // 509.1h), which is the declare-blockers step's first act; from
        // there to the end of combat an attacker is one or the other.
        Filter::Unblocked => {
            state.object(obj.id).is_some_and(|current| {
                current.version == obj.version && current.zone == Zone::Battlefield
            }) && state.combat.is_attacking(obj.id)
                && !state.combat.is_blocked(obj.id)
                && matches!(
                    state.turn.step,
                    crate::turn::Step::DeclareBlockers
                        | crate::turn::Step::CombatDamageFirst
                        | crate::turn::Step::CombatDamage
                        | crate::turn::Step::CombatEnd
                )
        }
        // The turn's record of arrivals. It is written where a `ZoneChanged`
        // into the battlefield is journaled, and it holds the id
        // `move_object` returns, so the handle a permanent has now is the one
        // written on the way in. It used to be a scan of the journal from
        // the turn's start, which the snapshot hash does not read (#241).
        //
        // No `Cause` is filtered out, and `Cause::Setup` is the one worth
        // saying so about. A seeded battlefield is recorded like any other
        // arrival while the game is built, and the first turn start, after
        // the mulligans, clears the record. An exclusion here was written
        // first and removed when injecting its removal left the test green;
        // `arrival_tests::a_permanent_entered_this_turn_only_on_the_turn_it_was_played`
        // asserts both halves of what makes it unnecessary.
        Filter::EnteredThisTurn => state.per_turn.entered_battlefield.contains(&obj.id),
        Filter::AttackedThisTurn => state
            .per_turn
            .attacked
            .iter()
            .any(|&(id, version, _)| id == obj.id && version == obj.version),
        Filter::ControlledSinceTurnBegan => {
            obj.zone == crate::zone::Zone::Battlefield
                && state
                    .players
                    .get(obj.controller.get() as usize)
                    .is_some_and(|p| obj.controlled_since <= p.turn_start_timestamp)
        }
        Filter::PutIntoGraveyardThisTurn => state.per_turn.entered_graveyard.contains(&obj.id),
        Filter::SurveilledByYouThisTurn => {
            obj.zone == crate::zone::Zone::Graveyard
                && state.per_turn.surveilled_cards.items.contains(&(
                    baylee_core::ids::DamageSourceRef {
                        object: obj.id,
                        version: obj.version,
                    },
                    you,
                ))
        }
        // The object's own counters. A leaves-the-battlefield trigger asks
        // the object as it last existed there, and `trigger::departed_matches`
        // hands in that object, counters and all.
        Filter::HasCounter(kind) => obj.counters.get(*kind) > 0,
        Filter::IsCommander => state.holds_commander(obj),
        Filter::HasAdventure => obj
            .printed_face()
            .and_then(|face| crate::state::CardLookup::card(state.bases.as_ref(), face.card()))
            .is_some_and(|def| def.faces.iter().any(|f| f.adventure)),
        Filter::HasRampage => {
            let printed = chars.abilities_lost.is_none()
                && !chars.rules_text_lost
                && state.printed_ability_list(obj.id).is_some_and(|list| {
                    list.abilities.iter().any(|a| {
                        matches!(
                            a,
                            baylee_cards_dsl::AbilityDef::Triggered {
                                trigger: baylee_cards_dsl::Trigger::Rampage,
                                ..
                            }
                        )
                    })
                });
            printed
                || state.effects.iter().any(|fx| {
                    matches!(
                        fx.modifier,
                        baylee_cards_dsl::Modifier::GrantTriggered {
                            trigger: baylee_cards_dsl::Trigger::Rampage,
                            ..
                        } | baylee_cards_dsl::Modifier::GrantTriggeredDefinition(
                            baylee_cards_dsl::AbilityDef::Triggered {
                                trigger: baylee_cards_dsl::Trigger::Rampage,
                                ..
                            },
                        )
                    ) && crate::effects::applies_to(state, fx, obj)
                })
        }
        Filter::HasCumulativeUpkeep => {
            let is_cu = |trigger: &baylee_cards_dsl::Trigger,
                         effects: &[baylee_cards_dsl::Effect]| {
                matches!(
                    trigger,
                    baylee_cards_dsl::Trigger::StepBegin {
                        step: baylee_cards_dsl::StepKind::Upkeep,
                        whose: baylee_cards_dsl::PlayerRel::You,
                    }
                ) && baylee_cards_dsl::Effect::is_cumulative_upkeep(effects)
            };
            let printed = chars.abilities_lost.is_none()
                && !chars.rules_text_lost
                && state.printed_ability_list(obj.id).is_some_and(|list| {
                    list.abilities.iter().any(|a| {
                        matches!(
                            a,
                            baylee_cards_dsl::AbilityDef::Triggered {
                                trigger,
                                effects,
                                ..
                            } if is_cu(trigger, effects)
                        )
                    })
                });
            printed
                || state.effects.iter().any(|fx| {
                    matches!(
                        &fx.modifier,
                        baylee_cards_dsl::Modifier::GrantTriggered {
                            trigger,
                            effects,
                            ..
                        }
                        | baylee_cards_dsl::Modifier::GrantTriggeredDefinition(
                            baylee_cards_dsl::AbilityDef::Triggered {
                                trigger,
                                effects,
                                ..
                            },
                        ) if is_cu(trigger, effects)
                    ) && crate::effects::applies_to(state, fx, obj)
                })
        }
        Filter::HasAnyCounter => obj.counters.iter().any(|(_, n)| n > 0),
        Filter::Enchanted => state.battlefield_seen().any(|id| {
            state.object(id).is_some_and(|aura| {
                aura.attached_to == Some(obj.id)
                    && aura
                        .characteristics()
                        .subtypes
                        .contains(baylee_core::generated::subtypes::enchantment::AURA)
            })
        }),
        Filter::Equipped => state.battlefield_seen().any(|id| {
            state.object(id).is_some_and(|equip| {
                equip.attached_to == Some(obj.id)
                    && equip
                        .characteristics()
                        .subtypes
                        .contains(baylee_core::generated::subtypes::artifact::EQUIPMENT)
            })
        }),
        Filter::CmcEqualsCountersOnSource(kind) => state
            .object(this)
            .is_some_and(|src| chars.mana_value() == u32::from(src.counters.get(*kind))),
        Filter::NotControlledByReferenceController => context
            .reference
            .and_then(|reference| state.object_or_departed(reference))
            .is_some_and(|reference| obj.controller != reference.controller),
        Filter::LowestManaValueOnBattlefield(inner) => {
            matches(inner)
                && state
                    .battlefield_seen()
                    .filter_map(|id| state.object(id))
                    .filter(|other| matches_with_context(inner, state, other, you, context))
                    .map(|other| other.characteristics().mana_value())
                    .min()
                    .is_some_and(|lowest| chars.mana_value() == lowest)
        }
        Filter::SameNameAsLinkedExile => {
            linked_exile_cards(state, you, context, None, &Filter::Any)
                .into_iter()
                .filter_map(|id| state.object(id))
                .any(|exiled| exiled.characteristics().name == chars.name)
        }
        // CR 201.2a: two objects share a name only when both have that name.
        // Phased-out permanents do not participate (CR 702.26b).
        Filter::SharesNameWithAnotherPermanent => {
            chars.name != crate::state::NAMELESS
                && state.battlefield_seen().any(|id| {
                    id != obj.id
                        && state.object(id).is_some_and(|other| {
                            other.kind == ObjectKind::Permanent
                                && other.characteristics().name == chars.name
                        })
                })
        }
        Filter::SharesColorWithExiledWithSource => {
            linked_exile_cards(state, you, context, None, &Filter::Any)
                .into_iter()
                .filter_map(|id| state.object(id))
                .any(|exiled| exiled.characteristics().colors.intersects(chars.colors))
        }
        Filter::SharesColorWithSource => {
            !chars.colors.is_empty()
                && state
                    .object_or_departed(this)
                    .is_some_and(|src| src.characteristics().colors.intersects(chars.colors))
        }
        Filter::SharesColorWithMostCommon(inner) => {
            use baylee_core::color::Color;
            let mut counts = [0_u32; 5];
            for other in state.battlefield_seen().filter_map(|id| state.object(id)) {
                if matches_with_context(inner, state, other, you, context) {
                    for color in Color::ALL {
                        if other.characteristics().colors.contains(color) {
                            counts[color as usize] = counts[color as usize].saturating_add(1);
                        }
                    }
                }
            }
            let most_common = counts.iter().copied().max().unwrap_or(0);
            most_common > 0
                && Color::ALL.into_iter().any(|color| {
                    counts[color as usize] == most_common && chars.colors.contains(color)
                })
        }
        Filter::CmcEqualsYourUnspentMana => state
            .players
            .get(usize::from(you.get()))
            .is_some_and(|player| u64::from(chars.mana_value()) == player.mana_pool.total()),
        // As the source last existed when it has left the battlefield
        // (CR 608.2h): Abu Ja'far's dies trigger reads its partners there.
        Filter::BlockingSource => {
            state.combat.is_blocking(obj.id, this) || left_combat_partner(state, this, obj, true)
        }
        Filter::BlockedBySource => {
            state.combat.is_blocking(this, obj.id) || left_combat_partner(state, this, obj, false)
        }
        Filter::Reference => context.reference == Some(obj.id),
        Filter::BlockingReference => context
            .reference
            .is_some_and(|reference| state.combat.is_blocking(obj.id, reference)),
        Filter::BlockedByReferenceThisTurn => context
            .reference
            .is_some_and(|reference| blocked_this_turn(state, reference, obj.id)),
        Filter::BlockedReferenceThisTurn => context
            .reference
            .is_some_and(|reference| blocked_this_turn(state, obj.id, reference)),
        Filter::AttachedTo(inner) => obj
            .attached_to
            .and_then(|host| state.object(host))
            .is_some_and(|host| matches_with_context(inner, state, host, you, context)),
        Filter::SharesTypeWithReference(types) => context
            .reference
            .and_then(|reference| state.object(reference))
            .is_some_and(|reference| {
                chars
                    .types
                    .intersection(reference.characteristics().types)
                    .intersects(*types)
            }),
        Filter::SharesCardTypeWithReference => context
            .reference
            .and_then(|reference| state.object(reference))
            .is_some_and(|reference| chars.types.intersects(reference.characteristics().types)),
        Filter::CastForBlitz => obj.riders.contains(&crate::object::Rider::Blitzed),
        Filter::CastForFlashback => obj.riders.contains(&crate::object::Rider::Flashback),
        Filter::CastForDash => obj.riders.contains(&crate::object::Rider::Dashed),
        Filter::FirstMatchingSpellThisTurn(inner) => {
            let items = &state.per_turn.cast_spells_this_turn.items;
            let limit = items
                .iter()
                .position(|rec| rec.source.object == obj.id && rec.source.version == obj.version)
                .unwrap_or(items.len());
            let mut caster_seats = [obj.controller];
            let _ = &mut caster_seats;
            !items[..limit].iter().any(|rec| {
                rec.controller == obj.controller
                    && matches_with_context(inner, state, &rec.snapshot, you, context)
            })
        }
        Filter::SharesCreatureTypeWithOtherTarget => context.reference.is_none_or(|reference| {
            reference != obj.id
                && state
                    .object(reference)
                    .is_some_and(|other| target_pair::shares(chars, other.characteristics()))
        }),
        Filter::DealtDamageToSourceThisTurn => dealt_damage_to_this_turn(state, this, obj),
        Filter::DealtDamageBySourceThisTurn => state.object(this).is_some_and(|source| {
            state.per_turn.damage_log.iter().any(|entry| {
                entry.source.object == this
                    && entry.source.version == source.version
                    && entry.to == crate::event::DamageTarget::Object(obj.id)
                    && entry.to_version == Some(obj.version)
            })
        }),
        Filter::BlockedByReference => context
            .reference
            .is_some_and(|r| state.combat.is_blocking(r, obj.id)),
        Filter::DealtDamageThisTurn => state.per_turn.damage_log.iter().any(|e| {
            e.to == crate::event::DamageTarget::Object(obj.id) && e.to_version == Some(obj.version)
        }),
        Filter::BlockedThisTurn => state
            .per_turn
            .blocks
            .iter()
            .any(|r| r.blocker.object == obj.id && r.blocker.version == obj.version),
        Filter::AttackedDuringControllersLastTurn => state
            .players
            .get(usize::from(obj.controller.get()))
            .is_some_and(|p| {
                p.last_turn_attackers
                    .contains(&baylee_core::ids::DamageSourceRef {
                        object: obj.id,
                        version: obj.version,
                    })
            }),
        Filter::CouldHaveAttackedThisTurn => state
            .per_turn
            .could_have_attacked
            .contains(&(obj.id, obj.version)),
        Filter::BlockedThisTurnBy(inner) => state.per_turn.blocks.iter().any(|record| {
            record.attacker.object == obj.id
                && record.attacker.version == obj.version
                && state
                    .object(record.blocker.object)
                    .filter(|b| b.version == record.blocker.version)
                    .is_some_and(|b| matches_with_context(inner, state, b, you, context))
        }),
        Filter::SpellTargetingAffected => state.spell_targets_affected(obj, this),
        Filter::CountersAtLeast(kind, n) => obj.counters.get(*kind) >= *n,
        Filter::MatchesChosenTypeOfSource => context
            .resolved_subtype
            .or(chosen_subtype)
            .or_else(|| state.object(this).and_then(|src| src.chosen_subtype))
            .is_some_and(|s| chars.subtypes.contains(s)),
        Filter::OfSourceChosenCardType => state
            .object(this)
            .and_then(crate::object::GameObject::chosen_card_type)
            .is_some_and(|chosen| chars.types.intersects(chosen)),
        Filter::OfSourceChosenColor | Filter::ChosenColor => {
            use baylee_core::color::Color;
            use baylee_core::mana::ManaColor;
            let chosen = match chosen_color
                .or_else(|| {
                    state
                        .resolution_chosen_color
                        .filter(|(src, _)| *src == this)
                        .map(|(_, c)| c)
                })
                .or_else(|| state.object(this).and_then(|src| src.chosen_color))
            {
                Some(ManaColor::White) => Some(Color::White),
                Some(ManaColor::Blue) => Some(Color::Blue),
                Some(ManaColor::Black) => Some(Color::Black),
                Some(ManaColor::Red) => Some(Color::Red),
                Some(ManaColor::Green) => Some(Color::Green),
                Some(ManaColor::Colorless) | None => None,
            };
            chosen.is_some_and(|c| chars.colors.contains(c))
        }
        // CR 105.2c: a colourless object shares a colour with nothing.
        Filter::SharesAColorWith(inner) => {
            !chars.colors.is_empty()
                && state.battlefield_seen().any(|id| {
                    id != obj.id
                        && state.object(id).is_some_and(|other| {
                            other.characteristics().colors.intersects(chars.colors)
                                && matches_with_context(inner, state, other, you, context)
                        })
                })
        }
        Filter::FaceDown => obj.status.contains(crate::object::Status::FACE_DOWN),
        Filter::SharesCreatureTypeWith(inner) => {
            if matches!(inner, Filter::This) {
                state
                    .object(this)
                    .is_some_and(|src| target_pair::shares(chars, src.characteristics()))
            } else {
                state.battlefield_seen().any(|id| {
                    id != obj.id
                        && state.object(id).is_some_and(|other| {
                            target_pair::shares(chars, other.characteristics())
                                && matches_with_context(inner, state, other, you, context)
                        })
                })
            }
        }
        // The live attachment first, then what `this` was wearing the moment
        // `obj` left the battlefield (CR 603.10a). The fallback is what lets
        // "whenever equipped creature dies" fire at all: the state-based
        // actions unattach the Equipment in the same `sba::run` fixpoint that
        // killed its host, and triggers are collected after it. It cannot
        // reach anything else — `ltb_attachments` only ever names a host that
        // is off the battlefield, and the move that brings one back clears
        // its entry first.
        Filter::AttachedToBySource => {
            state
                .object(this)
                .and_then(|src| src.attached_to)
                .is_some_and(|attached| attached == obj.id)
                || state
                    .ltb_attachments
                    .iter()
                    .any(|(host, worn)| *host == obj.id && worn.contains(&this))
        }
        // CR 303.4b: what the source enchants, equips or fortifies is what
        // the object is attached to, read from the attachment's side.
        Filter::AttachedToSource => obj.attached_to == Some(this),
        Filter::IsAttached => obj.is_attached(),
        Filter::Historic => {
            chars.types.contains(baylee_core::types::TypeSet::ARTIFACT)
                || chars
                    .supertypes
                    .contains(baylee_core::types::SupertypeSet::LEGENDARY)
                || chars
                    .subtypes
                    .contains(baylee_core::generated::subtypes::enchantment::SAGA)
        }
        Filter::HasCounters(kind) => match kind {
            Some(k) => obj.counters.get(*k) > 0,
            None => obj.counters.iter().any(|(_, n)| n > 0),
        },
        Filter::AnyCountersAtLeast(n) => {
            obj.counters.iter().fold(0_u32, |acc, (_, count)| {
                acc.saturating_add(u32::from(count))
            }) >= u32::from(*n)
        }
        Filter::ControlledByEnchantedPlayer => {
            enchanted_player(state, this) == Some(obj.controller)
        }
        Filter::PairedWithSource => state.paired_partner(this) == Some(obj.id),
        Filter::WarpedInExile => {
            obj.zone == crate::zone::Zone::Exile
                && obj
                    .riders
                    .iter()
                    .any(|r| matches!(r, crate::object::Rider::WarpedInExile(_)))
        }
        Filter::CmcAtMostSnowManaSpent => {
            chars.mana_value() <= amounts::snow_mana_spent_on(state, this)
        }
        Filter::Monstrous => {
            obj.zone == Zone::Battlefield
                && !obj.status.contains(crate::object::Status::PHASED_OUT)
                && obj.riders.contains(&crate::object::Rider::Monstrous)
        }
        Filter::Suspected => {
            obj.zone == Zone::Battlefield
                && !obj.status.contains(crate::object::Status::PHASED_OUT)
                && obj.riders.contains(&crate::object::Rider::Suspected)
        }
        Filter::Saddled => {
            obj.zone == Zone::Battlefield
                && !obj.status.contains(crate::object::Status::PHASED_OUT)
                && obj.riders.contains(&crate::object::Rider::Saddled)
        }
        Filter::RingBearer => {
            if obj.zone == Zone::Battlefield {
                !obj.status.contains(crate::object::Status::PHASED_OUT)
                    && state.is_ring_bearer(obj.id)
            } else {
                state
                    .journal
                    .entries()
                    .iter()
                    .rev()
                    .find_map(|entry| match &entry.event {
                        crate::event::GameEvent::ZoneChanged {
                            object,
                            from: Zone::Battlefield,
                            ..
                        } if *object == obj.id => entry.departure.as_ref().and_then(|d| {
                            (d.version == obj.version || d.version.wrapping_add(1) == obj.version)
                                .then_some(d.was_ring_bearer_of.is_some())
                        }),
                        _ => None,
                    })
                    .unwrap_or(false)
            }
        }
        Filter::Modified => object_is_modified(state, obj, chars),
        Filter::ControlledBy(rel) => {
            if let Some(seats) = players_for_source(*rel, state, you, Some(this)) {
                return seats.contains(&obj.controller);
            }
            match rel {
                baylee_cards_dsl::PlayerRel::EventPlayer
                | baylee_cards_dsl::PlayerRel::DamagedPlayer => state
                    .object(this)
                    .and_then(|o| {
                        o.riders.iter().find_map(|r| match r {
                            crate::object::Rider::EventPlayer(seat) => Some(*seat),
                            _ => None,
                        })
                    })
                    .is_some_and(|seat| !state.has_left(seat) && seat == obj.controller),
                baylee_cards_dsl::PlayerRel::ControllerOfTarget
                | baylee_cards_dsl::PlayerRel::TargetPlayerOrController
                | baylee_cards_dsl::PlayerRel::ControllerOfEvent => context
                    .reference
                    .and_then(|id| state.last_known_controller(id))
                    .is_some_and(|seat| !state.has_left(seat) && seat == obj.controller),
                baylee_cards_dsl::PlayerRel::OwnerOfEvent
                | baylee_cards_dsl::PlayerRel::OwnerOfTarget => context
                    .reference
                    .and_then(|id| state.object_or_departed(id))
                    .is_some_and(|o| !state.has_left(o.owner) && o.owner == obj.controller),
                _ => false,
            }
        }
        Filter::ControlledByChooser => false,
        Filter::ControlledByPlayerWhoChoseMode(mode) => state.object(this).is_some_and(|src| {
            src.riders.iter().any(|r| {
                matches!(
                    r,
                    crate::object::Rider::PlayerChosenMode(p, m)
                        if *p == obj.controller && *m == *mode
                )
            })
        }),
        Filter::CreatedBySource => state.source_identity(this).is_some_and(|src_ref| {
            obj.riders
                .contains(&crate::object::Rider::CreatedBy(src_ref))
        }),
        Filter::AttachedHostOfRemembered => state
            .resolution_remembered
            .as_ref()
            .filter(|(src, _)| *src == this)
            .is_some_and(|(_, rem)| {
                rem.iter().any(|r| {
                    state.object(r.reference.object).is_some_and(|att| {
                        att.zone == Zone::Battlefield
                            && att.version == r.reference.version
                            && att.attached_to == Some(obj.id)
                    })
                })
            }),
        Filter::OwnedBy(rel) => players_for_source(*rel, state, you, Some(this))
            .is_some_and(|seats| seats.contains(&obj.owner)),
        Filter::SameManaValueAsSource => {
            source_mana_value(state, this).is_some_and(|mv| chars.mana_value() == mv)
        }
        Filter::SharesSubtypeWithCommander => {
            // Eight `AND`s per commander, not one probe per subtype id.
            // The marker list rather than the command zone, for the reason
            // `resolve::mana` gives: Path of Ancestry has to keep working
            // once the commander it names is on the battlefield.
            let obj_subs = chars.subtypes;
            state
                .commanders
                .get(you.get() as usize)
                .into_iter()
                .flatten()
                .filter_map(|c| state.object(c.object))
                .any(|commander| obj_subs.intersects(commander.characteristics().subtypes))
        }
        Filter::HasKeyword(k) => chars.keywords.contains(*k),
        Filter::Kicked => obj.kicked(),
        Filter::CastFromHand => obj.cast_history.was_from_hand(),
        Filter::WasCast => obj.cast_history.caster().is_some(),
        Filter::WasCastByYou => obj.cast_history.caster() == Some(you),
        Filter::CastFromZone(zone) => object_was_cast_from_zone(obj, *zone),
        Filter::CmcAtMost(n) => chars.mana_value() <= *n,
        // The bound is the announced X on the ability's own source, which is
        // where `cast_wizard` writes it and what `res.x` is read from one
        // layer up. A source that is gone, or that announced nothing, bounds
        // at 0 rather than at everything: an unreadable bound that found the
        // whole library would be a tutor with no price.
        Filter::CmcAtMostX => chars.mana_value() <= announced_x(state, this),
        Filter::CmcExactlyX => chars.mana_value() == announced_x(state, this),
        Filter::TwiceCmcExactlyX => {
            chars.mana_value().saturating_mul(2) == announced_x(state, this)
        }
        // Converge's number, off the source where the payment wrote it; no
        // record is no mana spent, and so no colors.
        Filter::CmcAtMostColorsSpent => {
            let colors = state
                .object(this)
                .and_then(|o| o.paid.as_ref())
                .map_or(0, |p| p.colors_spent.len());
            chars.mana_value() <= u32::from(colors)
        }
        Filter::CmcAtLeast(n) => chars.mana_value() >= *n,
        Filter::ToughnessAtMost(n) => chars.toughness.is_some_and(|t| t <= *n),
        Filter::ToughnessAtLeast(n) => chars.toughness.is_some_and(|t| t >= *n),
        // `is_some_and`, so an object with no power at all — a land, an
        // instant on the stack — is not "a creature with power 4 or
        // greater" by default. The three neighbours above answer the same
        // way and for the same reason.
        Filter::PowerAtLeast(n) => chars.power.is_some_and(|p| p >= *n),
        Filter::PowerAtMost(n) => chars.power.is_some_and(|p| p <= *n),
        Filter::PowerAtMostSourcePower => chars
            .power
            .is_some_and(|power| source_power(state, this).is_some_and(|bound| power <= bound)),
        Filter::PowerLessThanSourcePower => below_source_power(chars.power, state, this),
        Filter::ToughnessLessThanSourcePower => below_source_power(chars.toughness, state, this),
        Filter::InZone(z) => obj.zone.matches_ref(*z),
        Filter::ControllerControls(inner) => state.battlefield_seen().any(|id| {
            state.object(id).is_some_and(|other| {
                other.controller == obj.controller
                    && matches_with_context(
                        inner,
                        state,
                        other,
                        obj.controller,
                        RuleContext {
                            source: id,
                            ..context
                        },
                    )
            })
        }),
        Filter::TappedToPayForSourceThisTurn => {
            last_battlefield_version(state, this).is_some_and(|src_ver| {
                state.per_turn.tapped_for_ability.contains(&(
                    baylee_core::ids::DamageSourceRef {
                        object: this,
                        version: src_ver,
                    },
                    baylee_core::ids::DamageSourceRef {
                        object: obj.id,
                        version: obj.version,
                    },
                ))
            })
        }
        Filter::CmcAtMostAmount(amt)
        | Filter::CmcEqualsAmount(amt)
        | Filter::PowerAtMostAmount(amt)
        | Filter::PowerAtLeastAmount(amt)
        | Filter::PowerEqualsAmount(amt)
        | Filter::ToughnessEqualsAmount(amt) => {
            let Some(_guard) = DynamicFilterDepthGuard::enter() else {
                return false;
            };
            let mag = amounts::amount_with_context(
                amt,
                state,
                you,
                context,
                Some(announced_x(state, this)),
            );
            match_dynamic_amount_filter(filter, chars, mag, amt.is_negative())
        }
        Filter::CmcEqualsChosenNumber
        | Filter::PowerEqualsChosenNumber
        | Filter::ToughnessEqualsChosenNumber => {
            let Some(chosen) = state
                .object(this)
                .or_else(|| state.damage_source(this, None))
                .and_then(crate::object::GameObject::chosen_number)
            else {
                return false;
            };
            match filter {
                Filter::CmcEqualsChosenNumber => chars.mana_value() == u32::from(chosen),
                Filter::PowerEqualsChosenNumber => chars.power == Some(i16::from(chosen)),
                Filter::ToughnessEqualsChosenNumber => chars.toughness == Some(i16::from(chosen)),
                _ => unreachable!(),
            }
        }
        Filter::IsRemembered
        | Filter::NotRemembered
        | Filter::SharesCardTypeWithRemembered
        | Filter::SharesNameWithRemembered => {
            let rem = state
                .resolution_remembered
                .as_ref()
                .filter(|(src, _)| *src == this)
                .map(|(_, r)| r.as_ref());
            crate::resolve::matches_remembered_filter(filter, state, obj, chars, rem)
        }
    }
}

const MAX_DYNAMIC_FILTER_DEPTH: u8 = 8;

std::thread_local! {
    static DYNAMIC_FILTER_DEPTH: core::cell::Cell<u8> = const { core::cell::Cell::new(0) };
}

/// Cycle-safe re-entrancy guard for [`Filter`] variants that evaluate [`baylee_cards_dsl::Amount`].
pub(crate) struct DynamicFilterDepthGuard;

impl DynamicFilterDepthGuard {
    pub(crate) fn enter() -> Option<Self> {
        DYNAMIC_FILTER_DEPTH.with(|cell| {
            let depth = cell.get();
            if depth >= MAX_DYNAMIC_FILTER_DEPTH {
                None
            } else {
                cell.set(depth + 1);
                Some(Self)
            }
        })
    }
}

impl Drop for DynamicFilterDepthGuard {
    fn drop(&mut self) {
        DYNAMIC_FILTER_DEPTH.with(|cell| {
            cell.set(cell.get().saturating_sub(1));
        });
    }
}

/// Compare `chars` against a pre-evaluated [`baylee_cards_dsl::Amount`] magnitude and sign.
pub(crate) fn match_dynamic_amount_filter(
    filter: &Filter,
    chars: &crate::object::Characteristics,
    mag: u32,
    negative: bool,
) -> bool {
    let bound = if negative {
        -i64::from(mag)
    } else {
        i64::from(mag)
    };
    match filter {
        Filter::CmcAtMostAmount(_) => i64::from(chars.mana_value()) <= bound,
        Filter::CmcEqualsAmount(_) => i64::from(chars.mana_value()) == bound,
        Filter::PowerAtMostAmount(_) => chars.power.is_some_and(|p| i64::from(p) <= bound),
        Filter::PowerAtLeastAmount(_) => chars.power.is_some_and(|p| i64::from(p) >= bound),
        Filter::PowerEqualsAmount(_) => chars.power.is_some_and(|p| i64::from(p) == bound),
        Filter::ToughnessEqualsAmount(_) => chars.toughness.is_some_and(|t| i64::from(t) == bound),
        _ => unreachable!("only dynamic amount filters"),
    }
}

/// What `id` attacks, or attacked before it was removed from combat (CR
/// 508.5), as long as it is still that creature: one that left the
/// battlefield is read as it last existed there, but one back on the
/// battlefield as a new object (CR 400.7) that never attacked this combat
/// attacked nothing.
pub(crate) fn last_defender(state: &GameState, id: ObjectId) -> Option<baylee_core::ids::Defender> {
    if let Some(defender) = state.combat.defender_of(id) {
        return Some(defender);
    }
    let (version, remembered) = state.combat.remembered_defender(id)?;
    let new_object_here = state
        .object(id)
        .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield && o.version != version);
    (!new_object_here).then_some(remembered)
}

/// Whether `partner` was in combat with `source` the moment `source` left
/// the battlefield ([`GameState::ltb_combat`]), blocking it (`blocking`) or
/// blocked by it, and is still the object it was. Only once `source` has
/// gone: a source on the battlefield is asked of the live combat.
fn left_combat_partner(
    state: &GameState,
    source: ObjectId,
    partner: &crate::object::GameObject,
    blocking: bool,
) -> bool {
    state
        .object(source)
        .is_none_or(|o| o.zone != crate::zone::Zone::Battlefield)
        && state
            .ltb_combat
            .iter()
            .find(|(id, _)| *id == source)
            .is_some_and(|(_, partners)| {
                partners.iter().any(|(r, blocks_it)| {
                    *blocks_it == blocking && r.object == partner.id && r.version == partner.version
                })
            })
}

/// Whether `blocker` blocked `attacker` earlier this turn, both as the
/// objects they are now (CR 400.7; [`crate::state::PerTurn::blocks`]).
pub(crate) fn blocked_this_turn(state: &GameState, blocker: ObjectId, attacker: ObjectId) -> bool {
    let (Some(b), Some(a)) = (
        last_battlefield_version(state, blocker),
        last_battlefield_version(state, attacker),
    ) else {
        return false;
    };
    state.per_turn.blocks.iter().any(|record| {
        record.blocker.object == blocker
            && record.blocker.version == b
            && record.attacker.object == attacker
            && record.attacker.version == a
    })
}

/// `id` as the permanent it is, or, once it has left the battlefield, as it
/// last existed there (CR 608.2h): Glyph of Doom's Wall that died in combat
/// still names the creatures it blocked. The version it had as it left is
/// the journal's record of the departure.
pub(crate) fn last_battlefield_version(state: &GameState, id: ObjectId) -> Option<u32> {
    let object = state.object(id)?;
    if object.zone == crate::zone::Zone::Battlefield {
        return Some(object.version);
    }
    state
        .journal
        .entries()
        .iter()
        .rev()
        .find_map(|entry| match &entry.event {
            crate::event::GameEvent::ZoneChanged {
                object,
                from: crate::zone::Zone::Battlefield,
                ..
            } if *object == id => entry.departure.as_ref().map(|d| d.version),
            _ => None,
        })
}

/// The incarnation of `source` the damage this turn counts against: the one
/// on the battlefield now, or the last one damaged before it left (a dies
/// trigger reads the creature that died, CR 603.10a).
fn damaged_incarnation(state: &GameState, source: ObjectId) -> Option<u32> {
    // The permanent as it last existed on the battlefield, not merely the
    // last one damaged: a creature damaged, blinked and then destroyed by
    // something else took no damage as the object that died (CR 400.7).
    last_battlefield_version(state, source)
}

/// Brine Hag's "creatures that dealt damage to it this turn": `obj`, as the
/// object it is now, dealt damage this turn to `source`'s incarnation.
fn dealt_damage_to_this_turn(
    state: &GameState,
    source: ObjectId,
    obj: &crate::object::GameObject,
) -> bool {
    let Some(version) = damaged_incarnation(state, source) else {
        return false;
    };
    state.per_turn.damage_log.iter().any(|e| {
        e.to == crate::event::DamageTarget::Object(source)
            && e.to_version == Some(version)
            && e.source.object == obj.id
            && e.source.version == obj.version
    })
}

/// `Amount::DamageThisTurn`: damage dealt this turn to `to` by sources the
/// filter matches as they were when they dealt it.
pub(crate) fn damage_this_turn(
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
    to: baylee_cards_dsl::DamageTo,
    by: &Filter,
    combat: Option<bool>,
) -> u32 {
    use baylee_cards_dsl::DamageTo;
    let source_incarnation = match to {
        DamageTo::Source => damaged_incarnation(state, context.source),
        _ => None,
    };
    state
        .per_turn
        .damage_log
        .iter()
        .filter(|e| combat.is_none_or(|c| c == e.combat))
        .filter(|e| match to {
            DamageTo::You => e.to == crate::event::DamageTarget::Player(you),
            DamageTo::Source => {
                e.to == crate::event::DamageTarget::Object(context.source)
                    && source_incarnation.is_some_and(|v| e.to_version == Some(v))
            }
            DamageTo::Anything => true,
        })
        .filter(|e| {
            matches_projected_with_context(
                by,
                state,
                &e.snapshot,
                e.snapshot.characteristics(),
                you,
                context,
            )
        })
        .map(|e| e.amount)
        .fold(0, u32::saturating_add)
}

/// The X announced for `this`, where `cast_wizard` and an activation write
/// it; 0 for a source that is gone or announced none. When an activated or
/// triggered ability of `this` is resolving on top of the stack with an
/// announced `x_value`, read that stack object's `x_value` first (`CR 107.3a`,
/// `CR 107.3m`, `CR 602.2b`).
pub(crate) fn announced_x(state: &GameState, this: ObjectId) -> u32 {
    if let Some(&top) = state.zones.list(crate::zone::ZoneLocation::Stack).last()
        && let Some(stack_obj) = state.object(top)
        && stack_obj.kind == crate::object::ObjectKind::AbilityOnStack
        && stack_obj.ability.is_some_and(|loc| loc.source == this)
        && (stack_obj.x_value > 0
            || (stack_obj.paid.is_some()
                && state
                    .object(this)
                    .is_none_or(|o| o.zone != crate::zone::Zone::Stack)))
    {
        return stack_obj.x_value;
    }
    state.object(this).map_or(0, |o| o.x_value)
}

/// Projected power of `this` on the battlefield, or as it last existed there
/// if it has left (CR 608.2h; Mentor CR 702.134b).
fn source_power(state: &GameState, this: ObjectId) -> Option<i16> {
    if let Some(obj) = state
        .object(this)
        .filter(|o| o.zone == crate::zone::Zone::Battlefield)
    {
        return obj.characteristics().power;
    }
    if let Some(version) = last_battlefield_version(state, this)
        && let Some(departed) = state.source_object(baylee_core::ids::DamageSourceRef {
            object: this,
            version,
        })
    {
        return departed.characteristics().power;
    }
    state.object(this).and_then(|o| o.characteristics().power)
}

/// Whether `stat` is less than the source's projected power (using last-known
/// battlefield power if the source left, CR 608.2h).
fn below_source_power(stat: Option<i16>, state: &GameState, this: ObjectId) -> bool {
    stat.zip(source_power(state, this))
        .is_some_and(|(stat, bound)| stat < bound)
}

fn source_mana_value(state: &GameState, this: ObjectId) -> Option<u32> {
    if let Some(&top) = state.zones.list(crate::zone::ZoneLocation::Stack).last()
        && let Some(stack_obj) = state.object(top)
        && stack_obj.kind == crate::object::ObjectKind::AbilityOnStack
        && stack_obj.ability.is_some_and(|loc| loc.source == this)
    {
        if let Some(version) = stack_obj.riders.iter().find_map(|r| match *r {
            crate::object::Rider::AbilitySourceVersion(v) => Some(v),
            _ => None,
        }) && let Some(src) = state.damage_source(this, Some(version))
        {
            return Some(src.characteristics().mana_value());
        }
        if let Some(discarded) = stack_obj
            .paid
            .as_ref()
            .and_then(|p| p.discarded.iter().find(|d| d.object == this).copied())
            && let Some(src) = state.source_object(discarded)
        {
            return Some(src.characteristics().mana_value());
        }
    }
    state
        .object_or_departed(this)
        .map(|o| o.characteristics().mana_value())
}

/// Whether `filter` reads the X announced for its source
/// ([`Filter::CmcAtMostX`], [`Filter::CmcExactlyX`]).
#[must_use]
pub fn reads_announced_x(filter: &Filter) -> bool {
    reads_announced_x_at(filter, 0)
}

fn reads_announced_x_at(filter: &Filter, depth: u8) -> bool {
    if depth >= MAX_DYNAMIC_FILTER_DEPTH {
        return false;
    }
    match filter {
        Filter::CmcAtMostX | Filter::CmcExactlyX | Filter::TwiceCmcExactlyX => true,
        Filter::CmcAtMostAmount(amt)
        | Filter::CmcEqualsAmount(amt)
        | Filter::PowerAtMostAmount(amt)
        | Filter::PowerAtLeastAmount(amt)
        | Filter::PowerEqualsAmount(amt)
        | Filter::ToughnessEqualsAmount(amt) => amount_reads_announced_x(amt, depth + 1),
        Filter::And(parts) | Filter::Or(parts) => {
            parts.iter().any(|f| reads_announced_x_at(f, depth))
        }
        Filter::Not(f) => reads_announced_x_at(f, depth),
        _ => false,
    }
}

fn amount_reads_announced_x(amt: &baylee_cards_dsl::Amount, depth: u8) -> bool {
    use baylee_cards_dsl::Amount as A;
    if depth >= MAX_DYNAMIC_FILTER_DEPTH {
        return false;
    }
    match amt {
        A::X | A::NegX | A::DoubleX | A::XPlusCommanderCasts | A::XIfSourceWasForetold { .. } => {
            true
        }
        A::Half { base: a, .. }
        | A::Divided { base: a, .. }
        | A::Times { base: a, .. }
        | A::Plus { base: a, .. }
        | A::Negated(a)
        | A::SaturatingSub { base: a, .. }
        | A::ConstantMinus { subtract: a, .. } => amount_reads_announced_x(a, depth + 1),
        A::Sum { left, right } | A::Difference { left, right } => {
            amount_reads_announced_x(left, depth + 1) || amount_reads_announced_x(right, depth + 1)
        }
        A::IfCondition {
            then, otherwise, ..
        } => {
            amount_reads_announced_x(then, depth + 1)
                || amount_reads_announced_x(otherwise, depth + 1)
        }
        A::DistinctColorsAmong(f)
        | A::BasicLandTypesAmong(f)
        | A::CountOf { filter: f, .. }
        | A::ControlledCount { filter: f, .. }
        | A::DamageThisTurn { by: f, .. }
        | A::GreatestPowerAmong(f)
        | A::GreatestManaValueAmong(f)
        | A::TotalPowerAmong(f)
        | A::EnteredThisTurn { filter: f, .. }
        | A::TokensCreatedThisTurn { filter: f, .. }
        | A::LeftZoneThisTurn { filter: f, .. }
        | A::PermanentsSacrificedThisTurn { filter: f, .. }
        | A::SpellsCastMatchingThisTurn { filter: f, .. }
        | A::CreaturesAttackedThisTurn { filter: f, .. }
        | A::DistinctManaValuesInZone { filter: f, .. }
        | A::DistinctPowersAmong(f)
        | A::HighestControlledCount { filter: f, .. }
        | A::TotalManaValueInZone { filter: f, .. }
        | A::DistinctExactManaCostsInZone { filter: f, .. } => reads_announced_x_at(f, depth + 1),
        _ => false,
    }
}

/// Whether some announced X would let `obj` match `filter`: the question an
/// offer asks of a spell like Spell Blast before its X exists. X is announced
/// (CR 601.2b) before targets are chosen (CR 601.2c), and the offer comes
/// before both, so "is there a target" can only mean "is there one for some
/// X".
///
/// The X atoms answer yes and every other atom is [`matches`]. A negated X
/// atom answers yes as well without being asked, which offers a spell whose
/// cast may then find nothing — the direction a wizard can reverse (CR
/// 601.2), where hiding a castable spell is not. No card negates one.
#[must_use]
pub fn matches_for_some_x(
    filter: &Filter,
    state: &GameState,
    obj: &GameObject,
    you: PlayerId,
    this: ObjectId,
) -> bool {
    matches_for_x(filter, state, obj, you, live_context(state, this), None)
}

/// Match an activation's own X without overwriting the permanent's cast X.
/// `None` is the pre-announcement probe: some X may satisfy each X atom.
pub(crate) fn matches_for_x(
    filter: &Filter,
    state: &GameState,
    obj: &GameObject,
    you: PlayerId,
    context: RuleContext,
    x: Option<u32>,
) -> bool {
    match filter {
        Filter::CmcAtMostX => x.is_none_or(|x| obj.characteristics().mana_value() <= x),
        Filter::CmcExactlyX => x.is_none_or(|x| obj.characteristics().mana_value() == x),
        Filter::TwiceCmcExactlyX => {
            x.is_none_or(|x| obj.characteristics().mana_value().saturating_mul(2) == x)
        }
        Filter::CmcAtMostAmount(amt)
        | Filter::CmcEqualsAmount(amt)
        | Filter::PowerAtMostAmount(amt)
        | Filter::PowerAtLeastAmount(amt)
        | Filter::PowerEqualsAmount(amt)
        | Filter::ToughnessEqualsAmount(amt)
            if amount_reads_announced_x(amt, 0) =>
        {
            let chars = obj.characteristics();
            let Some(x_val) = x else {
                return match filter {
                    Filter::PowerAtMostAmount(_)
                    | Filter::PowerAtLeastAmount(_)
                    | Filter::PowerEqualsAmount(_) => chars.power.is_some(),
                    Filter::ToughnessEqualsAmount(_) => chars.toughness.is_some(),
                    _ => true,
                };
            };
            let Some(_guard) = DynamicFilterDepthGuard::enter() else {
                return false;
            };
            let mag = amounts::amount_with_context(amt, state, you, context, Some(x_val));
            match_dynamic_amount_filter(filter, chars, mag, amt.is_negative())
        }
        Filter::And(parts) => parts
            .iter()
            .all(|f| matches_for_x(f, state, obj, you, context, x)),
        Filter::Or(parts) => parts
            .iter()
            .any(|f| matches_for_x(f, state, obj, you, context, x)),
        Filter::Not(f) if reads_announced_x(f) => {
            x.is_none() || !matches_for_x(f, state, obj, you, context, x)
        }
        other => matches_with_context(other, state, obj, you, context),
    }
}

/// CR 700.9: a creature on the battlefield (or in battlefield-departure LKI,
/// CR 603.10a / CR 608.2h) that has any counter on it, is equipped, or is
/// enchanted by an Aura controlled by that creature's controller.
fn object_is_modified(
    state: &GameState,
    obj: &GameObject,
    chars: &crate::object::Characteristics,
) -> bool {
    use baylee_core::generated::subtypes::{artifact::EQUIPMENT, enchantment::AURA};
    use baylee_core::types::TypeSet;
    if obj.zone == Zone::Battlefield {
        if obj.status.contains(crate::object::Status::PHASED_OUT)
            || !chars.types.contains(TypeSet::CREATURE)
        {
            return false;
        }
        if obj.counters.iter().any(|(_, n)| n > 0) {
            return true;
        }
        return state.battlefield_seen().any(|id| {
            state.object(id).is_some_and(|att| {
                att.attached_to == Some(obj.id)
                    && (att.characteristics().subtypes.contains(EQUIPMENT)
                        || (att.characteristics().subtypes.contains(AURA)
                            && att.controller == obj.controller))
            })
        });
    }
    let departed_battlefield = state
        .ltb_versions
        .iter()
        .any(|&(id, v)| id == obj.id && (v == obj.version || v.wrapping_add(1) == obj.version));
    if !departed_battlefield {
        return false;
    }
    let lki_chars = state.last_known_characteristics(obj.id).unwrap_or(chars);
    if !lki_chars.types.contains(TypeSet::CREATURE) {
        return false;
    }
    if obj.counters.iter().any(|(_, n)| n > 0)
        || state
            .ltb_counters
            .iter()
            .any(|(id, c)| *id == obj.id && c.iter().any(|(_, n)| n > 0))
    {
        return true;
    }
    let host_ctrl = state
        .last_known_controller(obj.id)
        .unwrap_or(obj.controller);
    state
        .ltb_attachments
        .iter()
        .find(|(host, _)| *host == obj.id)
        .is_some_and(|(_, worn)| {
            worn.iter().any(|&att_id| {
                if let Some(live) = state.object(att_id).filter(|o| o.zone == Zone::Battlefield) {
                    let sub = live.characteristics().subtypes;
                    return sub.contains(EQUIPMENT)
                        || (sub.contains(AURA) && live.controller == host_ctrl);
                }
                let Some(att_chars) = state.last_known_characteristics(att_id).or_else(|| {
                    state
                        .object_or_departed(att_id)
                        .map(GameObject::characteristics)
                }) else {
                    return false;
                };
                att_chars.subtypes.contains(EQUIPMENT)
                    || (att_chars.subtypes.contains(AURA)
                        && state.last_known_controller(att_id) == Some(host_ctrl))
            })
        })
}

#[must_use]
pub(crate) fn object_was_cast_from_zone(
    obj: &crate::object::GameObject,
    zone: baylee_cards_dsl::ZoneRef,
) -> bool {
    use baylee_cards_dsl::ZoneRef;
    if obj.cast_history.caster().is_none() {
        return false;
    }
    match zone {
        ZoneRef::Hand => obj.cast_history.was_from_hand(),
        ZoneRef::Graveyard => obj.cast_history.was_from_graveyard(),
        ZoneRef::Exile => obj.riders.contains(&crate::object::Rider::CastOriginZone(
            crate::zone::Zone::Exile,
        )),
        ZoneRef::Library => obj.riders.contains(&crate::object::Rider::CastOriginZone(
            crate::zone::Zone::Library,
        )),
        ZoneRef::Command => obj.riders.contains(&crate::object::Rider::CastOriginZone(
            crate::zone::Zone::Command,
        )),
        ZoneRef::NotBattlefield => true,
        ZoneRef::Battlefield | ZoneRef::Stack | ZoneRef::BattlefieldOrCommand => false,
    }
}

#[cfg(test)]
mod tests;
