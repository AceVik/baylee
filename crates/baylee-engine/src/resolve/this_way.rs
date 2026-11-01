//! "… this way" and "that many" (CR 608.2c): what the resolution in progress
//! has already done.
//!
//! Read off the journal, as [`super::reflexive`] reads its event: the
//! entries after this resolution's `StackObjectResolved` marker are this
//! resolution's own, because resolutions never interleave and nothing else
//! happens while one waits for an answer. A field on `Resolution` would
//! have to be carried through every literal of it and through every
//! suspension; the journal already is.

use baylee_cards_dsl::ThisWay;
use baylee_core::ids::{ObjectId, PlayerId};

use super::Resolution;
use crate::eval;
use crate::event::{Cause, DamageTarget, GameEvent, JournalEntry};
use crate::state::GameState;
use crate::zone::Zone;

/// The journal entries this resolution has written so far, or none when it
/// is not a resolution off the stack (a mana ability, CR 605.3b, never was
/// on it).
pub(super) fn entries<'a>(state: &'a GameState, res: &Resolution) -> &'a [JournalEntry] {
    // A mana ability's `on_stack` is its source permanent, which keeps the
    // `ObjectId` of the spell it resolved from: without this check it could
    // match that spell's marker and count everything since.
    if state
        .object(res.on_stack)
        .is_none_or(|o| o.zone != Zone::Stack)
    {
        return &[];
    }
    let all = state.journal.entries();
    let Some(start) = all
        .iter()
        .rposition(|e| matches!(e.event, GameEvent::StackObjectResolved { .. }))
    else {
        return &[];
    };
    match all[start].event {
        GameEvent::StackObjectResolved { object } if object == res.on_stack => {
            // One pass of a repeated process counts from where it began.
            let from = res
                .this_way_from
                .map_or(start + 1, |from| from.max(start + 1))
                .min(all.len());
            &all[from..]
        }
        _ => &[],
    }
}

/// [`baylee_cards_dsl::Amount::ThisWay`]: how much of `what` this resolution
/// has done, counting only `only`'s part when it names a player (an
/// instruction applied to each player in turn).
#[allow(clippy::too_many_lines)] // Keep each journal event/result pair in one flat match.
pub(super) fn count(
    state: &GameState,
    res: &Resolution,
    what: ThisWay,
    only: Option<PlayerId>,
) -> u32 {
    let mine = |player: PlayerId| only.is_none_or(|p| p == player);
    let mut total: u32 = 0;
    for entry in entries(state, res) {
        let n = match (&entry.event, what) {
            (
                GameEvent::DamageDealt {
                    source: Some(source),
                    target,
                    amount,
                    ..
                },
                ThisWay::DamageDealt | ThisWay::DamageDealtToYou,
            ) if *source == res.source => {
                let to_you = matches!(target, DamageTarget::Player(p) if *p == res.controller);
                let to_counted = match target {
                    DamageTarget::Player(p) => mine(*p),
                    DamageTarget::Object(_) => only.is_none(),
                };
                let wanted = match what {
                    ThisWay::DamageDealtToYou => to_you,
                    _ => to_counted,
                };
                if wanted { *amount } else { 0 }
            }
            (
                GameEvent::ZoneChanged {
                    object,
                    from: Zone::Battlefield,
                    to: Zone::Graveyard,
                    ..
                },
                ThisWay::Died(filter),
            )
            | (
                GameEvent::ZoneChanged {
                    object,
                    from: Zone::Battlefield,
                    to: Zone::Exile,
                    ..
                },
                ThisWay::ExiledFromBattlefield(filter),
            )
            | (
                GameEvent::ZoneChanged {
                    object,
                    from: Zone::Battlefield,
                    to: Zone::Hand,
                    ..
                },
                ThisWay::ReturnedToHand(filter),
            ) => u32::from(was(state, res, *object, filter, &mine)),
            (
                GameEvent::ZoneChanged {
                    object,
                    from: Zone::Battlefield,
                    cause: Cause::Destroy,
                    ..
                },
                ThisWay::Destroyed(filter),
            ) => u32::from(destroyed(state, res, entry, *object, filter, &mine)),
            (GameEvent::Discarded { player, .. }, ThisWay::Discarded) => u32::from(mine(*player)),
            (GameEvent::Discarded { object, player, .. }, ThisWay::DiscardedMatching(filter))
                if mine(*player) =>
            {
                u32::from(state.object(*object).is_some_and(|card| {
                    eval::matches_with_context(
                        filter,
                        state,
                        card,
                        res.controller,
                        res.rule_context(),
                    )
                }))
            }
            (
                GameEvent::ZoneChanged {
                    object,
                    from: Zone::Library,
                    to: Zone::Exile,
                    ..
                },
                ThisWay::ExiledFromLibrary(filter),
            ) => u32::from(state.object(*object).is_some_and(|card| {
                mine(card.owner)
                    && eval::matches_with_context(
                        filter,
                        state,
                        card,
                        res.controller,
                        res.rule_context(),
                    )
            })),
            (
                GameEvent::CounterChanged {
                    object,
                    kind: event_kind,
                    old,
                    new,
                    ..
                },
                ThisWay::CountersRemoved(kind),
            ) if *object == res.source && *event_kind == kind && new < old => {
                u32::from(old.saturating_sub(*new))
            }
            (GameEvent::ControllerChanged { .. }, ThisWay::ControlChanged) => 1,
            (
                GameEvent::ZoneChanged {
                    object,
                    from: Zone::Library,
                    to: Zone::Graveyard,
                    ..
                },
                ThisWay::Milled(filter),
            ) => u32::from(milled(state, res, *object, filter, &mine)),
            (
                GameEvent::ZoneChanged {
                    object,
                    from: Zone::Hand,
                    to: Zone::Library,
                    ..
                },
                ThisWay::PutIntoLibraryFromHand,
            ) => u32::from(state.object(*object).is_some_and(|o| mine(o.owner))),
            (GameEvent::Revealed { player, cards }, ThisWay::Revealed(filter)) if mine(*player) => {
                let ctx = res.rule_context();
                let you = res.controller;
                let matched = cards.iter().filter(|&&id| {
                    state
                        .object(id)
                        .is_some_and(|c| eval::matches_with_context(filter, state, c, you, ctx))
                });
                u32::try_from(matched.count()).unwrap_or(0)
            }
            (GameEvent::Revealed { player, cards }, ThisWay::RevealedManaValue)
                if mine(*player) =>
            {
                cards
                    .iter()
                    .filter_map(|&id| state.object(id).map(|c| c.characteristics().mana_value()))
                    .sum()
            }
            _ => 0,
        };
        total = total.saturating_add(n);
    }
    total
}

/// Whether the permanent that left as `object` matched `filter` as it last
/// existed on the battlefield (CR 608.2h), and belongs to the counted player.
fn was(
    state: &GameState,
    res: &Resolution,
    object: ObjectId,
    filter: &baylee_cards_dsl::Filter,
    mine: &impl Fn(PlayerId) -> bool,
) -> bool {
    let Some(card) = state.object(object) else {
        return false;
    };
    let chars = state
        .last_known_characteristics(object)
        .unwrap_or_else(|| card.characteristics());
    let controller = state.last_known_controller(object).unwrap_or(card.owner);
    mine(controller)
        && eval::matches_projected_with_context(
            filter,
            state,
            card,
            chars,
            res.controller,
            res.rule_context(),
        )
}

/// Destruction retains the departure controller even if the card moves again.
fn destroyed(
    state: &GameState,
    res: &Resolution,
    entry: &JournalEntry,
    object: ObjectId,
    filter: &baylee_cards_dsl::Filter,
    mine: &impl Fn(PlayerId) -> bool,
) -> bool {
    let Some(departure) = &entry.departure else {
        return was(state, res, object, filter, mine);
    };
    let Some(card) = state.damage_source(object, Some(departure.version)) else {
        return false;
    };
    mine(departure.controller)
        && eval::matches_projected_with_context(
            filter,
            state,
            card,
            card.characteristics(),
            res.controller,
            res.rule_context(),
        )
}

fn milled(
    state: &GameState,
    res: &Resolution,
    object: ObjectId,
    filter: &baylee_cards_dsl::Filter,
    mine: &impl Fn(PlayerId) -> bool,
) -> bool {
    state.object(object).is_some_and(|card| {
        mine(card.owner)
            && eval::matches_with_context(filter, state, card, res.controller, res.rule_context())
    })
}

/// The permanents the resolving source has dealt damage to so far in this
/// resolution (Spikefield Hazard's "a permanent dealt damage this way"), each
/// once, as the objects they are now.
pub(super) fn damaged_permanents(state: &GameState, res: &Resolution) -> Vec<ObjectId> {
    let mut out: Vec<ObjectId> = Vec::new();
    for entry in entries(state, res) {
        match entry.event {
            GameEvent::DamageDealt {
                source: Some(source),
                target: DamageTarget::Object(object),
                amount,
                ..
            } if source == res.source && amount > 0 && !out.contains(&object) => {
                out.push(object);
            }
            // CR 400.7: a later incarnation was not dealt that earlier damage.
            GameEvent::ZoneChanged { object, .. } => out.retain(|&id| id != object),
            _ => {}
        }
    }
    out
}

/// The tokens created on the battlefield so far in this resolution, each once,
/// as the objects they are now.
pub(super) fn created_tokens(state: &GameState, res: &Resolution) -> Vec<ObjectId> {
    let mut out: Vec<ObjectId> = Vec::new();
    for entry in entries(state, res) {
        if let GameEvent::ZoneChanged {
            object,
            from: Zone::OutsideGame,
            to: Zone::Battlefield,
            ..
        } = entry.event
            && state
                .object(object)
                .is_some_and(|o| !o.is_card() && o.zone == Zone::Battlefield)
            && !out.contains(&object)
        {
            out.push(object);
        }
    }
    out
}

/// The cards revealed from a hand so far in this resolution that are still in
/// their owner's hand, paired with the player who revealed them.
pub(super) fn revealed_hand_cards(
    state: &GameState,
    res: &Resolution,
) -> Vec<(ObjectId, PlayerId)> {
    let mut out: Vec<(ObjectId, PlayerId)> = Vec::new();
    for entry in entries(state, res) {
        if let GameEvent::Revealed { player, cards } = &entry.event {
            for &card in cards {
                if state
                    .object(card)
                    .is_some_and(|o| o.zone == Zone::Hand && o.owner == *player)
                    && !out.iter().any(|(c, _)| *c == card)
                {
                    out.push((card, *player));
                }
            }
        }
    }
    out
}
