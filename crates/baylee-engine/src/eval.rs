//! Evaluation of DSL data: filters, amounts, target options.
//!
//! All evaluation is pure read access to [`GameState`]; `you` is the
//! ability/spell controller, `this` its source object.

mod stack_filters;
pub(crate) mod target_domain;
pub(crate) mod target_pair;

use crate::object::{GameObject, Status};
use crate::state::GameState;
use crate::text_changes::{RuleContext, TextChangeMap};
use crate::zone::{Zone, ZoneLocation};
use baylee_cards_dsl::{Amount, Condition, Filter, PlayerRel, TargetSpec, ZoneSel};
use baylee_core::ids::{ObjectId, PlayerId};

/// Printed rules use the live source's words; captured stack rules use an
/// explicit [`RuleContext`] instead.
#[must_use]
#[inline]
pub fn live_context(state: &GameState, source: ObjectId) -> RuleContext {
    if state.text_changes.is_empty() {
        return RuleContext {
            source,
            text: TextChangeMap::IDENTITY,
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
#[allow(clippy::too_many_lines)] // One arm per Filter variant, with no wildcard.
pub fn matches_projected_with_context(
    filter: &Filter,
    state: &GameState,
    obj: &GameObject,
    chars: &crate::object::Characteristics,
    you: PlayerId,
    context: RuleContext,
) -> bool {
    if obj.is_merged_component() {
        return false;
    }
    let this = context.source;
    let matches = |f: &Filter| matches_projected_with_context(f, state, obj, chars, you, context);
    match &context.text.filter_leaf(*filter) {
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
            state.object(this).is_some_and(|source| source.watermark() == Some(mark))
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
        Filter::NamedBySource => state.object(this).is_some_and(|source| {
            let current = source.rules_acquisition();
            source.riders.iter().any(|rider| {
            matches!(rider, crate::object::Rider::ChosenName { acquisition, name, .. } if *acquisition == current && *name == chars.name)
        })
        }),
        Filter::IsToken => !obj.is_card(),
        Filter::ActivatedAbilityFrom(filter) => {
            stack_filters::activated_source_matches(filter, state, obj, you, context)
        }
        Filter::TargetsPermanentYouControl => {
            obj.targets.iter().chain(obj.second_targets()).any(|id| {
                state.object(*id).is_some_and(|target| {
                    target.zone == Zone::Battlefield && target.controller == you
                }) && state.targets_current_object(obj.id, *id)
            })
        }
        Filter::WithSingleTarget => {
            let players = obj.target_players.len()
                + usize::from(
                    obj.chosen_player
                        .is_some_and(|p| !obj.target_players.contains(p)),
                );
            obj.targets.len() + obj.second_targets().len() + players + obj.second_target_players().len() == 1
        }
        Filter::HasManaAbility => {
            (chars.has_mana_ability && chars.abilities_lost.is_none() && !chars.rules_text_lost)
                || !crate::casting::intrinsic_mana_colors(state, obj.id).is_empty()
                || crate::effects::granted_activated(state, obj.id).any(|a| a.mana_ability)
        }
        Filter::ControlledByYou => obj.controller == you,
        Filter::ControlledByOpponent => state.is_opponent(obj.controller, you),
        Filter::ControlledByActivePlayer => obj.controller == state.turn.active,
        // An attacking source's own defending player (CR 508.5: Floral
        // Spuzzem's "target artifact defending player controls" at a table of
        // three is the one it attacks); for anything else, any opponent of
        // the attacking player during combat, as before.
        Filter::ControlledByDefendingPlayer => {
            if last_defender(state, this).is_some() {
                crate::combat::historical_defending_player(state, this) == Some(obj.controller)
            } else {
                state.turn.phase == crate::turn::Phase::Combat
                    && state.is_opponent(obj.controller, state.turn.active)
            }
        }
        Filter::OwnedByYou => obj.owner == you,
        Filter::OwnedBySourceOwner => state.object_or_departed(this).is_some_and(|source| source.owner == obj.owner),
        Filter::Tapped => obj.status.contains(Status::TAPPED),
        Filter::Untapped => !obj.status.contains(Status::TAPPED),
        // A lookup and not a scan: this arm runs once per object whenever a
        // filter is walked over the battlefield (`CombatState`'s doc).
        Filter::Attacking => state.combat.is_attacking(obj.id),
        Filter::AttackingYou => state.combat.attackers().iter().any(|attacker| {
            attacker.creature == obj.id
                && attacker.defending == baylee_core::ids::Defender::Player(you)
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
        // The object's own counters. A leaves-the-battlefield trigger asks
        // the object as it last existed there, and `trigger::departed_matches`
        // hands in that object, counters and all.
        Filter::HasCounter(kind) => obj.counters.get(*kind) > 0,
        Filter::IsCommander => !state.merged_commanders(obj).is_empty(),
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
                        }
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
                        fx.modifier,
                        baylee_cards_dsl::Modifier::GrantTriggered {
                            ref trigger,
                            effects,
                            ..
                        } if is_cu(trigger, effects)
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
        Filter::BlockedByReferenceThisTurn => context.reference.is_some_and(|reference| {
            blocked_this_turn(state, reference, obj.id)
        }),
        Filter::BlockedReferenceThisTurn => context.reference.is_some_and(|reference| {
            blocked_this_turn(state, obj.id, reference)
        }),
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
        Filter::SharesCreatureTypeWithOtherTarget => context.reference.is_none_or(|reference| {
            reference != obj.id && state.object(reference).is_some_and(|other| {
                target_pair::shares(chars, other.characteristics())
            })
        }),
        Filter::DealtDamageToSourceThisTurn => dealt_damage_to_this_turn(state, this, obj),
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
        Filter::MatchesChosenTypeOfSource => state
            .object(this)
            .and_then(|src| src.chosen_subtype)
            .is_some_and(|s| chars.subtypes.contains(s)),
        Filter::OfSourceChosenColor => {
            use baylee_core::color::Color;
            use baylee_core::mana::ManaColor;
            let chosen = match state.object(this).and_then(|src| src.chosen_color) {
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
        Filter::IsAttached => obj.attached_to.is_some(),
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
        Filter::Kicked => obj.kicked,
        Filter::CastFromHand => obj.cast_history.was_from_hand(),
        Filter::CmcAtMost(n) => chars.mana_value() <= *n,
        // The bound is the announced X on the ability's own source, which is
        // where `cast_wizard` writes it and what `res.x` is read from one
        // layer up. A source that is gone, or that announced nothing, bounds
        // at 0 rather than at everything: an unreadable bound that found the
        // whole library would be a tutor with no price.
        Filter::CmcAtMostX => chars.mana_value() <= announced_x(state, this),
        Filter::CmcExactlyX => chars.mana_value() == announced_x(state, this),
        Filter::TwiceCmcExactlyX => chars.mana_value().saturating_mul(2) == announced_x(state, this),
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
        Filter::PowerAtMostSourcePower => chars.power.is_some_and(|power| {
            state
                .object(this)
                .is_some_and(|source| power <= source.characteristics().power.unwrap_or(0))
        }),
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
    }
}

/// Who controls the permanent `source` is attached to: "enchanted land's
/// controller", which need not be the Aura's own (CR 303.4e). `None` when the
/// source is gone or attached to nothing, or its host has left the game's
/// players.
#[must_use]
pub fn controller_of_attached(state: &GameState, source: ObjectId) -> Option<PlayerId> {
    let host = state.object(source)?.attached_to?;
    let seat = state.object(host)?.controller;
    (!state.has_left(seat)).then_some(seat)
}

/// Resolves a relative player reference to concrete players — or `None` for
/// relations the game state alone cannot answer.
///
/// `Chosen` is the player a spell or ability targeted and `ControllerOfTarget`
/// is read off its first object target, so both need the *resolution's* own
/// context and only [`crate::resolve::players_of`] has it. The `None` is the
/// whole point of the signature: this used to answer `vec![]` there, which
/// reads exactly like "no seats matched" and is what shipped Abraded Bluffs
/// as a land that deals no damage, Twining Twins' ward as a keyword that never
/// asks for its tax, Path to Exile without its ramp and Bojuka Bog as a swamp.
/// A caller that has a `Resolution` must not be able to swallow that silently.
#[must_use]
pub fn players(rel: PlayerRel, state: &GameState, you: PlayerId) -> Option<Vec<PlayerId>> {
    Some(match rel {
        PlayerRel::You => vec![you],
        PlayerRel::Opponent | PlayerRel::EachOpponent => state
            .players
            .iter()
            .filter(|p| state.is_opponent(p.id, you) && !p.has_lost())
            .map(|p| p.id)
            .collect(),
        PlayerRel::EachPlayer => state
            .players
            .iter()
            .filter(|p| !p.has_lost())
            .map(|p| p.id)
            .collect(),
        PlayerRel::EachOtherPlayer => state
            .players
            .iter()
            .filter(|p| p.id != you && !p.has_lost())
            .map(|p| p.id)
            .collect(),
        PlayerRel::ActivePlayer => state
            .players
            .iter()
            .filter(|p| p.id == state.turn.active && !p.has_lost())
            .map(|p| p.id)
            .collect(),
        PlayerRel::MostLife => most_life(state).into_iter().collect(),
        PlayerRel::MostLifeOrTied => {
            let best = state
                .players
                .iter()
                .filter(|p| !p.has_lost())
                .map(|p| p.life)
                .max();
            state
                .players
                .iter()
                .filter(|p| !p.has_lost() && Some(p.life) == best)
                .map(|p| p.id)
                .collect()
        }
        PlayerRel::ControllerOfTarget
        | PlayerRel::TargetPlayerOrController
        | PlayerRel::ControllerOfEvent
        | PlayerRel::Recipient
        | PlayerRel::DefendingPlayer
        | PlayerRel::OwnerOfEvent
        | PlayerRel::Chosen
        | PlayerRel::DamagedPlayer
        | PlayerRel::EventPlayer
        | PlayerRel::OwnerOfSource
        | PlayerRel::ControllerOfAttached => {
            return None;
        }
    })
}

/// The one player still in the game with more life than each other one
/// ("the player with the most life", Ghazbán Ogre), or `None` on a tie.
#[must_use]
pub fn most_life(state: &GameState) -> Option<PlayerId> {
    let mut alive = state.players.iter().filter(|p| !p.has_lost());
    let first = alive.next()?;
    let (mut best, mut tied) = (first, false);
    for p in alive {
        if p.life > best.life {
            best = p;
            tied = false;
        } else if p.life == best.life {
            tied = true;
        }
    }
    (!tied).then_some(best.id)
}

/// "Defending player" as an ability of `source` means it (CR 508.5): the
/// player `source` is attacking, or the controller of the planeswalker or
/// protector of the battle it is attacking, and once it has been removed
/// from combat, the one it was attacking before
/// ([`crate::combat::CombatState::last_defender_of`]). A source that did
/// not attack this combat names the one opponent of a two-player game
/// during combat (CR 506.2), and nobody at a larger table, where CR 508.5a
/// makes it one specific player that no attack names.
#[must_use]
pub fn defending_player_of(state: &GameState, source: ObjectId, you: PlayerId) -> Option<PlayerId> {
    // Attacking, or attacking earlier this combat and removed since (CR
    // 508.5: "the player that creature was attacking before it was removed
    // from combat").
    if last_defender(state, source).is_some() {
        return crate::combat::historical_defending_player(state, source)
            .filter(|p| !state.has_left(*p));
    }
    if state.turn.phase != crate::turn::Phase::Combat {
        return None;
    }
    let mut opponents = state
        .players
        .iter()
        .filter(|p| state.is_opponent(p.id, you) && !p.has_lost());
    match (opponents.next(), opponents.next()) {
        (Some(only), None) => Some(only.id),
        _ => None,
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

/// The graveyard cards a `CardInGraveyard` spec may point at.
///
/// Legality is enumerated *before* any resolution exists, so the two context
/// relations have no answer here and an empty list is the honest one — this
/// is the **only** caller allowed to read [`players`]' `None` as "nobody".
/// [`players`] has four callers in all and the other three
/// (`resolve::players_of` and two in `team_tests`) `expect` a relation the
/// state can answer.
///
/// `Chosen` is the exception, and it is not a context relation here: it is
/// "from a single graveyard" (Unlicensed Hearse), and the graveyard is chosen
/// as part of choosing the targets. So every graveyard is enumerated — what
/// the offer counts, and what CR 608.2b re-checks a target against — and
/// `start_activation` narrows the list to the graveyard its player named.
fn graveyard_options(
    filter: &Filter,
    rel: PlayerRel,
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
) -> Vec<ObjectId> {
    let rel = if rel == PlayerRel::Chosen {
        PlayerRel::EachPlayer
    } else {
        rel
    };
    let seats = if rel == PlayerRel::DefendingPlayer {
        defending_player_of(state, context.source, you)
            .into_iter()
            .collect()
    } else {
        let Some(seats) = players(rel, state, you) else {
            return Vec::new();
        };
        seats
    };
    let mut out = Vec::new();
    for player in seats {
        out.extend(
            state
                .zones
                .list(ZoneLocation::Graveyard(player))
                .iter()
                .filter(|id| {
                    state
                        .object(**id)
                        .is_some_and(|o| matches_with_context(filter, state, o, you, context))
                })
                .copied(),
        );
    }
    out
}

/// "The tapped creature's power" (station, CR 702.184a): the permanent the
/// payment of `paid_on`'s cost tapped, at its power now if it is still on
/// the battlefield as that object, and as it last existed there otherwise
/// (CR 608.2h). Negative power puts no counters (CR 107.1b); nothing
/// tapped, 0.
#[must_use]
pub fn tapped_power(state: &GameState, paid_on: ObjectId) -> u32 {
    let Some((tapped, version)) = state
        .object(paid_on)
        .and_then(|o| o.paid.as_ref())
        .and_then(|p| p.tapped)
    else {
        return 0;
    };
    let power = state
        .object(tapped)
        .filter(|o| o.zone == crate::zone::Zone::Battlefield && o.version == version)
        .map(|o| o.characteristics().power.unwrap_or(0))
        .or_else(|| {
            state
                .ltb_powers
                .iter()
                .find(|(id, _)| *id == tapped)
                .map(|(_, power)| *power)
        })
        .unwrap_or(0);
    u32::try_from(power.max(0)).unwrap_or(0)
}

/// Evaluates an [`Amount`].
#[must_use]
pub fn amount(
    amount: &Amount,
    state: &GameState,
    you: PlayerId,
    this: ObjectId,
    x: Option<u32>,
) -> u32 {
    amount_with_context(amount, state, you, live_context(state, this), x)
}

/// Evaluate an amount using the captured semantic text for its nested filters.
#[must_use]
#[allow(clippy::too_many_lines)] // One arm per `Amount` variant; splitting it hides the match.
pub fn amount_with_context(
    amount: &Amount,
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
    x: Option<u32>,
) -> u32 {
    let this = context.source;
    match amount {
        Amount::Fixed(n) | Amount::NegXFixed(n) => *n,
        Amount::CreaturesEnteredUnderYourControlThisTurn => state.per_turn.creatures_entered
            .get(usize::from(you.get())).copied().unwrap_or(0),
        Amount::X | Amount::NegX => x.unwrap_or(0),
        // The magnitude, like every other arm here: the sign is
        // `Amount::is_negative`'s question and no caller of this reads one.
        Amount::Negated(inner) => self::amount_with_context(inner, state, you, context, x),
        Amount::Plus { base, offset } => {
            self::amount_with_context(base, state, you, context, x).saturating_add(*offset)
        }
        Amount::SaturatingSub { base, subtract } => {
            self::amount_with_context(base, state, you, context, x).saturating_sub(*subtract)
        }
        Amount::ConstantMinus { constant, subtract } => {
            constant.saturating_sub(self::amount_with_context(subtract, state, you, context, x))
        }
        Amount::GreatestCommanderManaValue => greatest_commander_mana_value(state, you),
        Amount::HandSize(who) => players(*who, state, you)
            .and_then(|p| p.first().copied())
            .map_or(0, |p| {
                u32::try_from(state.zones.list(ZoneLocation::Hand(p)).len()).unwrap_or(u32::MAX)
            }),
        Amount::ControlledCount { who, filter } => players(*who, state, you)
            .and_then(|p| p.first().copied())
            .map_or(0, |controller| {
                count_controlled_by(state, controller, you, filter, context)
            }),
        Amount::LibrarySize(who) => players(*who, state, you)
            .and_then(|p| p.first().copied())
            .map_or(0, |p| {
                u32::try_from(state.zones.list(ZoneLocation::Library(p)).len()).unwrap_or(u32::MAX)
            }),
        Amount::Half { base, round_up } => {
            let n = self::amount_with_context(base, state, you, context, x);
            if *round_up { n.div_ceil(2) } else { n / 2 }
        }
        Amount::Times { base, factor } => {
            self::amount_with_context(base, state, you, context, x).saturating_mul(*factor)
        }
        Amount::Sum { left, right } => self::amount_with_context(left, state, you, context, x)
            .saturating_add(self::amount_with_context(right, state, you, context, x)),
        Amount::Difference { left, right } => {
            self::amount_with_context(left, state, you, context, x)
                .saturating_sub(self::amount_with_context(right, state, you, context, x))
        }
        Amount::Players(who) => players(*who, state, you)
            .map_or(0, |seats| u32::try_from(seats.len()).unwrap_or(u32::MAX)),
        Amount::DoubleX => x.unwrap_or(0).saturating_mul(2),
        Amount::XPlusCommanderCasts => {
            x.unwrap_or(0)
                + state
                    .commander_casts
                    .get(you.get() as usize)
                    .copied()
                    .unwrap_or(0)
        }
        Amount::DistinctColorsAmong(filter) => distinct_colors(state, you, filter, context),
        Amount::BasicLandTypesAmong(filter) => basic_land_types(state, you, filter, context),
        Amount::BlockersBeyondFirst => {
            u32::try_from(state.combat.blocker_count(this).saturating_sub(1)).unwrap_or(u32::MAX)
        }
        Amount::SourcePower => state
            .object(this)
            .and_then(|o| o.characteristics().power)
            .map_or(0, |p| p.max(0) as u32),
        Amount::CountersOnYou(kind) => state.players[usize::from(you.get())]
            .counters()
            .into_iter()
            .find_map(|(k, n)| (k == *kind).then_some(u32::from(n)))
            .unwrap_or(0),
        Amount::CountersOnSource(kind) => state
            .object(this)
            .map_or(0, |o| u32::from(o.counters.get(*kind))),
        // Resolved in resolve.rs, which has the stack object these read.
        Amount::DevotionToManaColor
        | Amount::TargetPower
        | Amount::TargetToughness
        | Amount::TargetCmc
        | Amount::EventLastToughness
        | Amount::EventPower
        | Amount::EventManaValue
        | Amount::EventAmount
        | Amount::TargetsPutIntoGraveyard
        | Amount::ThisWay(_)
        | Amount::ThisWayBy { .. }
        // A payment's sacrifices, read off the stack object
        // (`resolve::amount2`).
        | Amount::SacrificedTotalPower => 0,
        Amount::DamageThisTurn { to, by, combat } => {
            damage_this_turn(state, you, context, *to, by, *combat)
        }
        // The object the payment wrote it on. A resolution asks
        // `resolve::amount2`, which reads the stack object: an activated
        // ability's source is the permanent and its payment is on the
        // ability. Here, with no resolution, `this` is all there is, which
        // answers for a spell (its own source) and 0 for anything else.
        Amount::SacrificedManaValue | Amount::SacrificedPower | Amount::SacrificedToughness => {
            crate::resolve::sacrificed_amount(state.object(this), amount)
        }
        Amount::ManaSpentToCast => state
            .object(this)
            .and_then(|o| o.paid.as_ref())
            .map_or(0, |p| p.mana_spent),
        Amount::TappedPower => tapped_power(state, this),
        Amount::CreaturesDiedThisTurn => state.per_turn.creatures_died,
        Amount::UntappedLandsAtTurnStart => state.per_turn.untapped_lands_at_start,
        Amount::DamageDealtToYouThisTurn => state
            .per_turn
            .damage_dealt_to
            .get(you.get() as usize)
            .copied()
            .unwrap_or(0),
        Amount::CountOf { filter, zone } => count_in_zone(filter, *zone, state, you, context),
        Amount::OpponentsLifeLostThisTurn => opponents_life_lost(state, you),
    }
}

fn greatest_commander_mana_value(state: &GameState, you: PlayerId) -> u32 {
    state
        .commanders
        .get(you.get() as usize)
        .into_iter()
        .flatten()
        .filter_map(|commander| state.object(commander.object))
        .map(|object| object.characteristics().mana_value())
        .max()
        .unwrap_or(0)
}

fn opponents_life_lost(state: &GameState, you: PlayerId) -> u32 {
    state
        .players
        .iter()
        .filter(|p| !state.has_left(p.id) && state.is_opponent(p.id, you))
        .fold(0u32, |total, p| {
            total.saturating_add(state.per_turn.life_lost_amount[usize::from(p.id.get())])
        })
}

fn distinct_colors(state: &GameState, you: PlayerId, filter: &Filter, context: RuleContext) -> u32 {
    u32::from(colors_among(state, you, filter, context).len())
}

/// Distinct represented colors, with duplicates collapsed in a five-bit set.
pub(crate) fn colors_among(
    state: &GameState,
    you: PlayerId,
    filter: &Filter,
    context: RuleContext,
) -> baylee_core::color::ColorSet {
    let mut colors = baylee_core::color::ColorSet::EMPTY;
    for id in state.battlefield_seen() {
        if let Some(obj) = state.object(id)
            && matches_with_context(filter, state, obj, you, context)
        {
            colors = colors.union(obj.characteristics().colors);
        }
    }
    colors
}

fn basic_land_types(
    state: &GameState,
    you: PlayerId,
    filter: &Filter,
    context: RuleContext,
) -> u32 {
    let mut seen = baylee_core::types::SubtypeSet::EMPTY;
    for id in state.battlefield_seen() {
        if let Some(obj) = state.object(id)
            && matches_with_context(filter, state, obj, you, context)
        {
            seen.union_with(obj.characteristics().subtypes);
        }
    }
    // Domain counts distinct types, not lands: union before intersecting
    // with CR 305.6's shared set of basic land types.
    baylee_core::types::SubtypeSet::BASIC_LANDS
        .iter()
        .filter(|t| seen.contains(*t))
        .count() as u32
}

/// Cards in exile linked to `context.source` (at `source_version` if given,
/// or its current version otherwise) that match `filter`.
pub(crate) fn linked_exile_cards(
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
    source_version: Option<u32>,
    filter: &Filter,
) -> Vec<ObjectId> {
    let source = context.source;
    let source_ref = source_version
        .map(|version| baylee_core::ids::DamageSourceRef {
            object: source,
            version,
        })
        .or_else(|| state.source_identity(source));
    let mut out = Vec::new();
    for player in &state.players {
        for &id in state.zones.list(ZoneLocation::Exile(player.id)) {
            if let Some(obj) = state.object(id)
                && obj.is_card()
                && obj.riders.iter().any(|r| match *r {
                    crate::object::Rider::Linked { host, .. } => {
                        host == source
                            && source_ref.is_none_or(|s| {
                                state
                                    .source_memory
                                    .linked
                                    .get(&baylee_core::ids::DamageSourceRef {
                                        object: obj.id,
                                        version: obj.version,
                                    })
                                    .is_none_or(|&linked| linked == s)
                            })
                    }
                    crate::object::Rider::ExiledWith { host, version } => {
                        host == source
                            && source_ref.is_none_or(|s| {
                                version == u16::try_from(s.version).unwrap_or(u16::MAX)
                            })
                    }
                    _ => false,
                })
                && matches_with_context(filter, state, obj, you, context)
            {
                out.push(id);
            }
        }
    }
    out
}

/// Visits visible objects in a selected zone without copying its index.
fn visit_zone(
    zone: ZoneSel,
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
    mut visit: impl FnMut(ObjectId),
) {
    let location = match zone {
        ZoneSel::Battlefield => {
            state.battlefield_seen().for_each(visit);
            return;
        }
        ZoneSel::GraveyardOpponents => {
            for player in &state.players {
                if !player.has_lost() && state.is_opponent(player.id, you) {
                    for &id in state.zones.list(ZoneLocation::Graveyard(player.id)) {
                        visit(id);
                    }
                }
            }
            return;
        }
        ZoneSel::GraveyardAll => {
            for player in &state.players {
                for &id in state.zones.list(ZoneLocation::Graveyard(player.id)) {
                    visit(id);
                }
            }
            return;
        }
        ZoneSel::LinkedExile => {
            for id in linked_exile_cards(state, you, context, None, &Filter::Any) {
                visit(id);
            }
            return;
        }
        ZoneSel::LibraryYou => ZoneLocation::Library(you),
        ZoneSel::GraveyardYou => ZoneLocation::Graveyard(you),
        ZoneSel::HandActivePlayer => ZoneLocation::Hand(state.turn.active),
        ZoneSel::HandYou => ZoneLocation::Hand(you),
        ZoneSel::HandChosen => {
            if let Some(chosen) = state.object(context.source).and_then(|o| o.chosen_player) {
                ZoneLocation::Hand(chosen)
            } else {
                return;
            }
        }
    };
    for &id in state.zones.list(location) {
        visit(id);
    }
}

/// Colors of matching objects in the selected zone; colorless contributes none.
pub(crate) fn colors_in_zone(
    filter: &Filter,
    zone: ZoneSel,
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
) -> baylee_core::color::ColorSet {
    let mut colors = baylee_core::color::ColorSet::EMPTY;
    visit_zone(zone, state, you, context, |id| {
        if let Some(object) = state.object(id)
            && matches_with_context(filter, state, object, you, context)
        {
            colors = colors.union(object.characteristics().colors);
        }
    });
    colors
}

/// Counts matching objects in the selected zone from this ability's perspective.
fn count_in_zone(
    filter: &Filter,
    zone: ZoneSel,
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
) -> u32 {
    let mut count = 0u32;
    visit_zone(zone, state, you, context, |id| {
        if state
            .object(id)
            .is_some_and(|object| matches_with_context(filter, state, object, you, context))
        {
            count = count.saturating_add(1);
        }
    });
    count
}

/// Whether a card's stated [`Condition`] holds right now.
///
/// One vocabulary and one reading, for every ability kind that states a
/// condition. It began as `Engine::check_activation_condition`, a method
/// answering it for `AbilityDef::ActivatedConditional` alone — but a
/// condition is a sentence about the game and not about how the ability
/// gets used, and the intervening-`if` clause of CR 603.4 asks the same
/// sentence of a *triggered* ability. A second copy over there would have
/// been the third byte-identical reading of a DSL predicate this engine has
/// grown, which is the shape [`crate::effects::applies_to`] was extracted
/// to stop.
///
/// A free function over [`GameState`] rather than a method, because the
/// callers no longer share a receiver: `crate::trigger::collect` is handed
/// a state and a lookup and has no `Engine` to ask.
///
/// `you` is the ability's controller and `source` the object it is printed
/// on; both matter — `ControlCount` counts one player's battlefield and
/// `CountersOnSelf` reads one permanent. A `source` that has left the game
/// answers **false** for the counter conditions, which is the honest
/// answer: a permanent that is gone has no counters on it.
///
/// **It reads the state it is called in**, which is what the two CR 603.4
/// checks want and is worth saying because the first of them is not quite
/// the trigger event's own moment: triggers are collected in a sweep over
/// new journal entries, so a condition is read when the sweep runs rather
/// than at the instant the event happened. For a turn-based trigger — the
/// upkeep, which is where this clause is commonest — the sweep is the next
/// thing that runs and the two are the same moment.
#[must_use]
#[allow(clippy::too_many_lines)] // one arm per `Condition`: the match is the list
pub fn condition_holds(
    state: &GameState,
    you: PlayerId,
    source: ObjectId,
    condition: Condition,
) -> bool {
    condition_holds_with_context(state, you, live_context(state, source), condition)
}

/// Evaluate a condition using an ability's captured or grant-origin wording.
#[must_use]
#[allow(clippy::too_many_lines)] // One arm per Condition variant.
pub fn condition_holds_with_context(
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
    condition: Condition,
) -> bool {
    let source = context.source;
    match condition {
        Condition::DescendedThisTurn => state
            .per_turn
            .descended
            .get(usize::from(you.get()))
            .copied()
            .unwrap_or(false),
        Condition::SacrificedSubtypeThisTurn { subtype, count } => {
            state
                .per_turn
                .sacrificed_subtypes
                .get(&(you, subtype))
                .copied()
                .unwrap_or(0)
                >= count
        }
        Condition::SourceWasCastByYou => state
            .object(source)
            .is_some_and(|object| object.cast_history.caster() == Some(you)),
        Condition::DiscardedForCost(filter) => state
            .object(source)
            .and_then(|object| object.paid.as_ref())
            .is_some_and(|paid| {
                paid.discarded.iter().any(|reference| {
                    state.source_object(*reference).is_some_and(|object| {
                        matches_with_context(filter, state, object, you, context)
                    })
                })
            }),
        Condition::SacrificedForCost(filter) => state
            .object(source)
            .and_then(|object| object.paid.as_ref())
            .and_then(|paid| paid.sacrificed)
            .and_then(|(object, version)| {
                state.source_object(baylee_core::ids::DamageSourceRef { object, version })
            })
            .is_some_and(|object| matches_with_context(filter, state, object, you, context)),
        // These need the triggering incarnation, supplied separately, or a
        // resolution (`resolve::immediate::resolution_condition`).
        Condition::ControlCountExceptEvent(_, _)
        | Condition::EventBlockedByOthers { .. }
        | Condition::TargetSpellWouldDestroy(_) => false,
        // "Only after combat": a main or ending phase after this turn's
        // combat (the postcombat main phase or the end phase).
        Condition::ChosenOpponentControls(filter) => state
            .object(source)
            .and_then(crate::object::GameObject::chosen_opponent)
            .filter(|p| !state.has_left(*p))
            .is_some_and(|opponent| {
                state.battlefield_seen().any(|id| {
                    state.object(id).is_some_and(|o| {
                        o.controller == opponent
                            && matches_with_context(filter, state, o, you, context)
                    })
                })
            }),
        Condition::AfterCombat => matches!(
            state.turn.phase,
            crate::turn::Phase::SecondMain | crate::turn::Phase::Ending
        ),
        Condition::Revolt => state.per_turn.revolt[usize::from(you.get())],
        Condition::CastInstantOrSorceryThisTurn => {
            state.per_turn.instant_or_sorcery_cast[usize::from(you.get())]
        }
        Condition::OpponentWasDealtAtLeast(minimum) => state.players.iter().any(|p| {
            let player = p.id;
            !state.has_left(player)
                && state.is_opponent(player, you)
                && state.per_turn.damage_dealt_to[usize::from(player.get())] >= minimum
        }),
        Condition::EveryHandEmpty => state.players.iter().all(|p| {
            let player = p.id;
            state.has_left(player) || state.zones.list(ZoneLocation::Hand(player)).is_empty()
        }),
        Condition::AnyLibraryAtMost(maximum) => state.players.iter().any(|p| {
            let player = p.id;
            !state.has_left(player)
                && state.zones.list(ZoneLocation::Library(player)).len() <= maximum as usize
        }),
        Condition::ControlledCreaturePowerAtLeast(minimum) => {
            state
                .battlefield_seen()
                .filter_map(|id| state.object(id))
                .filter(|o| {
                    o.controller == you
                        && o.characteristics()
                            .types
                            .contains(baylee_core::types::TypeSet::CREATURE)
                })
                .map(|o| i64::from(o.characteristics().power.unwrap_or(0)))
                .sum::<i64>()
                >= i64::from(minimum)
        }
        Condition::SourceAttackedDuringYourLastTurn => state.object(source).is_some_and(|object| {
            state.players[usize::from(you.get())]
                .last_turn_attackers
                .contains(&baylee_core::ids::DamageSourceRef {
                    object: source,
                    version: object.version,
                })
        }),
        Condition::AttackedWithAtLeast(minimum) => {
            let mut seen = std::collections::BTreeSet::new();
            state
                .per_turn
                .attacked
                .iter()
                .filter(|&&(id, version, player)| player == you && seen.insert((id, version)))
                .take(minimum as usize)
                .count()
                == minimum as usize
        }
        Condition::SourceStartedTurnUntapped => state.object(source).is_some_and(|object| {
            object.zone == Zone::Battlefield
                && object
                    .status
                    .contains(crate::object::Status::UNTAPPED_AT_TURN_START)
        }),
        Condition::SourceDamagedOpponentThisTurn => state.object(source).is_some_and(|object| {
            state
                .per_turn
                .player_damage
                .iter()
                .any(|(identity, player)| {
                    identity.object == source
                        && identity.version == object.version
                        && state.is_opponent(*player, you)
                })
        }),
        Condition::OpponentPoisonAtLeast(minimum) => state
            .players
            .iter()
            .any(|player| state.is_opponent(player.id, you) && player.poison >= minimum),
        Condition::AmountAtLeast(amount, minimum) => {
            amount_with_context(&amount, state, you, context, None) >= minimum
        }
        Condition::CreaturesDiedThisTurnAtLeast(n) => state.per_turn.creatures_died >= n,
        Condition::EnteredSubtypeUnderYourControlThisTurn(types) => state
            .per_turn
            .entered_subtypes
            .get(usize::from(you.get()))
            .is_some_and(|entered| types.iter().any(|kind| entered.contains(*kind))),
        Condition::DuringYourFirstTurns(maximum) => {
            state.turn.active == you
                && (1..=maximum).contains(&state.players[usize::from(you.get())].turns_started)
        }
        Condition::YourTurn => state.turn.active == you,
        Condition::LifeAtMost(n) => state.players[you.get() as usize].life <= n,
        Condition::APlayerHasMostLife => most_life(state).is_some(),
        // The announced X on the source, where `cast_wizard` writes it and
        // where `Filter::CmcAtMostX` reads it.
        Condition::XAtLeast(n) => state.object(source).map_or(0, |o| o.x_value) >= n,
        // No last turn at the first upkeep of the game, so nothing was cast
        // in it and nothing wasn't: both sentences are false there.
        Condition::NoSpellsCastLastTurn => {
            state.previous_turn.is_some_and(|p| p.spells_by_all == 0)
        }
        Condition::YouCastNoSpellThisTurn => state.per_turn.spells_cast_by(you) == 0,
        Condition::APlayerCastLastTurnAtLeast(n) => state
            .previous_turn
            .is_some_and(|p| p.most_by_one >= u32::from(n)),
        Condition::ControlCount(filter, min) => {
            let count = state
                .battlefield_seen()
                .filter(|&id| {
                    state.object(id).is_some_and(|o| {
                        o.controller == you
                            && matches_with_context(
                                filter,
                                state,
                                o,
                                you,
                                RuleContext {
                                    source: id,
                                    ..context
                                },
                            )
                    })
                })
                .count();
            count >= min as usize
        }
        // The same walk as above with the comparison turned round, written
        // out rather than shared: `count >= min` and `count <= max` read
        // the identical board and a helper taking an ordering would put the
        // one thing that differs behind a parameter.
        Condition::ControlCountAtMost(filter, max) => {
            let count = state
                .battlefield_seen()
                .filter(|&id| {
                    state.object(id).is_some_and(|o| {
                        o.controller == you
                            && matches_with_context(
                                filter,
                                state,
                                o,
                                you,
                                RuleContext {
                                    source: id,
                                    ..context
                                },
                            )
                    })
                })
                .count();
            count <= max as usize
        }
        // The two walks above over the whole battlefield: nobody's side of
        // the table in particular.
        Condition::BattlefieldCount(filter, min) => {
            battlefield_count(state, you, filter, context) >= min as usize
        }
        Condition::BattlefieldCountAtMost(filter, max) => {
            battlefield_count(state, you, filter, context) <= max as usize
        }
        Condition::ControlDistinctNames(filter, min) => {
            // A phased-out land is treated as though it does not exist
            // (CR 702.26b), so its name is not one of yours.
            let mut names: Vec<baylee_core::ids::NameRef> = state
                .battlefield_seen()
                .filter_map(|id| state.object(id))
                .filter(|o| {
                    o.controller == you
                        && matches_with_context(
                            filter,
                            state,
                            o,
                            you,
                            RuleContext {
                                source: o.id,
                                ..context
                            },
                        )
                })
                .map(|o| o.characteristics().name)
                .filter(|name| *name != crate::state::NAMELESS)
                .collect();
            names.sort_unstable();
            names.dedup();
            names.len() >= min as usize
        }
        // `you` in the filter is the **opponent** being counted, not the
        // ability's controller: "an opponent controls four or more lands"
        // asks the question of each seat in turn, and a filter evaluated
        // with the asker's seat would read `ControlledByYou` backwards.
        Condition::NotStartingPlayer => state.starting_player != you,
        Condition::OpponentControlsMore(filter) | Condition::OpponentControlsMoreBy(filter, _) => {
            let minimum = if let Condition::OpponentControlsMoreBy(_, n) = condition {
                usize::try_from(n).unwrap_or(usize::MAX)
            } else {
                1
            };
            let mut counts = vec![0usize; state.players.len()];
            for id in state.battlefield_seen() {
                if let Some(object) = state.object(id)
                    && matches_with_context(filter, state, object, you, context)
                    && let Some(count) = counts.get_mut(usize::from(object.controller.get()))
                {
                    *count += 1;
                }
            }
            let yours = counts.get(usize::from(you.get())).copied().unwrap_or(0);
            counts.iter().enumerate().any(|(seat, &count)| {
                u8::try_from(seat).is_ok_and(|seat| state.is_opponent(PlayerId::new(seat), you))
                    && count >= yours.saturating_add(minimum)
            })
        }
        Condition::GraveyardTypesAmongAtLeast { types, minimum } => {
            (graveyard_card_types(state, you).bits() & types.bits()).count_ones()
                >= u32::from(minimum)
        }
        Condition::OpponentControlCountAtMost(filter, maximum) => {
            let mut counts = vec![0u32; state.players.len()];
            for id in state.battlefield_seen() {
                if let Some(object) = state.object(id)
                    && matches_with_context(filter, state, object, you, context)
                    && let Some(count) = counts.get_mut(usize::from(object.controller.get()))
                {
                    *count = count.saturating_add(1);
                }
            }
            counts.iter().enumerate().any(|(seat, &count)| {
                u8::try_from(seat).is_ok_and(|seat| state.is_opponent(PlayerId::new(seat), you))
                    && count <= maximum
            })
        }
        Condition::OpponentControlCount(filter, min) => (0..state.players.len())
            .map(|i| PlayerId::new(i as u8))
            .filter(|id| state.is_opponent(*id, you))
            .any(|them| {
                state
                    .battlefield_seen()
                    .filter(|&id| {
                        state.object(id).is_some_and(|o| {
                            o.controller == them
                                && matches_with_context(
                                    filter,
                                    state,
                                    o,
                                    you,
                                    RuleContext {
                                        source: id,
                                        ..context
                                    },
                                )
                        })
                    })
                    .count()
                    >= min as usize
            }),
        Condition::LandsPlayedThisTurnAtLeast(n) => state
            .players
            .get(you.get() as usize)
            .is_some_and(|p| p.lands_played_this_turn >= n),
        Condition::HandSizeAtMost(max) => {
            state.zones.list(ZoneLocation::Hand(you)).len() <= max as usize
        }
        Condition::APlayerHandSizeAtMost(n) => (0..state.players.len())
            .map(|seat| PlayerId::new(seat as u8))
            .any(|player| {
                !state.has_left(player)
                    && state.zones.list(ZoneLocation::Hand(player)).len() <= usize::from(n)
            }),
        Condition::HandSizeExactly(n) => {
            state.zones.list(ZoneLocation::Hand(you)).len() == n as usize
        }
        Condition::OpponentGraveyardCountAtLeast(min) => (0..state.players.len())
            .map(|i| PlayerId::new(i as u8))
            .filter(|id| state.is_opponent(*id, you))
            .any(|id| state.zones.list(ZoneLocation::Graveyard(id)).len() >= min as usize),
        // Your own graveyard, and only yours — threshold counts the cards
        // the *controller* of the ability has, so the seat is `you` and not
        // a scan. The line above is the same question asked of an opponent
        // and answers `any`, because "an opponent has seven" is true of a
        // table where one of three does.
        Condition::GraveyardTypesAtLeast(min) => {
            graveyard_card_types(state, you).bits().count_ones() >= u32::from(min)
        }
        Condition::GraveyardCountAtLeast(min) => {
            state.zones.list(ZoneLocation::Graveyard(you)).len() >= min as usize
        }
        Condition::GraveyardCardsAbove(filter, min) => {
            let cards = state.zones.list(ZoneLocation::Graveyard(you));
            state
                .object(source)
                .is_some_and(|o| o.is_card() && o.zone == Zone::Graveyard && o.owner == you)
                && cards.iter().position(|id| *id == source).is_some_and(|at| {
                    cards[at + 1..]
                        .iter()
                        .filter_map(|id| state.object(*id))
                        .filter(|o| {
                            o.is_card() && matches_with_context(filter, state, o, you, context)
                        })
                        .count()
                        >= usize::from(min)
                })
        }
        Condition::CountersOnSelf(kind, min) => state
            .object(source)
            .is_some_and(|o| o.counters.get(kind) >= u16::from(min)),
        Condition::CountersOnSelfExactly(kind, n) => state
            .object(source)
            .is_some_and(|o| o.counters.get(kind) == u16::from(n)),
        Condition::CountersOnSelfBetween(kind, min, max) => state
            .object(source)
            .is_some_and(|o| (u16::from(min)..=u16::from(max)).contains(&o.counters.get(kind))),
        Condition::EnduringStory => state
            .players
            .get(usize::from(you.get()))
            .is_some_and(|p| p.enduring_story),
        Condition::MaxSpeed => state
            .players
            .get(usize::from(you.get()))
            .is_some_and(|p| p.speed == 4),
        Condition::CitysBlessing => state
            .players
            .get(usize::from(you.get()))
            .is_some_and(|p| p.citys_blessing),
        Condition::Station(min) => state.object(source).is_some_and(|o| {
            o.counters.get(baylee_cards_dsl::CounterKind::Charge) >= u16::from(min)
        }),
        // `is_some_and`, so a source that is no longer in the arena does not
        // match: an ability is a separate object from its source the moment
        // it goes on the stack (CR 113.7a), and "if this land is tapped"
        // asked of a land that has left the battlefield has nothing to be
        // true of. Every other sentence here already fails the same way.
        Condition::Any(parts) => parts
            .iter()
            .any(|part| condition_holds_with_context(state, you, context, *part)),
        Condition::Not(part) => !condition_holds_with_context(state, you, context, *part),
        // Dash's return (CR 702.109a): the rider the cast wrote goes with
        // the permanent the spell became and is given up by every other
        // move (`GameState::move_object`), so a permanent that left the
        // battlefield and came back is not the one the dash cost was paid
        // for (CR 400.7).
        Condition::DashCostPaid => state.object(source).is_some_and(|o| {
            o.zone == crate::zone::Zone::Battlefield
                && o.riders.contains(&crate::object::Rider::Dashed)
        }),
        // "Unless it escaped" (CR 702.138b): the rider the escape cast wrote,
        // kept by the spell and the permanent it becomes and by nothing
        // later (`GameState::move_object`).
        Condition::Escaped => state
            .object(source)
            .is_some_and(|o| o.riders.contains(&crate::object::Rider::Escaped)),
        Condition::AttachedMatches(filter) => attached_for_ability(state, source, None, None)
            .is_some_and(|o| matches_with_context(filter, state, o, you, context)),
        Condition::SourceMatches(filter) => state
            .object(source)
            .is_some_and(|o| matches_with_context(filter, state, o, you, context)),
        Condition::DuringCombat => state.turn.phase == crate::turn::Phase::Combat,
        Condition::AttackedOrBlockedThisCombat => state
            .object(source)
            .is_some_and(|o| state.combat.participants.contains(&(source, o.version))),
        Condition::All(all) => all
            .iter()
            .all(|c| condition_holds_with_context(state, you, context, *c)),
        Condition::OpponentsTurn => state.is_opponent(state.turn.active, you),
        Condition::DuringStep(kind) => state.turn.step.kind() == Some(kind),
        // CR 506.7: "before" a point of the turn is before that point's
        // place in it, whether or not the step itself happens (506.7e).
        Condition::BeforeStep(kind) => state.turn.position() < crate::turn::position_of(kind),
        Condition::CanSacrifice(filter) => {
            !controlled_matching_with_context(state, you, filter, you, context).is_empty()
        }
        Condition::EvenCountersOnSelf => state.object(source).is_some_and(|o| {
            o.counters
                .iter()
                .map(|(_, n)| u32::from(n))
                .sum::<u32>()
                .is_multiple_of(2)
        }),
    }
}

/// The permanents `player` controls that `filter` matches: what a player
/// asked to sacrifice, destroy or return "a [filter]" may pick
/// (`Effect::SacrificeFilter` and its siblings), and what
/// [`Condition::CanSacrifice`] asks about.
///
/// `you` and `source` are the effect's own controller and source, which is
/// what a filter reads "you" and "this" as — not the player being asked. A
/// phased-out permanent is treated as though it does not exist (CR
/// 702.26b), so it is not one of them.
#[must_use]
pub fn controlled_matching(
    state: &GameState,
    player: PlayerId,
    filter: &Filter,
    you: PlayerId,
    source: ObjectId,
) -> Vec<ObjectId> {
    controlled_matching_with_context(state, player, filter, you, live_context(state, source))
}

/// Candidates for a sacrifice or similar choice with captured rules text.
#[must_use]
pub fn controlled_matching_with_context(
    state: &GameState,
    player: PlayerId,
    filter: &Filter,
    you: PlayerId,
    context: RuleContext,
) -> Vec<ObjectId> {
    state
        .battlefield_seen()
        .filter(|id| {
            state.object(*id).is_some_and(|o| {
                o.controller == player && matches_with_context(filter, state, o, you, context)
            })
        })
        .collect()
}

/// The permanents on the battlefield `filter` matches, whoever controls them,
/// asked with the ability's source as `this` so `Filter::Another` excludes
/// the source itself (City in a Bottle).
///
/// A phased-out permanent is treated as though it does not exist (CR
/// 702.26b), so it is not on the battlefield this counts.
fn battlefield_count(
    state: &GameState,
    you: PlayerId,
    filter: &Filter,
    context: RuleContext,
) -> usize {
    state
        .battlefield_seen()
        .filter(|id| {
            state
                .object(*id)
                .is_some_and(|o| matches_with_context(filter, state, o, you, context))
        })
        .count()
}

/// The intervening-`if` clause of a triggered ability (CR 603.4), asked of
/// an ability that may not print one at all.
///
/// An absent clause is the trivially true one, so this is what both of the
/// rule's two checks call: the clause is asked once where the ability would
/// trigger and once where it would resolve, and the two must be the same
/// question or a card would trigger on one reading and vanish on the other.
#[must_use]
pub fn intervening_if(
    state: &GameState,
    condition: Option<Condition>,
    you: PlayerId,
    source: ObjectId,
) -> bool {
    condition.is_none_or(|c| condition_holds(state, you, source, c))
}

/// Resolution's intervening-if check with the source incarnation that
/// triggered. A graveyard-order clause cannot find a later object that
/// reuses the same engine handle (CR 400.7).
pub(crate) fn intervening_if_for_incarnation(
    state: &GameState,
    condition: Option<Condition>,
    you: PlayerId,
    source: ObjectId,
    version: Option<u32>,
    attachment: Option<(ObjectId, u32)>,
    event: Option<baylee_core::ids::DamageSourceRef>,
) -> bool {
    match condition {
        Some(Condition::All(conditions)) => conditions.iter().all(|condition| {
            intervening_if_for_incarnation(
                state,
                Some(*condition),
                you,
                source,
                version,
                attachment,
                event,
            )
        }),
        Some(Condition::Any(conditions)) => conditions.iter().any(|condition| {
            intervening_if_for_incarnation(
                state,
                Some(*condition),
                you,
                source,
                version,
                attachment,
                event,
            )
        }),
        Some(Condition::Not(condition)) => !intervening_if_for_incarnation(
            state,
            Some(*condition),
            you,
            source,
            version,
            attachment,
            event,
        ),
        Some(Condition::SourceWasCastByYou) => state
            .damage_source(source, version)
            .is_some_and(|object| object.cast_history.caster() == Some(you)),
        Some(Condition::AttachedMatches(filter)) => {
            attached_for_ability(state, source, version, attachment)
                .is_some_and(|o| matches(filter, state, o, you, source))
        }
        Some(Condition::GraveyardCardsAbove(_, _)) => {
            version.is_none_or(|version| state.object(source).is_some_and(|o| o.version == version))
                && intervening_if(state, condition, you, source)
        }
        Some(condition) => {
            condition_with_event(state, you, live_context(state, source), condition, event)
        }
        None => true,
    }
}

/// An event-dependent clause excludes an incarnation, never a reusable handle.
pub(crate) fn condition_reads_event(condition: Condition) -> bool {
    match condition {
        Condition::ControlCountExceptEvent(_, _) | Condition::EventBlockedByOthers { .. } => true,
        Condition::All(conditions) | Condition::Any(conditions) => {
            conditions.iter().copied().any(condition_reads_event)
        }
        Condition::Not(condition) => condition_reads_event(*condition),
        _ => false,
    }
}

/// Evaluate a condition with the triggering incarnation available to its clauses.
pub(crate) fn condition_with_event(
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
    condition: Condition,
    event: Option<baylee_core::ids::DamageSourceRef>,
) -> bool {
    match condition {
        Condition::ControlCountExceptEvent(filter, minimum) => {
            let Some(event) = event else { return false };
            state
                .battlefield_seen()
                .filter(|&id| {
                    state.object(id).is_some_and(|object| {
                        object.controller == you
                            && (id != event.object || object.version != event.version)
                            && matches_with_context(filter, state, object, you, context)
                    })
                })
                .take(usize::from(minimum))
                .count()
                >= usize::from(minimum)
        }
        // Wall of Caltrops: the creature it blocks, as that event names it.
        Condition::EventBlockedByOthers { some, none } => {
            let Some(event) = event else { return false };
            if state
                .object(event.object)
                .is_none_or(|o| o.version != event.version)
            {
                return false;
            }
            let blockers: Vec<&crate::object::GameObject> = state
                .combat
                .blockers
                .iter()
                .filter(|b| b.attacker == event.object)
                .filter_map(|b| state.object(b.blocker))
                .collect();
            blockers.iter().any(|b| {
                b.id != context.source && matches_with_context(some, state, b, you, context)
            }) && !blockers
                .iter()
                .any(|b| matches_with_context(none, state, b, you, context))
        }
        Condition::All(conditions) => conditions
            .iter()
            .all(|&condition| condition_with_event(state, you, context, condition, event)),
        Condition::Any(conditions) => conditions
            .iter()
            .any(|&condition| condition_with_event(state, you, context, condition, event)),
        Condition::Not(condition) => !condition_with_event(state, you, context, *condition, event),
        _ => condition_holds_with_context(state, you, context, condition),
    }
}

/// A resolving ability reads its source's current attachment, or the
/// attachment recorded as that source left (CR 608.2h). A later incarnation
/// of either permanent cannot stand in for the old one (CR 400.7).
pub(crate) fn attached_for_ability(
    state: &GameState,
    source: ObjectId,
    version: Option<u32>,
    attachment: Option<(ObjectId, u32)>,
) -> Option<&crate::object::GameObject> {
    let current = state.object(source).filter(|o| {
        o.zone == crate::zone::Zone::Battlefield
            && version.is_none_or(|version| o.version == version)
            && !o.status.contains(crate::object::Status::PHASED_OUT)
    });
    let (host, version) = if let Some(source) = current {
        let host = state.object(source.attached_to?)?;
        (host.id, host.version)
    } else {
        attachment?
    };
    state.object(host).filter(|o| {
        o.zone == crate::zone::Zone::Battlefield
            && o.version == version
            && !o.status.contains(crate::object::Status::PHASED_OUT)
    })
}

/// Distinct card types in one graveyard. A multi-type card contributes every
/// type; repeated types, supertypes, subtypes, and non-card objects add none.
pub(crate) fn graveyard_card_types(
    state: &GameState,
    player: PlayerId,
) -> baylee_core::types::TypeSet {
    use baylee_core::types::TypeSet;
    state
        .zones
        .list(ZoneLocation::Graveyard(player))
        .iter()
        .filter_map(|id| state.object(*id).filter(|o| o.is_card()))
        .fold(TypeSet::EMPTY, |types, card| {
            types.union(card.characteristics().types)
        })
}

/// Protection (CR 702.16): does `object` have protection from a filter
/// that `source` matches? Checked for damage, targeting, and blocking.
#[must_use]
pub fn protected_from(state: &GameState, object: ObjectId, source: ObjectId) -> bool {
    let Some(src) = state.object(source) else {
        return false;
    };
    protected_from_object(state, object, src)
}

fn protected_from_object(
    state: &GameState,
    object: ObjectId,
    src: &crate::object::GameObject,
) -> bool {
    let Some(obj) = state.object(object) else {
        return false;
    };
    let source = src.id;
    state.effects.iter().any(|fx| {
        let baylee_cards_dsl::Modifier::ProtectionFrom(f) = fx.modifier else {
            return false;
        };
        crate::effects::applies_to(state, fx, obj)
            && matches_with_context(
                f,
                state,
                src,
                fx.controller,
                RuleContext {
                    source: fx.source.unwrap_or(source),
                    text: state.effect_text(fx),
                    reference: None,
                },
            )
    })
}

/// Does a static ability on `object` say it can't be the target of `source`
/// ([`baylee_cards_dsl::Modifier::CantBeTargetedBy`], Thrun, Breaker of
/// Silence)? `source` is the spell, or the source of the ability, doing the
/// targeting, and the filter is asked of it with the effect's controller as
/// "you" — the same reading [`protected_from`] gives protection, whose
/// targeting half this is. `CantBeTargetedByAbilitiesFrom` (Artifact Ward)
/// asks only of an ability's source: a spell is on the stack as it chooses
/// its targets (CR 601.2a, 601.2c), and an ability's source is not the
/// object on the stack.
#[must_use]
pub fn untargetable_by_source(state: &GameState, object: ObjectId, source: ObjectId) -> bool {
    let Some(src) = state.object(source) else {
        return false;
    };
    untargetable_by_source_object(state, object, src)
}

fn untargetable_by_source_object(
    state: &GameState,
    object: ObjectId,
    src: &crate::object::GameObject,
) -> bool {
    let Some(obj) = state.object(object) else {
        return false;
    };
    let source = src.id;
    let spell = src.zone == crate::zone::Zone::Stack;
    state.effects.iter().any(|fx| {
        let f = match fx.modifier {
            baylee_cards_dsl::Modifier::CantBeTargetedBy(f) => f,
            baylee_cards_dsl::Modifier::CantBeTargetedByAbilitiesFrom(f) if !spell => f,
            _ => return false,
        };
        crate::effects::applies_to(state, fx, obj)
            && matches_with_context(
                f,
                state,
                src,
                fx.controller,
                RuleContext {
                    source: fx.source.unwrap_or(source),
                    text: state.effect_text(fx),
                    reference: None,
                },
            )
    })
}

/// A rule permission with a continuing set: new opponents' creatures are
/// covered too, and losing the resolving source does not end the permission.
fn ignores_opponents_hexproof(state: &GameState, you: PlayerId) -> bool {
    state.effects.iter().any(|effect| {
        effect.controller == you
            && effect.modifier == baylee_cards_dsl::Modifier::IgnoreOpponentsHexproof
    })
}

/// Hexproof (CR 702.11b) and shroud (CR 702.18a): does `object` refuse to
/// be targeted by a spell or ability `you` control?
///
/// Both keywords function only while the object is on the battlefield —
/// a card in hand printed with hexproof is targetable in the graveyard,
/// and every other zone. Player hexproof is a different thing entirely
/// (`Modifier::PlayerHexproof`, checked when a spell chooses a player).
#[must_use]
pub fn untargetable_by(state: &GameState, object: ObjectId, you: PlayerId) -> bool {
    let Some(obj) = state.object(object) else {
        return false;
    };
    if obj.zone != crate::zone::Zone::Battlefield {
        return false;
    }
    let keywords = obj.characteristics().keywords;
    // Shroud stops everyone, its own controller included.
    if keywords.contains(baylee_cards_dsl::KeywordSet::SHROUD) {
        return true;
    }
    // Hexproof stops opponents only (CR 702.11b), so a teammate may
    // target it — and in a game with no teams that is everyone else.
    keywords.contains(baylee_cards_dsl::KeywordSet::HEXPROOF)
        && state.is_opponent(obj.controller, you)
        && !(obj
            .characteristics()
            .types
            .contains(baylee_core::types::TypeSet::CREATURE)
            && ignores_opponents_hexproof(state, you))
}

/// Whether rules-modifying effects permit this Aura to enchant the host.
/// Targeting restrictions and the Aura's own enchant filter are separate.
#[must_use]
pub fn permits_enchantment(state: &GameState, host: ObjectId, aura: ObjectId) -> bool {
    let Some(object) = state.object(host) else {
        return false;
    };
    !state.effects.iter().any(|fx| {
        (match fx.modifier {
            baylee_cards_dsl::Modifier::CantBeEnchanted => true,
            baylee_cards_dsl::Modifier::CantBeEnchantedExceptSource => fx.source != Some(aura),
            _ => false,
        }) && crate::effects::applies_to(state, fx, object)
    })
}

/// The players a spell or ability may target.
///
/// A player is untargetable only through an effect, never through a
/// characteristic — there is nothing on a player for a `Filter` to match —
/// so this is a list, not a filtered enumeration like the object half.
#[must_use]
pub fn target_player_options(state: &GameState, spec: &TargetSpec, you: PlayerId) -> Vec<PlayerId> {
    if !matches!(
        spec,
        TargetSpec::AnyTarget
            | TargetSpec::AnyPlayer
            | TargetSpec::AnyOpponent
            | TargetSpec::PlayerWhoAttackedThisTurn
            | TargetSpec::OpponentOrObject(_)
            | TargetSpec::PlayerOrObject(_)
    ) {
        return Vec::new();
    }
    let opponents_only = matches!(
        spec,
        TargetSpec::AnyOpponent | TargetSpec::OpponentOrObject(_)
    );
    let attacked_only = matches!(spec, TargetSpec::PlayerWhoAttackedThisTurn);
    // Build the small seat set once instead of rescanning all effects for
    // each potential player target.
    let mut protected = baylee_core::ids::SeatSet::new();
    let mut everything = baylee_core::ids::SeatSet::new();
    let mut ignores_hexproof = false;
    for effect in state.effects.iter() {
        match effect.modifier {
            baylee_cards_dsl::Modifier::PlayerProtectionFromEverything { who } => {
                if let Some(players) = players(who, state, effect.controller) {
                    for player in players {
                        everything.insert(player);
                    }
                }
            }
            baylee_cards_dsl::Modifier::PlayerProtectionFromOpponents { who } => {
                if let Some(players) = players(who, state, effect.controller) {
                    for player in players {
                        if state.is_opponent(player, you) {
                            everything.insert(player);
                        }
                    }
                }
            }
            baylee_cards_dsl::Modifier::PlayerHexproof => protected.insert(effect.controller),
            baylee_cards_dsl::Modifier::IgnoreOpponentsHexproof if effect.controller == you => {
                ignores_hexproof = true;
            }
            _ => {}
        }
    }
    state
        .players
        .iter()
        .filter(|p| {
            if p.has_lost() || everything.contains(p.id) {
                return false;
            }
            // "Target opponent" is a choice over a smaller set, not a
            // different kind of choice (CR 115.1) — and the set is the
            // opponents, so a teammate is out of it as surely as you are.
            if opponents_only && !state.is_opponent(p.id, you) {
                return false;
            }
            if attacked_only
                && !state
                    .per_turn
                    .attacked
                    .iter()
                    .any(|&(_, _, who)| who == p.id)
            {
                return false;
            }
            // Player hexproof also stops only opponents (CR 702.11c).
            !state.is_opponent(p.id, you) || ignores_hexproof || !protected.contains(p.id)
        })
        .map(|p| p.id)
        .collect()
}

/// The object half of "any target" (CR 115.4): every creature, planeswalker
/// and battle on the battlefield.
///
/// Filtering by type rather than by a `Filter` is deliberate — the printed
/// words are a fixed list the rules maintain, not a card's own predicate.
fn any_target_objects(state: &GameState) -> Vec<ObjectId> {
    state
        .battlefield_view()
        .iter()
        .filter(|id| {
            state.object(**id).is_some_and(|o| {
                let types = o.characteristics().types;
                types.contains(baylee_core::types::TypeSet::CREATURE)
                    || types.contains(baylee_core::types::TypeSet::PLANESWALKER)
                    || types.contains(baylee_core::types::TypeSet::BATTLE)
            })
        })
        .copied()
        .collect()
}

fn named_graveyard_options(
    filter: &'static Filter,
    player: PlayerId,
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
) -> Vec<ObjectId> {
    if state.has_left(player) {
        return Vec::new();
    }
    state
        .zones
        .list(ZoneLocation::Graveyard(player))
        .iter()
        .copied()
        .filter(|id| {
            state
                .object(*id)
                .is_some_and(|object| matches_with_context(filter, state, object, you, context))
        })
        .collect()
}

/// [`TargetSpec::CardInGraveyardBelowValue`]'s options: the graveyard cards
/// matching `filter` whose mana value is less than `limit`.
fn graveyard_options_below(
    filter: &'static baylee_cards_dsl::Filter,
    rel: baylee_cards_dsl::PlayerRel,
    limit: u32,
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
) -> Vec<ObjectId> {
    graveyard_options(filter, rel, state, you, context)
        .into_iter()
        .filter(|id| {
            state
                .object(*id)
                .is_some_and(|o| o.characteristics().mana_value() < limit)
        })
        .collect()
}

/// The object options of the specs that are about one player's permanents.
fn objects_of_a_player(
    spec: &TargetSpec,
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
) -> Vec<ObjectId> {
    match *spec {
        // The object half of "target opponent or [filter]"; the opponents
        // come from `target_player_options`, offered beside it.
        TargetSpec::OpponentOrObject(filter) | TargetSpec::PlayerOrObject(filter) => {
            choice_options_with_context(&TargetSpec::Object(filter), state, you, context)
        }
        TargetSpec::ObjectControlledBy(filter, player) => {
            let mut all =
                choice_options_with_context(&TargetSpec::Object(filter), state, you, context);
            all.retain(|id| state.object(*id).is_some_and(|o| o.controller == player));
            all
        }
        // Unbound, "that player" is nobody yet: the engine asks these only
        // after binding them to `ObjectControlledBy`.
        _ => Vec::new(),
    }
}

/// [`TargetSpec::ObjectOfEachOpponent`]'s options: every opponent's at once,
/// which each question narrows to one (`Engine::ask_next_opponent`).
fn opponents_objects(
    filter: &'static baylee_cards_dsl::Filter,
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
) -> Vec<ObjectId> {
    let mut all = choice_options_with_context(&TargetSpec::Object(filter), state, you, context);
    all.retain(|id| {
        state
            .object(*id)
            .is_some_and(|o| state.is_opponent(o.controller, you))
    });
    all
}

/// The X announced for `this`, where `cast_wizard` and an activation write
/// it; 0 for a source that is gone or announced none.
fn announced_x(state: &GameState, this: ObjectId) -> u32 {
    state.object(this).map_or(0, |o| o.x_value)
}

/// Whether `stat` is less than the source's projected power. A source with
/// no power, or none at all, bounds nothing in, and neither does an object
/// with no such number.
fn below_source_power(stat: Option<i16>, state: &GameState, this: ObjectId) -> bool {
    let bound = state.object(this).and_then(|o| o.characteristics().power);
    stat.zip(bound).is_some_and(|(stat, bound)| stat < bound)
}

/// Whether `filter` reads the X announced for its source
/// ([`Filter::CmcAtMostX`], [`Filter::CmcExactlyX`]).
#[must_use]
pub fn reads_announced_x(filter: &Filter) -> bool {
    match filter {
        Filter::CmcAtMostX | Filter::CmcExactlyX | Filter::TwiceCmcExactlyX => true,
        Filter::And(parts) | Filter::Or(parts) => parts.iter().any(reads_announced_x),
        Filter::Not(f) => reads_announced_x(f),
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
fn matches_for_x(
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

/// Target options before announcing X (`None`) or with the announced value.
/// The widened enumeration still enforces protection and targeting rules.
pub(crate) fn target_options_with_x(
    spec: &TargetSpec,
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
    x: Option<u32>,
) -> Vec<ObjectId> {
    let Some((widened, filter)) = crate::casting::x_bounded(*spec) else {
        return target_options_with_context(spec, state, you, context);
    };
    let mut options = target_options_with_context(&widened, state, you, context);
    options.retain(|id| {
        state
            .object(*id)
            .is_some_and(|obj| matches_for_x(filter, state, obj, you, context, x))
    });
    options
}

/// Enumerate one instance using the complete selected targeting domain.
pub(crate) fn target_options_in_domain(
    spec: &TargetSpec,
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
    x: Option<u32>,
    wall_only: bool,
) -> Vec<ObjectId> {
    let mut options = target_options_with_x(spec, state, you, context, x);
    target_domain::retain(state, &mut options, wall_only);
    options
}

/// Legal target options for a [`TargetSpec`] (empty = cannot be chosen).
#[must_use]
pub fn target_options(
    spec: &TargetSpec,
    state: &GameState,
    you: PlayerId,
    this: ObjectId,
) -> Vec<ObjectId> {
    target_options_with_context(spec, state, you, live_context(state, this))
}

/// Legal target options using frozen ability text or a grant's original words.
#[must_use]
pub fn target_options_with_context(
    spec: &TargetSpec,
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
) -> Vec<ObjectId> {
    let this = context.source;
    // Protection (CR 702.16b) keeps out matching sources, and so does a
    // printed "can't be the target of" sentence; hexproof and shroud
    // (CR 702.11b/702.18b) keep out whole classes of chooser.
    let mut options = choice_options_with_context(spec, state, you, context)
        .into_iter()
        .filter(|id| {
            !protected_from(state, *id, this)
                && !untargetable_by_source(state, *id, this)
                && !untargetable_by(state, *id, you)
        })
        .collect();
    target_pair::retain_partners(state, spec, context, &mut options);
    options
}

/// Targeting with the characteristics of the chosen face and casting form.
pub(crate) fn spell_target_options_for(
    spec: &TargetSpec,
    state: &GameState,
    you: PlayerId,
    source: ObjectId,
    face: Option<(&baylee_cards_dsl::CardDef, usize)>,
    form: Option<crate::casting::SpellForm>,
    wall_only: bool,
) -> Vec<ObjectId> {
    spell_target_options_relative(spec, state, you, source, face, form, wall_only, None)
}

/// [`spell_target_options_for`] for a second instance of "target" whose
/// requirement names the first (`Filter::Reference` and its family, CR
/// 601.2c chooses them in order): `reference` is the first target.
#[allow(clippy::too_many_arguments)] // the face, the form and the domain are each the caller's
pub(crate) fn spell_target_options_relative(
    spec: &TargetSpec,
    state: &GameState,
    you: PlayerId,
    source: ObjectId,
    face: Option<(&baylee_cards_dsl::CardDef, usize)>,
    form: Option<crate::casting::SpellForm>,
    wall_only: bool,
    reference: Option<ObjectId>,
) -> Vec<ObjectId> {
    let Some(original) = state.object(source) else {
        return Vec::new();
    };
    let mut spell = original.clone();
    if let Some((def, face)) = face {
        let Some(printed) = def.faces.get(face) else {
            return Vec::new();
        };
        let name = state.names.find(printed.name).unwrap_or(original.base.name);
        spell.base =
            std::sync::Arc::new(crate::object::Characteristics::from_face(def, face, name));
        spell.cache.clear();
        spell.face_index = u8::try_from(face).unwrap_or(0);
    }
    if let Some(form) = form {
        spell = form.project(&spell);
    }
    spell.zone = crate::zone::Zone::Stack;
    spell.kind = crate::object::ObjectKind::Spell;
    spell.controller = you;
    spell.cache.clear();
    let projected = crate::layers::recompute(state, &spell);
    spell.base = std::sync::Arc::new(projected.characteristics);
    spell.controller = projected.controller;
    let mut context = live_context(state, source);
    context.reference = reference;
    let mut options = choice_options_with_context(spec, state, you, context)
        .into_iter()
        .filter(|id| {
            !protected_from_object(state, *id, &spell)
                && !untargetable_by_source_object(state, *id, &spell)
                && aura_target_permitted(state, &spell, *id)
                && !untargetable_by(state, *id, you)
        })
        .collect();
    target_domain::retain(state, &mut options, wall_only);
    target_pair::retain_partners(state, spec, context, &mut options);
    options
}

fn aura_target_permitted(state: &GameState, spell: &GameObject, host: ObjectId) -> bool {
    spell.kind != crate::object::ObjectKind::Spell
        || !spell
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::enchantment::AURA)
        || crate::permanent_guards::permits_new_enchantment(state, host, spell.id)
}

/// The objects `spec` describes, for a choice that does **not** target.
///
/// "Unless that object or player is identified by the word 'target' in the
/// text of that spell or ability, or the rule for that keyword ability,
/// it's not a target" (CR 115.10a). Protection, hexproof and shroud are
/// all about being targeted, so they keep nothing off this list: a Clone
/// may enter as a copy of a creature with shroud, since entering as a copy
/// is how it enters (CR 707.5) and not an ability that targets. The same
/// enumeration as [`target_options`], without its last filter.
#[must_use]
pub fn choice_options(
    spec: &TargetSpec,
    state: &GameState,
    you: PlayerId,
    this: ObjectId,
) -> Vec<ObjectId> {
    choice_options_with_context(spec, state, you, live_context(state, this))
}

/// [`choice_options`] with frozen ability text or a grant's original words.
#[must_use]
pub fn choice_options_with_context(
    spec: &TargetSpec,
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
) -> Vec<ObjectId> {
    let this = context.source;
    match spec {
        TargetSpec::Object(filter) => state
            .battlefield_view()
            .iter()
            .filter(|id| {
                state
                    .object(**id)
                    .is_some_and(|o| matches_with_context(filter, state, o, you, context))
            })
            .copied()
            .collect(),
        TargetSpec::ObjectOfEachOpponent(filter) => opponents_objects(filter, state, you, context),
        TargetSpec::OpponentOrObject(_)
        | TargetSpec::PlayerOrObject(_)
        | TargetSpec::ObjectControlledBy(..)
        | TargetSpec::ObjectOfFirstTargetsPlayer(_)
        | TargetSpec::ObjectOfEventPlayer(_) => objects_of_a_player(spec, state, you, context),
        TargetSpec::Spell(filter) => state
            .zones
            .list(ZoneLocation::Stack)
            .iter()
            .filter(|id| {
                // Uncounterable spells are targets like any other (#243):
                // it is the counter that asks, as it resolves
                // (`GameObject::can_be_countered`).
                state.object(**id).is_some_and(|o| {
                    o.kind == crate::object::ObjectKind::Spell
                        && matches_with_context(filter, state, o, you, context)
                })
            })
            .copied()
            .collect(),
        TargetSpec::CardInGraveyard(filter, rel) => {
            graveyard_options(filter, *rel, state, you, context)
        }
        TargetSpec::CardInFirstTargetsGraveyard(_) | TargetSpec::CardInGraveyardBelowEvent(..) => {
            Vec::new()
        }
        TargetSpec::CardInPlayersGraveyard(filter, player) => {
            named_graveyard_options(filter, *player, state, you, context)
        }
        TargetSpec::CardInGraveyardBelowValue(filter, rel, limit) => {
            graveyard_options_below(filter, *rel, *limit, state, you, context)
        }
        // "Target spell or permanent" (Venser, Shaper Savant; the laces):
        // the spells on the stack and the permanents on the battlefield, and
        // nothing else the stack holds. An activated or triggered ability
        // there is neither: abilities on the stack "aren't spells"
        // (CR 113.9), and a permanent is a card or token on the battlefield
        // (CR 110.1). Offering them put each Venser trigger on the menu of
        // the next one stacked over it: in l29 game 1930 thirty token
        // Vensers were asked over a menu that grew by one a question.
        TargetSpec::StackOrBattlefield(filter) => {
            let spells = state.zones.list(ZoneLocation::Stack).iter().filter(|id| {
                state
                    .object(**id)
                    .is_some_and(|o| o.kind == crate::object::ObjectKind::Spell)
            });
            let mut out: Vec<ObjectId> = spells
                .chain(state.battlefield_view().iter())
                .filter(|id| {
                    state
                        .object(**id)
                        .is_some_and(|o| matches_with_context(filter, state, o, you, context))
                })
                .copied()
                .collect();
            out.sort();
            out.dedup();
            out
        }
        TargetSpec::ThisObject => vec![this],
        TargetSpec::AbilityOnStack(filter) => state
            .zones
            .list(ZoneLocation::Stack)
            .iter()
            .filter(|id| {
                state.object(**id).is_some_and(|o| {
                    o.kind == crate::object::ObjectKind::AbilityOnStack
                        && matches_with_context(filter, state, o, you, context)
                })
            })
            .copied()
            .collect(),
        TargetSpec::SpellOrAbility(filter) => state
            .zones
            .list(ZoneLocation::Stack)
            .iter()
            .filter(|id| {
                state.object(**id).is_some_and(|o| {
                    matches!(
                        o.kind,
                        crate::object::ObjectKind::Spell
                            | crate::object::ObjectKind::AbilityOnStack
                    ) && matches_with_context(filter, state, o, you, context)
                })
            })
            .copied()
            .collect(),
        // "Any target" (CR 115.4) is a creature, a planeswalker, a battle
        // or a player. Only the object half is enumerated here; the players
        // come from `target_player_options`, and the two lists are offered
        // as one choice.
        TargetSpec::AnyTarget => any_target_objects(state),
        // EventObject is implicit (no player choice); player targeting
        // resolves via ChoosePlayer in the casting wizard.
        TargetSpec::EventObject
        | TargetSpec::Player(_)
        | TargetSpec::AnyPlayer
        | TargetSpec::AnyOpponent
        | TargetSpec::PlayerWhoAttackedThisTurn => vec![],
    }
}

/// The players a spell or ability on the stack chose for its first instance
/// of "target" under `spec`.
///
/// They ride in two fields. `target_players` is the set "any target" named,
/// and `chosen_player` is the one seat a player target named, but it is also
/// written by choices that are not targets at all. So the second is read
/// only for the three specs that can name a player, which is exactly the
/// set [`target_player_options`] answers for: folding it in for any other
/// spec would put a chosen player in front of an enumeration that never
/// offered them.
#[must_use]
pub fn targeted_players(obj: &GameObject, spec: &TargetSpec) -> baylee_core::ids::SeatSet {
    let mut players = baylee_core::ids::SeatSet::new();
    if matches!(
        spec,
        TargetSpec::AnyTarget
            | TargetSpec::AnyPlayer
            | TargetSpec::AnyOpponent
            | TargetSpec::PlayerWhoAttackedThisTurn
            | TargetSpec::OpponentOrObject(_)
            | TargetSpec::PlayerOrObject(_)
    ) {
        players = obj.target_players;
        if let Some(player) = obj.chosen_player {
            players.insert(player);
        }
    }
    players
}

/// What a spell or ability on the stack may legally target under `spec` now:
/// the objects and the players.
///
/// It is the enumeration that offered the targets in the first place,
/// [`target_options`] and [`target_player_options`], asked with the same
/// `(you, this)` the cast wizard passes: the object's controller, and the
/// object itself or, for an ability, its source. CR 608.2b's re-check
/// (`Engine::instance_legality`) and a change of targets (CR 115.7,
/// `resolve::retarget`) both ask it, so what one of them calls legal the
/// other does too.
///
/// The object itself is never among them (CR 115.5).
#[must_use]
pub fn stack_target_options(
    state: &GameState,
    obj: &GameObject,
    spec: &TargetSpec,
) -> (Vec<ObjectId>, Vec<PlayerId>) {
    stack_target_options_relative(state, obj, spec, None)
}

/// [`stack_target_options`] for a second instance of "target" whose
/// requirement names the first: `reference` is the first target.
#[must_use]
pub fn stack_target_options_relative(
    state: &GameState,
    obj: &GameObject,
    spec: &TargetSpec,
    reference: Option<baylee_core::ids::DamageSourceRef>,
) -> (Vec<ObjectId>, Vec<PlayerId>) {
    stack_target_options_impl(state, obj, spec, reference, false)
}

/// Candidate replacement targets, before joint restrictions are checked against
/// the complete prospective target list by the retarget continuation.
pub(crate) fn stack_retarget_options(
    state: &GameState,
    obj: &GameObject,
    spec: &TargetSpec,
) -> (Vec<ObjectId>, Vec<PlayerId>) {
    stack_target_options_impl(state, obj, spec, None, true)
}

fn stack_target_options_impl(
    state: &GameState,
    obj: &GameObject,
    spec: &TargetSpec,
    reference: Option<baylee_core::ids::DamageSourceRef>,
    retargeting: bool,
) -> (Vec<ObjectId>, Vec<PlayerId>) {
    let you = obj.controller;
    let this = if obj.kind == crate::object::ObjectKind::AbilityOnStack {
        obj.ability.map_or(obj.id, |loc| loc.source)
    } else {
        obj.id
    };
    let context = RuleContext {
        source: this,
        text: live_context(state, obj.id).text,
        reference: reference.map(|reference| reference.object),
    };
    let mut objects = target_pair::stack_options(state, obj, spec, context, reference, retargeting)
        .unwrap_or_else(|| target_options_with_x(spec, state, you, context, Some(obj.x_value)));
    target_domain::retain(state, &mut objects, target_domain::stack_walls(obj));
    objects.retain(|id| *id != obj.id && aura_target_permitted(state, obj, *id));
    (objects, target_player_options(state, spec, you))
}

pub(crate) fn count_controlled_by(
    state: &GameState,
    controller: PlayerId,
    you: PlayerId,
    filter: &Filter,
    context: RuleContext,
) -> u32 {
    u32::try_from(
        state
            .battlefield_view()
            .iter()
            .filter(|id| {
                state.object(**id).is_some_and(|object| {
                    object.controller == controller
                        && matches_with_context(filter, state, object, you, context)
                })
            })
            .count(),
    )
    .unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests;
