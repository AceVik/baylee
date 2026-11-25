//! Amounts: what a DSL `Amount` counts on the current state.

use super::{damage_this_turn, live_context, matches_with_context};
use crate::state::GameState;
use crate::text_changes::RuleContext;
use crate::zone::ZoneLocation;
use baylee_cards_dsl::{Amount, Filter, ZoneSel};
use baylee_core::ids::{ObjectId, PlayerId};

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

/// Units of snow mana spent on `object` (either via live `PaidRecord` on the
/// stack or via `Rider::SnowManaSpent` carried onto a permanent or trigger,
/// `CR 107.4h`, `CR 400.7d`, `CR 608.2h`).
#[must_use]
pub(crate) fn object_snow_mana_spent(object: &crate::object::GameObject) -> u32 {
    object
        .paid
        .as_ref()
        .map(|p| p.snow_mana_spent())
        .or_else(|| {
            object.riders.iter().find_map(|r| match r {
                crate::object::Rider::SnowManaSpent(n) => Some(*n),
                _ => None,
            })
        })
        .unwrap_or(0)
}

/// Units of snow mana spent on `source` (using `object_or_departed` for LKI).
#[must_use]
pub(crate) fn snow_mana_spent_on(state: &GameState, source: ObjectId) -> u32 {
    state
        .object_or_departed(source)
        .map_or(0, object_snow_mana_spent)
}

/// Total mana spent to cast `source` (`CR 601.2h`, `CR 400.7d`, `CR 608.2h`),
/// reading either `source`'s live `PaidRecord` on the stack or, for a permanent
/// that entered from a cast spell, the stack incarnation immediately preceding
/// its battlefield arrival. Returns `0` when `source` was not cast.
#[must_use]
pub(crate) fn mana_spent_on(state: &GameState, source: ObjectId) -> u32 {
    if let Some(paid) = state.object(source).and_then(|o| o.paid.as_ref()) {
        return paid.mana_spent;
    }
    let Some(bf_version) = super::last_battlefield_version(state, source) else {
        return 0;
    };
    let entered_from_cast = state
        .source_object(baylee_core::ids::DamageSourceRef {
            object: source,
            version: bf_version,
        })
        .is_some_and(|perm| perm.cast_history.caster().is_some());
    if !entered_from_cast || bf_version == 0 {
        return 0;
    }
    state
        .source_object(baylee_core::ids::DamageSourceRef {
            object: source,
            version: bf_version - 1,
        })
        .filter(|prev| prev.zone == crate::zone::Zone::Stack)
        .and_then(|prev| prev.paid.as_ref())
        .map_or(0, |p| p.mana_spent)
}

/// Number of permanents sacrificed as `source` entered the battlefield
/// (`EnterModifier::Devour` / `SacrificeForPowerToughness`, `CR 702.82b`).
#[must_use]
pub(crate) fn entry_sacrifices_on(state: &GameState, source: ObjectId) -> u32 {
    state.object_or_departed(source).map_or(0, |object| {
        object
            .riders
            .iter()
            .find_map(|r| match r {
                crate::object::Rider::EntrySacrifices(n) => Some(*n),
                _ => None,
            })
            .unwrap_or(0)
    })
}

/// Power of the permanent beheld or card revealed from hand to pay `source`'s
/// cost (`Amount::BeheldOrRevealedPower`, `CR 608.2h`, *Dragon's Fire*): if a
/// battlefield permanent was beheld and is still that exact incarnation on the
/// battlefield, reads its current projected power; if it left the battlefield,
/// reads its battlefield LKI power; otherwise reads the power snapshot recorded
/// when the cost was paid. Negative power clamps to `0` (`CR 107.1b`).
#[must_use]
pub fn beheld_or_revealed_power(
    state: &GameState,
    source: ObjectId,
    source_version: Option<u32>,
) -> u32 {
    let Some((bf_ref, saved)) = state
        .damage_source(source, source_version)
        .or_else(|| state.object_or_departed(source))
        .and_then(|obj| {
            obj.riders.iter().find_map(|r| match *r {
                crate::object::Rider::BeheldCostPower(src, power, _) => Some((src, power)),
                _ => None,
            })
        })
    else {
        return 0;
    };
    let power = if let Some(bf) = bf_ref {
        state
            .object(bf.object)
            .filter(|o| o.zone == crate::zone::Zone::Battlefield && o.version == bf.version)
            .map(|o| i32::from(o.characteristics().power.unwrap_or(0)))
            .or_else(|| {
                state
                    .ltb_versions
                    .iter()
                    .any(|&(lid, lver)| lid == bf.object && lver == bf.version)
                    .then(|| {
                        state
                            .ltb_powers
                            .iter()
                            .find(|(id, _)| *id == bf.object)
                            .map(|(_, p)| i32::from(*p))
                    })
                    .flatten()
            })
            .unwrap_or(saved)
    } else {
        saved
    };
    u32::try_from(power.max(0)).unwrap_or(0)
}

/// Mana value of the permanent beheld or card revealed from hand to pay
/// `source`'s cost (`Amount::BeheldOrRevealedManaValue`, `CR 608.2h`, `CR 202.3`).
#[must_use]
pub fn beheld_or_revealed_mana_value(
    state: &GameState,
    source: ObjectId,
    source_version: Option<u32>,
) -> u32 {
    let Some((bf_ref, saved)) = state
        .damage_source(source, source_version)
        .or_else(|| state.object_or_departed(source))
        .and_then(|obj| {
            obj.riders.iter().find_map(|r| match *r {
                crate::object::Rider::BeheldCostPower(src, _, mv) => Some((src, mv)),
                _ => None,
            })
        })
    else {
        return 0;
    };
    if let Some(bf) = bf_ref
        && let Some(o) = state
            .object(bf.object)
            .filter(|o| o.zone == crate::zone::Zone::Battlefield && o.version == bf.version)
    {
        return o.characteristics().mana_value();
    }
    saved
}

const MAX_AMOUNT_EVAL_DEPTH: u8 = 16;

thread_local! {
    static AMOUNT_EVAL_DEPTH: std::cell::Cell<u8> = const { std::cell::Cell::new(0) };
}

struct AmountEvalDepthGuard;

impl AmountEvalDepthGuard {
    fn enter() -> Option<Self> {
        AMOUNT_EVAL_DEPTH.with(|depth| {
            let current = depth.get();
            if current >= MAX_AMOUNT_EVAL_DEPTH {
                return None;
            }
            depth.set(current + 1);
            Some(Self)
        })
    }
}

impl Drop for AmountEvalDepthGuard {
    fn drop(&mut self) {
        AMOUNT_EVAL_DEPTH.with(|depth| {
            depth.set(depth.get().saturating_sub(1));
        });
    }
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
pub fn amount_with_context(
    amount: &Amount,
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
    x: Option<u32>,
) -> u32 {
    amount_with_version(amount, state, you, context, x, None)
}

/// Evaluate an amount using captured semantic text and an optional exact source
/// incarnation (CR 400.7 / CR 608.2h for intervening-`if` resolution checks
/// after the source has departed).
#[must_use]
#[allow(clippy::too_many_lines)] // One arm per `Amount` variant; splitting it hides the match.
pub(crate) fn amount_with_version(
    amount: &Amount,
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
    x: Option<u32>,
    source_version: Option<u32>,
) -> u32 {
    let Some(_guard) = AmountEvalDepthGuard::enter() else {
        return 0;
    };
    let this = context.source;
    let eval_nested =
        |inner: &Amount| amount_with_version(inner, state, you, context, x, source_version);
    match amount {
        Amount::Fixed(n) | Amount::NegXFixed(n) => *n,
        Amount::CreaturesEnteredUnderYourControlThisTurn => state.per_turn.creatures_entered
            .get(usize::from(you.get())).copied().unwrap_or(0),
        Amount::X | Amount::NegX => x.unwrap_or(0),
        Amount::TimesKicked => state
            .object(this)
            .map_or(0, |object| u32::from(object.times_kicked)),
        Amount::XIfSourceWasForetold { otherwise } => {
            if state.object(this).is_some_and(|object| {
                object
                    .riders
                    .iter()
                    .any(|rider| matches!(rider, crate::object::Rider::Foretold { .. }))
            }) {
                x.unwrap_or(0)
            } else {
                *otherwise
            }
        }
        // The magnitude, like every other arm here: the sign is
        // `Amount::is_negative`'s question and no caller of this reads one.
        Amount::Negated(inner) => eval_nested(inner),
        Amount::Plus { base, offset } => eval_nested(base).saturating_add(*offset),
        Amount::SaturatingSub { base, subtract } => eval_nested(base).saturating_sub(*subtract),
        Amount::ConstantMinus { constant, subtract } => constant.saturating_sub(eval_nested(subtract)),
        Amount::GreatestCommanderManaValue => greatest_commander_mana_value(state, you),
        Amount::HandSize(who) => super::players_for_source(*who, state, you, Some(context.source))
            .and_then(|p| p.first().copied())
            .map_or(0, |p| {
                u32::try_from(state.zones.list(ZoneLocation::Hand(p)).len()).unwrap_or(u32::MAX)
            }),
        Amount::LifeTotal(who) => super::players_for_source(*who, state, you, Some(context.source))
            .and_then(|p| p.first().copied())
            .map_or(0, |p| u32::try_from(state.players[usize::from(p.get())].life).unwrap_or(0)),
        Amount::StartingLifeTotal(who) => {
            super::players_for_source(*who, state, you, Some(context.source))
                .and_then(|p| p.first().copied())
                .map_or(0, |p| {
                    u32::try_from(state.players[usize::from(p.get())].starting_life).unwrap_or(0)
                })
        }
        Amount::LifeGainedThisTurn(who) => {
            super::players_for_source(*who, state, you, Some(context.source))
                .and_then(|p| p.first().copied())
                .map_or(0, |p| {
                    state
                        .per_turn
                        .life_gained_amount
                        .get(usize::from(p.get()))
                        .copied()
                        .unwrap_or(0)
                })
        }
        Amount::ControlledCount { who, filter } => {
            super::players_for_source(*who, state, you, Some(context.source))
                .and_then(|p| p.first().copied())
                .map_or(0, |controller| {
                    count_controlled_by(state, controller, you, filter, context)
                })
        }
        Amount::LibrarySize(who) => {
            super::players_for_source(*who, state, you, Some(context.source))
                .and_then(|p| p.first().copied())
                .map_or(0, |p| {
                    u32::try_from(state.zones.list(ZoneLocation::Library(p)).len())
                        .unwrap_or(u32::MAX)
                })
        }
        Amount::Half { base, round_up } => {
            let n = eval_nested(base);
            if *round_up { n.div_ceil(2) } else { n / 2 }
        }
        Amount::Divided { base, divisor, round_up } => {
            let n = eval_nested(base);
            if *round_up { n.div_ceil(divisor.get()) } else { n / divisor.get() }
        }
        Amount::Times { base, factor } => eval_nested(base).saturating_mul(*factor),
        Amount::Sum { left, right } => eval_nested(left).saturating_add(eval_nested(right)),
        Amount::Difference { left, right } => eval_nested(left).saturating_sub(eval_nested(right)),
        Amount::Players(who) => super::players_for_source(*who, state, you, Some(context.source))
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
        Amount::SourcePower => super::source_power(state, this).map_or(0, |p| p.max(0) as u32),
        Amount::CountersOnYou(kind) => state.players[usize::from(you.get())]
            .counters()
            .into_iter()
            .find_map(|(k, n)| (k == *kind).then_some(u32::from(n)))
            .unwrap_or(0),
        Amount::CountersOnSource(kind) => source_object_for_version(state, this, source_version)
            .map_or(0, |o| u32::from(o.counters.get(*kind))),
        Amount::DistinctCounterKindsOnSource => {
            source_object_for_version(state, this, source_version).map_or(0, |o| {
                u32::try_from(o.counters.iter().filter(|&(_, n)| n > 0).count()).unwrap_or(u32::MAX)
            })
        }
        Amount::HighestOpponentHandSize => highest_opponent_hand_size(state, you),
        Amount::EnteredThisTurn {
            filter,
            from,
            to,
            owner_or_ctrl,
            exclude_source,
        } => super::players_for_source(*owner_or_ctrl, state, you, Some(context.source))
            .map_or(0, |seats| {
                entered_this_turn_for_seats(
                    state,
                    you,
                    context,
                    filter,
                    *from,
                    *to,
                    &seats,
                    *exclude_source,
                    source_version,
                )
            }),
        Amount::SpellsCastMatchingThisTurn { filter, caster } => {
            super::players_for_source(*caster, state, you, Some(context.source))
                .map_or(0, |seats| {
                    spells_cast_matching_for_seats(state, you, context, filter, &seats)
                })
        }
        Amount::DistinctCardTypesCastThisTurn(who) => {
            super::players_for_source(*who, state, you, Some(context.source))
                .map_or(0, |seats| distinct_card_types_cast_for_seats(state, &seats))
        }
        Amount::CreaturesAttackedThisTurn { filter, controller } => {
            super::players_for_source(*controller, state, you, Some(context.source))
                .map_or(0, |seats| {
                    creatures_attacked_for_seats(state, you, context, filter, &seats)
                })
        }
        Amount::DistinctManaValuesInZone { zone, filter } => {
            distinct_mana_values_in_zone(filter, *zone, state, you, context)
        }
        // Resolved in resolve.rs, which has the stack object these read.
        Amount::DevotionToManaColor
        | Amount::TargetPower
        | Amount::TargetToughness
        | Amount::TargetCmc
        | Amount::EventLastToughness
        | Amount::EventPower
        | Amount::EventManaValue
        | Amount::EventAmount
        | Amount::LastDieRoll
        | Amount::CoinsWonThisResolution
        | Amount::TargetsPutIntoGraveyard
        | Amount::ThisWay(_)
        | Amount::ThisWayBy { .. }
        // A payment's sacrifices, read off the stack object
        // (`resolve::amount2`).
        | Amount::SacrificedTotalPower
        | Amount::SacrificedTotalToughness
        | Amount::SacrificedTotalManaValue
        | Amount::SacrificedDistinctColors => 0,
        Amount::ResolutionChosenNumber => state
            .resolution_chosen_number
            .filter(|(src, _)| *src == this)
            .map_or(0, |(_, n)| n),
        Amount::RememberedCount
        | Amount::RememberedManaValue
        | Amount::RememberedPower
        | Amount::RememberedToughness => {
            let rem = state
                .resolution_remembered
                .as_ref()
                .filter(|(src, _)| *src == this)
                .map(|(_, r)| r.as_ref());
            crate::resolve::remembered_amount(amount, rem)
        }
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
        Amount::ManaSpentToCast => mana_spent_on(state, this),
        Amount::ManaTypeSpentOnX(color) => state
            .object(this)
            .and_then(|o| o.paid.as_ref())
            .map_or(0, |p| p.mana_types_spent_on_x[color.index()]),
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
        Amount::OpponentsYouAttackedThisCombat => state.combat.opponents_attacked_by(state, you),
        Amount::OpponentsYouAreAttacking => state.combat.opponents_being_attacked_by(state, you),
        Amount::DamageDealtToOpponentsThisTurn => damage_dealt_to_opponents_this_turn(state, you),
        Amount::SnowManaSpentToCast => snow_mana_spent_on(state, this),
        Amount::EntrySacrifices => entry_sacrifices_on(state, this),
        Amount::SpellsWarpedThisTurn(who) => {
            super::players_for_source(*who, state, you, Some(context.source)).map_or(0, |seats| {
                u32::try_from(
                    state
                        .per_turn
                        .spells_warped
                        .iter()
                        .filter(|p| seats.contains(p))
                        .count(),
                )
                .unwrap_or(u32::MAX)
            })
        }
        Amount::ManaExpendedOnSpellsThisTurn(who) => {
            super::players_for_source(*who, state, you, Some(context.source)).map_or(0, |seats| {
                seats
                    .iter()
                    .fold(0u32, |acc, &p| acc.saturating_add(state.per_turn.mana_expended_by(p)))
            })
        }
        Amount::IfCondition {
            condition,
            then,
            otherwise,
        } => {
            let holds = match source_version {
                Some(_) => {
                    super::condition_with_event(state, you, context, **condition, source_version, None)
                }
                None => super::condition_holds_with_context(state, you, context, **condition),
            };
            let branch = if holds { then } else { otherwise };
            eval_nested(branch)
        }
        Amount::GreatestPowerAmong(filter) => greatest_among(state, you, filter, context, true),
        Amount::GreatestManaValueAmong(filter) => {
            greatest_among(state, you, filter, context, false)
        }
        Amount::Devotion { who, colors } => {
            super::players_for_source(*who, state, you, Some(context.source))
                .map_or(0, |seats| devotion_for_players(state, &seats, *colors))
        }
        Amount::PartySize(who) => super::players_for_source(*who, state, you, Some(context.source))
            .map_or(0, |seats| {
                seats
                    .into_iter()
                    .map(|seat| crate::layers::counts::live_party_size(state, seat))
                    .max()
                    .unwrap_or(0)
            }),
        Amount::SpellsCastThisTurn(who) => {
            super::players_for_source(*who, state, you, Some(context.source)).map_or(0, |seats| {
                seats
                    .into_iter()
                    .fold(0_u32, |acc, p| acc.saturating_add(state.per_turn.spells_cast_by(p)))
            })
        }
        Amount::CardsDrawnThisTurn(who) => {
            super::players_for_source(*who, state, you, Some(context.source)).map_or(0, |seats| {
                seats.into_iter().fold(0_u32, |acc, p| {
                    acc.saturating_add(
                        state
                            .per_turn
                            .draws
                            .get(usize::from(p.get()))
                            .copied()
                            .unwrap_or(0),
                    )
                })
            })
        }
        Amount::TotalPowerAmong(filter) => total_power_among(state, you, filter, context),
        Amount::BeheldOrRevealedPower => beheld_or_revealed_power(state, this, source_version),
        Amount::BeheldOrRevealedManaValue => {
            beheld_or_revealed_mana_value(state, this, source_version)
        }
    }
}

fn greatest_among(
    state: &GameState,
    you: PlayerId,
    filter: &Filter,
    context: RuleContext,
    read_power: bool,
) -> u32 {
    let metric = |obj: &crate::object::GameObject| -> Option<u32> {
        let chars = obj.characteristics();
        if read_power {
            chars.power.map(|p| u32::try_from(p.max(0)).unwrap_or(0))
        } else {
            Some(chars.mana_value())
        }
    };
    if crate::state::filter_reaches_other_zones(filter) {
        state
            .arena
            .iter()
            .filter(|(_, o)| {
                o.zone != crate::zone::Zone::OutsideGame
                    && !(o.zone == crate::zone::Zone::Battlefield
                        && o.status.contains(crate::object::Status::PHASED_OUT))
                    && matches_with_context(filter, state, o, you, context)
            })
            .filter_map(|(_, o)| metric(o))
            .max()
            .unwrap_or(0)
    } else {
        state
            .battlefield_seen()
            .filter_map(|id| state.object(id))
            .filter(|o| matches_with_context(filter, state, o, you, context))
            .filter_map(metric)
            .max()
            .unwrap_or(0)
    }
}

/// Devotion to `colors` across `seats` (`CR 700.5`): counts each mana symbol
/// whose colors intersect `colors` once per symbol among permanents controlled
/// by those players on the battlefield, excluding nonmodal DFC / disturb / meld
/// back faces which have no mana cost (`CR 712.8e`).
pub(crate) fn devotion_for_players(
    state: &GameState,
    seats: &[PlayerId],
    colors: baylee_core::color::ColorSet,
) -> u32 {
    if colors.is_empty() || seats.is_empty() {
        return 0;
    }
    let mut total = 0_u32;
    for id in state.battlefield_seen() {
        let Some(object) = state.object(id) else {
            continue;
        };
        if !seats.contains(&object.controller) {
            continue;
        }
        let chars = object.characteristics();
        if chars.front_mana_value.is_some() {
            continue;
        }
        for symbol in chars.mana_cost.symbols() {
            if symbol.colors().intersects(colors) {
                total = total.saturating_add(1);
            }
        }
    }
    total
}

fn total_power_among(
    state: &GameState,
    you: PlayerId,
    filter: &Filter,
    context: RuleContext,
) -> u32 {
    let mut sum: i64 = 0;
    for id in state.battlefield_seen() {
        let Some(obj) = state.object(id) else {
            continue;
        };
        if matches_with_context(filter, state, obj, you, context)
            && let Some(power) = obj.characteristics().power
        {
            sum = sum.saturating_add(i64::from(power));
        }
    }
    u32::try_from(sum.max(0)).unwrap_or(u32::MAX)
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
    // Earlier life loss still counts if that opponent has since left the game.
    state
        .players
        .iter()
        .filter(|p| state.is_opponent(p.id, you))
        .fold(0u32, |total, p| {
            total.saturating_add(state.per_turn.life_lost_amount[usize::from(p.id.get())])
        })
}

pub(crate) fn damage_dealt_to_opponents_this_turn(state: &GameState, you: PlayerId) -> u32 {
    state
        .players
        .iter()
        .filter(|p| !state.has_left(p.id) && state.is_opponent(p.id, you))
        .fold(0u32, |total, p| {
            total.saturating_add(state.per_turn.damage_dealt_to[usize::from(p.id.get())])
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
                && (obj.riders.iter().any(|r| match *r {
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
                }) || state
                    .damage_source(source, source_version)
                    .is_some_and(|src_obj| {
                        src_obj
                            .riders
                            .contains(&crate::object::Rider::SpellCostExile(
                                baylee_core::ids::DamageSourceRef {
                                    object: obj.id,
                                    version: obj.version,
                                },
                            ))
                    }))
                && matches_with_context(filter, state, obj, you, context)
            {
                out.push(id);
            }
        }
    }
    out
}

/// Face-up cards in exile linked to `source`'s current incarnation, in seat and
/// exile order. Face-down exiled cards (`Status::FACE_DOWN`, CR 406.3) are
/// never returned, because their characteristics are hidden from the public
/// table.
#[must_use]
pub fn public_linked_exile_cards(state: &GameState, source: ObjectId) -> Vec<ObjectId> {
    if !state
        .players
        .iter()
        .any(|p| !state.zones.list(ZoneLocation::Exile(p.id)).is_empty())
    {
        return Vec::new();
    }
    let Some(src_obj) = state.object(source) else {
        return Vec::new();
    };
    let source_ref = baylee_core::ids::DamageSourceRef {
        object: source,
        version: src_obj.version,
    };
    let acquisition = src_obj.rules_acquisition();
    let mut out = Vec::new();
    for player in &state.players {
        for &id in state.zones.list(ZoneLocation::Exile(player.id)) {
            if let Some(obj) = state.object(id)
                && obj.is_card()
                && !obj.status.contains(crate::object::Status::FACE_DOWN)
                && (obj.riders.iter().any(|r| match *r {
                    crate::object::Rider::Linked { host, .. } => {
                        host == source
                            && state
                                .source_memory
                                .linked
                                .get(&baylee_core::ids::DamageSourceRef {
                                    object: obj.id,
                                    version: obj.version,
                                })
                                .is_none_or(|&linked| linked == source_ref)
                    }
                    crate::object::Rider::ExiledWith { host, version } => {
                        host == source
                            && version == u16::try_from(source_ref.version).unwrap_or(u16::MAX)
                    }
                    crate::object::Rider::LinkedExile {
                        source: linked_source,
                        acquisition: linked_acquisition,
                    } => linked_source == source_ref && linked_acquisition == acquisition,
                    _ => false,
                }) || src_obj
                    .riders
                    .contains(&crate::object::Rider::SpellCostExile(
                        baylee_core::ids::DamageSourceRef {
                            object: obj.id,
                            version: obj.version,
                        },
                    )))
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
        ZoneSel::GraveyardOf(who) => {
            for player in
                super::players_for_source(who, state, you, Some(context.source)).unwrap_or_default()
            {
                for &id in state.zones.list(ZoneLocation::Graveyard(player)) {
                    visit(id);
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
        ZoneSel::ExileAll => {
            for player in &state.players {
                for &id in state.zones.list(ZoneLocation::Exile(player.id)) {
                    visit(id);
                }
            }
            return;
        }
        ZoneSel::HandOf(who) => {
            for player in
                super::players_for_source(who, state, you, Some(context.source)).unwrap_or_default()
            {
                for &id in state.zones.list(ZoneLocation::Hand(player)) {
                    visit(id);
                }
            }
            return;
        }
        ZoneSel::LibraryOf(who) => {
            for player in
                super::players_for_source(who, state, you, Some(context.source)).unwrap_or_default()
            {
                for &id in state.zones.list(ZoneLocation::Library(player)) {
                    visit(id);
                }
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

fn source_object_for_version(
    state: &GameState,
    this: ObjectId,
    source_version: Option<u32>,
) -> Option<&crate::object::GameObject> {
    match source_version {
        Some(v) => state.damage_source(this, Some(v)),
        None => state.object(this),
    }
}

/// Greatest number of cards in hand among active opponents of `you` (`0` if none).
pub(crate) fn highest_opponent_hand_size(state: &GameState, you: PlayerId) -> u32 {
    state
        .players
        .iter()
        .filter(|p| !state.has_left(p.id) && state.is_opponent(p.id, you))
        .map(|p| {
            u32::try_from(state.zones.list(ZoneLocation::Hand(p.id)).len()).unwrap_or(u32::MAX)
        })
        .max()
        .unwrap_or(0)
}

/// Count of frozen per-turn zone changes matching `filter`, `from`, `to`, and `seats`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn entered_this_turn_for_seats(
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
    filter: &Filter,
    from: Option<baylee_cards_dsl::ZoneRef>,
    to: baylee_cards_dsl::ZoneRef,
    seats: &[PlayerId],
    exclude_source: bool,
    source_version: Option<u32>,
) -> u32 {
    if seats.is_empty() {
        return 0;
    }
    let excluded = exclude_source
        .then(|| {
            source_version
                .map(|version| baylee_core::ids::DamageSourceRef {
                    object: context.source,
                    version,
                })
                .or_else(|| state.source_identity(context.source))
        })
        .flatten();
    let mut count = 0_u32;
    for record in &state.per_turn.zone_changes.items {
        if !record.to.matches_ref(to) {
            continue;
        }
        if let Some(expected_from) = from
            && !record.from.is_some_and(|rf| rf.matches_ref(expected_from))
        {
            continue;
        }
        if excluded.is_some_and(|ex| record.object == ex) {
            continue;
        }
        let Some(obj) = record.snapshot.as_deref().or_else(|| {
            state
                .object(record.object.object)
                .filter(|o| o.version == record.object.version)
        }) else {
            continue;
        };
        let seat = if record.to.is_shared() {
            if record.snapshot.is_none() {
                obj.controller
            } else {
                record.controller
            }
        } else {
            record.owner
        };
        if !seats.contains(&seat) {
            continue;
        }
        if matches_with_context(filter, state, obj, you, context) {
            count = count.saturating_add(1);
        }
    }
    count
}

/// Spells cast this turn by any player in `seats` matching `filter` as cast.
pub(crate) fn spells_cast_matching_for_seats(
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
    filter: &Filter,
    seats: &[PlayerId],
) -> u32 {
    if seats.is_empty() {
        return 0;
    }
    let mut count = 0_u32;
    for record in &state.per_turn.spells_cast_log.items {
        if seats.contains(&record.caster)
            && matches_with_context(filter, state, &record.snapshot, you, context)
        {
            count = count.saturating_add(1);
        }
    }
    count
}

/// Distinct CR 205.2a card types among spells cast this turn by players in `seats`.
pub(crate) fn distinct_card_types_cast_for_seats(state: &GameState, seats: &[PlayerId]) -> u32 {
    use baylee_core::types::TypeSet;
    if seats.is_empty() {
        return 0;
    }
    let mut types = TypeSet::EMPTY;
    for record in &state.per_turn.spells_cast_log.items {
        if seats.contains(&record.caster) {
            types = types.union(record.snapshot.characteristics().types);
        }
    }
    // CR 205.2a / CR 205.3g: Attraction is an artifact subtype, not a card type.
    types.difference(TypeSet::ATTRACTION).bits().count_ones()
}

/// Distinct creature incarnations declared as attackers this turn by players in
/// `seats` that match `filter` (live if still present, or LKI if departed).
pub(crate) fn creatures_attacked_for_seats(
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
    filter: &Filter,
    seats: &[PlayerId],
) -> u32 {
    if seats.is_empty() {
        return 0;
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut count = 0_u32;
    for &(id, version, attacking_player) in &state.per_turn.attacked {
        if !seats.contains(&attacking_player) || !seen.insert((id, version)) {
            continue;
        }
        let Some(obj) = state.damage_source(id, Some(version)) else {
            continue;
        };
        if matches_with_context(filter, state, obj, you, context) {
            count = count.saturating_add(1);
        }
    }
    count
}

/// Number of distinct mana values (CR 202.3) among matching objects in `zone`.
fn distinct_mana_values_in_zone(
    filter: &Filter,
    zone: ZoneSel,
    state: &GameState,
    you: PlayerId,
    context: RuleContext,
) -> u32 {
    let require_card = !matches!(zone, ZoneSel::Battlefield);
    let mut values = std::collections::BTreeSet::<u32>::new();
    visit_zone(zone, state, you, context, |id| {
        if let Some(object) = state.object(id)
            && (!require_card || object.is_card())
            && matches_with_context(filter, state, object, you, context)
        {
            values.insert(object.characteristics().mana_value());
        }
    });
    u32::try_from(values.len()).unwrap_or(u32::MAX)
}
