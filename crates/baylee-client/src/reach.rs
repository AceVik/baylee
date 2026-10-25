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
///
/// And, while this seat declares attackers or blockers, its own creatures the
/// declaration can take now ([`Interaction::is_selectable`], read off the
/// engine's `Pending`): a blocker only where it may block the attacker in
/// focus. A board of a dozen creatures blocking a flier lit nothing, and
/// the one card a click would take looked like the eleven it would refuse
/// (report 31). The other seats' creatures in the declaration (the attackers
/// aimed at, a planeswalker attacked) are left to the focus ring.
pub(super) fn activatable(duel: &Duel) -> std::collections::HashSet<ObjectId> {
    let mut lit = std::collections::HashSet::new();
    activatable_into(duel, &mut lit);
    lit
}

pub(crate) fn activatable_into(duel: &Duel, lit: &mut std::collections::HashSet<ObjectId>) {
    lit.clear();
    if let Some(legal) = duel
        .interaction
        .as_ref()
        .and_then(Interaction::legal_actions)
    {
        lit.extend(
            legal
                .mana_abilities
                .iter()
                .copied()
                .chain(legal.abilities.iter().map(|(source, _)| *source)),
        );
    }
    if let (Some(choice), Some(view)) = (
        duel.interaction.as_ref().filter(|i| i.is_combat()),
        duel.view.as_ref(),
    ) {
        lit.extend(
            view.battlefield
                .iter()
                .filter(|o| o.controller == view.seat && choice.is_selectable(o.id))
                .map(|o| o.id),
        );
    }
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
#[allow(clippy::too_many_lines)]
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
    let Some((pool, life)) = view
        .seat(baylee_client_core::decision::resource_player(view))
        .map(|s| (s.mana_pool, s.life))
    else {
        return std::collections::HashSet::new();
    };
    let affordable = |id, cost: baylee_core::mana::ManaCost| {
        // An `{X}` is the player's to announce, after the cast is begun
        // (CR 601.2b): the least it can be is 0, so a card is in reach when
        // the rest of its cost is. Refusing every X cost left Prismatic
        // Ending (`{X}{W}`) answering no click while the engine offered it
        // (#17); the X question is then capped by what the mana pays.
        let cost = least_x(cost).with_more_generic(
            legal.spell_increase(id, baylee_engine::choice::CastModeKind::Normal),
        );
        // A card already in `legal.castable` is in this sweep only when
        // `phyrexian_life_only` kept it here so untapped lands can offer to
        // pay the Phyrexian mana instead of life. Settling those symbols with
        // life again would plan zero taps and mark a life-only card as needing
        // land taps even over off-colour lands.
        if legal.castable.contains(&id) {
            manasources::plan_for_spell(&cost, &pool, view, legal, id).is_some()
        } else {
            manasources::plan_with_life_for_spell(&cost, &pool, view, legal, id, life).is_some()
        }
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
            // Only a mode with a price of its own is left (Cyclonic Rift with
            // nothing to bounce, #00): the printed cost reaches nothing the
            // engine will cast, so only the modes castmodes can pay count.
            if baylee_cards::by_index(card.card.index)
                .is_some_and(|def| def.faces[0].kicked_targets.is_some())
                || crate::targeting::only_a_dearer_mode_is_left(view, card.card)
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
        // And any exile zone, for a card the engine offers to cast from exile
        // once its mana is made (`legal.payable`: an adventure's creature half,
        // CR 715.3d, or an exile cast permission). Permission is a game-state
        // fact `PublicObject` does not carry, so `legal.payable` is the gate:
        // the engine has already checked the permission and target legality.
        .chain(
            view.exile
                .iter()
                .flatten()
                .filter(|o| !legal.castable.contains(&o.id) && legal.payable.contains(&o.id))
                .filter(|o| {
                    o.card.is_some_and(|card| {
                        baylee_client_core::timing::allows(view, o.types, has_flash(card))
                    })
                })
                .filter(|o| {
                    o.card.and_then(printed_face).is_some_and(|face| {
                        face.mana_cost.symbols().next().is_some()
                            && affordable(o.id, face.mana_cost)
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

/// `cost` with each `{X}` at its least, 0 (CR 107.3): what reaching for an X
/// spell has to be able to pay before the player names the X.
pub(super) fn least_x(cost: baylee_core::mana::ManaCost) -> baylee_core::mana::ManaCost {
    if cost.has_variable() {
        cost.with_x(0)
    } else {
        cost
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_client_core::test_support::{ViewBuilder, token};
    use baylee_core::ids::Defender;
    use baylee_engine::choice::BlockOption;

    fn obj(slot: u32) -> ObjectId {
        ObjectId::new(slot, 0)
    }

    /// My creatures 10 (may block only attacker 1), 11 (only attacker 2) and
    /// 12 (may block neither); the opponent's attackers 1 and 2. `asked` is
    /// the seat the block question is for.
    fn blocking_duel(asked: u8) -> Duel {
        let view = ViewBuilder::new(2)
            .with_battlefield(
                0,
                vec![
                    token(10, 0, "Wall", 0, 4),
                    token(11, 0, "Hawk", 1, 1),
                    token(12, 0, "Sleeper", 1, 1),
                ],
            )
            .with_battlefield(
                1,
                vec![
                    token(1, 1, "Earth Elemental", 4, 4),
                    token(2, 1, "Sky Drake", 2, 2),
                ],
            )
            .build();
        let pending = Pending::ChooseBlockers {
            demands: Vec::new(),
            player: PlayerId::new(asked),
            attacker: PlayerId::new(1),
            blockers: vec![
                BlockOption {
                    blocker: obj(10),
                    attackers: vec![obj(1)],
                },
                BlockOption {
                    blocker: obj(11),
                    attackers: vec![obj(2)],
                },
            ],
            capacity: Vec::new(),
            obeying: Vec::new(),
            bounds: Vec::new(),
        };
        Duel {
            interaction: Some(Interaction::new(pending, view.seat)),
            view: Some(view),
            ..Duel::default()
        }
    }

    /// Report 31: with a dozen creatures the one a click would take looked
    /// like the eleven it would refuse. While blockers are declared the
    /// creatures that may block the attacker in aim wear the offer light,
    /// and nothing else on the table does.
    #[test]
    fn blockers_that_may_block_the_aimed_attacker_are_lit_and_nothing_else() {
        let duel = blocking_duel(0);
        let lit = activatable(&duel);
        assert!(
            lit.contains(&obj(10)),
            "10 may block the attacker in aim and was not lit: {lit:?}"
        );
        assert!(
            !lit.contains(&obj(11)),
            "11 may only block the other attacker: {lit:?}"
        );
        assert!(!lit.contains(&obj(12)), "12 may block nothing: {lit:?}");
        assert!(
            !lit.contains(&obj(1)) && !lit.contains(&obj(2)),
            "the opponent's attackers are left to the focus ring: {lit:?}"
        );
    }

    /// The lit set follows the aim, which is why `track_proposals` rebuilds
    /// the board when it moves.
    #[test]
    fn the_lit_blockers_follow_the_attacker_in_aim() {
        let mut duel = blocking_duel(0);
        assert!(
            duel.interaction.as_mut().unwrap().toggle(obj(2))
                == baylee_client_core::interaction::SelectionOutcome::Added,
            "aiming at the second attacker"
        );
        let lit = activatable(&duel);
        assert!(lit.contains(&obj(11)), "11 may block 2 now: {lit:?}");
        assert!(!lit.contains(&obj(10)), "10 may not block 2: {lit:?}");
    }

    /// Another seat's block question offers none of this seat's creatures,
    /// so none of them is lit.
    #[test]
    fn another_seats_block_question_lights_none_of_my_creatures() {
        let duel = blocking_duel(1);
        let lit = activatable(&duel);
        assert!(
            lit.is_empty(),
            "it is not this seat's declaration, yet these were lit: {lit:?}"
        );
    }

    /// Declaring attackers lights this seat's candidates, not the opponent's
    /// creatures and not a creature that cannot attack.
    #[test]
    fn declaring_attackers_lights_my_candidates_only() {
        let view = ViewBuilder::new(2)
            .with_battlefield(
                0,
                vec![token(10, 0, "Soldier", 1, 1), token(11, 0, "Wall", 0, 4)],
            )
            .with_battlefield(1, vec![token(1, 1, "Earth Elemental", 4, 4)])
            .build();
        let pending = Pending::ChooseAttackers {
            alone: Vec::new(),
            minimum_required: None,
            player: view.seat,
            attackers: vec![obj(10)],
            defenders: vec![Defender::Player(PlayerId::new(1))],
            required: Vec::new(),
            limits: Vec::new(),
        };
        let duel = Duel {
            interaction: Some(Interaction::new(pending, view.seat)),
            view: Some(view),
            ..Duel::default()
        };
        let lit = activatable(&duel);
        assert!(lit.contains(&obj(10)), "the candidate was not lit: {lit:?}");
        assert!(
            !lit.contains(&obj(11)) && !lit.contains(&obj(1)),
            "a creature that cannot attack, or the opponent's, was lit: {lit:?}"
        );
    }

    #[test]
    fn adventure_exiled_card_and_phyrexian_metamorph_are_reachable_with_untapped_lands() {
        use baylee_core::generated::subtypes::land;
        use baylee_core::ids::PrintRef;
        use baylee_core::types::{SubtypeSet, TypeSet};
        use baylee_engine::choice::LegalActions;
        use baylee_view::{CardIdentity, HandObject, PublicObject};

        let rider_def = baylee_cards::decks::by_name("Murderous Rider")
            .and_then(baylee_cards::by_index)
            .expect("Murderous Rider in registry");
        let metamorph_def = baylee_cards::decks::by_name("Phyrexian Metamorph")
            .and_then(baylee_cards::by_index)
            .expect("Phyrexian Metamorph in registry");

        let swamp = |slot: u32| PublicObject {
            types: TypeSet::LAND,
            subtypes: SubtypeSet::from_slice(&[land::SWAMP]),
            power: None,
            toughness: None,
            ..token(slot, 0, "Swamp", 0, 0)
        };
        let mut rider_exiled = token(50, 0, "Murderous Rider", 2, 3);
        rider_exiled.card = Some(CardIdentity {
            index: rider_def.index,
            print: PrintRef::new(0),
            face: 0,
        });

        let metamorph_hand = HandObject {
            id: obj(60),
            card: CardIdentity {
                index: metamorph_def.index,
                print: PrintRef::new(0),
                face: 0,
            },
            name: "Phyrexian Metamorph".to_string(),
            mana_value: 4,
            colors: baylee_core::color::ColorSet::default(),
            types: metamorph_def.faces[0].types,
            commander: false,
        };

        let mut view = ViewBuilder::new(2)
            .with_battlefield(0, vec![swamp(1), swamp(2), swamp(3)])
            .with_exile(0, vec![rider_exiled])
            .build();
        view.hand.push(metamorph_hand);

        let legal = LegalActions {
            mana_abilities: vec![obj(1), obj(2), obj(3)],
            payable: vec![obj(50), obj(60)],
            ..LegalActions::default()
        };
        let pending = Pending::Priority {
            player: view.seat,
            legal: Box::new(legal),
        };
        let duel = Duel {
            interaction: Some(Interaction::new(pending, view.seat)),
            view: Some(view),
            ..Duel::default()
        };

        let reach = reachable(&duel);
        assert!(
            reach.contains(&obj(50)),
            "Murderous Rider in adventure exile should be reachable with 3 Swamps: {reach:?}"
        );
        assert!(
            reach.contains(&obj(60)),
            "Phyrexian Metamorph in hand should be reachable with 3 Swamps + 2 life: {reach:?}"
        );
        assert_eq!(
            crate::mana_for(&duel, obj(50)).map(|p| p.taps()),
            Some(3),
            "mana_for should plan 3 Swamps for Murderous Rider in exile"
        );
        assert_eq!(
            crate::mana_for(&duel, obj(60)).map(|p| p.taps()),
            Some(3),
            "mana_for should plan 3 Swamps for Phyrexian Metamorph in hand"
        );
    }
}
