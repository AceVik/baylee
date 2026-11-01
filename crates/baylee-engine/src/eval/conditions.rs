//! Conditions: whether a stated, intervening-if or event-reading `Condition` holds.

use super::{amount_with_context, live_context, matches, matches_with_context, most_life};
use crate::state::GameState;
use crate::text_changes::RuleContext;
use crate::zone::{Zone, ZoneLocation};
use baylee_cards_dsl::{Condition, Filter};
use baylee_core::ids::{ObjectId, PlayerId};

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
        Condition::SourceBlockedOrWasBlockedSinceYourLastUpkeep => {
            super::upkeep_history::blocked_since_last_upkeep(state, you, source)
        }
        Condition::SomeDefendingPlayerControlsNone(filter) => {
            super::defender_conditions::some_defender_controls_none(state, you, context, filter)
        }
        Condition::CreatureDamagedBySourceDiedThisTurn => state
            .source_identity(source)
            .is_some_and(|source| state.source_damage_caused_death(source)),
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
        Condition::ChosenColorIsUniqueMostCommon(filter) => {
            let Some(object) = state.object(source) else {
                return false;
            };
            let (Some(opponent), Some(color)) = (object.chosen_opponent(), object.chosen_color)
            else {
                return false;
            };
            if state.has_left(opponent) || color == baylee_core::mana::ManaColor::Colorless {
                return false;
            }
            let mut counts = [0usize; 5];
            for object in state.battlefield_seen().filter_map(|id| state.object(id)) {
                if object.controller != opponent
                    || !matches_with_context(filter, state, object, you, context)
                {
                    continue;
                }
                for candidate in baylee_core::color::Color::ALL {
                    if object.characteristics().colors.contains(candidate) {
                        counts[candidate as usize] += 1;
                    }
                }
            }
            let chosen = color.index();
            counts[chosen] > 0
                && counts
                    .iter()
                    .enumerate()
                    .all(|(index, count)| index == chosen || *count < counts[chosen])
        }
        Condition::AfterCombat => matches!(
            state.turn.phase,
            crate::turn::Phase::SecondMain | crate::turn::Phase::Ending
        ),
        Condition::Revolt => state.per_turn.revolt[usize::from(you.get())],
        Condition::CastInstantOrSorceryThisTurn => {
            state.per_turn.instant_or_sorcery_cast[usize::from(you.get())]
        }
        Condition::CastCreatureThisTurn => {
            let seat = usize::from(you.get());
            state.per_turn.spells_cast_by(you)
                > state
                    .per_turn
                    .noncreature_spells
                    .get(seat)
                    .copied()
                    .unwrap_or(0)
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
        Condition::AmountIsEven(amount) => {
            amount_with_context(&amount, state, you, context, None).is_multiple_of(2)
        }
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
        Condition::HandSizeAtTurnStartAtMost(max) => {
            state.per_turn.hand_size_at_start <= u32::from(max)
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
        Some(Condition::CreatureDamagedBySourceDiedThisTurn) => version
            .or_else(|| state.object(source).map(|object| object.version))
            .is_some_and(|version| {
                state.source_damage_caused_death(baylee_core::ids::DamageSourceRef {
                    object: source,
                    version,
                })
            }),
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
