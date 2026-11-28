//! Conditions: whether a stated, intervening-if or event-reading `Condition` holds.

use super::{amount_with_context, live_context, matches, matches_with_context, most_life};
use crate::state::GameState;
use crate::text_changes::RuleContext;
use crate::zone::{Zone, ZoneLocation};
use baylee_cards_dsl::{Condition, Filter};
use baylee_core::ids::{ObjectId, PlayerId};

pub(crate) fn discarded_for_cost(
    state: &GameState,
    paid_on: ObjectId,
    you: PlayerId,
    context: RuleContext,
    filter: &Filter,
) -> bool {
    state
        .object(paid_on)
        .and_then(|object| object.paid.as_ref())
        .is_some_and(|paid| {
            paid.discarded
                .iter()
                .filter_map(|reference| state.source_object(*reference))
                .any(|object| matches_with_context(filter, state, object, you, context))
        })
}

pub(crate) fn sacrificed_for_cost(
    state: &GameState,
    paid_on: ObjectId,
    you: PlayerId,
    context: RuleContext,
    filter: &Filter,
) -> bool {
    state
        .object(paid_on)
        .and_then(|object| object.paid.as_ref())
        .and_then(|paid| paid.sacrificed)
        .and_then(|(object, version)| {
            state.source_object(baylee_core::ids::DamageSourceRef { object, version })
        })
        .is_some_and(|object| matches_with_context(filter, state, object, you, context))
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
        Condition::SourceBlockedOrWasBlockedSinceYourLastUpkeep => {
            super::upkeep_history::blocked_since_last_upkeep(state, you, source)
        }
        Condition::SourceCameUnderYourControlSinceYourLastUpkeep => {
            super::upkeep_history::came_under_your_control(state, you, source, None)
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
        Condition::SourceWasCast => state
            .object(source)
            .is_some_and(|object| object.cast_history.caster().is_some()),
        Condition::CastFromOtherThanHand => state.object(source).is_some_and(|object| {
            object.cast_history.caster().is_some() && !object.cast_history.was_from_hand()
        }),
        Condition::SourceCastFromZone(zone) => state
            .object(source)
            .is_some_and(|object| super::object_was_cast_from_zone(object, zone)),
        Condition::DiscardedForCost(filter) => {
            discarded_for_cost(state, source, you, context, filter)
        }
        Condition::SacrificedForCost(filter) => {
            sacrificed_for_cost(state, source, you, context, filter)
        }
        // These need the triggering incarnation, supplied separately, or a
        // resolution (`resolve::immediate::resolution_condition`).
        Condition::ControlCountExceptEvent(_, _)
        | Condition::EventBlockedByOthers { .. }
        | Condition::SourceWasBlockedThisTurn
        | Condition::SoulbondEventCanPair
        | Condition::TargetSpellWouldDestroy(_)
        | Condition::EvolveFromEvent
        | Condition::RememberedMatches(_)
        | Condition::YouAreRememberedPlayer => false,
        // "Only after combat": a main or ending phase after this turn's
        // combat (the postcombat main phase or the end phase).
        Condition::ChosenOpponentControls(filter) => state
            .object(source)
            .and_then(crate::object::GameObject::chosen_opponent_for_rules)
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
            let (Some(opponent), Some(color)) =
                (object.chosen_opponent_for_rules(), object.chosen_color)
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
        Condition::LifeBelowStarting => {
            let player = &state.players[usize::from(you.get())];
            player.life < player.starting_life
        }
        Condition::APlayerHasMostLife => most_life(state).is_some(),
        // The announced X on the source (or resolving ability on the stack),
        // where `cast_wizard` / activation writes it and `Filter::CmcAtMostX` reads it.
        Condition::XAtLeast(n) => super::announced_x(state, source) >= n,
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
        Condition::ControlMoreThanEachOtherPlayer(filter) => {
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
            counts.iter().enumerate().all(|(seat, &count)| {
                u8::try_from(seat).is_ok_and(|seat| {
                    let player = PlayerId::new(seat);
                    player == you || state.has_left(player) || yours > count
                })
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
        Condition::AnyGraveyardCountAtLeast(min) => state.players.iter().any(|player| {
            !player.has_lost()
                && state.zones.list(ZoneLocation::Graveyard(player.id)).len() >= usize::from(min)
        }),
        Condition::CountersPlacedOnSourceThisTurn { kind, by_you } => {
            counters_placed_on_source_this_turn(state, you, source, None, kind, by_you)
        }
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
        Condition::ClassLevelExactly(_) | Condition::ClassLevelAtLeast(_) => {
            condition_with_event(state, you, context, condition, None, None)
        }
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
        Condition::KickedWith(index) => state.object(source).is_some_and(|o| o.kicked_with(index)),
        Condition::BlitzCostPaid => state.object(source).is_some_and(|o| {
            o.zone == crate::zone::Zone::Battlefield
                && o.riders.contains(&crate::object::Rider::Blitzed)
        }),
        Condition::AltCostPaid => state.object(source).is_some_and(|o| o.alt_cast),
        Condition::OpponentLostLifeThisTurn => state.players.iter().any(|p| {
            !state.has_left(p.id)
                && state.is_opponent(p.id, you)
                && state.per_turn.life_lost[usize::from(p.id.get())]
        }),
        Condition::Surge => {
            let side = state.side_of(you);
            state.players.iter().any(|p| {
                !state.has_left(p.id)
                    && state.side_of(p.id) == side
                    && state.per_turn.spells_cast_by(p.id) > 0
            })
        }
        Condition::Prowl => {
            let Some(spell_obj) = state.object(source) else {
                return false;
            };
            let front = crate::casting::split_front(state, spell_obj);
            let spell_obj = front.as_ref().unwrap_or(spell_obj);
            let creature_types = spell_obj
                .characteristics()
                .subtypes
                .intersection(baylee_core::types::SubtypeSet::ALL_CREATURE);
            !creature_types.is_empty()
                && state.per_turn.damage_log.iter().any(|entry| {
                    entry.combat
                        && entry.amount > 0
                        && matches!(entry.to, crate::event::DamageTarget::Player(_))
                        && entry.snapshot.controller == you
                        && entry
                            .snapshot
                            .characteristics()
                            .subtypes
                            .intersects(creature_types)
                })
        }
        Condition::Bargained => state
            .object(source)
            .is_some_and(|object| object_was_bargained(state, object)),
        Condition::OptionalCostPaid => state.object(source).is_some_and(object_optional_cost_paid),
        Condition::ControlledSubtypeOnCast(sub) => state.object(source).is_some_and(|o| {
            o.riders
                .contains(&crate::object::Rider::ControlledSubtypeOnCast(sub))
        }),
        Condition::Prototyped => state
            .object(source)
            .is_some_and(|object| object_is_prototyped(state, object)),
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
        Condition::DuringStep(kind) => match kind {
            baylee_cards_dsl::StepKind::FirstMain => {
                state.turn.step == crate::turn::Step::Main && state.per_turn.main_phases_begun == 1
            }
            baylee_cards_dsl::StepKind::SecondMain => {
                state.turn.step == crate::turn::Step::Main && state.per_turn.main_phases_begun == 2
            }
            baylee_cards_dsl::StepKind::PostcombatMain => {
                state.turn.step == crate::turn::Step::Main && state.per_turn.main_phases_begun > 1
            }
            other => state.turn.step.kind() == Some(other),
        },
        // CR 506.8: "before" a point of the turn is before that point's
        // place in it, whether or not the step itself happens (506.7e).
        Condition::BeforeStep(kind) => match kind {
            baylee_cards_dsl::StepKind::FirstMain => {
                state.per_turn.main_phases_begun == 0
                    && state.turn.position() < crate::turn::position_of(kind)
            }
            baylee_cards_dsl::StepKind::SecondMain | baylee_cards_dsl::StepKind::PostcombatMain => {
                state.per_turn.main_phases_begun < 2
                    && state.turn.position() < crate::turn::position_of(kind)
            }
            other => state.turn.position() < crate::turn::position_of(other),
        },
        Condition::CanSacrifice(filter) => {
            controlled_matching_with_context(state, you, filter, you, context)
                .into_iter()
                .any(|id| state.can_be_sacrificed(id, Some(you)))
        }
        Condition::EvenCountersOnSelf => state.object(source).is_some_and(|o| {
            o.counters
                .iter()
                .map(|(_, n)| u32::from(n))
                .sum::<u32>()
                .is_multiple_of(2)
        }),
        Condition::ControlYourCommander => state.battlefield_seen().any(|id| {
            state
                .object(id)
                .is_some_and(|o| o.controller == you && state.holds_commander_of(you, o))
        }),
        Condition::SourceInZone(zone) => state.object(source).is_some_and(|o| {
            o.zone.matches_ref(zone)
                && (o.zone != Zone::Battlefield
                    || !o.status.contains(crate::object::Status::PHASED_OUT))
                && (o.zone != Zone::Exile || !o.status.contains(crate::object::Status::FACE_DOWN))
        }),
        Condition::InCombatAfterBlockers => {
            state.turn.phase == crate::turn::Phase::Combat
                && matches!(
                    state.turn.step,
                    crate::turn::Step::DeclareBlockers
                        | crate::turn::Step::CombatDamageFirst
                        | crate::turn::Step::CombatDamage
                        | crate::turn::Step::CombatEnd
                )
        }
        Condition::ObjectMatches { what, filter } => match *what {
            baylee_cards_dsl::TargetSpec::ThisObject => state.object(source).is_some_and(|o| {
                o.zone == Zone::Battlefield
                    && !o.status.contains(crate::object::Status::PHASED_OUT)
                    && matches_with_context(filter, state, o, you, context)
            }),
            _ => false,
        },
        Condition::AmountCompare { left, op, right } => op.holds(
            amount_with_context(left, state, you, context, None),
            amount_with_context(right, state, you, context, None),
        ),
        Condition::IsRenowned => state
            .object(source)
            .is_some_and(|object| object_is_renowned(state, object)),
        Condition::IsPaired => state.paired_partner(source).is_some(),
        Condition::SoulbondSelfCanPair => {
            state.is_unpaired_creature_controlled_by(source, None, you)
                && state.battlefield_seen().any(|id| {
                    id != source && state.is_unpaired_creature_controlled_by(id, None, you)
                })
        }
        Condition::GiftWasPromised => state
            .object_or_departed(source)
            .is_some_and(object_gift_was_promised),
        Condition::OpponentDealtDamageThisTurn => {
            super::amounts::damage_dealt_to_opponents_this_turn(state, you) > 0
        }
        Condition::WarpCostPaid => state.object(source).is_some_and(|o| {
            matches!(
                o.zone,
                crate::zone::Zone::Stack | crate::zone::Zone::Battlefield
            ) && o.riders.contains(&crate::object::Rider::WarpedSpell)
        }),
        Condition::MayhemCostPaid => state.object(source).is_some_and(|o| {
            matches!(
                o.zone,
                crate::zone::Zone::Stack | crate::zone::Zone::Battlefield
            ) && o.riders.contains(&crate::object::Rider::MayhemPaid)
        }),
        Condition::FirstMatchingSpellThisTurn(filter) => {
            let items = &state.per_turn.cast_spells_this_turn.items;
            let limit = state
                .object(source)
                .and_then(|obj| {
                    items.iter().position(|rec| {
                        rec.source.object == obj.id && rec.source.version == obj.version
                    })
                })
                .unwrap_or(items.len());
            !items[..limit].iter().any(|rec| {
                rec.controller == you && matches(filter, state, &rec.snapshot, you, source)
            })
        }
        Condition::SpellWarpedThisTurn(who) => match who {
            None => !state.per_turn.spells_warped.is_empty(),
            Some(rel) => {
                super::players_for_source(rel, state, you, Some(source)).is_some_and(|seats| {
                    state
                        .per_turn
                        .spells_warped
                        .iter()
                        .any(|p| seats.contains(p))
                })
            }
        },
        Condition::SnowManaWasSpent => super::amounts::snow_mana_spent_on(state, source, None) > 0,
        Condition::IsMonstrous => state
            .object(source)
            .is_some_and(|object| object_is_monstrous(state, object)),
        Condition::IsSuspected => state
            .object(source)
            .is_some_and(|object| object_is_suspected(state, object)),
        Condition::ChosenMode(mode) => state.object(source).is_some_and(|object| {
            object
                .riders
                .contains(&crate::object::Rider::ChosenMode(mode))
        }),
        Condition::WasForetold => object_was_foretold(state, source, None),
        Condition::WasSneaked => object_was_sneaked(state, source, None),
        Condition::TreasureManaWasSpent => object_spent_treasure_mana(state, source, None),
        Condition::OpponentLifeAtMost(max) => state
            .players
            .iter()
            .any(|p| !p.has_lost() && state.is_opponent(p.id, you) && p.life <= max),
        Condition::PlayerAttackedYouThisTurn(who) => {
            super::players_for_source(who, state, you, Some(source)).is_some_and(|seats| {
                seats
                    .into_iter()
                    .any(|seat| state.per_turn.attacked_players.items.contains(&(seat, you)))
            })
        }
        Condition::CapturedCountAtLeast(min) => state
            .resolution_remembered
            .as_ref()
            .is_some_and(|(_, list)| u32::try_from(list.len()).unwrap_or(u32::MAX) >= min),
        Condition::CapturedMatches(filter) => {
            state
                .resolution_remembered
                .as_ref()
                .is_some_and(|(_, list)| {
                    list.iter().any(|rem| {
                        state
                            .source_object(rem.reference)
                            .or_else(|| state.object_or_departed(rem.reference.object))
                            .is_some_and(|obj| super::matches(filter, state, obj, you, source))
                    })
                })
        }
        Condition::HasRingBearer => state
            .players
            .get(usize::from(you.get()))
            .and_then(|p| p.ring_bearer)
            .and_then(|id| state.object(id))
            .is_some_and(|o| {
                o.zone == crate::zone::Zone::Battlefield
                    && !o.status.contains(crate::object::Status::PHASED_OUT)
                    && o.controller == you
            }),
        Condition::DealtCombatDamageToPlayerByAssassinOrCommanderThisTurn => {
            state.per_turn.damage_log.iter().any(|entry| {
                entry.combat
                    && entry.amount > 0
                    && matches!(entry.to, crate::event::DamageTarget::Player(_))
                    && entry.snapshot.controller == you
                    && entry
                        .snapshot
                        .characteristics()
                        .types
                        .contains(baylee_core::types::TypeSet::CREATURE)
                    && (entry
                        .snapshot
                        .characteristics()
                        .subtypes
                        .contains(baylee_core::generated::subtypes::creature::ASSASSIN)
                        || state.holds_commander(&entry.snapshot))
            })
        }
        Condition::CommittedCrimeThisTurn(who) => {
            super::players_for_source(who, state, you, Some(source))
                .is_some_and(|seats| seats.iter().any(|&p| state.per_turn.committed_crime(p)))
        }
        Condition::ExpendedThisTurn(threshold) => state.per_turn.mana_expended_by(you) >= threshold,
        Condition::TributeWasNotPaid => !state
            .object_or_departed(source)
            .is_some_and(|obj| obj.riders.contains(&crate::object::Rider::TributePaid)),
        Condition::HasInitiative(who) => super::players_for_source(who, state, you, Some(source))
            .is_some_and(|seats| seats.iter().any(|&p| state.initiative == Some(p))),
        Condition::IsMonarch(who) => super::players_for_source(who, state, you, Some(source))
            .is_some_and(|seats| seats.iter().any(|&p| state.monarch == Some(p))),
        Condition::CompletedADungeon(who) => {
            super::players_for_source(who, state, you, Some(source)).is_some_and(|seats| {
                seats.iter().any(|&p| {
                    state
                        .players
                        .get(usize::from(p.get()))
                        .is_some_and(|pl| pl.dungeons_completed > 0)
                })
            })
        }
        Condition::OpponentHasNoCardsInHand => state.players.iter().any(|p| {
            !p.has_lost()
                && state.is_opponent(p.id, you)
                && state
                    .zones
                    .list(crate::zone::ZoneLocation::Hand(p.id))
                    .is_empty()
        }),
        Condition::AnyPlayerLifeAtMostHalfStarting => state
            .players
            .iter()
            .any(|p| !p.has_lost() && p.life <= p.starting_life.div_euclid(2)),
        Condition::YouControlSharingCreatureType(min_count) => {
            you_control_sharing_creature_type(state, you, min_count)
        }
        Condition::PlayerAttackedYouLastTurn => state
            .last_turn_attacked_players
            .items
            .iter()
            .any(|rec| rec.defender == you),
        Condition::IsNight => state.day_night == Some(crate::turn::DayNight::Night),
        Condition::IsDay => state.day_night == Some(crate::turn::DayNight::Day),
        Condition::TriggerOnly(inner) => condition_holds_with_context(state, you, context, *inner),
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

/// Event-side facts captured on a triggered ability for CR 603.4
/// intervening-`if` checks: the triggering object incarnation and the captured
/// event amount (including the arming resolution's `ThisWay` count on a
/// reflexive trigger, CR 603.12).
#[derive(Clone, Copy, Default)]
pub(crate) struct TriggerEventContext {
    pub object: Option<baylee_core::ids::DamageSourceRef>,
    pub amount: Option<u32>,
}

/// Resolution's intervening-if check with the source incarnation that
/// triggered. A graveyard-order clause cannot find a later object that
/// reuses the same engine handle (CR 400.7).
#[cfg(test)]
pub(crate) fn intervening_if_for_incarnation(
    state: &GameState,
    condition: Option<Condition>,
    you: PlayerId,
    source: ObjectId,
    version: Option<u32>,
    attachment: Option<(ObjectId, u32)>,
    event: Option<baylee_core::ids::DamageSourceRef>,
) -> bool {
    intervening_if_for_event(
        state,
        condition,
        you,
        source,
        version,
        attachment,
        TriggerEventContext {
            object: event,
            amount: None,
        },
    )
}

/// Intervening-`if` check (CR 603.4) with both the source incarnation and
/// captured event context (triggering object incarnation and event amount).
#[allow(clippy::too_many_lines)]
pub(crate) fn intervening_if_for_event(
    state: &GameState,
    condition: Option<Condition>,
    you: PlayerId,
    source: ObjectId,
    version: Option<u32>,
    attachment: Option<(ObjectId, u32)>,
    event: TriggerEventContext,
) -> bool {
    match condition {
        Some(Condition::All(conditions)) => conditions.iter().all(|condition| {
            intervening_if_for_event(
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
            intervening_if_for_event(
                state,
                Some(*condition),
                you,
                source,
                version,
                attachment,
                event,
            )
        }),
        Some(Condition::Not(condition)) => !intervening_if_for_event(
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
        Some(Condition::SourceWasCast) => state
            .damage_source(source, version)
            .is_some_and(|object| object.cast_history.caster().is_some()),
        Some(Condition::CastFromOtherThanHand) => {
            state.damage_source(source, version).is_some_and(|object| {
                object.cast_history.caster().is_some() && !object.cast_history.was_from_hand()
            })
        }
        Some(Condition::SourceCastFromZone(zone)) => state
            .damage_source(source, version)
            .is_some_and(|object| super::object_was_cast_from_zone(object, zone)),
        Some(Condition::KickedWith(index)) => state
            .damage_source(source, version)
            .is_some_and(|object| object.kicked_with(index)),
        Some(Condition::AltCostPaid) => state
            .damage_source(source, version)
            .is_some_and(|object| object.alt_cast),
        Some(Condition::Bargained) => state
            .damage_source(source, version)
            .is_some_and(|object| object_was_bargained(state, object)),
        Some(Condition::OptionalCostPaid) => state
            .damage_source(source, version)
            .is_some_and(object_optional_cost_paid),
        Some(Condition::ControlledSubtypeOnCast(sub)) => {
            state.damage_source(source, version).is_some_and(|object| {
                object
                    .riders
                    .contains(&crate::object::Rider::ControlledSubtypeOnCast(sub))
            })
        }
        Some(Condition::GiftWasPromised) => state
            .damage_source(source, version)
            .is_some_and(object_gift_was_promised),
        Some(Condition::Prototyped) => state
            .damage_source(source, version)
            .is_some_and(|object| object_is_prototyped(state, object)),
        Some(Condition::IsRenowned) => state
            .damage_source(source, version)
            .is_some_and(|object| object.riders.contains(&crate::object::Rider::Renowned)),
        Some(Condition::WarpCostPaid) => state
            .damage_source(source, version)
            .is_some_and(|object| object.riders.contains(&crate::object::Rider::WarpedSpell)),
        Some(Condition::MayhemCostPaid) => state
            .damage_source(source, version)
            .is_some_and(|object| object.riders.contains(&crate::object::Rider::MayhemPaid)),
        Some(Condition::SnowManaWasSpent) => state
            .damage_source(source, version)
            .is_some_and(|object| super::amounts::object_snow_mana_spent(object) > 0),
        Some(Condition::IsMonstrous) => state
            .damage_source(source, version)
            .is_some_and(|object| object.riders.contains(&crate::object::Rider::Monstrous)),
        Some(Condition::IsSuspected) => state
            .damage_source(source, version)
            .is_some_and(|object| object.riders.contains(&crate::object::Rider::Suspected)),
        Some(Condition::ChosenMode(mode)) => {
            state.damage_source(source, version).is_some_and(|object| {
                object
                    .riders
                    .contains(&crate::object::Rider::ChosenMode(mode))
            })
        }
        Some(Condition::WasForetold) => object_was_foretold(state, source, version),
        Some(Condition::WasSneaked) => object_was_sneaked(state, source, version),
        Some(Condition::TreasureManaWasSpent) => object_spent_treasure_mana(state, source, version),
        Some(Condition::SourceCameUnderYourControlSinceYourLastUpkeep) => {
            super::upkeep_history::came_under_your_control(state, you, source, version)
        }
        Some(Condition::AttachedMatches(filter)) => {
            attached_for_ability(state, source, version, attachment)
                .is_some_and(|o| matches(filter, state, o, you, source))
        }
        Some(Condition::GraveyardCardsAbove(_, _) | Condition::SourceInZone(_)) => {
            version.is_none_or(|version| state.object(source).is_some_and(|o| o.version == version))
                && intervening_if(state, condition, you, source)
        }
        Some(Condition::Station(_) | Condition::TriggerOnly(_)) | None => true,
        Some(condition) => condition_with_event_amount(
            state,
            you,
            live_context(state, source),
            condition,
            version,
            event.object,
            event.amount,
        ),
    }
}

/// An event-dependent clause excludes an incarnation, never a reusable handle.
pub(crate) fn condition_reads_event(condition: Condition) -> bool {
    match condition {
        Condition::ControlCountExceptEvent(_, _)
        | Condition::EventBlockedByOthers { .. }
        | Condition::SourceWasBlockedThisTurn
        | Condition::SoulbondSelfCanPair
        | Condition::SoulbondEventCanPair
        | Condition::EvolveFromEvent
        | Condition::ObjectMatches {
            what: &baylee_cards_dsl::TargetSpec::EventObject,
            ..
        } => true,
        Condition::All(conditions) | Condition::Any(conditions) => {
            conditions.iter().copied().any(condition_reads_event)
        }
        Condition::Not(condition) | Condition::TriggerOnly(condition) => {
            condition_reads_event(*condition)
        }
        _ => false,
    }
}

/// Evaluate a condition with the source and event incarnations available.
pub(crate) fn condition_with_event(
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
    condition: Condition,
    source_version: Option<u32>,
    event: Option<baylee_core::ids::DamageSourceRef>,
) -> bool {
    condition_with_event_amount(state, you, context, condition, source_version, event, None)
}

/// Evaluate a condition with the source incarnation, event incarnation, and
/// optional captured event amount available (CR 603.4 / CR 603.12).
#[allow(clippy::too_many_lines)] // One arm per Condition variant inspecting source/event incarnations.
pub(crate) fn condition_with_event_amount(
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
    condition: Condition,
    source_version: Option<u32>,
    event: Option<baylee_core::ids::DamageSourceRef>,
    event_amount: Option<u32>,
) -> bool {
    match condition {
        Condition::ClassLevelExactly(level) => state
            .damage_source(context.source, source_version)
            .is_some_and(|o| o.zone == Zone::Battlefield && o.class_level() == level),
        Condition::ClassLevelAtLeast(level) => state
            .damage_source(context.source, source_version)
            .is_some_and(|o| o.zone == Zone::Battlefield && o.class_level() >= level),
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
        Condition::SourceWasBlockedThisTurn => source_version.is_some_and(|version| {
            let attacker = baylee_core::ids::DamageSourceRef {
                object: context.source,
                version,
            };
            state
                .per_turn
                .blocks
                .iter()
                .any(|record| record.attacker == attacker)
        }),
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
        Condition::ObjectMatches {
            what: &baylee_cards_dsl::TargetSpec::EventObject,
            filter,
        } => event_object_matches(state, you, context, filter, event),
        Condition::All(conditions) => conditions.iter().all(|&condition| {
            condition_with_event_amount(
                state,
                you,
                context,
                condition,
                source_version,
                event,
                event_amount,
            )
        }),
        Condition::Any(conditions) => conditions.iter().any(|&condition| {
            condition_with_event_amount(
                state,
                you,
                context,
                condition,
                source_version,
                event,
                event_amount,
            )
        }),
        Condition::Not(condition) => !condition_with_event_amount(
            state,
            you,
            context,
            *condition,
            source_version,
            event,
            event_amount,
        ),
        Condition::TriggerOnly(condition) => condition_with_event_amount(
            state,
            you,
            context,
            *condition,
            source_version,
            event,
            event_amount,
        ),
        Condition::SoulbondSelfCanPair => {
            state.is_unpaired_creature_controlled_by(context.source, source_version, you)
                && state.battlefield_seen().any(|id| {
                    id != context.source && state.is_unpaired_creature_controlled_by(id, None, you)
                })
        }
        Condition::SoulbondEventCanPair => event.is_some_and(|ev| {
            state.can_soulbond_pair(
                you,
                context.source,
                source_version,
                ev.object,
                Some(ev.version),
            )
        }),
        Condition::EvolveFromEvent => evolve_holds(state, context.source, source_version, event),
        Condition::TributeWasNotPaid => tribute_was_not_paid(state, context.source, source_version),
        Condition::OptionalCostPaid => state
            .damage_source(context.source, source_version)
            .is_some_and(object_optional_cost_paid),
        Condition::ControlledSubtypeOnCast(sub) => state
            .damage_source(context.source, source_version)
            .is_some_and(|object| {
                object
                    .riders
                    .contains(&crate::object::Rider::ControlledSubtypeOnCast(sub))
            }),
        Condition::CountersPlacedOnSourceThisTurn { kind, by_you } => {
            counters_placed_on_source_this_turn(
                state,
                you,
                context.source,
                source_version,
                kind,
                by_you,
            )
        }
        Condition::AmountCompare { left, op, right } => op.holds(
            super::amounts::amount_with_event_amount(
                left,
                state,
                you,
                context,
                None,
                source_version,
                event_amount,
            ),
            super::amounts::amount_with_event_amount(
                right,
                state,
                you,
                context,
                None,
                source_version,
                event_amount,
            ),
        ),
        Condition::AmountAtLeast(amount, minimum) => {
            super::amounts::amount_with_event_amount(
                &amount,
                state,
                you,
                context,
                None,
                source_version,
                event_amount,
            ) >= minimum
        }
        Condition::AmountIsEven(amount) => super::amounts::amount_with_event_amount(
            &amount,
            state,
            you,
            context,
            None,
            source_version,
            event_amount,
        )
        .is_multiple_of(2),
        _ => condition_holds_with_context(state, you, context, condition),
    }
}

fn event_object_matches(
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
    filter: &baylee_cards_dsl::Filter,
    event: Option<baylee_core::ids::DamageSourceRef>,
) -> bool {
    let Some(event) = event else { return false };
    let Some(obj) = state
        .source_object(event)
        .or_else(|| state.object_or_departed(event.object))
    else {
        return false;
    };
    if let Some(was) = state.last_known_characteristics(event.object) {
        let mut as_it_was = obj.clone();
        as_it_was.counters = state
            .ltb_counters
            .iter()
            .find(|(id, _)| *id == event.object)
            .map_or_default(|(_, counters)| counters.clone());
        super::matches_projected_with_context(filter, state, &as_it_was, was, you, context)
    } else {
        matches_with_context(filter, state, obj, you, context)
    }
}

fn tribute_was_not_paid(
    state: &GameState,
    source: baylee_core::ids::ObjectId,
    source_version: Option<u32>,
) -> bool {
    let paid = if let Some(version) = source_version {
        state
            .source_object(baylee_core::ids::DamageSourceRef {
                object: source,
                version,
            })
            .is_some_and(|obj| obj.riders.contains(&crate::object::Rider::TributePaid))
    } else {
        state
            .object_or_departed(source)
            .is_some_and(|obj| obj.riders.contains(&crate::object::Rider::TributePaid))
    };
    !paid
}

fn battlefield_or_lki_chars(
    state: &GameState,
    id: ObjectId,
    version: Option<u32>,
) -> Option<&crate::object::Characteristics> {
    if let Some(obj) = state.object(id).filter(|o| {
        o.zone == Zone::Battlefield
            && !o.status.contains(crate::object::Status::PHASED_OUT)
            && version.is_none_or(|v| o.version == v)
    }) {
        return Some(obj.characteristics());
    }
    if let Some(v) = version {
        if let Some(obj) = state
            .source_object(baylee_core::ids::DamageSourceRef {
                object: id,
                version: v,
            })
            .filter(|o| o.zone == Zone::Battlefield)
        {
            return Some(obj.characteristics());
        }
        if state
            .ltb_versions
            .iter()
            .any(|&(lid, lver)| lid == id && lver == v)
        {
            return state.last_known_characteristics(id);
        }
    }
    None
}

/// Evolve's intervening `if` (CR 702.100a, CR 702.100c, CR 603.4, CR 608.2h):
/// both objects are creatures (on the battlefield or in battlefield departure
/// LKI) and the entering creature has strictly greater power or toughness.
pub(crate) fn evolve_holds(
    state: &GameState,
    source: ObjectId,
    source_version: Option<u32>,
    event: Option<baylee_core::ids::DamageSourceRef>,
) -> bool {
    let Some(event) = event else { return false };
    if event.object == source && source_version.is_none_or(|v| v == event.version) {
        return false;
    }
    let Some(src_chars) = battlefield_or_lki_chars(state, source, source_version) else {
        return false;
    };
    let Some(ent_chars) = battlefield_or_lki_chars(state, event.object, Some(event.version)) else {
        return false;
    };
    if !src_chars
        .types
        .contains(baylee_core::types::TypeSet::CREATURE)
        || !ent_chars
            .types
            .contains(baylee_core::types::TypeSet::CREATURE)
    {
        return false;
    }
    let (Some(src_p), Some(src_t), Some(ent_p), Some(ent_t)) = (
        src_chars.power,
        src_chars.toughness,
        ent_chars.power,
        ent_chars.toughness,
    ) else {
        return false;
    };
    ent_p > src_p || ent_t > src_t
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

pub(crate) fn object_was_foretold(
    state: &GameState,
    source: ObjectId,
    version: Option<u32>,
) -> bool {
    let Some(obj) = state
        .damage_source(source, version)
        .or_else(|| state.object_or_departed(source))
    else {
        return false;
    };
    if obj
        .riders
        .iter()
        .any(|r| matches!(r, crate::object::Rider::Foretold { .. }))
    {
        return true;
    }
    if obj.cast_history.caster().is_some()
        && let Some(prev_v) = obj.version.checked_sub(1)
        && let Some(prev) = state.source_object(baylee_core::ids::DamageSourceRef {
            object: source,
            version: prev_v,
        })
        && prev.zone == Zone::Stack
    {
        return prev
            .riders
            .iter()
            .any(|r| matches!(r, crate::object::Rider::Foretold { .. }));
    }
    false
}

pub(crate) fn object_was_sneaked(
    state: &GameState,
    source: ObjectId,
    version: Option<u32>,
) -> bool {
    let Some(obj) = state
        .damage_source(source, version)
        .or_else(|| state.object_or_departed(source))
    else {
        return false;
    };
    if obj.riders.contains(&crate::object::Rider::Sneaked) {
        return true;
    }
    if obj.cast_history.caster().is_some()
        && let Some(prev_v) = obj.version.checked_sub(1)
        && let Some(prev) = state.source_object(baylee_core::ids::DamageSourceRef {
            object: source,
            version: prev_v,
        })
        && prev.zone == Zone::Stack
    {
        return prev.riders.contains(&crate::object::Rider::Sneaked);
    }
    false
}

pub(crate) fn object_spent_treasure_mana(
    state: &GameState,
    source: ObjectId,
    version: Option<u32>,
) -> bool {
    let Some(obj) = state
        .damage_source(source, version)
        .or_else(|| state.object_or_departed(source))
    else {
        return false;
    };
    if obj.treasure_mana_spent() {
        return true;
    }
    if let Some(paid) = obj.paid.as_ref() {
        return crate::mana_provenance::paid_spent_treasure_mana(state, paid);
    }
    false
}

fn object_was_bargained(_state: &GameState, object: &crate::object::GameObject) -> bool {
    object.riders.contains(&crate::object::Rider::Bargained)
}

fn object_optional_cost_paid(object: &crate::object::GameObject) -> bool {
    object
        .riders
        .contains(&crate::object::Rider::OptionalCostPaid)
}

fn object_gift_was_promised(object: &crate::object::GameObject) -> bool {
    object
        .riders
        .iter()
        .any(|r| matches!(r, crate::object::Rider::GiftPromisedTo(_)))
}

fn object_is_prototyped(_state: &GameState, object: &crate::object::GameObject) -> bool {
    object.prototyped
}

fn object_is_renowned(_state: &GameState, object: &crate::object::GameObject) -> bool {
    object.zone == Zone::Battlefield
        && !object.status.contains(crate::object::Status::PHASED_OUT)
        && object.riders.contains(&crate::object::Rider::Renowned)
}

fn object_is_monstrous(_state: &GameState, object: &crate::object::GameObject) -> bool {
    object.zone == Zone::Battlefield
        && !object.status.contains(crate::object::Status::PHASED_OUT)
        && object.riders.contains(&crate::object::Rider::Monstrous)
}

fn object_is_suspected(_state: &GameState, object: &crate::object::GameObject) -> bool {
    object.zone == Zone::Battlefield
        && !object.status.contains(crate::object::Status::PHASED_OUT)
        && object.riders.contains(&crate::object::Rider::Suspected)
}

pub(crate) fn counters_placed_on_source_this_turn(
    state: &GameState,
    you: PlayerId,
    source: ObjectId,
    source_version: Option<u32>,
    kind: baylee_cards_dsl::CounterKind,
    by_you: bool,
) -> bool {
    let Some(reference) = source_version
        .map(|version| baylee_core::ids::DamageSourceRef {
            object: source,
            version,
        })
        .or_else(|| state.source_identity(source))
    else {
        return false;
    };
    if by_you {
        state
            .per_turn
            .counter_placers
            .items
            .contains(&crate::state::CounterPlacerRecord {
                target: reference,
                kind,
                by: you,
            })
    } else {
        state
            .per_turn
            .counter_placements
            .contains(&(reference, kind))
    }
}

/// Whether `you` control at least `min_count` creatures on the battlefield
/// that all share at least one creature subtype.
fn you_control_sharing_creature_type(state: &GameState, you: PlayerId, min_count: u32) -> bool {
    let min = usize::try_from(min_count).unwrap_or(usize::MAX);
    if min == 0 {
        return true;
    }
    let mut types_per_creature: smallvec::SmallVec<[baylee_core::types::SubtypeSet; 8]> =
        smallvec::SmallVec::new();
    for id in state.battlefield_seen() {
        let Some(obj) = state.object(id) else {
            continue;
        };
        if obj.controller == you
            && obj
                .characteristics()
                .types
                .contains(baylee_core::types::TypeSet::CREATURE)
        {
            types_per_creature.push(obj.characteristics().subtypes);
        }
    }
    if types_per_creature.len() < min {
        return false;
    }
    if min == 1 {
        return true;
    }
    let mut candidate_types = baylee_core::types::SubtypeSet::EMPTY;
    for &subtypes in &types_per_creature[..=types_per_creature.len() - min] {
        candidate_types.union_with(subtypes);
    }
    let all_creature_types = baylee_core::generated::subtypes::ALL_CREATURE_TYPES;
    candidate_types.iter().any(|t| {
        all_creature_types.contains(t)
            && types_per_creature.iter().filter(|s| s.contains(t)).count() >= min
    })
}
