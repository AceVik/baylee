//! Counts used by power and toughness layer effects.

use super::{Characteristics, GameObject, GameState, ObjectId, PlayerId, eval};

/// Whether a count's filter picks its objects by what they are attached to
/// (`Filter::AttachedToSource`, alone or in an `And`), which no control
/// clause scopes.
pub(super) fn counts_by_attachment(filter: &baylee_cards_dsl::Filter) -> bool {
    use baylee_cards_dsl::Filter;
    match filter {
        Filter::AttachedToSource => true,
        Filter::And(parts) => parts.iter().any(counts_by_attachment),
        _ => false,
    }
}

/// Whether a count's filter binds `Filter::This` / `Filter::Another` /
/// `Filter::AttachedToSource` to each affected permanent rather than the static
/// ability's own source (`Coat of Arms`, `Alpha Status`, Aura counts).
fn rebinds_source_to_affected(filter: &baylee_cards_dsl::Filter) -> bool {
    use baylee_cards_dsl::Filter;
    match filter {
        Filter::AttachedToSource => true,
        Filter::SharesCreatureTypeWith(inner) => matches!(inner, Filter::This),
        Filter::And(parts) | Filter::Or(parts) => parts.iter().any(rebinds_source_to_affected),
        Filter::Not(inner) => rebinds_source_to_affected(inner),
        _ => false,
    }
}

pub(super) fn count_controlled(
    state: &GameState,
    obj: &GameObject,
    c: &Characteristics,
    controller: Option<PlayerId>,
    you: PlayerId,
    filter: &baylee_cards_dsl::Filter,
    context: crate::text_changes::RuleContext,
) -> usize {
    let context = if rebinds_source_to_affected(filter) {
        crate::text_changes::RuleContext {
            source: obj.id,
            ..context
        }
    } else {
        context
    };
    state
        .battlefield_seen()
        .filter_map(|id| state.object(id))
        .filter(|o| {
            controller.is_none_or(|p| o.controller == p)
                && if o.id == obj.id {
                    crate::eval::matches_projected_with_context(filter, state, o, c, you, context)
                } else {
                    crate::eval::matches_with_context(filter, state, o, you, context)
                }
        })
        .count()
}

/// The number a [`PtCount`](baylee_cards_dsl::PtCount) names, for
/// `CharacteristicPT` (7a) and `SetPTToCount` (7b) alike; `you` is whose
/// permanents `YouControl` counts. Either count reads other objects, so
/// both set `read_board`.
pub(super) fn pt_count(
    state: &GameState,
    obj: &GameObject,
    c: &Characteristics,
    you: PlayerId,
    count: baylee_cards_dsl::PtCount,
    context: crate::text_changes::RuleContext,
    read_board: &mut bool,
) -> i16 {
    *read_board = true;
    let n = match count {
        baylee_cards_dsl::PtCount::Fixed(n) => usize::from(n),
        baylee_cards_dsl::PtCount::DistinctColorsAmong(filter) => {
            distinct_colors_among(state, obj, c, you, filter, context)
        }
        baylee_cards_dsl::PtCount::ChosenNumber => obj.chosen_number().map_or(0, usize::from),
        baylee_cards_dsl::PtCount::Complement { from, count } => {
            return i16::try_from(from)
                .unwrap_or(i16::MAX)
                .saturating_sub(pt_count(state, obj, c, you, *count, context, read_board));
        }
        baylee_cards_dsl::PtCount::Plus { base, offset } => {
            return pt_count(state, obj, c, you, *base, context, read_board)
                .saturating_add(i16::try_from(offset).unwrap_or(i16::MAX));
        }
        baylee_cards_dsl::PtCount::YouControl(filter) => {
            count_controlled(state, obj, c, Some(you), you, filter, context)
        }
        baylee_cards_dsl::PtCount::OnBattlefield(filter) => {
            count_controlled(state, obj, c, None, you, filter, context)
        }
        baylee_cards_dsl::PtCount::ControllerOrDefenderControls(filter) => {
            let controller = if state.combat.is_attacking(obj.id) {
                defending_player_of(state, obj.id)
            } else {
                Some(obj.controller)
            };
            controller.map_or(0, |controller| {
                count_controlled(state, obj, c, Some(controller), you, filter, context)
            })
        }
        baylee_cards_dsl::PtCount::DefendingPlayerControls(filter) => {
            match defending_player_of(state, obj.id) {
                Some(defending) => {
                    count_controlled(state, obj, c, Some(defending), you, filter, context)
                }
                None => 0,
            }
        }
        baylee_cards_dsl::PtCount::InAllGraveyards(filter) => {
            cards_in_all_graveyards(state, obj, c, you, filter, context)
        }
        baylee_cards_dsl::PtCount::ChosenPlayerControls(filter) => {
            obj.chosen_opponent().map_or(0, |chosen| {
                count_controlled(state, obj, c, Some(chosen), you, filter, context)
            })
        }
        baylee_cards_dsl::PtCount::CardTypesInAllGraveyards => {
            card_types_in_graveyards(state, |_| true)
        }
        baylee_cards_dsl::PtCount::CardTypesInOpponentsGraveyards => {
            card_types_in_graveyards(state, |seat| state.is_opponent(seat, you))
        }
        baylee_cards_dsl::PtCount::ExiledWithThis => cards_exiled_with(state, obj),
        baylee_cards_dsl::PtCount::CountersOnThis(kind) => usize::from(
            state
                .object(context.source)
                .unwrap_or(obj)
                .counters
                .get(kind),
        ),
        baylee_cards_dsl::PtCount::EntrySacrifices => obj
            .riders
            .iter()
            .find_map(|r| match r {
                crate::object::Rider::EntrySacrifices(count) => {
                    Some(usize::try_from(*count).expect("arena count fits usize"))
                }
                _ => None,
            })
            .unwrap_or(0),
        baylee_cards_dsl::PtCount::CardsInHand(who) => {
            cards_in_hand_count(state, you, who, context)
        }
        baylee_cards_dsl::PtCount::InGraveyards { who, filter } => {
            eval::players_for_source(who, state, you, Some(context.source)).map_or(0, |seats| {
                cards_in_selected_graveyards(state, obj, c, you, &seats, filter, context)
            })
        }
        baylee_cards_dsl::PtCount::DistinctNamesAmong(filter) => {
            distinct_names_among(state, obj, c, you, filter, context)
        }
        baylee_cards_dsl::PtCount::Domain(who) => domain_count(state, obj, c, you, who, context),
        baylee_cards_dsl::PtCount::LifeTotal(who) => {
            return life_total_count(state, you, who, context);
        }
        baylee_cards_dsl::PtCount::Twice(base) => {
            return pt_count(state, obj, c, you, *base, context, read_board).saturating_mul(2);
        }
        baylee_cards_dsl::PtCount::GreatestManaValueAmong(filter) => {
            greatest_mana_value_among(state, obj, c, you, filter, context)
        }
        baylee_cards_dsl::PtCount::PartySize(who) => {
            max_party_size(state, obj, c, you, who, context)
        }
    };
    i16::try_from(n).unwrap_or(i16::MAX)
}

fn cards_in_hand_count(
    state: &GameState,
    you: PlayerId,
    who: baylee_cards_dsl::PlayerRel,
    context: crate::text_changes::RuleContext,
) -> usize {
    eval::players_for_source(who, state, you, Some(context.source)).map_or(0, |seats| {
        seats
            .into_iter()
            .map(|seat| {
                state
                    .zones
                    .list(crate::zone::ZoneLocation::Hand(seat))
                    .len()
            })
            .sum()
    })
}

fn max_party_size(
    state: &GameState,
    obj: &GameObject,
    c: &Characteristics,
    you: PlayerId,
    who: baylee_cards_dsl::PlayerRel,
    context: crate::text_changes::RuleContext,
) -> usize {
    eval::players_for_source(who, state, you, Some(context.source)).map_or(0, |seats| {
        seats
            .into_iter()
            .map(|seat| party_size_of(state, obj, c, seat))
            .max()
            .unwrap_or(0)
    })
}

fn distinct_colors_among(
    state: &GameState,
    obj: &GameObject,
    c: &Characteristics,
    you: PlayerId,
    filter: &baylee_cards_dsl::Filter,
    context: crate::text_changes::RuleContext,
) -> usize {
    let mut colors = baylee_core::color::ColorSet::EMPTY;
    for id in state.battlefield_seen() {
        if let Some(other) = state.object(id) {
            let chars = if id == obj.id {
                c
            } else {
                other.characteristics()
            };
            if eval::matches_projected_with_context(filter, state, other, chars, you, context) {
                colors = colors.union(chars.colors);
            }
        }
    }
    usize::from(colors.len())
}

fn distinct_names_among(
    state: &GameState,
    obj: &GameObject,
    c: &Characteristics,
    you: PlayerId,
    filter: &baylee_cards_dsl::Filter,
    context: crate::text_changes::RuleContext,
) -> usize {
    let mut names = std::collections::BTreeSet::new();
    for id in state.battlefield_seen() {
        if let Some(other) = state.object(id) {
            let chars = if id == obj.id {
                c
            } else {
                other.characteristics()
            };
            if eval::matches_projected_with_context(filter, state, other, chars, you, context) {
                let name = state.names.get(chars.name);
                if !name.is_empty() {
                    names.insert(name);
                }
            }
        }
    }
    names.len()
}

fn domain_count(
    state: &GameState,
    obj: &GameObject,
    c: &Characteristics,
    you: PlayerId,
    who: baylee_cards_dsl::PlayerRel,
    context: crate::text_changes::RuleContext,
) -> usize {
    eval::players_for_source(who, state, you, Some(context.source)).map_or(0, |seats| {
        let mut seen = baylee_core::types::SubtypeSet::EMPTY;
        for id in state.battlefield_seen() {
            if let Some(other) = state.object(id)
                && seats.contains(&other.controller)
            {
                let chars = if id == obj.id {
                    c
                } else {
                    other.characteristics()
                };
                if chars.types.contains(baylee_core::types::TypeSet::LAND) {
                    seen.union_with(chars.subtypes);
                }
            }
        }
        baylee_core::types::SubtypeSet::BASIC_LANDS
            .iter()
            .filter(|t| seen.contains(*t))
            .count()
    })
}

fn life_total_count(
    state: &GameState,
    you: PlayerId,
    who: baylee_cards_dsl::PlayerRel,
    context: crate::text_changes::RuleContext,
) -> i16 {
    let total: i64 =
        eval::players_for_source(who, state, you, Some(context.source)).map_or(0, |seats| {
            seats
                .into_iter()
                .filter_map(|p| state.players.get(usize::from(p.get())))
                .map(|pl| i64::from(pl.life))
                .sum()
        });
    total.clamp(i64::from(i16::MIN), i64::from(i16::MAX)) as i16
}

fn greatest_mana_value_among(
    state: &GameState,
    obj: &GameObject,
    c: &Characteristics,
    you: PlayerId,
    filter: &baylee_cards_dsl::Filter,
    context: crate::text_changes::RuleContext,
) -> usize {
    let mut greatest = 0_u32;
    for id in state.battlefield_seen() {
        if let Some(other) = state.object(id) {
            let chars = if id == obj.id {
                c
            } else {
                other.characteristics()
            };
            if eval::matches_projected_with_context(filter, state, other, chars, you, context) {
                greatest = greatest.max(chars.mana_value());
            }
        }
    }
    usize::try_from(greatest).unwrap_or(usize::MAX)
}

/// Evaluate a live count for a rule outside the P/T layers, using the same
/// filters and projected characteristics as the layer implementation.
pub(crate) fn live_pt_count(
    state: &GameState,
    obj: &GameObject,
    you: PlayerId,
    count: baylee_cards_dsl::PtCount,
) -> i16 {
    pt_count_for_resolution(
        state,
        obj,
        you,
        count,
        crate::eval::live_context(state, obj.id),
    )
}

pub(crate) fn pt_count_for_resolution(
    state: &GameState,
    obj: &GameObject,
    you: PlayerId,
    count: baylee_cards_dsl::PtCount,
    context: crate::text_changes::RuleContext,
) -> i16 {
    let mut read_board = false;
    pt_count(
        state,
        obj,
        obj.characteristics(),
        you,
        count,
        context,
        &mut read_board,
    )
}

/// This layer only reads a currently attacking creature's defending player.
/// Combat history keeps the original player when its attacked permanent leaves.
fn defending_player_of(state: &GameState, attacker: ObjectId) -> Option<PlayerId> {
    state
        .combat
        .is_attacking(attacker)
        .then(|| crate::combat::historical_defending_player(state, attacker))
        .flatten()
}

/// The number of card types (CR 205.2a) among cards in selected graveyards.
///
/// Read off each card's own characteristics: a card in a graveyard is
/// what it prints (CR 400.7 left every effect on it behind), and a
/// double-faced card is its front face there (CR 712.6a). Cards only
/// ([`GameObject::is_card`]): a token or a copy that died lies in the
/// graveyard until state-based actions remove it (CR 704.5d, 704.5e), and
/// the engine projects before it checks them.
fn card_types_in_graveyards(state: &GameState, include: impl Fn(PlayerId) -> bool) -> usize {
    use baylee_core::types::TypeSet;
    const CARD_TYPES: [TypeSet; 9] = [
        TypeSet::ARTIFACT,
        TypeSet::BATTLE,
        TypeSet::CREATURE,
        TypeSet::ENCHANTMENT,
        TypeSet::INSTANT,
        TypeSet::KINDRED,
        TypeSet::LAND,
        TypeSet::PLANESWALKER,
        TypeSet::SORCERY,
    ];
    let mut seen = TypeSet::EMPTY;
    for player in 0..state.players.len() {
        let seat = PlayerId::new(u8::try_from(player).unwrap_or(u8::MAX));
        if !include(seat) {
            continue;
        }
        for id in state.zones.list(crate::zone::ZoneLocation::Graveyard(seat)) {
            if let Some(o) = state.object(*id).filter(|o| o.is_card()) {
                seen = seen.union(o.characteristics().types);
            }
        }
    }
    CARD_TYPES.iter().filter(|t| seen.contains(**t)).count()
}

/// The cards in exile that were exiled with `host` as it is now (CR 406.6):
/// a [`crate::object::Rider::ExiledWith`] naming its id and its version, so
/// a host that left and came back counts none of them (CR 400.7). Cards
/// only, as the Hearse says ([`GameObject::is_card`]).
fn cards_exiled_with(state: &GameState, host: &GameObject) -> usize {
    let mark = crate::object::Rider::ExiledWith {
        host: host.id,
        version: crate::object::Rider::version_of(host),
    };
    (0..state.players.len())
        .map(|seat| PlayerId::new(u8::try_from(seat).unwrap_or(u8::MAX)))
        .flat_map(|seat| state.zones.list(crate::zone::ZoneLocation::Exile(seat)))
        .filter(|id| {
            state
                .object(**id)
                .is_some_and(|o| o.is_card() && o.riders.contains(&mark))
        })
        .count()
}

fn cards_in_all_graveyards(
    state: &GameState,
    obj: &GameObject,
    c: &Characteristics,
    you: PlayerId,
    filter: &baylee_cards_dsl::Filter,
    context: crate::text_changes::RuleContext,
) -> usize {
    let seats: Vec<PlayerId> = (0..state.players.len())
        .map(|seat| PlayerId::new(u8::try_from(seat).expect("player index fits")))
        .collect();
    cards_in_selected_graveyards(state, obj, c, you, &seats, filter, context)
}

fn cards_in_selected_graveyards(
    state: &GameState,
    obj: &GameObject,
    c: &Characteristics,
    you: PlayerId,
    seats: &[PlayerId],
    filter: &baylee_cards_dsl::Filter,
    context: crate::text_changes::RuleContext,
) -> usize {
    seats
        .iter()
        .flat_map(|&player| {
            state
                .zones
                .list(crate::zone::ZoneLocation::Graveyard(player))
        })
        .filter_map(|id| state.object(*id))
        .filter(|card| {
            card.is_card()
                && eval::matches_projected_with_context(
                    filter,
                    state,
                    card,
                    if card.id == obj.id {
                        c
                    } else {
                        card.characteristics()
                    },
                    you,
                    context,
                )
        })
        .count()
}

/// Maximum number of creatures in `player`'s party (up to one each of Cleric,
/// Rogue, Warrior, and Wizard, `0..=4`, CR 700.8a–c), computed via exact
/// subset matching over the 4 party roles.
fn party_size_of(
    state: &GameState,
    obj: &GameObject,
    c: &Characteristics,
    player: PlayerId,
) -> usize {
    use baylee_core::generated::subtypes::creature;
    const ROLES: [baylee_core::ids::SubtypeId; 4] = [
        creature::CLERIC,
        creature::ROGUE,
        creature::WARRIOR,
        creature::WIZARD,
    ];
    let mut reachable = 1_u16;
    for id in state.battlefield_seen() {
        let Some(other) = state.object(id) else {
            continue;
        };
        if other.controller != player {
            continue;
        }
        let chars = if id == obj.id {
            c
        } else {
            other.characteristics()
        };
        if !chars.types.contains(baylee_core::types::TypeSet::CREATURE) {
            continue;
        }
        let mut mask = 0_u8;
        for (idx, &role) in ROLES.iter().enumerate() {
            if chars.subtypes.contains(role) {
                mask |= 1 << idx;
            }
        }
        if mask == 0 {
            continue;
        }
        let mut next = reachable;
        for role in 0..4_usize {
            if (mask & (1 << role)) != 0 {
                for subset in 0..16_usize {
                    if (reachable & (1 << subset)) != 0 && (subset & (1 << role)) == 0 {
                        next |= 1 << (subset | (1 << role));
                    }
                }
            }
        }
        reachable = next;
        if (reachable & (1 << 15)) != 0 {
            return 4;
        }
    }
    (0..16_u16)
        .filter(|s| (reachable & (1 << s)) != 0)
        .map(|s| s.count_ones() as usize)
        .max()
        .unwrap_or(0)
}
