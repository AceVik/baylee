//! What this seat could do with each card right now: the offers `rebuild_board` lights.

#[allow(clippy::wildcard_imports)] // the parent's own vocabulary
use super::*;

/// Which permanents have an ability that can be activated right now.
///
/// Straight off `LegalActions`, both halves of it: `mana_abilities` names a
/// source once however many mana abilities it has, `abilities` names a
/// `(source, index)` pair per ability. The table only needs "does this card
/// have anything to do", so both collapse to the same set of sources — the
/// menu that asks *which* one is built later, from the same list, by
/// `abilities::options`.
pub(super) fn activatable(duel: &Duel) -> std::collections::HashSet<ObjectId> {
    duel.interaction
        .as_ref()
        .and_then(Interaction::legal_actions)
        .map(|legal| {
            legal
                .mana_abilities
                .iter()
                .copied()
                .chain(legal.abilities.iter().map(|(source, _)| *source))
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn ability_reach(duel: &Duel) -> std::collections::HashSet<ObjectId> {
    let Some(view) = &duel.view else {
        return std::collections::HashSet::new();
    };
    let Some(legal) = duel
        .interaction
        .as_ref()
        .and_then(Interaction::legal_actions)
    else {
        return std::collections::HashSet::new();
    };
    legal
        .unpaid_abilities
        .iter()
        .filter_map(|&(source, index, _)| {
            abilities::mana_for(view, legal, source, index)
                .is_some()
                .then_some(source)
        })
        .collect()
}

/// Whether the engine offers `card` only because 2 life pays its Phyrexian
/// symbols (CR 107.4f): the floating pool cannot pay them in mana. A click
/// on it would charge the life without a word, so where lands could pay the
/// mana it stays [`reachable`]; the click taps them first and the engine
/// asks which way to pay (`manaplan` rule 2: life is the player's decision).
pub(super) fn phyrexian_life_only(
    card: &baylee_view::HandObject,
    pool: &baylee_view::ManaPoolView,
) -> bool {
    manasources::hand_cost(card).is_some_and(|cost| {
        cost.phyrexian_count() > 0 && baylee_client_core::manaplan::plan(&cost, pool, &[]).is_none()
    })
}

/// Which cards a tap or two would make castable: in hand, in the command
/// zone, and in the seat's own graveyard.
///
/// The engine answers "castable" against the mana already floating, which is
/// the correct rules answer and a hand that looks empty to a player with five
/// untapped lands. This is the other half of that question, and the board
/// model draws it differently from `playable` on purpose: one is what the
/// game says, the other is what this client is offering to do about it.
///
/// Two questions, both of which have to be answered yes. "Can the lands pay
/// for it" is [`baylee_client_core::manaplan`]; "could it be cast at all right
/// now" is [`baylee_client_core::timing`], and it was missing — so a sorcery
/// was lit and clickable on an opponent's turn and over a stack that had not
/// resolved. The offer is not free: taking it taps the lands *first*, and the
/// engine then refuses the cast, which spends the turn's mana on nothing.
pub(super) fn reachable(duel: &Duel) -> std::collections::HashSet<ObjectId> {
    let Some(view) = duel.view.as_ref() else {
        return std::collections::HashSet::new();
    };
    // A controlled resolving instruction exposes its exact legal actions;
    // ordinary timing/affordability must not invent additional plays.
    if baylee_client_core::decision::resource_player(view) != view.seat {
        return std::collections::HashSet::new();
    }
    let Some(legal) = duel
        .interaction
        .as_ref()
        .and_then(Interaction::legal_actions)
    else {
        return std::collections::HashSet::new();
    };
    // Nothing to reach for while there is nothing to tap.
    let sources = manasources::sources(view, legal);
    if sources.is_empty() {
        return std::collections::HashSet::new();
    }
    let Some(pool) = view
        .seat(baylee_client_core::decision::resource_player(view))
        .map(|s| s.mana_pool)
    else {
        return std::collections::HashSet::new();
    };
    let affordable = |id, cost: baylee_core::mana::ManaCost| {
        let cost = cost.with_more_generic(
            legal.spell_increase(id, baylee_engine::choice::CastModeKind::Normal),
        );
        baylee_client_core::manaplan::plan(&cost, &pool, &sources).is_some()
    };
    baylee_client_core::decision::hand(view)
        .iter()
        .filter(|card| !legal.lands.contains(&card.id))
        .filter(|card| !legal.castable.contains(&card.id) || phyrexian_life_only(card, &pool))
        // Types off the view, because those are the *projected* ones; flash
        // off the printed card, because a `HandObject` carries no keywords.
        .filter(|card| baylee_client_core::timing::allows(view, card.types, has_flash(card.card)))
        // And it has to print a cost at all (CR 202.1b). A blank cost is
        // payable by an empty pool, so a suspend-only card was "reachable"
        // with a plan of no taps at all: the click armed a run, the run
        // finished at once and asked the engine to cast a card it will never
        // offer. The engine has the same rule in `casting::has_a_printed_cost`
        // — this is the client not offering what that will refuse.
        .filter(|card| {
            manasources::hand_cost(card).is_some_and(|cost| cost.symbols().next().is_some())
                || !castmodes::reachable_modes(view, legal, card.id).is_empty()
        })
        .filter(|card| {
            if baylee_cards::by_index(card.card.index)
                .is_some_and(|def| def.faces[0].kicked_targets.is_some())
            {
                return !castmodes::reachable_modes(view, legal, card.id).is_empty();
            }
            manasources::hand_cost(card).is_some_and(|cost| affordable(card.id, cost))
                || !castmodes::reachable_modes(view, legal, card.id).is_empty()
        })
        // And there has to be something to point it at (CR 601.2c). Last in
        // the chain because it is the only filter here that walks the
        // battlefield, so it is asked only about a card the rest already
        // agreed on. `targeting` withholds the offer solely when it can
        // *prove* no legal target exists and offers whenever it cannot tell,
        // which is the opposite default from `castmodes::parts_payable` and
        // deliberately so — the reasoning is in that module's header.
        .filter(|card| !targeting::provably_targetless(view, card.card))
        .map(|card| card.id)
        // The command zone is castable from too (CR 903.8), and leaving it
        // out is why a commander could not be played. A card the engine has
        // not offered yet — because the mana is not floating — is reached
        // for by tapping lands, and this set is the whole of what the client
        // will offer to do that for. It only ever read the hand, so the one
        // card a commander deck is built around answered no click at all:
        // not `castable`, not `reachable`, no ability to activate, so the
        // tap fell through to the last branch and opened the zone browser.
        //
        // The tax is part of the cost and has to be added here rather than
        // read off the card: CR 903.8 is `{2}` more generic for each previous
        // cast *of that commander*, which is why `CommanderView::casts` is
        // per commander and not per seat.
        .chain(
            view.seat(baylee_client_core::decision::resource_player(view))
                .into_iter()
                .flat_map(|seat| seat.commanders.iter())
                .filter(|c| !legal.castable.contains(&c.object))
                // Still *in* the zone. A commander on the battlefield or in a
                // graveyard is named by the same `CommanderView` — the handle
                // follows the card through every move (CR 400.7) — and only
                // the one standing in the command zone is castable from it.
                .filter(|c| {
                    view.command
                        .get(view.seat.get() as usize)
                        .is_some_and(|zone| zone.iter().any(|o| o.id == c.object))
                })
                // And it is cast at the timing it would have from a hand
                // (CR 903.8) — which for the legend most commander decks are
                // built around is sorcery speed, so the same gate.
                .filter(|c| {
                    c.card.is_some_and(|card| {
                        baylee_client_core::timing::allows(
                            view,
                            printed_face(card)
                                .map_or(baylee_core::types::TypeSet::EMPTY, |f| f.types),
                            has_flash(card),
                        )
                    })
                })
                .filter(|c| {
                    commander_cost(c).is_some_and(|cost| {
                        affordable(c.object, cost.with_more_generic(2 * c.casts))
                    })
                })
                .map(|c| c.object),
        )
        // And the seat's own graveyard, for a card it may flash back
        // (CR 702.34a; #242). The engine offers such a card in `castable`
        // only once its cost is floating, exactly as it does a hand card, so
        // until this read the graveyard, Snapcaster Mage's Opt was a card
        // nothing could click on: it ended the turn in the graveyard beside
        // the untapped Island that paid for it.
        //
        // Priced at the flashback cost the view names and never at the card's
        // own. The two agree for the grants this pool has (Snapcaster, Past
        // in Flames), and they are different numbers for a card that prints
        // flashback (Think Twice: `{1}{U}` in hand, `{2}{U}` from the
        // graveyard). `PublicObject::flashback` is `None` in every zone and
        // for every card the seat may not cast that way, so the zone and the
        // owner are asked here only because no other graveyard can answer.
        .chain(
            view.graveyards
                .get(view.seat.get() as usize)
                .into_iter()
                .flatten()
                .filter(|o| !legal.castable.contains(&o.id))
                .filter(|o| {
                    o.card.is_some_and(|card| {
                        baylee_client_core::timing::allows(view, o.types, has_flash(card))
                    })
                })
                .filter(|o| {
                    o.flashback.is_some_and(|cost| {
                        cost.symbols().next().is_some() && affordable(o.id, cost)
                    })
                })
                .filter(|o| {
                    o.card
                        .is_some_and(|card| !targeting::provably_targetless(view, card))
                })
                .map(|o| o.id),
        )
        .collect()
}

/// Which cards in hand a tap or two would make **suspendable**.
///
/// [`reachable`]'s counterpart, and it needs one because suspend is paid for
/// like anything else: the engine offers `legal.suspendable` only once the
/// cost is floating, so a suspend card over untapped lands is a card with
/// nothing to do — which is precisely the report this answers, *"so first tap
/// and pay mana, then suspend"*.
///
/// The timing gate is [`baylee_client_core::timing::sorcery_window`] and not
/// `allows`: suspend's ability says "activate only as a sorcery" (CR 702.62a)
/// whatever the card's own type is, and the engine gates its offer on exactly
/// that. Flash on the card would be the wrong question — an instant with
/// suspend still may not suspend at instant speed.
pub(super) fn suspend_reach(duel: &Duel) -> std::collections::HashSet<ObjectId> {
    let Some(view) = duel.view.as_ref() else {
        return std::collections::HashSet::new();
    };
    let Some(legal) = duel
        .interaction
        .as_ref()
        .and_then(Interaction::legal_actions)
    else {
        return std::collections::HashSet::new();
    };
    if !baylee_client_core::timing::sorcery_window(view) {
        return std::collections::HashSet::new();
    }
    let sources = manasources::sources(view, legal);
    if sources.is_empty() {
        return std::collections::HashSet::new();
    }
    let Some(pool) = view
        .seat(baylee_client_core::decision::resource_player(view))
        .map(|s| s.mana_pool)
    else {
        return std::collections::HashSet::new();
    };
    baylee_client_core::decision::hand(view)
        .iter()
        .filter(|card| !legal.suspendable.contains(&card.id))
        .filter(|card| {
            suspend_cost(card.card).is_some_and(|cost| {
                baylee_client_core::manaplan::plan(&cost, &pool, &sources).is_some()
            })
        })
        .map(|card| card.id)
        .collect()
}

/// What a card's suspend ability costs, or `None` when it has none.
///
/// Off the card and not off the view: `AbilityDef::Suspend` is printed data,
/// and a `HandObject` carries the projected characteristics rather than the
/// abilities. The card's own list rather than the face's, for the reason
/// `keywords_for_face` exists — a single-faced card keeps its abilities on
/// the `CardDef` and leaves the face's list empty.
pub(super) fn suspend_cost(card: baylee_view::CardIdentity) -> Option<baylee_core::mana::ManaCost> {
    let def = baylee_cards::by_index(card.index)?;
    def.abilities.iter().find_map(|a| match a {
        baylee_cards_dsl::AbilityDef::Suspend { cost, .. } => Some(*cost),
        _ => None,
    })
}

/// A commander's printed cost, before CR 903.8's tax.
///
/// The card is named on the [`baylee_view::CommanderView`] itself rather than
/// looked up through the object, because a commander is public in every zone
/// (CR 903.3) and the view says so there whatever zone it is sitting in.
/// The printed face a `CardIdentity` names, out of the compiled registry.
///
/// A card the registry does not have answers `None`, and both callers read
/// that as the cautious thing: no types and no flash, which
/// [`baylee_client_core::timing::allows`] treats as sorcery speed. That is
/// the same reading `manasources::hand_cost` already gives such a card by
/// offering no cost at all.
pub(super) fn printed_face(
    card: baylee_view::CardIdentity,
) -> Option<&'static baylee_cards_dsl::FaceDef> {
    let def = baylee_cards::by_index(card.index)?;
    def.faces.get(card.face as usize).or(def.faces.first())
}

/// Whether the card behind a view's handle prints flash (CR 702.8a).
///
/// `keywords_for_face` and not `face.keywords`, because a single-faced card
/// keeps its keywords on the `CardDef` and leaves the face's list empty.
pub(super) fn has_flash(card: baylee_view::CardIdentity) -> bool {
    baylee_cards::by_index(card.index).is_some_and(|def| {
        def.keywords_for_face(card.face as usize)
            .contains(baylee_cards_dsl::KeywordSet::FLASH)
    })
}

pub(super) fn commander_cost(
    c: &baylee_view::CommanderView,
) -> Option<baylee_core::mana::ManaCost> {
    let card = c.card?;
    let def = baylee_cards::by_index(card.index)?;
    let face = def.faces.get(card.face as usize).or(def.faces.first())?;
    Some(face.mana_cost)
}
