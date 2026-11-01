//! Searching a library, revealing until, and putting what was found.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;

/// One library search, as the three search effects describe it.
#[derive(Clone, Copy)]
pub(super) struct Search {
    /// Whose library.
    pub(super) library: PlayerId,
    /// Who chooses, and who gets what is found.
    pub(super) searcher: PlayerId,
    /// What may be found.
    pub(super) filter: &'static baylee_cards_dsl::Filter,
    /// A mana-value bound the resolution computed, on top of `filter`.
    pub(super) bound: Option<(baylee_cards_dsl::ManaValueCmp, u32)>,
    /// Where each find goes. A search that may find more cards than the
    /// list is long sends every card past its end where the last one goes.
    pub(super) finds: &'static [baylee_cards_dsl::effect::Find],
    /// How many cards may be found, when that is a number the resolution
    /// read rather than `finds.len()` (Nylea's Intervention's "up to X").
    pub(super) count: Option<u8>,
    /// "With different names": one card of each name is offered, so every
    /// answer has the property.
    pub(super) distinct_names: bool,
    /// See [`AwaitingOp::SearchLibrary`]'s `split`.
    pub(super) split: Option<u8>,
    /// Whether fewer than the most may be found.
    pub(super) optional: bool,
}

/// Whether a card's mana value meets a bound the resolution computed
/// ([`baylee_cards_dsl::ManaValueBound`]); no bound is met by every card.
pub(super) fn within(
    o: &crate::object::GameObject,
    bound: Option<(baylee_cards_dsl::ManaValueCmp, u32)>,
) -> bool {
    bound.is_none_or(|(cmp, n)| {
        let mv = o.characteristics().mana_value();
        match cmp {
            baylee_cards_dsl::ManaValueCmp::AtMost => mv <= n,
            baylee_cards_dsl::ManaValueCmp::Exactly => mv == n,
        }
    })
}

/// How many cards a search may find: a count the resolution read
/// (`SearchLibraryUpTo`), as many as match for "any number of" (a repeating
/// last find), and otherwise one per find.
///
/// Never more than match: a player told to find two cards finds as many as
/// possible when the zone doesn't contain enough (CR 701.23d), and a menu of
/// one that demands two has no answer.
pub(super) fn most_found(
    count: Option<u8>,
    finds: &[baylee_cards_dsl::effect::Find],
    matching: usize,
) -> usize {
    let most = if let Some(n) = count {
        usize::from(n)
    } else if finds.last().is_some_and(|f| f.repeats) {
        matching
    } else {
        finds.len()
    };
    most.min(matching)
}

/// Ashiok reaches only a search by an opposing spell/ability's own controller.
pub(super) fn allowed(
    state: &GameState,
    res: &Resolution,
    library: PlayerId,
    searcher: PlayerId,
) -> bool {
    if state.has_left(searcher) || state.has_left(library) {
        return false;
    }
    !(searcher == res.controller
        && library == res.controller
        && state.effects.iter().any(|fx| {
            matches!(fx.modifier, baylee_cards_dsl::Modifier::OpponentsCantSearch)
                && state.is_opponent(fx.controller, res.controller)
        }))
}

pub(super) fn basic_land(
    state: &mut GameState,
    res: &mut Resolution,
    player: PlayerId,
    count: u8,
) -> Option<Pending> {
    begin_search(
        state,
        res,
        Search {
            library: player,
            searcher: player,
            filter: &baylee_cards_dsl::Filter::BASIC_LAND,
            bound: None,
            finds: &[baylee_cards_dsl::Find::BATTLEFIELD_TAPPED],
            count: Some(count),
            distinct_names: false,
            split: None,
            optional: true,
        },
    )
}

fn search_matches(
    state: &GameState,
    res: &Resolution,
    searcher: PlayerId,
    filter: &baylee_cards_dsl::Filter,
    bound: Option<(baylee_cards_dsl::ManaValueCmp, u32)>,
    id: ObjectId,
) -> bool {
    state.object(id).is_some_and(|o| {
        let ctx = res.rule_context();
        let matched = res.x.map_or_else(
            || eval::matches_with_context(filter, state, o, searcher, ctx),
            |x| eval::matches_for_x(filter, state, o, searcher, ctx, Some(x)),
        );
        matched && within(o, bound)
    })
}

fn inspected_cards(state: &GameState, library: PlayerId, searcher: PlayerId) -> Vec<ObjectId> {
    let library_cards = state.zones.list(ZoneLocation::Library(library));
    let limit = state
        .effects
        .iter()
        .filter_map(|fx| {
            if let baylee_cards_dsl::Modifier::OpponentsSearchTop(n) = fx.modifier {
                state
                    .is_opponent(fx.controller, searcher)
                    .then_some(usize::from(n))
            } else {
                None
            }
        })
        .min()
        .unwrap_or(library_cards.len());
    library_cards.iter().rev().take(limit).copied().collect()
}

/// Opens a search and exposes exactly the permitted library segment.
/// Empty libraries and prohibited searches need no inspection acknowledgement.
pub(super) fn begin_search(
    state: &mut GameState,
    res: &mut Resolution,
    search: Search,
) -> Option<Pending> {
    res.subject.found.clear();
    let Search {
        library,
        searcher,
        filter,
        bound,
        finds,
        count,
        distinct_names,
        split,
        optional,
    } = search;
    if !allowed(state, res, library, searcher) {
        // CR 609.3: prohibiting the search does not prohibit the separate
        // shuffle instruction. Optional searches reject consent before here.
        if !state.has_left(library) && !state.has_left(searcher) {
            finish_shuffle(state, res, library);
        }
        return None;
    }
    state
        .journal
        .record(GameEvent::LibrarySearched { player: searcher });
    // Opposition Agent: an opponent of the searching player takes the
    // search over — they choose, and the find goes to exile playable by
    // them. That is controlling the searching player (CR 722.2), which a
    // player who has left the game does not (CR 800.4b). The card reaches a
    // player searching *their* library, so a search through somebody
    // else's is left alone.
    let takeover = (searcher == library)
        .then(|| {
            state
                .effects
                .iter()
                .filter(|fx| {
                    matches!(fx.modifier, baylee_cards_dsl::Modifier::SearchTakeover)
                        && state.is_opponent(fx.controller, searcher)
                        && !state.has_left(fx.controller)
                })
                // Multiple player-control effects overwrite in timestamp
                // order; the newest Agent gets the searching player's choices.
                .max_by_key(|fx| fx.timestamp)
                .map(|fx| fx.controller)
        })
        .flatten();
    let inspected = inspected_cards(state, library, searcher);
    let mut options: Vec<ObjectId> = inspected
        .iter()
        .rev()
        .copied()
        .filter(|&id| search_matches(state, res, searcher, filter, bound, id))
        .collect();
    if distinct_names {
        let mut seen: Vec<baylee_core::ids::NameRef> = Vec::new();
        options.retain(|id| {
            let Some(name) = state.object(*id).map(|o| o.characteristics().name) else {
                return false;
            };
            if seen.contains(&name) {
                return false;
            }
            seen.push(name);
            true
        });
    }
    if inspected.is_empty() {
        finish_shuffle(state, res, library);
        return None;
    }
    let want = u8::try_from(most_found(count, finds, options.len())).unwrap_or(u8::MAX);
    // CR 701.23b: a hidden-zone search with a stated quality may fail to find.
    let least = if optional || !matches!(filter, baylee_cards_dsl::Filter::Any) {
        0
    } else {
        want
    };
    let reveal = reveals(filter, finds);
    let player = if let Some(agent) = takeover {
        res.awaiting = Some(AwaitingOp::SearchTakeover {
            inspected,
            agent,
            finds,
            reveal,
            library,
            split,
        });
        agent
    } else {
        res.awaiting = Some(AwaitingOp::SearchLibrary {
            inspected,
            finds,
            reveal,
            library,
            receiver: searcher,
            split,
        });
        searcher
    };
    Some(Pending::ChooseCards {
        player,
        options,
        min: least,
        max: want,
        prompt: ChoicePrompt::SearchLibrary,
        total: None,
    })
}

/// Whether a search shows what it found.
///
/// The printed cards agree on a rule rather than deciding one by one: a
/// search narrower than "a card" reveals its find on the way to a hidden
/// zone, and a search that ends somewhere public does not — the card is
/// about to be visible anyway. Of the 1015 printed searches in the scripts
/// reference, none reveals where this says it should not, so the flag a
/// card file would carry could only ever be wrong.
pub(super) fn reveals(
    filter: &'static baylee_cards_dsl::Filter,
    finds: &[baylee_cards_dsl::effect::Find],
) -> bool {
    // A fork's other destination counts: Archdruid's Charm's creature goes
    // to the hand, and the card says "reveal it".
    fn hidden(find: &baylee_cards_dsl::effect::Find) -> bool {
        matches!(find.dest, SearchDest::Hand | SearchDest::TopOfLibrary)
            || find.instead_if.is_some_and(|(_, then)| hidden(then))
    }
    !matches!(filter, baylee_cards_dsl::Filter::Any) && finds.iter().any(hidden)
}

/// Where one found card goes: the find its position names — the last one
/// again past the end — and then, if that find forks on the card, the
/// branch the card matches. `None` only for a search with no finds.
pub(super) fn find_for(
    state: &GameState,
    res: &Resolution,
    receiver: PlayerId,
    finds: &'static [baylee_cards_dsl::effect::Find],
    at: usize,
    card: ObjectId,
) -> Option<baylee_cards_dsl::effect::Find> {
    // Past the end the last find again: for a repeating find ("any number
    // of"), and for a search whose count the resolution read
    // (`SearchLibraryUpTo`), which lists one find for every card. Any other
    // search offers no more cards than it has finds.
    let find = *finds.get(at).or_else(|| finds.last())?;
    Some(resolve_find(state, res, receiver, find, card))
}

/// Evaluate destination and entry-counter alternatives before the card moves.
pub(super) fn resolve_find(
    state: &GameState,
    res: &Resolution,
    receiver: PlayerId,
    mut find: baylee_cards_dsl::Find,
    card: ObjectId,
) -> baylee_cards_dsl::Find {
    while let Some((filter, then)) = find.instead_if {
        let matched = state.object(card).is_some_and(|o| {
            eval::matches_with_context(filter, state, o, receiver, res.rule_context())
        });
        if !matched {
            break;
        }
        find = *then;
    }
    find
}

/// Puts a card a player found in their library where the text sends it:
/// their hand, the top of their library, or the battlefield.
///
/// One door for the search (after its shuffle) and for a revealed top card
/// (Coiling Oracle), so a card put onto the battlefield from the library
/// becomes a permanent the same way whichever sentence put it there.
pub(super) fn put_found(state: &mut GameState, player: PlayerId, card: ObjectId, dest: SearchDest) {
    let to = match dest {
        SearchDest::Exile => ZoneLocation::Exile(state.object(card).map_or(player, |o| o.owner)),
        SearchDest::Graveyard => {
            ZoneLocation::Graveyard(state.object(card).map_or(player, |o| o.owner))
        }
        SearchDest::Hand => ZoneLocation::Hand(player),
        SearchDest::TopOfLibrary => ZoneLocation::Library(player),
        SearchDest::Battlefield => {
            if let Some(obj) = state.object_mut(card) {
                obj.kind = ObjectKind::Permanent;
                // "Put it onto the battlefield" names no controller, so it
                // is the player told to put it there (CR 110.2a), written
                // where it arrives rather than inherited from the last time
                // the card was on the battlefield.
                obj.set_controller(player);
            }
            ZoneLocation::Battlefield
        }
    };
    let _ = state.move_object(card, to, ZonePosition::Top, Cause::Effect);
}

/// "Reveal cards from the top of your library until you reveal a [filter]
/// card. Put that card [`found`] and the rest on the bottom of your library
/// in a random order." Every card turned over is shown to every player
/// (CR 701.20a) before anything moves; the match goes where `found` says,
/// and the rest are ordered by the table's generator and put on the bottom.
/// Returns the match, if the library held one.
pub(super) fn reveal_until(
    state: &mut GameState,
    you: PlayerId,
    source: ObjectId,
    filter: &'static baylee_cards_dsl::Filter,
    found: SearchDest,
) -> Option<ObjectId> {
    let (revealed, hit) = reveal_until_first(state, you, source, filter);
    if let Some(card) = hit {
        put_found(state, you, card, found);
    }
    let rest: Vec<ObjectId> = revealed.into_iter().filter(|c| Some(*c) != hit).collect();
    bottom_in_random_order(state, you, rest);
    hit
}

/// Reveal through the first match, recording the whole reveal before moving
/// anything. The top card is the last entry in the library's zone list.
fn reveal_until_first(
    state: &mut GameState,
    you: PlayerId,
    source: ObjectId,
    filter: &'static baylee_cards_dsl::Filter,
) -> (Vec<ObjectId>, Option<ObjectId>) {
    let mut revealed = Vec::new();
    let mut hit = None;
    for &card in state.zones.list(ZoneLocation::Library(you)).iter().rev() {
        revealed.push(card);
        if state
            .object(card)
            .is_some_and(|o| eval::matches(filter, state, o, you, source))
        {
            hit = Some(card);
            break;
        }
    }
    if !revealed.is_empty() {
        state.journal.record(GameEvent::Revealed {
            player: you,
            cards: revealed.clone(),
        });
    }
    (revealed, hit)
}

/// The general reveal-until operation with independently chosen destinations
/// for the first match and every other card revealed before it.
pub(super) fn reveal_until_matching(
    state: &mut GameState,
    you: PlayerId,
    source: ObjectId,
    filter: &'static baylee_cards_dsl::Filter,
    matched: SearchDest,
    otherwise: SearchDest,
) {
    let (revealed, hit) = reveal_until_first(state, you, source, filter);
    for card in revealed {
        put_found(
            state,
            you,
            card,
            if Some(card) == hit {
                matched
            } else {
                otherwise
            },
        );
    }
}

/// "…on the bottom of your library in a random order": the table's
/// generator orders the cards, and nobody is asked.
pub(super) fn bottom_in_random_order(state: &mut GameState, you: PlayerId, cards: Vec<ObjectId>) {
    // Cascade's uncast cards really do move there from exile.
    state.put_on_bottom_in_random_order(you, cards);
}

/// The library half of `SearchLibraryOrGraveyard`: the search
/// `Effect::SearchLibrary` makes for one card, shuffle and all. `find` stays
/// a reference: the search keeps a `&'static [Find]`, borrowed from the card.
#[allow(clippy::trivially_copy_pass_by_ref)]
pub(super) fn search_library_for_one(
    state: &mut GameState,
    res: &mut Resolution,
    filter: &'static baylee_cards_dsl::Filter,
    find: &'static baylee_cards_dsl::effect::Find,
) -> Option<Pending> {
    let you = res.controller;
    begin_search(
        state,
        res,
        Search {
            library: you,
            searcher: you,
            filter,
            bound: None,
            finds: core::slice::from_ref(find),
            count: None,
            distinct_names: false,
            split: None,
            optional: false,
        },
    )
}

/// A few spells explicitly postpone the shuffle past another instruction.
pub(super) fn finish_shuffle(state: &mut GameState, res: &Resolution, library: PlayerId) {
    if !matches!(
        res.effects.get(res.pc),
        Some(
            Effect::SearchLibraryBeforeShuffle { .. }
                | Effect::EachPlayerSearchBeforeShuffle { .. }
                | Effect::SearchWithManaDifference { .. }
        )
    ) {
        state.shuffle_library(library);
    }
}

/// Consent precedes inspection; declining never shuffles the library.
pub(super) fn resume_consent(
    state: &mut GameState,
    res: &mut Resolution,
    answer: bool,
) -> Option<Flow> {
    let Some(AwaitingOp::MaySearchBasicLand { player, count }) = res.awaiting else {
        return None;
    };
    res.awaiting = None;
    let since = state.journal.last_seq();
    if answer && let Some(pending) = basic_land(state, res, player, count) {
        return Some(next_choice(state, res, since, pending));
    }
    Some(finish_choice(state, res, since))
}
