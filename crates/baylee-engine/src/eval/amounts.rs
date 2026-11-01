//! Amounts: what a DSL `Amount` counts on the current state.

use super::{damage_this_turn, live_context, matches_with_context, players};
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
        Amount::LifeTotal(who) => players(*who, state, you)
            .and_then(|p| p.first().copied())
            .map_or(0, |p| u32::try_from(state.players[usize::from(p.get())].life).unwrap_or(0)),
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
        Amount::Divided { base, divisor, round_up } => {
            let n = self::amount_with_context(base, state, you, context, x);
            if *round_up { n.div_ceil(divisor.get()) } else { n / divisor.get() }
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
        ZoneSel::GraveyardOf(who) => {
            for player in super::players(who, state, you).unwrap_or_default() {
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
