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

/// Opens a library search: the question, or nothing when there is nothing
/// to find or nobody may search.
pub(super) fn begin_search(
    state: &mut GameState,
    res: &mut Resolution,
    search: Search,
) -> Option<Pending> {
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
    // Ashiok, Dream Render: "spells and abilities your opponents control
    // can't cause their controller to search their library". The one
    // search it reaches is the controller's own of their own library — a
    // Boseiju that makes *its victim* search, or a Bribery through someone
    // else's library, is not that sentence.
    let you = res.controller;
    if searcher == you
        && library == you
        && state.effects.iter().any(|fx| {
            matches!(fx.modifier, baylee_cards_dsl::Modifier::OpponentsCantSearch)
                && state.is_opponent(fx.controller, you)
        })
    {
        return None;
    }
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
    let mut options: Vec<ObjectId> = state
        .zones
        .list(ZoneLocation::Library(library))
        .iter()
        .filter(|id| {
            state.object(**id).is_some_and(|o| {
                eval::matches_with_context(filter, state, o, searcher, res.rule_context())
                    && within(o, bound)
            })
        })
        .copied()
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
    if options.is_empty() {
        // Hidden zone: failing to find is always legal (CR 701.23b).
        state.shuffle_library(library);
        return None;
    }
    // How many cards this search may produce, and how few it may settle
    // for: "up to two" is optional with two finds, "search for a basic land
    // card" is one find and mandatory.
    let want = u8::try_from(most_found(count, finds, options.len())).unwrap_or(u8::MAX);
    if want == 0 {
        // "Up to X" with X = 0: the library is searched and nothing can be
        // found, which is a search that ends in a shuffle and asks nobody.
        state.shuffle_library(library);
        return None;
    }
    let least = if optional { 0 } else { want };
    let reveal = reveals(filter, finds);
    let player = if let Some(agent) = takeover {
        res.awaiting = Some(AwaitingOp::SearchTakeover {
            agent,
            finds,
            reveal,
            library,
            split,
        });
        agent
    } else {
        res.awaiting = Some(AwaitingOp::SearchLibrary {
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
    let mut find = *finds.get(at).or_else(|| finds.last())?;
    while let Some((filter, then)) = find.instead_if {
        let matched = state.object(card).is_some_and(|o| {
            eval::matches_with_context(filter, state, o, receiver, res.rule_context())
        });
        if !matched {
            break;
        }
        find = *then;
    }
    Some(find)
}

/// Puts a card a player found in their library where the text sends it:
/// their hand, the top of their library, or the battlefield.
///
/// One door for the search (after its shuffle) and for a revealed top card
/// (Coiling Oracle), so a card put onto the battlefield from the library
/// becomes a permanent the same way whichever sentence put it there.
pub(super) fn put_found(state: &mut GameState, player: PlayerId, card: ObjectId, dest: SearchDest) {
    let to = match dest {
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
    let library: Vec<ObjectId> = state.zones.list(ZoneLocation::Library(you)).clone();
    let mut revealed = Vec::new();
    let mut hit = None;
    // The top card is the last in the list.
    for &card in library.iter().rev() {
        revealed.push(card);
        if state
            .object(card)
            .is_some_and(|o| eval::matches(filter, state, o, you, source))
        {
            hit = Some(card);
            break;
        }
    }
    if revealed.is_empty() {
        return None;
    }
    state.journal.record(GameEvent::Revealed {
        player: you,
        cards: revealed.clone(),
    });
    if let Some(card) = hit {
        put_found(state, you, card, found);
    }
    let rest: Vec<ObjectId> = revealed.into_iter().filter(|c| Some(*c) != hit).collect();
    bottom_in_random_order(state, you, rest);
    hit
}

/// "…on the bottom of your library in a random order": the table's
/// generator orders the cards, and nobody is asked.
pub(super) fn bottom_in_random_order(
    state: &mut GameState,
    you: PlayerId,
    mut cards: Vec<ObjectId>,
) {
    state.rng.shuffle(&mut cards);
    for card in cards {
        let _ = state.move_object(
            card,
            ZoneLocation::Library(you),
            ZonePosition::Bottom,
            Cause::Effect,
        );
    }
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
