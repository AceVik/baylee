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

pub(super) fn count_controlled(
    state: &GameState,
    obj: &GameObject,
    c: &Characteristics,
    controller: Option<PlayerId>,
    you: PlayerId,
    filter: &baylee_cards_dsl::Filter,
    context: crate::text_changes::RuleContext,
) -> usize {
    let context = if counts_by_attachment(filter) {
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
            let mut colors = baylee_core::color::ColorSet::EMPTY;
            for id in state.battlefield_seen() {
                if let Some(other) = state.object(id) {
                    let chars = if id == obj.id {
                        c
                    } else {
                        other.characteristics()
                    };
                    if eval::matches_projected_with_context(
                        filter, state, other, chars, you, context,
                    ) {
                        colors = colors.union(chars.colors);
                    }
                }
            }
            usize::from(colors.len())
        }
        baylee_cards_dsl::PtCount::ChosenNumber => obj
            .riders
            .iter()
            .find_map(|rider| match rider {
                crate::object::Rider::ChosenNumber {
                    acquisition,
                    number,
                } if *acquisition == obj.rules_acquisition() => Some(usize::from(*number)),
                _ => None,
            })
            .unwrap_or(0),
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
        baylee_cards_dsl::PtCount::CardTypesInAllGraveyards => card_types_in_all_graveyards(state),
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
    };
    i16::try_from(n).unwrap_or(i16::MAX)
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

/// The number of card types (CR 205.2a) among cards in all graveyards.
///
/// Read off each card's own characteristics: a card in a graveyard is
/// what it prints (CR 400.7 left every effect on it behind), and a
/// double-faced card is its front face there (CR 712.8a). Cards only
/// ([`GameObject::is_card`]): a token or a copy that died lies in the
/// graveyard until state-based actions remove it (CR 704.5d, 704.5e), and
/// the engine projects before it checks them.
fn card_types_in_all_graveyards(state: &GameState) -> usize {
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
    (0..state.players.len())
        .map(|seat| PlayerId::new(u8::try_from(seat).expect("player index fits")))
        .flat_map(|player| {
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
