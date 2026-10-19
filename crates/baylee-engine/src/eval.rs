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
            | TargetSpec::OpponentOrObject(_)
            | TargetSpec::PlayerOrObject(_)
    ) {
        return Vec::new();
    }
    let opponents_only = matches!(
        spec,
        TargetSpec::AnyOpponent | TargetSpec::OpponentOrObject(_)
    );
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
        Filter::CmcAtMostX | Filter::CmcExactlyX => true,
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
        | TargetSpec::AnyOpponent => vec![],
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
mod tests {
    use super::*;
    use crate::object::ObjectKind;
    use crate::state::CardLookup;
    use baylee_cards_dsl::KeywordSet;
    use baylee_core::ids::{CardIndex, SubtypeId};
    use baylee_core::preset::{FormatId, GamePreset, HouseRules, SeatController, SeatSpec};
    use baylee_core::types::{SubtypeSet, TypeSet};

    /// No cards: these tests are about keywords, not about whichever
    /// printed card happens to carry one.
    struct NoCards;
    impl CardLookup for NoCards {
        fn card(&self, _: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
            None
        }
    }

    const P0: PlayerId = PlayerId::new(0);
    const P1: PlayerId = PlayerId::new(1);

    static ANY_CREATURE: Filter = Filter::HasType(TypeSet::CREATURE);

    fn empty_state() -> GameState {
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
                seed: 1,
                house_rules: HouseRules::default(),
                modifiers: vec![],
                prints: vec![],
                seats: vec![seat(), seat()],
            },
            &NoCards,
        )
        .expect("an empty two-seat board")
    }

    fn creature(state: &mut GameState, controller: PlayerId, keywords: KeywordSet) -> ObjectId {
        let name = state.names.intern("Test Creature");
        let id = state.create_bare(
            controller,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        let b = state.object_mut(id).expect("just created").base_mut();
        b.types = TypeSet::CREATURE;
        b.power = Some(1);
        b.toughness = Some(1);
        b.keywords = keywords;
        id
    }

    /// "That player" of a step trigger is whoever's turn it is (CR 102.1),
    /// read from the state alone; the enchanted permanent's controller needs
    /// the source and is the resolution's to answer.
    #[test]
    fn the_active_player_is_whoever_s_turn_it_is() {
        let mut state = empty_state();
        state.turn.active = P1;
        assert_eq!(players(PlayerRel::ActivePlayer, &state, P0), Some(vec![P1]));
        state.turn.active = P0;
        assert_eq!(players(PlayerRel::ActivePlayer, &state, P1), Some(vec![P0]));
        assert_eq!(players(PlayerRel::ControllerOfAttached, &state, P0), None);

        let host = land(&mut state, P1, &[]);
        let aura = creature(&mut state, P0, KeywordSet::EMPTY);
        assert_eq!(
            controller_of_attached(&state, aura),
            None,
            "attached to nothing"
        );
        state.object_mut(aura).expect("just made").attached_to = Some(host);
        assert_eq!(
            controller_of_attached(&state, aura),
            Some(P1),
            "the host's controller, not the Aura's (CR 303.4e)"
        );
    }

    fn buried_card(state: &mut GameState, types: TypeSet) -> ObjectId {
        let name = state.names.intern("Graveyard fixture");
        let id = state.create_bare(P0, ObjectKind::Card, name, ZoneLocation::Graveyard(P0));
        let object = state.object_mut(id).expect("created card");
        object.card = Some(crate::object::CardRef {
            index: CardIndex::new(1),
            print: baylee_core::ids::PrintRef::new(0),
        });
        object.base_mut().types = types;
        id
    }

    #[test]
    fn graveyard_cards_above_counts_only_matching_cards_above_the_source() {
        let mut state = empty_state();
        for _ in 0..3 {
            buried_card(&mut state, TypeSet::CREATURE);
        }
        let source = buried_card(&mut state, TypeSet::CREATURE);
        let condition = Condition::GraveyardCardsAbove(&ANY_CREATURE, 3);
        assert!(!condition_holds(&state, P0, source, condition));
        for _ in 0..2 {
            buried_card(&mut state, TypeSet::CREATURE);
        }
        buried_card(&mut state, TypeSet::LAND);
        let token = buried_card(&mut state, TypeSet::CREATURE);
        state.object_mut(token).expect("token fixture").card = None;
        let copy = buried_card(&mut state, TypeSet::CREATURE);
        state
            .object_mut(copy)
            .expect("copy fixture")
            .riders
            .push(crate::object::Rider::SpellCopy);
        assert!(!condition_holds(&state, P0, source, condition));
        buried_card(&mut state, TypeSet::CREATURE);
        assert!(condition_holds(&state, P0, source, condition));
        assert!(!condition_holds(&state, P1, source, condition));
        state
            .move_object(
                source,
                ZoneLocation::Exile(P0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
        assert!(!condition_holds(
            &state,
            P0,
            source,
            Condition::GraveyardCardsAbove(&ANY_CREATURE, 0)
        ));
    }

    #[test]
    fn graveyard_order_condition_cannot_find_a_later_incarnation() {
        let mut state = empty_state();
        let source = buried_card(&mut state, TypeSet::CREATURE);
        for _ in 0..3 {
            buried_card(&mut state, TypeSet::CREATURE);
        }
        let condition = Some(Condition::All(&[Condition::GraveyardCardsAbove(
            &Filter::CREATURE,
            3,
        )]));
        let version = state.object(source).expect("source").version;
        assert!(intervening_if_for_incarnation(
            &state,
            condition,
            P0,
            source,
            Some(version),
            None,
            None
        ));
        state
            .move_object(
                source,
                ZoneLocation::Exile(P0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .unwrap();
        state
            .move_object(
                source,
                ZoneLocation::Graveyard(P0),
                crate::zone::ZonePosition::Bottom,
                crate::event::Cause::Effect,
            )
            .unwrap();
        assert!(
            intervening_if(&state, condition, P0, source),
            "the new card independently satisfies the clause"
        );
        assert!(
            !intervening_if_for_incarnation(
                &state,
                condition,
                P0,
                source,
                Some(version),
                None,
                None
            ),
            "the old trigger cannot find that new card"
        );
    }

    /// A land on `controller`'s battlefield carrying `subtypes`.
    fn land(state: &mut GameState, controller: PlayerId, subtypes: &[SubtypeId]) -> ObjectId {
        let name = state.names.intern("Test Land");
        let id = state.create_bare(
            controller,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        let b = state.object_mut(id).expect("just created").base_mut();
        b.types = TypeSet::LAND;
        b.subtypes = SubtypeSet::from_slice(subtypes);
        id
    }

    /// Domain counts **types**, and that is the whole reason it is a variant
    /// of its own rather than either of the two amounts that were already
    /// there. `Amount::CountOf` counts objects, so two Forests would answer
    /// 2 where the card wants 1 and a Tundra 1 where it wants 2; a land is
    /// colourless, so `Amount::DistinctColorsAmong` answers 0 for any board
    /// of them. Both readings are asserted here beside the right one, so a
    /// later simplification back onto either is a red test rather than a
    /// card that quietly pumps by the wrong number.
    #[test]
    fn a_domain_count_is_over_basic_land_types_and_not_over_lands() {
        use baylee_core::generated::subtypes::land;
        static DOMAIN: Amount = Amount::BasicLandTypesAmong(&Filter::YOUR_LAND);
        static YOUR_LANDS: Filter = Filter::YOUR_LAND;

        let mut state = empty_state();
        let forest = land(&mut state, P0, &[land::FOREST]);
        let count = |state: &GameState| self::amount(&DOMAIN, state, P0, forest, None);

        assert_eq!(count(&state), 1, "one Forest is one basic land type");
        land(&mut state, P0, &[land::FOREST]);
        assert_eq!(
            count(&state),
            1,
            "a second Forest adds no type — this is where a count of objects parts company"
        );
        land(&mut state, P0, &[land::PLAINS, land::ISLAND]);
        assert_eq!(count(&state), 3, "a Tundra is two types on one land");
        land(&mut state, P1, &[land::MOUNTAIN, land::SWAMP]);
        assert_eq!(
            count(&state),
            3,
            "and the filter is `lands you control`: an opponent's Badlands is not yours"
        );

        // The two amounts this is not, over the same board.
        assert_eq!(
            self::amount(
                &Amount::CountOf {
                    filter: &YOUR_LANDS,
                    zone: ZoneSel::Battlefield
                },
                &state,
                P0,
                forest,
                None
            ),
            3,
            "three lands, four types — a count of objects is not domain"
        );
        assert_eq!(
            self::amount(
                &Amount::DistinctColorsAmong(&YOUR_LANDS),
                &state,
                P0,
                forest,
                None
            ),
            0,
            "and a land is colourless, whatever mana it makes"
        );

        // CR 305.6 names five and no other land type joins them.
        land(&mut state, P0, &[land::DESERT, land::GATE]);
        assert_eq!(
            count(&state),
            3,
            "Desert and Gate are land types (CR 205.3i) and not basic ones"
        );
    }

    /// `Filter::CmcAtMostX` reads its bound off the **source**, which is the
    /// only place the announced number exists at match time: a filter is
    /// evaluated with `this` in hand and with no `x` beside it. The pool
    /// cards that print "mana value X or less" are searches, so the bound
    /// being wrong is a tutor that finds a card it may not find.
    ///
    /// Zero is asserted beside the two live bounds because that is the
    /// answer for an ability that announced nothing, and it has to *mean*
    /// zero: a filter that could not read its bound and matched everything
    /// would be a tutor with no price at all.
    #[test]
    fn a_mana_value_bound_of_x_is_read_off_the_ability_source() {
        static WITHIN_X: Filter = Filter::CmcAtMostX;
        let mut state = empty_state();
        let source = creature(&mut state, P0, KeywordSet::EMPTY);
        let free = creature(&mut state, P0, KeywordSet::EMPTY);
        let two = creature(&mut state, P0, KeywordSet::EMPTY);
        state
            .object_mut(two)
            .expect("just created")
            .base_mut()
            .mana_cost = baylee_core::mana::ManaCost::parse("{1}{G}");

        let reaches = |state: &GameState, id: ObjectId| {
            let obj = state.object(id).expect("on the battlefield");
            matches(&WITHIN_X, state, obj, P0, source)
        };

        assert!(
            reaches(&state, free),
            "X is 0 and a cost of nothing is 0 or less"
        );
        assert!(
            !reaches(&state, two),
            "and a two-drop is not — an unreadable bound that matched would              be a tutor with no price"
        );

        state.object_mut(source).expect("just created").x_value = 2;
        assert!(reaches(&state, two), "X = 2 reaches a two-drop");

        state.object_mut(source).expect("just created").x_value = 1;
        assert!(
            !reaches(&state, two),
            "and X = 1 does not: the bound is the announced number and not              merely whether one was announced"
        );
    }

    /// Who `chooser` may point a creature-targeting spell at.
    fn targets(state: &GameState, chooser: PlayerId, source: ObjectId) -> Vec<ObjectId> {
        target_options(&TargetSpec::Object(&ANY_CREATURE), state, chooser, source)
    }

    /// Gives `object` protection from `filter`, as a continuous effect the
    /// seat `controller` controls.
    fn protect(
        state: &mut GameState,
        controller: PlayerId,
        object: ObjectId,
        filter: &'static Filter,
    ) {
        let filter_of = crate::effects::EffectFilter::object(state, object);
        state.effects.register(crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: Some(object),
            controller,
            origin: crate::effects::EffectOrigin::Resolution,
            layer: baylee_cards_dsl::Modifier::ProtectionFrom(filter).layer(),
            timestamp: 1,
            duration: baylee_cards_dsl::Duration::Indefinitely,
            filter: filter_of,
            modifier: baylee_cards_dsl::Modifier::ProtectionFrom(filter),
        });
    }

    /// Protection (CR 702.16) is two questions and both are asked of a
    /// different object: which permanent *has* it, and whether the thing
    /// coming at it matches the filter. An effect naming one creature does
    /// not protect the one beside it, and a creature with protection from
    /// artifacts is not protected from a creature.
    #[test]
    fn protection_is_read_off_the_protected_object_and_the_thing_it_faces() {
        static ARTIFACTS: Filter = Filter::ARTIFACT;
        let mut state = empty_state();
        let mine = creature(&mut state, P0, KeywordSet::EMPTY);
        let beside = creature(&mut state, P0, KeywordSet::EMPTY);
        let theirs = creature(&mut state, P1, KeywordSet::EMPTY);

        assert!(
            !protected_from(&state, mine, theirs),
            "nothing is protected from anything to begin with"
        );
        protect(&mut state, P0, mine, &ANY_CREATURE);
        assert!(protected_from(&state, mine, theirs));
        assert!(
            protected_from(&state, mine, beside),
            "protection from creatures is from all of them, mine included"
        );
        assert!(
            !protected_from(&state, beside, theirs),
            "and the creature standing next to it has none"
        );

        let other = creature(&mut state, P0, KeywordSet::EMPTY);
        protect(&mut state, P0, other, &ARTIFACTS);
        assert!(
            !protected_from(&state, other, theirs),
            "protection from artifacts is not protection from a creature"
        );
    }

    /// A choice that does not target (CR 115.10a), such as what a Clone
    /// enters as a copy of (CR 707.5), is offered what targeting refuses:
    /// an opponent's hexproof creature, a creature with shroud and one with
    /// protection from creatures are all off `target_options` for a
    /// creature source, and all on `choice_options` (the copy question's
    /// list, `copy_on_enter_question`). Red when the choice borrows the
    /// targeting filter, as the copy question did.
    #[test]
    fn a_choice_that_does_not_target_ignores_hexproof_shroud_and_protection() {
        let mut state = empty_state();
        let clone = creature(&mut state, P0, KeywordSet::EMPTY);
        let plain = creature(&mut state, P1, KeywordSet::EMPTY);
        let hexproof = creature(&mut state, P1, KeywordSet::HEXPROOF);
        let shroud = creature(&mut state, P1, KeywordSet::SHROUD);
        let protected = creature(&mut state, P1, KeywordSet::EMPTY);
        protect(&mut state, P1, protected, &ANY_CREATURE);
        let spec = TargetSpec::Object(&ANY_CREATURE);

        let targeted = targets(&state, P0, clone);
        assert!(targeted.contains(&plain));
        for refused in [hexproof, shroud, protected] {
            assert!(!targeted.contains(&refused), "a target: {refused:?}");
        }
        let chosen = choice_options(&spec, &state, P0, clone);
        for offered in [plain, hexproof, shroud, protected] {
            assert!(chosen.contains(&offered), "not a target: {offered:?}");
        }
    }

    /// **Whose opponent** is read off the seat that controls the protection
    /// effect and not off the permanent that carries it (CR 109.5: "you" is
    /// the controller of the ability). The two are the same on every printed
    /// card, which is exactly why a reader that used the permanent's
    /// controller would look right — so the case that tells them apart is
    /// the one worth writing down.
    #[test]
    fn protection_from_an_opponent_means_the_effects_controllers_opponent() {
        static OPPONENTS: Filter = Filter::ControlledByOpponent;
        let mut state = empty_state();
        let mine = creature(&mut state, P0, KeywordSet::EMPTY);
        let beside = creature(&mut state, P0, KeywordSet::EMPTY);
        let theirs = creature(&mut state, P1, KeywordSet::EMPTY);

        protect(&mut state, P0, mine, &OPPONENTS);
        assert!(protected_from(&state, mine, theirs));
        assert!(
            !protected_from(&state, mine, beside),
            "a creature I control is not one my opponent controls"
        );

        // The same protection, granted by the seat across the table: the
        // filter now reads from their side, so it is my own creature the
        // permanent is protected from.
        let odd = creature(&mut state, P0, KeywordSet::EMPTY);
        protect(&mut state, P1, odd, &OPPONENTS);
        assert!(
            protected_from(&state, odd, beside),
            "from their seat, a creature I control is an opponent's"
        );
        assert!(
            !protected_from(&state, odd, theirs),
            "and their own creature is not"
        );
    }

    /// An object that is not in the arena is protected from nothing and
    /// protects against nothing: both halves answer `false` rather than
    /// panicking, which is what lets a damage step ask about a creature that
    /// a state-based action has already taken away.
    #[test]
    fn protection_asked_about_something_that_is_gone_is_simply_false() {
        let mut state = empty_state();
        let mine = creature(&mut state, P0, KeywordSet::EMPTY);
        let theirs = creature(&mut state, P1, KeywordSet::EMPTY);
        protect(&mut state, P0, mine, &ANY_CREATURE);
        assert!(protected_from(&state, mine, theirs));

        let gone = ObjectId::new(9_999, 0);
        assert!(!protected_from(&state, gone, theirs));
        assert!(!protected_from(&state, mine, gone));
    }

    /// Hexproof (CR 702.11b) stops opponents and nobody else.
    #[test]
    fn hexproof_hides_a_creature_from_its_controllers_opponents() {
        let mut state = empty_state();
        let mine = creature(&mut state, P0, KeywordSet::HEXPROOF);
        let plain = creature(&mut state, P0, KeywordSet::EMPTY);
        let source = creature(&mut state, P1, KeywordSet::EMPTY);

        let theirs = targets(&state, P1, source);
        assert!(
            !theirs.contains(&mine),
            "an opponent could target a hexproof creature"
        );
        assert!(
            theirs.contains(&plain),
            "the opponent lost sight of an ordinary creature too"
        );

        assert!(
            targets(&state, P0, mine).contains(&mine),
            "hexproof stopped its own controller"
        );
    }

    /// Shroud (CR 702.18a) stops everyone, controller included — that is
    /// the whole difference between the two keywords.
    #[test]
    fn shroud_hides_a_creature_from_everyone() {
        let mut state = empty_state();
        let shrouded = creature(&mut state, P0, KeywordSet::SHROUD);
        let source = creature(&mut state, P1, KeywordSet::EMPTY);

        assert!(!targets(&state, P1, source).contains(&shrouded));
        assert!(
            !targets(&state, P0, shrouded).contains(&shrouded),
            "shroud let its own controller target it"
        );
    }

    /// Both keywords are battlefield-only (CR 702.11b): a card printed
    /// with hexproof is an ordinary target in any other zone.
    #[test]
    fn hexproof_does_not_function_outside_the_battlefield() {
        let mut state = empty_state();
        let card = creature(&mut state, P0, KeywordSet::HEXPROOF);
        state
            .move_object(
                card,
                ZoneLocation::Graveyard(P0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::TurnBased,
            )
            .expect("the creature dies");
        assert!(
            !untargetable_by(&state, card, P1),
            "hexproof kept working in the graveyard"
        );
    }

    /// `LacksType(t)` and `Not(&HasType(t))` are the same predicate, which
    /// is what lets the pool spell "not a creature" exactly one way.
    ///
    /// The two arms of the `match` above are literal negations of each
    /// other, so this could be read off the source — and that is precisely
    /// why it is worth a test: the equivalence is an *invariant* the card
    /// pool now depends on, not an accident of how those two lines happen
    /// to be written today. Every type, and both answers, so a rule that
    /// stopped negating would fail here rather than in a card.
    #[test]
    fn lacking_a_type_is_not_having_it() {
        // Written out in pairs rather than built in the loop: `Not` holds a
        // `&'static Filter`, and a reference taken to a value made from a
        // loop variable cannot be promoted (E0716). The pairing is the
        // assertion, so writing it is no loss.
        static NEGATIONS: &[(TypeSet, Filter, Filter)] = &[
            (
                TypeSet::ARTIFACT,
                Filter::LacksType(TypeSet::ARTIFACT),
                Filter::Not(&Filter::HasType(TypeSet::ARTIFACT)),
            ),
            (
                TypeSet::CREATURE,
                Filter::LacksType(TypeSet::CREATURE),
                Filter::Not(&Filter::HasType(TypeSet::CREATURE)),
            ),
            (
                TypeSet::ENCHANTMENT,
                Filter::LacksType(TypeSet::ENCHANTMENT),
                Filter::Not(&Filter::HasType(TypeSet::ENCHANTMENT)),
            ),
            (
                TypeSet::INSTANT,
                Filter::LacksType(TypeSet::INSTANT),
                Filter::Not(&Filter::HasType(TypeSet::INSTANT)),
            ),
            (
                TypeSet::KINDRED,
                Filter::LacksType(TypeSet::KINDRED),
                Filter::Not(&Filter::HasType(TypeSet::KINDRED)),
            ),
            (
                TypeSet::LAND,
                Filter::LacksType(TypeSet::LAND),
                Filter::Not(&Filter::HasType(TypeSet::LAND)),
            ),
            (
                TypeSet::PLANESWALKER,
                Filter::LacksType(TypeSet::PLANESWALKER),
                Filter::Not(&Filter::HasType(TypeSet::PLANESWALKER)),
            ),
            (
                TypeSet::SORCERY,
                Filter::LacksType(TypeSet::SORCERY),
                Filter::Not(&Filter::HasType(TypeSet::SORCERY)),
            ),
            (
                TypeSet::BATTLE,
                Filter::LacksType(TypeSet::BATTLE),
                Filter::Not(&Filter::HasType(TypeSet::BATTLE)),
            ),
        ];

        let mut state = empty_state();
        let obj = creature(&mut state, P0, KeywordSet::EMPTY);
        let object = state.object(obj).expect("just created");
        for (t, lacks, negated) in NEGATIONS {
            assert_eq!(
                matches(lacks, &state, object, P0, obj),
                matches(negated, &state, object, P0, obj),
                "LacksType and Not(HasType) disagreed about {t:?}"
            );
        }

        // The counter-test: the object really is a creature and really is
        // not a land, so the loop above compared both answers and not one
        // answer nine times.
        assert!(matches(&Filter::CREATURE, &state, object, P0, obj));
        assert!(matches(&Filter::NONLAND, &state, object, P0, obj));
        assert!(!matches(&Filter::NONCREATURE, &state, object, P0, obj));
    }

    /// The `None` is the whole point of the signature. Answering `vec![]`
    /// for a relation this function cannot resolve reads exactly like "no
    /// seats matched", and that is what shipped Abraded Bluffs as a land
    /// that deals no damage and Bojuka Bog as a swamp: only a caller
    /// holding a `Resolution` can answer these two, and it must not be able
    /// to swallow them silently.
    #[test]
    fn a_relation_the_state_cannot_answer_is_none_rather_than_nobody() {
        let state = empty_state();
        assert_eq!(players(PlayerRel::ControllerOfTarget, &state, P0), None);
        assert_eq!(players(PlayerRel::Chosen, &state, P0), None);

        assert_eq!(players(PlayerRel::You, &state, P0), Some(vec![P0]));
        assert_eq!(players(PlayerRel::Opponent, &state, P0), Some(vec![P1]));
        assert_eq!(
            players(PlayerRel::EachPlayer, &state, P0),
            Some(vec![P0, P1])
        );
        assert_eq!(
            players(PlayerRel::EachOpponent, &state, P1),
            Some(vec![P0]),
            "the relation is read from whoever is asking"
        );
    }

    /// A player who has lost is not a player the game asks anything of —
    /// "each opponent loses 1 life" resolving after somebody conceded must
    /// not find them.
    #[test]
    fn a_player_who_has_lost_is_no_longer_each_player() {
        let mut state = empty_state();
        state.players[1].loss = Some(crate::event::LossReason::Conceded);
        assert_eq!(players(PlayerRel::EachPlayer, &state, P0), Some(vec![P0]));
        assert_eq!(players(PlayerRel::EachOpponent, &state, P0), Some(vec![]));
        assert_eq!(
            players(PlayerRel::You, &state, P0),
            Some(vec![P0]),
            "and you are still you"
        );
    }

    /// "If you control three or more creatures" is about the asker's own
    /// board, whoever else has one.
    #[test]
    fn a_control_count_counts_the_askers_own_permanents() {
        let mut state = empty_state();
        let mine = creature(&mut state, P0, KeywordSet::EMPTY);
        creature(&mut state, P0, KeywordSet::EMPTY);
        creature(&mut state, P1, KeywordSet::EMPTY);

        let two = Condition::ControlCount(&ANY_CREATURE, 2);
        let three = Condition::ControlCount(&ANY_CREATURE, 3);
        assert!(condition_holds(&state, P0, mine, two));
        assert!(
            !condition_holds(&state, P0, mine, three),
            "theirs is theirs"
        );
        assert!(!condition_holds(&state, P1, mine, two));
    }

    /// A power comparison reads the **projected** number, and an object
    /// with no power at all is not "a creature with power 4 or greater".
    ///
    /// Six cards in this pool print a power or toughness comparison and
    /// every one of them had the ability that states it taken off the card
    /// while `Filter` could not say it — Bonders' Enclave was a land with
    /// no draw, Access Tunnel a land with one ability. The `is_some_and` is
    /// the half that is easy to get wrong in the other direction: a land
    /// answering `0 <= 3` would make Access Tunnel target one.
    #[test]
    fn a_power_comparison_reads_a_projected_number_and_an_object_that_has_none_is_not_small() {
        let mut state = empty_state();
        let c = creature(&mut state, P0, KeywordSet::EMPTY);
        let l = land(&mut state, P0, &[]);
        let ask = |state: &GameState, f: &Filter, id: ObjectId| {
            matches(f, state, state.object(id).expect("still here"), P0, id)
        };

        // A 1/1 to begin with.
        assert!(ask(&state, &Filter::PowerAtMost(3), c));
        assert!(!ask(&state, &Filter::PowerAtLeast(4), c));
        assert!(!ask(&state, &Filter::ToughnessAtLeast(4), c));

        // The land has neither number, so it is on **no** side of either
        // comparison — the bound is not a default of nought.
        assert!(!ask(&state, &Filter::PowerAtMost(3), l));
        assert!(!ask(&state, &Filter::PowerAtLeast(4), l));
        assert!(!ask(&state, &Filter::ToughnessAtLeast(4), l));
        assert!(
            !ask(&state, &Filter::ToughnessAtMost(3), l),
            "the predicate that was already here answers the same way, \
             which is what makes the three new ones its siblings"
        );

        // Grown to a 4/4 — through the base characteristics, because this
        // module tests the predicate and not the layer system. A card that
        // reached 4 power under an anthem reaches it here the same way,
        // since `matches` is handed the projected characteristics either
        // way.
        {
            let b = state.object_mut(c).expect("still here").base_mut();
            b.power = Some(4);
            b.toughness = Some(4);
        }
        state.invalidate_projections();
        assert!(ask(&state, &Filter::PowerAtLeast(4), c));
        assert!(ask(&state, &Filter::ToughnessAtLeast(4), c));
        assert!(
            !ask(&state, &Filter::PowerAtMost(3), c),
            "and the creature has grown out of the other card's restriction"
        );
    }

    /// "Activate only during combat" holds in each step of the combat phase
    /// (CR 506.1), whoever's turn it is, and in no other phase.
    #[test]
    fn during_combat_is_the_combat_phase_of_any_turn() {
        use crate::turn::{Phase, Step};
        let mut state = empty_state();
        let this = creature(&mut state, P0, KeywordSet::EMPTY);
        let during = |state: &GameState| condition_holds(state, P0, this, Condition::DuringCombat);
        for (phase, step, holds) in [
            (Phase::FirstMain, Step::Main, false),
            (Phase::Combat, Step::CombatBegin, true),
            (Phase::Combat, Step::DeclareBlockers, true),
            (Phase::Combat, Step::CombatEnd, true),
            (Phase::SecondMain, Step::Main, false),
            (Phase::Ending, Step::End, false),
        ] {
            state.turn.phase = phase;
            state.turn.step = step;
            assert_eq!(during(&state), holds, "{step:?}");
        }
        state.turn.phase = Phase::Combat;
        state.turn.step = Step::DeclareAttackers;
        state.turn.active = P1;
        assert!(during(&state), "the opponent's combat too");
    }

    /// "For each creature that died this turn" reads the turn's tally, which
    /// the move to a graveyard writes and the turn's end clears.
    #[test]
    fn the_turns_deaths_are_the_tally_the_moves_wrote() {
        let mut state = empty_state();
        let this = creature(&mut state, P0, KeywordSet::EMPTY);
        let died = Amount::CreaturesDiedThisTurn;
        assert_eq!(amount(&died, &state, P0, this, None), 0);
        for _ in 0..2 {
            let dying = creature(&mut state, P1, KeywordSet::EMPTY);
            state
                .move_object(
                    dying,
                    ZoneLocation::Graveyard(P1),
                    crate::zone::ZonePosition::Top,
                    crate::event::Cause::Effect,
                )
                .expect("it moves");
        }
        assert_eq!(amount(&died, &state, P0, this, None), 2, "either player's");
    }

    /// "Toughness less than Stone Giant's power": compared with the
    /// source's projected power, strictly, and a source with no power
    /// bounds nothing in.
    #[test]
    fn a_comparison_with_the_sources_power_is_strict_and_reads_the_source() {
        let mut state = empty_state();
        let giant = creature(&mut state, P0, KeywordSet::EMPTY);
        let small = creature(&mut state, P0, KeywordSet::EMPTY);
        let land = land(&mut state, P0, &[]);
        {
            let b = state.object_mut(giant).expect("seated").base_mut();
            b.power = Some(3);
            b.toughness = Some(4);
        }
        {
            let b = state.object_mut(small).expect("seated").base_mut();
            b.power = Some(3);
            b.toughness = Some(2);
        }
        state.invalidate_projections();
        let ask = |state: &GameState, f: &Filter, id: ObjectId, source: ObjectId| {
            matches(f, state, state.object(id).expect("still here"), P0, source)
        };
        assert!(ask(
            &state,
            &Filter::ToughnessLessThanSourcePower,
            small,
            giant
        ));
        assert!(
            !ask(&state, &Filter::PowerLessThanSourcePower, small, giant),
            "3 is not less than 3"
        );
        assert!(
            !ask(&state, &Filter::ToughnessLessThanSourcePower, giant, giant),
            "the giant's own 4 is not less than its 3"
        );
        assert!(
            !ask(&state, &Filter::ToughnessLessThanSourcePower, small, land),
            "a source with no power bounds nothing in"
        );
        state.object_mut(giant).expect("seated").base_mut().power = Some(2);
        state.invalidate_projections();
        assert!(
            !ask(&state, &Filter::ToughnessLessThanSourcePower, small, giant),
            "the source's power as it is now"
        );
    }

    /// "If no creatures are on the battlefield" (Pestilence) counts both
    /// sides of the table, where "you control" counts one.
    #[test]
    fn a_battlefield_count_is_everybodys() {
        let mut state = empty_state();
        let theirs = creature(&mut state, P1, KeywordSet::EMPTY);

        let none = Condition::BattlefieldCountAtMost(&ANY_CREATURE, 0);
        let some = Condition::BattlefieldCount(&ANY_CREATURE, 1);
        assert!(!condition_holds(&state, P0, theirs, none), "theirs counts");
        assert!(condition_holds(&state, P0, theirs, some));
        assert!(
            condition_holds(
                &state,
                P0,
                theirs,
                Condition::ControlCountAtMost(&ANY_CREATURE, 0)
            ),
            "the control: P0 controls none of it"
        );
        let bare = empty_state();
        assert!(condition_holds(&bare, P0, theirs, none));
        assert!(!condition_holds(&bare, P0, theirs, some));
    }

    /// Blaze of Glory's "creature defending player controls": during the
    /// combat phase, a creature an opponent of the active player controls
    /// (CR 506.2, 802.2), whoever `you` is; outside it, nobody's. A
    /// teammate of the active player defends nothing.
    #[test]
    fn a_defending_players_creatures_are_the_active_players_opponents_in_combat() {
        static DEFENDING: Filter = Filter::ControlledByDefendingPlayer;
        let mut state = empty_state();
        let mine = creature(&mut state, P0, KeywordSet::EMPTY);
        let theirs = creature(&mut state, P1, KeywordSet::EMPTY);
        let asks = |state: &GameState, id| {
            state
                .object(id)
                .is_some_and(|o| matches(&DEFENDING, state, o, P1, id))
        };
        state.turn.active = P0;
        state.turn.phase = crate::turn::Phase::FirstMain;
        assert!(!asks(&state, theirs), "no defending player outside combat");
        state.turn.phase = crate::turn::Phase::Combat;
        assert!(asks(&state, theirs), "whoever `you` is");
        assert!(!asks(&state, mine), "the attacking player defends nothing");
        state.turn.active = P1;
        assert!(asks(&state, mine) && !asks(&state, theirs));
        for player in &mut state.players {
            player.team = Some(1);
        }
        assert!(!asks(&state, mine), "a teammate is not attacked");
    }

    /// Karma's "Swamps they control" is the active player's, and Spell
    /// Blast's "mana value X" is the X its own source announced.
    #[test]
    fn the_active_players_permanents_and_a_mana_value_of_exactly_x() {
        static ACTIVES: Filter = Filter::ControlledByActivePlayer;
        static EXACTLY_X: Filter = Filter::CmcExactlyX;
        let mut state = empty_state();
        let mine = creature(&mut state, P0, KeywordSet::EMPTY);
        let theirs = creature(&mut state, P1, KeywordSet::EMPTY);
        state.turn.active = P1;
        let asks = |state: &GameState, id, filter: &Filter| {
            state
                .object(id)
                .is_some_and(|o| matches(filter, state, o, P0, id))
        };
        assert!(asks(&state, theirs, &ACTIVES), "whoever `you` is");
        assert!(!asks(&state, mine, &ACTIVES));
        state.turn.active = P0;
        assert!(asks(&state, mine, &ACTIVES));

        let spell = creature(&mut state, P0, KeywordSet::EMPTY);
        let value = state
            .object(mine)
            .map_or(0, |o| o.characteristics().mana_value());
        let check = |state: &GameState| {
            state
                .object(mine)
                .is_some_and(|o| matches(&EXACTLY_X, state, o, P0, spell))
        };
        state.object_mut(spell).expect("here").x_value = value;
        assert!(check(&state));
        state.object_mut(spell).expect("here").x_value = value + 1;
        assert!(!check(&state), "at most X would have said yes");
    }

    /// "You control no artifacts" is not the negation of a minimum with the
    /// number moved: it is a different comparison, and Glimmervoid and
    /// Thran Quarry both shipped with the clause **off** while it could not
    /// be said — which is a land that never sacrifices itself.
    #[test]
    fn a_control_count_downwards_is_a_different_question_from_one_upwards() {
        let mut state = empty_state();
        let mine = creature(&mut state, P0, KeywordSet::EMPTY);

        let none = Condition::ControlCountAtMost(&ANY_CREATURE, 0);
        let one = Condition::ControlCountAtMost(&ANY_CREATURE, 1);
        assert!(!condition_holds(&state, P0, mine, none));
        assert!(condition_holds(&state, P0, mine, one));
        assert!(
            condition_holds(&state, P1, mine, none),
            "the other seat controls none of it"
        );

        // On a board with none of them the sentence turns over, and its
        // upward twin turns over with it — the two really are reading one
        // count and not two.
        let bare = empty_state();
        assert!(condition_holds(&bare, P0, mine, none));
        assert!(!condition_holds(
            &bare,
            P0,
            mine,
            Condition::ControlCount(&ANY_CREATURE, 1)
        ));
    }

    /// "An opponent controls four or more lands" is `any` over the seats
    /// and never your own board: Tectonic Edge had been activating on a
    /// table where nobody else had a land at all, which is a Wasteland.
    #[test]
    fn an_opponent_control_count_is_any_other_seat_and_never_your_own() {
        let mut state = empty_state();
        let mine = creature(&mut state, P0, KeywordSet::EMPTY);
        creature(&mut state, P0, KeywordSet::EMPTY);
        let two = Condition::OpponentControlCount(&ANY_CREATURE, 2);

        assert!(
            !condition_holds(&state, P0, mine, two),
            "two of your own are not two of an opponent's"
        );
        assert!(
            condition_holds(&state, P1, mine, two),
            "and they are, asked from the other side"
        );

        creature(&mut state, P1, KeywordSet::EMPTY);
        creature(&mut state, P1, KeywordSet::EMPTY);
        assert!(condition_holds(&state, P0, mine, two));
    }

    /// Hellbent and Library of Alexandria are one word apart and never the
    /// same board: "at most nought" is true of an empty hand and "exactly
    /// seven" stops being true the moment the seventh card is drawn on.
    #[test]
    fn a_hand_size_is_counted_on_the_asking_seats_own_hand() {
        let mut state = empty_state();
        let source = creature(&mut state, P0, KeywordSet::EMPTY);
        let deal = |state: &mut GameState, who: PlayerId, n: usize| {
            for _ in 0..n {
                let name = state.names.intern("Test Card");
                state.create_bare(who, ObjectKind::Card, name, ZoneLocation::Hand(who));
            }
        };

        let empty = Condition::HandSizeAtMost(0);
        let seven = Condition::HandSizeExactly(7);
        assert!(condition_holds(&state, P0, source, empty));
        assert!(!condition_holds(&state, P0, source, seven));

        deal(&mut state, P1, 7);
        assert!(
            condition_holds(&state, P0, source, empty),
            "their hand is not yours"
        );
        assert!(!condition_holds(&state, P0, source, seven));
        assert!(condition_holds(&state, P1, source, seven));

        deal(&mut state, P0, 7);
        assert!(!condition_holds(&state, P0, source, empty));
        assert!(condition_holds(&state, P0, source, seven));

        deal(&mut state, P0, 1);
        assert!(
            !condition_holds(&state, P0, source, seven),
            "the eighth card is what makes this land stop working"
        );
    }

    /// Two sentences that look alike and are not: "if it has three or more"
    /// against "if it has exactly three". A card that removes counters as a
    /// cost sits on both sides of that difference.
    #[test]
    fn at_least_and_exactly_are_different_questions() {
        let mut state = empty_state();
        let source = creature(&mut state, P0, KeywordSet::EMPTY);
        state
            .object_mut(source)
            .expect("just made it")
            .counters
            .add(baylee_cards_dsl::CounterKind::P1P1, 3);

        let kind = baylee_cards_dsl::CounterKind::P1P1;
        assert!(condition_holds(
            &state,
            P0,
            source,
            Condition::CountersOnSelf(kind, 3)
        ));
        assert!(condition_holds(
            &state,
            P0,
            source,
            Condition::CountersOnSelf(kind, 2)
        ));
        assert!(!condition_holds(
            &state,
            P0,
            source,
            Condition::CountersOnSelf(kind, 4)
        ));
        assert!(condition_holds(
            &state,
            P0,
            source,
            Condition::CountersOnSelfExactly(kind, 3)
        ));
        assert!(!condition_holds(
            &state,
            P0,
            source,
            Condition::CountersOnSelfExactly(kind, 2)
        ));
    }

    /// A leveler's `{LEVEL N1-N2}` band (CR 711.2a) is closed at both ends:
    /// Hexdrinker's "LEVEL 3-7" holds at three and at seven and at neither
    /// two nor eight.
    #[test]
    fn a_level_band_holds_at_both_ends_and_not_beyond_them() {
        let mut state = empty_state();
        let source = creature(&mut state, P0, KeywordSet::EMPTY);
        let kind = baylee_cards_dsl::CounterKind::Level;
        let band = Condition::CountersOnSelfBetween(kind, 3, 7);
        let mut held = Vec::new();
        for _ in 0..9 {
            held.push(condition_holds(&state, P0, source, band));
            state
                .object_mut(source)
                .expect("just made it")
                .counters
                .add(kind, 1);
        }
        assert_eq!(
            held,
            [false, false, false, true, true, true, true, true, false],
            "levels 0 to 8"
        );
    }

    /// CR 113.7a: an ability is a separate object from its source the
    /// moment it goes on the stack, so "if this land is tapped" asked of a
    /// land that has left the battlefield has nothing to be true of.
    #[test]
    fn a_condition_about_a_source_that_is_gone_is_false() {
        let mut state = empty_state();
        let source = creature(&mut state, P0, KeywordSet::EMPTY);
        let about_itself = Condition::SourceMatches(&ANY_CREATURE);
        assert!(condition_holds(&state, P0, source, about_itself));

        state.arena.remove(source);
        assert!(!condition_holds(&state, P0, source, about_itself));
        assert!(
            !condition_holds(
                &state,
                P0,
                source,
                Condition::CountersOnSelf(baylee_cards_dsl::CounterKind::P1P1, 0)
            ),
            "every other sentence about the source fails the same way, \
             including the one a zero would otherwise make trivially true"
        );
    }

    /// CR 603.4 asks the clause twice — once where the ability would
    /// trigger and once where it would resolve — so both readings go
    /// through one function. An ability that prints no clause at all is the
    /// trivially true one, and an absent clause answering `false` would
    /// silence every trigger in the pool that does not print one.
    #[test]
    fn an_ability_with_no_intervening_if_has_a_true_one() {
        let mut state = empty_state();
        let source = creature(&mut state, P0, KeywordSet::EMPTY);

        assert!(intervening_if(&state, None, P0, source));
        assert!(intervening_if(
            &state,
            Some(Condition::ControlCount(&ANY_CREATURE, 1)),
            P0,
            source
        ));
        assert!(!intervening_if(
            &state,
            Some(Condition::ControlCount(&ANY_CREATURE, 2)),
            P0,
            source
        ));
    }

    /// "Target spell or permanent" is the spells on the stack and the
    /// permanents on the battlefield. An ability on the stack is neither
    /// (CR 113.9, CR 110.1), and offering one let every Venser trigger aim
    /// at the triggers stacked under it until the menu outgrew a `u8` (l29
    /// game 1930). The spell is asserted beside it, so a fix that dropped
    /// the stack half altogether is red too.
    #[test]
    fn a_spell_or_permanent_is_never_an_ability_on_the_stack() {
        static SPELL_OR_PERMANENT: TargetSpec = TargetSpec::StackOrBattlefield(&Filter::Any);
        let mut state = empty_state();
        let permanent = creature(&mut state, P1, KeywordSet::EMPTY);
        let name = state.names.intern("Test Spell");
        let spell = state.create_bare(P1, ObjectKind::Spell, name, ZoneLocation::Stack);
        let name = state.names.intern("Test Ability");
        let ability = state.create_bare(P1, ObjectKind::AbilityOnStack, name, ZoneLocation::Stack);

        let options = target_options(&SPELL_OR_PERMANENT, &state, P0, permanent);
        assert!(
            options.contains(&spell),
            "a spell on the stack: {options:?}"
        );
        assert!(
            options.contains(&permanent),
            "a permanent on the battlefield: {options:?}"
        );
        assert!(
            !options.contains(&ability),
            "an ability on the stack is no spell: {options:?}"
        );
        assert_eq!(options.len(), 2, "and nothing else: {options:?}");
    }
    #[test]
    fn combat_participation_survives_removal_but_not_a_new_incarnation_or_combat() {
        let mut state = empty_state();
        let id = creature(&mut state, P0, KeywordSet::EMPTY);
        let version = state.object(id).unwrap().version;
        state.combat.participants.push((id, version));
        let participated =
            |s: &GameState| condition_holds(s, P0, id, Condition::AttackedOrBlockedThisCombat);
        assert!(participated(&state));
        state.combat.remove_from_combat(id);
        assert!(
            participated(&state),
            "removal cannot undo an attack or block declaration"
        );
        state.object_mut(id).unwrap().version += 1;
        assert!(
            !participated(&state),
            "a later incarnation did not participate"
        );
        state.combat.participants.push((id, version + 1));
        assert!(participated(&state));
        state.combat = crate::combat::CombatState::default();
        assert!(!participated(&state), "an additional combat starts fresh");
    }

    #[test]
    fn losing_all_abilities_removes_intrinsic_land_mana_as_well_as_printed_mana() {
        let mut state = empty_state();
        let id = land(
            &mut state,
            P0,
            &[baylee_core::generated::subtypes::land::FOREST],
        );
        assert!(!crate::casting::intrinsic_mana_colors(&state, id).is_empty());
        assert!(matches(
            &Filter::HasManaAbility,
            &state,
            state.object(id).unwrap(),
            P0,
            id
        ));
        state.object_mut(id).unwrap().base_mut().abilities_lost = std::num::NonZeroU32::new(1);
        assert!(crate::casting::intrinsic_mana_colors(&state, id).is_empty());
        assert!(!matches(
            &Filter::HasManaAbility,
            &state,
            state.object(id).unwrap(),
            P0,
            id
        ));
    }
}
