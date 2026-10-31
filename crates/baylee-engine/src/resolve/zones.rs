//! Zone movements: exile, blink, bounce, destruction, sacrifice,
//! graveyard recursion, mill — and countering spells/abilities (their
//! target's journey from the stack to another zone).

#[allow(clippy::wildcard_imports)] // family modules share the resolve vocabulary
use super::*;

mod sweep_filter;

/// Whether "sacrifice this" can still happen (CR 701.21a): "To sacrifice
/// a permanent, its controller moves it from the battlefield directly to
/// its owner's graveyard. A player can't sacrifice something that isn't a
/// permanent, or something that's a permanent they don't control."
///
/// Both halves are read, against the ability's controller. Before this
/// guard, `SacrificeSelf` moved its source from wherever it was. A land
/// bounced in response to its own enter trigger went from hand to
/// graveyard. A destroyed one went from graveyard to graveyard as a new
/// object (CR 400.7). A permanent whose control changed in response was
/// sacrificed by the player who no longer controlled it. Phased out is
/// read as well, because a phased-out permanent is treated as though it
/// does not exist (CR 702.26b). `battlefield_view` leaves it out for the
/// same reason.
///
/// It is the sibling of the battlefield check in `Engine::demand_echo`.
/// `resolve::exec_choice` reads it too, so "you may sacrifice this" is not
/// offered when the answer "yes" is impossible (CR 608.2d).
pub(super) fn can_sacrifice_self(state: &GameState, res: &Resolution) -> bool {
    super::subjects::source(state, res).is_some_and(|r| super::subjects::on_battlefield(state, r))
        && state.object(res.source).is_some_and(|o| {
            o.zone == crate::zone::Zone::Battlefield
                && o.controller == res.controller
                && !o.status.contains(crate::object::Status::PHASED_OUT)
        })
}

/// The object an effect's own [`TargetSpec`] names at resolution.
///
/// Every spec but one is chosen at CR 601.2c and read back out of
/// `res.targets`; the field on the effect is then only a record of what was
/// asked for. [`TargetSpec::ThisObject`] is the exception, because it names
/// the source and **nothing is chosen** — which is the whole difference
/// between "return Oboro to its owner's hand" and "return target land": no
/// question is asked, and hexproof has nothing to answer.
///
/// `res.source` and `res.on_stack` are separate fields, so this is the
/// source permanent and never the ability object resolving above it.
///
/// Three cards in the pool spell it, across two effects, and all three were
/// silently doing nothing before this existed (#147): Oboro, Palace in the
/// Clouds and Ghost Town through `ReturnToHand`, and The Tabernacle at
/// Pendrell Vale through `Destroy`. Each of those arms read an empty
/// `res.targets`, and a `None` there is how every one of them says "nothing
/// to move" — so the failure was silent by construction, and `xtask
/// validate` and the pool lints could not see it either, because all three
/// cards say the right thing.
///
/// The Tabernacle is the one that says which object `res.source` has to be.
/// Its sentence is granted to every creature (`Modifier::GrantTriggered`),
/// and `trigger.rs` pushes the granted trigger with `source: permanent` —
/// the creature the ability was granted *to*, not the land that granted it.
/// So "destroy this creature" destroys the creature, which is both the
/// printed sentence and what a granted ability's source means.
///
/// [`TargetSpec::EventObject`] is the **second** implicit one, and it was
/// found the same way a second time. A triggered ability fills `res.targets`
/// from the event object only when its *target requirement* says
/// `EventObject`, and "when enchanted creature dies, return it to the
/// battlefield" names no target at all (CR 115.1: an ability targets only
/// where it says "target"). So Journey to Eternity read an empty list, left
/// the creature in the graveyard, and returned its own back face
/// transformed — a `Coverage::Implemented` card doing half of what it
/// prints, silent for exactly the reason the three above were.
///
/// Which is why the catch-all is gone. The arm that hid this one was
/// `_ => res.targets.first()`, and a spec that names something rather than
/// asking for it reads as "nobody chose anything" there. Every variant is
/// listed now, so the next implicit spec is a compile error instead of a
/// card that quietly does nothing.
pub(super) fn spec_object(
    state: &GameState,
    res: &Resolution,
    target: TargetSpec,
) -> Option<ObjectId> {
    match target {
        TargetSpec::ThisObject => super::subjects::source(state, res)
            .filter(|r| super::subjects::is_current(state, *r))
            .map(|r| r.object),
        TargetSpec::EventObject => super::subjects::event(state, res)
            .filter(|reference| super::subjects::is_current(state, *reference))
            .map(|reference| reference.object),
        // The one spec that is only ever a second instance of "target"
        // names the object that instance chose.
        TargetSpec::ObjectOfFirstTargetsPlayer(_) | TargetSpec::CardInFirstTargetsGraveyard(_) => {
            res.second_targets.first().copied()
        }
        TargetSpec::Object(_)
        | TargetSpec::ObjectOfEachOpponent(_)
        | TargetSpec::OpponentOrObject(_)
        | TargetSpec::PlayerOrObject(_)
        | TargetSpec::ObjectControlledBy(..)
        | TargetSpec::ObjectOfEventPlayer(_)
        | TargetSpec::Spell(_)
        | TargetSpec::StackOrBattlefield(_)
        | TargetSpec::CardInGraveyard(..)
        | TargetSpec::CardInPlayersGraveyard(..)
        | TargetSpec::CardInGraveyardBelowEvent(..)
        | TargetSpec::CardInGraveyardBelowValue(..)
        | TargetSpec::AbilityOnStack(_)
        | TargetSpec::SpellOrAbility(_)
        | TargetSpec::Player(_)
        | TargetSpec::AnyPlayer
        | TargetSpec::AnyOpponent
        | TargetSpec::PlayerWhoAttackedThisTurn
        | TargetSpec::AnyTarget => res.targets.first().copied(),
    }
}

/// Every object an effect's [`TargetSpec`] names at resolution, not just the
/// first.
///
/// An effect whose spec is *chosen* reads the whole of `res.targets`, because
/// one instance of the word "target" can carry a count: Reveillark returns
/// "up to two target creature cards" and Entreat the Dead returns X of them,
/// and both were written against [`spec_object`] — which answers `first()` —
/// so each of them brought exactly one card back while claiming
/// `Coverage::Implemented`. `Effect::Exile` had the same bug and the same
/// fix, one card earlier (Pit of Offerings).
///
/// The two implicit specs stay singular by construction: neither names a
/// list, and `res.targets` for an untargeted synthetic trigger holds at most
/// its one implicit target, so reading it here would add nothing.
pub(super) fn spec_objects(
    state: &GameState,
    res: &Resolution,
    target: TargetSpec,
) -> SmallVec<[ObjectId; 2]> {
    match target {
        TargetSpec::CardInFirstTargetsGraveyard(_) => res.second_targets.iter().copied().collect(),
        TargetSpec::ThisObject
        | TargetSpec::EventObject
        | TargetSpec::ObjectOfFirstTargetsPlayer(_) => {
            spec_object(state, res, target).into_iter().collect()
        }
        _ => res.targets.clone(),
    }
}

/// Whether an opponent of `you` became the monarch after the ability
/// `on_stack` was put on the stack: the journal's record of the stacking,
/// then a [`GameEvent::BecameMonarch`] after it.
///
/// The stacking stands in for the triggering (CR 610.3b), which it follows
/// with nothing resolving in between. An object with no such record, a
/// spell, finds nothing and exiles as before.
fn an_opponent_was_crowned_since(state: &GameState, on_stack: ObjectId, you: PlayerId) -> bool {
    let entries = state.journal.entries();
    let Some(stacked) = entries.iter().rposition(
        |e| matches!(e.event, GameEvent::AbilityTriggered { object, .. } if object == on_stack),
    ) else {
        return false;
    };
    entries[stacked + 1..].iter().any(|e| {
        matches!(e.event, GameEvent::BecameMonarch { player } if state.is_opponent(player, you))
    })
}

// Explicit movement instructions can use CR 400.7e's departure destination;
// ordinary This readers retain the source incarnation instead.
pub(super) fn moving_objects(
    state: &GameState,
    res: &Resolution,
    target: TargetSpec,
) -> SmallVec<[ObjectId; 2]> {
    if matches!(target, TargetSpec::ThisObject) {
        super::subjects::moving_source(state, res)
            .map(|r| r.object)
            .into_iter()
            .collect()
    } else {
        spec_objects(state, res, target)
    }
}

/// Executes one zone-movement effect.
#[allow(clippy::too_many_lines)] // the zone vocabulary is one flat table
pub(super) fn exec(state: &mut GameState, res: &mut Resolution, op: Effect) -> Option<Pending> {
    let you = res.controller;
    match op {
        Effect::ReturnSourceFromGraveyard { .. }
        | Effect::GraveyardToBattlefieldLinked { .. }
        | Effect::GraveyardToBattlefield { .. }
        | Effect::Assimilate { .. }
        | Effect::ExileSelfReturnAsFace { .. } => super::reanimation::exec(state, res, op),
        Effect::Exile { target } => {
            // Every target and not the first. A card may name more than one
            // — Pit of Offerings prints "exile up to three target cards from
            // graveyards" and carries `TargetReq::up_to(.., 3)` — and a
            // reader that took `first()` exiled one of them and left the
            // rest where they were, with the card still claiming
            // `Coverage::Implemented`. A single-target exile has one entry
            // in `res.targets`, so nothing else in the pool moves.
            for target_id in moving_objects(state, res, target) {
                if matches!(target, TargetSpec::EventObject)
                    && state.object(target_id).is_none_or(|o| {
                        o.zone != crate::zone::Zone::Battlefield
                            || o.status.contains(crate::object::Status::PHASED_OUT)
                    })
                {
                    continue;
                }
                let owner = state.object(target_id).map_or(you, |o| o.owner);
                if let Some(obj) = state.object_mut(target_id) {
                    obj.kind = ObjectKind::Card;
                }
                let _ = state.move_object(
                    target_id,
                    ZoneLocation::Exile(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
        Effect::Blink { owner_control, .. } => {
            if let Some(&target_id) = res.targets.first() {
                let components = state.merged_components(target_id);
                let owner = state.object(target_id).map_or(you, |o| o.owner);
                if let Some(obj) = state.object_mut(target_id) {
                    obj.kind = ObjectKind::Card;
                }
                let _ = state.move_object(
                    target_id,
                    ZoneLocation::Exile(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
                if !owner_control && state.has_left(you) {
                    return None;
                }
                // A merged permanent leaves once, then each physical card returns
                // separately. Tokens cannot return after changing zones.
                for card in components {
                    if let Some(obj) = state
                        .object_mut(card)
                        .filter(|o| o.zone == crate::zone::Zone::Exile && o.is_card())
                    {
                        obj.kind = ObjectKind::Permanent;
                        obj.set_controller(if owner_control { obj.owner } else { you });
                        let _ = state.move_object(
                            card,
                            ZoneLocation::Battlefield,
                            ZonePosition::Top,
                            Cause::Effect,
                        );
                    }
                }
            }
            None
        }
        Effect::ReturnToHand { target } => {
            let targets = moving_objects(state, res, target);
            if !targets.is_empty() {
                let planned: SmallVec<[(ObjectId, ZoneLocation); 2]> = targets
                    .iter()
                    .map(|&id| {
                        let owner = state.object(id).map_or(you, |o| o.owner);
                        (id, ZoneLocation::Hand(owner))
                    })
                    .collect();
                // CR 903.9b, before the kind flips below: this operation
                // re-runs from the top once every owner has answered.
                if let Some(pending) = ask_commander_replace(state, res, &planned) {
                    return Some(pending);
                }
                for (target_id, dest) in planned {
                    if let Some(obj) = state.object_mut(target_id) {
                        obj.kind = ObjectKind::Card;
                    }
                    let _ = state.move_object(target_id, dest, ZonePosition::Top, Cause::Effect);
                }
            }
            None
        }
        Effect::ReturnAllToHand { filter, .. } | Effect::ReturnAllOwnedToHand { filter, .. } => {
            let (opponents_only, owners) = match op {
                Effect::ReturnAllToHand { opponents_only, .. } => (opponents_only, None),
                Effect::ReturnAllOwnedToHand { owner, .. } => {
                    (false, Some(players_of(owner, state, you, res)))
                }
                _ => unreachable!("the return-all family"),
            };
            let all: Vec<ObjectId> = state
                .battlefield_seen()
                .filter(|id| {
                    state.object(*id).is_some_and(|o| {
                        (!opponents_only || state.is_opponent(o.controller, you))
                            && owners
                                .as_ref()
                                .is_none_or(|owners| owners.contains(&o.owner))
                            && eval::matches_with_context(filter, state, o, you, res.rule_context())
                    })
                })
                .collect();
            let moves: Vec<(ObjectId, ZoneLocation)> = all
                .iter()
                .map(|&id| {
                    let owner = state.object(id).map_or(you, |o| o.owner);
                    (id, ZoneLocation::Hand(owner))
                })
                .collect();
            // CR 903.9b "may apply more than once to the same event": a
            // wrath that catches two commanders asks both owners, and only
            // then does any card move. The last answer re-enters this arm
            // from the top, so `all` is gathered a second time — which is
            // the same set only because nothing above this line mutates.
            // That is the precondition `ask_commander_replace` documents,
            // and a mutation added before it would break this silently.
            if let Some(pending) = ask_commander_replace(state, res, &moves) {
                return Some(pending);
            }
            for (id, to) in moves {
                if let Some(obj) = state.object_mut(id) {
                    obj.kind = ObjectKind::Card;
                }
                let _ = state.move_object(id, to, ZonePosition::Top, Cause::Effect);
            }
            None
        }
        Effect::DestroyAll { filter, .. } | Effect::DestroyAllWithManaValue { filter, .. } => {
            let (no_regen, value) = match op {
                Effect::DestroyAll { no_regen, .. } => (no_regen, None),
                Effect::DestroyAllWithManaValue { value, .. } => {
                    (false, Some(amount2(&value, state, you, res)))
                }
                _ => unreachable!("destruction family"),
            };
            // Not a phased-out creature: "Destroy all creatures" passes over
            // one (CR 702.26b's example).
            let all: Vec<ObjectId> = state
                .battlefield_seen()
                .filter(|id| {
                    state.object(*id).is_some_and(|o| {
                        value.is_none_or(|value| o.characteristics().mana_value() == value)
                            && sweep_filter::matches(filter, state, o, res)
                    })
                })
                .collect();
            sba::destroy_all(state, &all, no_regen);
            None
        }
        // Farewell's four sweeps. Nothing is targeted, and a phased-out
        // permanent is treated as though it doesn't exist (CR 702.26b),
        // which `battlefield_seen` is.
        Effect::ExileAll { filter, linked } => {
            let all: Vec<ObjectId> = state
                .battlefield_seen()
                .filter(|id| {
                    state.object(*id).is_some_and(|o| {
                        eval::matches_with_context(filter, state, o, you, res.rule_context())
                    })
                })
                .collect();
            for id in all {
                let Some(owner) = state.object(id).map(|o| o.owner) else {
                    continue;
                };
                if linked && let Some(obj) = state.object_mut(id) {
                    obj.kind = ObjectKind::Card;
                    obj.riders.push(crate::object::Rider::Linked {
                        host: res.source,
                        until: None,
                    });
                }
                let _ = state.move_object(
                    id,
                    ZoneLocation::Exile(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
        // Maelstrom Pulse's sweep. The name is read now, off the projected
        // characteristics of the permanent still on the battlefield — the
        // card effect destroying it comes after this one — and a nameless
        // one names nothing (CR 201.2a). Phased-out permanents are treated as
        // though they don't exist (CR 702.26b), which `battlefield_seen` is.
        Effect::DestroyOthersNamedLike { target } => {
            let named = spec_object(state, res, target)?;
            let name = state
                .object(named)
                .filter(|o| o.zone == crate::zone::Zone::Battlefield)
                .map(|o| o.characteristics().name)
                .filter(|n| *n != crate::state::NAMELESS)?;
            let others: Vec<ObjectId> = state
                .battlefield_seen()
                .filter(|&id| {
                    id != named
                        && state.object(id).is_some_and(|o| {
                            o.kind == ObjectKind::Permanent && o.characteristics().name == name
                        })
                })
                .collect();
            for id in others {
                sba::destroy(state, id);
            }
            None
        }
        Effect::ExileGraveyard { player } => {
            // Bojuka Bog says "target player's graveyard" — `Chosen`, which
            // `eval::players` answers with nothing at all. It shipped as a
            // Swamp that costs a land drop.
            for player in players_of(player, state, you, res) {
                let cards: Vec<ObjectId> =
                    state.zones.list(ZoneLocation::Graveyard(player)).clone();
                for card in cards {
                    let _ = state.move_object(
                        card,
                        ZoneLocation::Exile(player),
                        ZonePosition::Top,
                        Cause::Effect,
                    );
                }
            }
            None
        }
        Effect::GraveyardToHand { target } => {
            let moves: Vec<_> = moving_objects(state, res, target)
                .into_iter()
                .filter_map(|id| state.object(id).map(|o| (id, ZoneLocation::Hand(o.owner))))
                .collect();
            // Ask every commander's owner before moving any selected card.
            // CR 903.9b is a separate choice from leaving a graveyard (903.9a).
            if let Some(pending) = ask_commander_replace(state, res, &moves) {
                return Some(pending);
            }
            for (id, destination) in moves {
                let _ = state.move_object(id, destination, ZonePosition::Top, Cause::Effect);
            }
            None
        }
        Effect::GraveyardAllToHand { filter } => {
            // Chosen before anything moves: CR 903.9b's question comes
            // first, and its last answer re-enters here with the same list.
            let moves: Vec<(ObjectId, ZoneLocation)> = state
                .zones
                .list(ZoneLocation::Graveyard(you))
                .iter()
                .copied()
                .filter(|id| {
                    state.object(*id).is_some_and(|o| {
                        eval::matches_with_context(filter, state, o, you, res.rule_context())
                    })
                })
                .map(|id| (id, ZoneLocation::Hand(you)))
                .collect();
            if let Some(pending) = ask_commander_replace(state, res, &moves) {
                return Some(pending);
            }
            for (card, to) in moves {
                let _ = state.move_object(card, to, ZonePosition::Top, Cause::Effect);
            }
            None
        }
        Effect::GraveyardToTop { target } => {
            if let Some(&target_id) = moving_objects(state, res, target).first() {
                let owner = state.object(target_id).map_or(you, |o| o.owner);
                // CR 903.9b.
                if let Some(pending) =
                    ask_commander_replace(state, res, &[(target_id, ZoneLocation::Library(owner))])
                {
                    return Some(pending);
                }
                let _ = state.move_object(
                    target_id,
                    ZoneLocation::Library(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
        Effect::PutSourceOnTopOfLibrary => {
            let source = super::subjects::moving_source(state, res)?.object;
            let owner = state.object(source).map_or(you, |o| o.owner);
            // CR 903.9b: a commander that puts *itself* on top is still
            // being put into a library, and its owner still chooses.
            if let Some(pending) =
                ask_commander_replace(state, res, &[(source, ZoneLocation::Library(owner))])
            {
                return Some(pending);
            }
            let _ = state.move_object(
                source,
                ZoneLocation::Library(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
            None
        }
        Effect::BottomCardFromHand {
            player,
            filter,
            then,
        } => {
            let player = players_of(player, state, you, res).first().copied()?;
            let options: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Hand(player))
                .iter()
                .filter(|id| {
                    state.object(**id).is_some_and(|o| {
                        eval::matches_with_context(filter, state, o, player, res.rule_context())
                    })
                })
                .copied()
                .collect();
            if options.is_empty() {
                return None;
            }
            // Two different seats, and they used to be one. The hand is the
            // target's and the card goes under *their* library, which is what
            // the continuation carries; the **choice** is the ability's
            // controller's — Vendilion Clique reads "look at target player's
            // hand. *You* may choose a nonland card from it", and the whole
            // point of the card is that you see someone else's hand and
            // decide. Asking the owner to pick which of their own cards to
            // bury inverted it: a seat attacked by the Clique chose their
            // worst card and thanked you for the draw.
            res.awaiting = Some(AwaitingOp::BottomFromHand { player, then });
            Some(Pending::ChooseCards {
                player: you,
                options,
                min: 0,
                max: 1,
                prompt: ChoicePrompt::Generic,
                total: None,
            })
        }
        Effect::GraveyardToLibraryBottom { player } => {
            let players = players_of(player, state, you, res);
            let mut cards: Vec<_> = players
                .into_iter()
                .filter(|p| !state.has_left(*p))
                .flat_map(|p| state.zones.list(ZoneLocation::Graveyard(p)).iter().copied())
                .collect();
            let moves: Vec<_> = cards
                .iter()
                .filter_map(|&card| {
                    state
                        .object(card)
                        .map(|o| (card, ZoneLocation::Library(o.owner)))
                })
                .collect();
            if let Some(pending) = ask_commander_replace(state, res, &moves) {
                return Some(pending);
            }
            state.rng.shuffle(&mut cards);
            for card in cards {
                if let Some(owner) = state.object(card).map(|o| o.owner) {
                    let _ = state.move_object(
                        card,
                        ZoneLocation::Library(owner),
                        ZonePosition::Bottom,
                        Cause::Effect,
                    );
                }
            }
            None
        }
        Effect::PutSelfOntoBattlefield => {
            let subject = super::subjects::source(state, res)
                .filter(|r| super::subjects::is_current(state, *r))?;
            let owner = state.object(subject.object)?.owner;
            put_found(state, owner, subject.object, SearchDest::Battlefield);
            None
        }
        Effect::PutSourceFromHandOntoBattlefield => {
            let subject = super::subjects::source(state, res)
                .filter(|r| super::subjects::is_current(state, *r))?;
            let object = state.object(subject.object)?;
            if object.zone != crate::zone::Zone::Hand || object.zone_owner != Some(you) {
                return None;
            }
            put_found(state, you, subject.object, SearchDest::Battlefield);
            None
        }
        Effect::ShuffleTargetsIntoOwnersLibraries => {
            let moves: Vec<_> = res
                .targets
                .iter()
                .filter_map(|&card| {
                    state
                        .object(card)
                        .map(|object| (card, ZoneLocation::Library(object.owner)))
                })
                .collect();
            if let Some(pending) = ask_commander_replace(state, res, &moves) {
                return Some(pending);
            }
            let mut owners = baylee_core::ids::SeatSet::new();
            for (card, destination) in moves {
                if let ZoneLocation::Library(owner) = destination {
                    owners.insert(owner);
                }
                let _ = state.move_object(card, destination, ZonePosition::Top, Cause::Effect);
            }
            for owner in owners.iter() {
                state.shuffle_library(owner);
            }
            None
        }
        Effect::ShuffleGraveyardIntoLibrary => {
            let graveyard: Vec<ObjectId> = state.zones.list(ZoneLocation::Graveyard(you)).clone();
            let moves: Vec<(ObjectId, ZoneLocation)> = graveyard
                .iter()
                .map(|&card| (card, ZoneLocation::Library(you)))
                .collect();
            // CR 903.9b, asked before the shuffle rather than after it: a
            // commander picked back out of a shuffled library is a commander
            // whose owner has been told where it landed.
            if let Some(pending) = ask_commander_replace(state, res, &moves) {
                return Some(pending);
            }
            for (card, to) in moves {
                let _ = state.move_object(card, to, ZonePosition::Top, Cause::Effect);
            }
            state.shuffle_library(you);
            None
        }
        Effect::PhaseOutFilter(filter) => {
            let objects: Vec<_> = state
                .battlefield_seen()
                .filter(|id| {
                    state.object(*id).is_some_and(|object| {
                        eval::matches_with_context(filter, state, object, you, res.rule_context())
                    })
                })
                .collect();
            state.phase_out(&objects);
            None
        }
        Effect::PhaseOut { target } => {
            // Every target: Clever Concealment's "any number of target
            // nonland permanents" phases out as many as it named, and a
            // target that became illegal already left the list (CR 608.2b).
            // What is attached to each goes with it (CR 702.26g).
            match target {
                Some(_) => {
                    let targets = res.targets.clone();
                    state.phase_out(&targets);
                }
                None => state.phase_out(&[res.source]),
            }
            None
        }
        Effect::PhaseOutUntilSourceLeaves { tap_on_phase_in } => {
            // CR 610.3a, 610.4: if the source has already left the
            // battlefield before the ability resolves, the target does not
            // phase out at all.
            // Phasing is not a zone change: a phased-out source has not
            // fulfilled the event that ends this one-shot effect.
            let host_gone = state
                .object(res.source)
                .is_none_or(|host| host.zone != crate::zone::Zone::Battlefield)
                || state
                    .object(res.on_stack)
                    .is_some_and(|ability| ability.source_power_lki.is_some());
            if host_gone {
                return None;
            }
            let targets: Vec<ObjectId> = res
                .targets
                .iter()
                .copied()
                .filter(|&id| {
                    state.object(id).is_some_and(|o| {
                        o.zone == crate::zone::Zone::Battlefield
                            && !o.status.contains(crate::object::Status::PHASED_OUT)
                    })
                })
                .collect();
            for &target_id in &targets {
                if let Some(obj) = state.object_mut(target_id) {
                    obj.riders
                        .push(crate::object::Rider::PhasedOutUntilHostLeaves {
                            host: res.source,
                            tap_on_phase_in,
                        });
                }
            }
            state.phase_out(&targets);
            None
        }
        Effect::ExileLinked { until, .. } => {
            let until = until.map(|until| match until {
                baylee_cards_dsl::ExileUntil::SourceLeavesBattlefield => {
                    crate::object::LinkUntil::HostLeaves
                }
                baylee_cards_dsl::ExileUntil::OpponentBecomesMonarch => {
                    crate::object::LinkUntil::OpponentBecomesMonarch { of: you }
                }
            });
            // "Until this creature leaves the battlefield", and it already
            // has: the card does not move (CR 610.3a, 610.3b). Either it is
            // not there now, or it left while this waited on the stack and
            // came back as a new object, which the stack object wrote down
            // as it went (`source_power_lki`, frozen on the way out).
            if until == Some(crate::object::LinkUntil::HostLeaves)
                && (state
                    .object(res.source)
                    .is_none_or(|host| host.zone != crate::zone::Zone::Battlefield)
                    || state
                        .object(res.on_stack)
                        .is_some_and(|ability| ability.source_power_lki.is_some()))
            {
                return None;
            }
            // "Until an opponent becomes the monarch", and one already has
            // since this was put on the stack: the card does not move either
            // (CR 610.3a, 610.3b).
            if until == Some(crate::object::LinkUntil::OpponentBecomesMonarch { of: you })
                && an_opponent_was_crowned_since(state, res.on_stack, you)
            {
                return None;
            }
            if let Some(&target_id) = res.targets.first() {
                let components = state.merged_components(target_id);
                let owner = state.object(target_id).map_or(you, |o| o.owner);
                if let Some(obj) = state.object_mut(target_id) {
                    obj.kind = ObjectKind::Card;
                    obj.riders.push(crate::object::Rider::Linked {
                        host: res.source,
                        until,
                    });
                }
                let _ = state.move_object(
                    target_id,
                    ZoneLocation::Exile(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
                if let Some(version) = super::source_version(state, res)
                    .or_else(|| state.object(res.source).map(|o| o.version))
                {
                    for component in components {
                        if let Some(object) = state
                            .object_mut(component)
                            .filter(|o| o.zone == crate::zone::Zone::Exile)
                        {
                            let link = crate::object::Rider::Linked {
                                host: res.source,
                                until,
                            };
                            if !object.riders.contains(&link) {
                                object.riders.push(link);
                            }
                            state.remember_link(
                                component,
                                baylee_core::ids::DamageSourceRef {
                                    object: res.source,
                                    version,
                                },
                            );
                        }
                    }
                }
            }
            None
        }
        Effect::RevealLibrarySearch { filter, optional } => {
            let options: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Library(you))
                .iter()
                .rev()
                .copied()
                .filter(|&card| {
                    state.object(card).is_some_and(|object| {
                        eval::matches_with_context(filter, state, object, you, res.rule_context())
                    })
                })
                .collect();
            if options.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::RevealLibrarySearch);
            Some(Pending::ChooseCards {
                player: you,
                options,
                min: u8::from(!optional),
                max: 1,
                prompt: ChoicePrompt::Generic,
                total: None,
            })
        }
        Effect::ExileChosenFromHandWithSource(filter) => {
            let options: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Hand(you))
                .iter()
                .copied()
                .filter(|&card| {
                    state.object(card).is_some_and(|object| {
                        eval::matches_with_context(filter, state, object, you, res.rule_context())
                    })
                })
                .collect();
            if options.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::ExileFromHandWithSource);
            Some(Pending::ChooseCards {
                player: you,
                options,
                min: 1,
                max: 1,
                prompt: ChoicePrompt::Generic,
                total: None,
            })
        }
        // Every target, each exiled with the source (CR 406.6) as the
        // object it is now.
        Effect::ExileTargetsWithSource => {
            let source = state
                .recorded_ability_source(res.on_stack)
                .or_else(|| super::subjects::source(state, res))
                .or_else(|| {
                    super::source_version(state, res).map(|version| {
                        baylee_core::ids::DamageSourceRef {
                            object: res.source,
                            version,
                        }
                    })
                })
                .unwrap_or(baylee_core::ids::DamageSourceRef {
                    object: res.source,
                    version: 0,
                });
            let version = source.version;
            let acquisition = super::subjects::acquisition(state, res);
            for &target in &res.targets {
                let components = state.merged_components(target);
                let owner = state.object(target).map_or(you, |o| o.owner);
                if let Some(obj) = state.object_mut(target) {
                    obj.kind = ObjectKind::Card;
                    obj.riders.push(crate::object::Rider::ExiledWith {
                        host: source.object,
                        version: u16::try_from(version).unwrap_or(u16::MAX),
                    });
                }
                let _ = state.move_object(
                    target,
                    ZoneLocation::Exile(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
                for component in components {
                    if let Some(card) = state
                        .object_mut(component)
                        .filter(|card| card.zone == crate::zone::Zone::Exile)
                    {
                        let with = crate::object::Rider::ExiledWith {
                            host: source.object,
                            version: u16::try_from(version).unwrap_or(u16::MAX),
                        };
                        if !card.riders.contains(&with) {
                            card.riders.push(with);
                        }
                        card.riders.push(crate::object::Rider::LinkedExile {
                            source,
                            acquisition,
                        });
                    }
                }
            }
            None
        }
        Effect::SacrificeSelf => {
            if !can_sacrifice_self(state, res) {
                return None;
            }
            if let Some(obj) = state.object_mut(res.source) {
                obj.kind = ObjectKind::Card;
            }
            let _ = state.sacrifice_object(res.source, Cause::Effect);
            None
        }
        // "Sacrifice that creature": by the ability's controller, and only a
        // permanent of theirs that is still the object the spec names
        Effect::SacrificeAll { filter } => {
            let all: Vec<ObjectId> = state
                .battlefield_seen()
                .filter(|id| {
                    state.object(*id).is_some_and(|o| {
                        eval::matches_with_context(filter, state, o, you, res.rule_context())
                    })
                })
                .collect();
            super::sacrifice_together(state, &all);
            None
        }
        // CR 701.21a: a player sacrifices only a permanent they control.
        Effect::SacrificeObject { target } => {
            let id = spec_object(state, res, target)?;
            let _owner = state.object(id).and_then(|o| {
                (o.zone == crate::zone::Zone::Battlefield
                    && o.controller == res.controller
                    && !o.status.contains(crate::object::Status::PHASED_OUT))
                .then_some(o.owner)
            })?;
            if let Some(obj) = state.object_mut(id) {
                obj.kind = ObjectKind::Card;
            }
            let _ = state.sacrifice_object(id, Cause::Effect);
            None
        }
        Effect::PutTargetOnTopOfLibrary | Effect::PutTargetOnBottomOfLibrary => {
            let top = matches!(op, Effect::PutTargetOnTopOfLibrary);
            let moves: Vec<(ObjectId, ZoneLocation)> = res
                .targets
                .iter()
                .map(|&target| {
                    let owner = state.object(target).map_or(you, |o| o.owner);
                    (target, ZoneLocation::Library(owner))
                })
                .collect();
            // CR 903.9b. The tuck this rule exists for.
            if let Some(pending) = ask_commander_replace(state, res, &moves) {
                return Some(pending);
            }
            for (target, to) in moves {
                if let Some(obj) = state.object_mut(target) {
                    obj.kind = ObjectKind::Card;
                }
                let position = if top {
                    ZonePosition::Top
                } else {
                    ZonePosition::Bottom
                };
                let _ = state.move_object(target, to, position, Cause::Effect);
            }
            None
        }
        Effect::RollDiePutTargetIntoLibraryBeneathTop { sides } => {
            let moves: Vec<(ObjectId, ZoneLocation)> = res
                .targets
                .iter()
                .map(|&target| {
                    let owner = state.object(target).map_or(you, |o| o.owner);
                    (target, ZoneLocation::Library(owner))
                })
                .collect();
            if let Some(pending) = ask_commander_replace(state, res, &moves) {
                return Some(pending);
            }
            let x = (state.rng.below(u64::from(sides.max(1))) as usize) + 1;
            for (target, to) in moves {
                if let Some(obj) = state.object_mut(target) {
                    obj.kind = ObjectKind::Card;
                }
                let index = state.zones.list(to).len().saturating_sub(x);
                let _ = state.move_object(target, to, ZonePosition::Index(index), Cause::Effect);
            }
            None
        }
        Effect::PutOnBottomOfLibraryFromGraveyard { target } => {
            // CR 400.7, as `GraveyardToBattlefield` asks it: a card that left
            // the graveyard in response is a new object, and "it" is gone.
            let moves: Vec<(ObjectId, ZoneLocation)> = moving_objects(state, res, target)
                .into_iter()
                .filter_map(|card| {
                    let obj = state.object(card)?;
                    (obj.zone == crate::zone::Zone::Graveyard)
                        .then_some((card, ZoneLocation::Library(obj.owner)))
                })
                .collect();
            // CR 903.9b: a commander's owner may put it in the command zone.
            if let Some(pending) = ask_commander_replace(state, res, &moves) {
                return Some(pending);
            }
            for (card, to) in moves {
                let _ = state.move_object(card, to, ZonePosition::Bottom, Cause::Effect);
            }
            None
        }
        Effect::ExileSource => {
            let owner = state.object(res.source).map_or(you, |o| o.owner);
            if let Some(obj) = state.object_mut(res.source) {
                obj.kind = ObjectKind::Card;
            }
            let _ = state.move_object(
                res.source,
                ZoneLocation::Exile(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
            None
        }
        Effect::ExileSourceWithCounters { kind, n } => {
            let owner = state.object(res.source).map_or(you, |o| o.owner);
            if let Some(obj) = state.object_mut(res.source) {
                obj.kind = ObjectKind::Card;
            }
            let _ = state.move_object(
                res.source,
                ZoneLocation::Exile(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
            if state
                .object(res.source)
                .is_some_and(|o| o.zone == crate::zone::Zone::Exile)
            {
                crate::replacement::put_counters_by(state, you, res.source, kind, n);
            }
            None
        }
        Effect::MoveSourceToOwnerGraveyard => {
            if let Some(source) = super::subjects::source(state, res)
                && super::subjects::is_current(state, source)
                && let Some(owner) = state
                    .object(source.object)
                    .filter(|o| o.zone == crate::zone::Zone::Exile)
                    .map(|o| o.owner)
            {
                let _ = state.move_object(
                    source.object,
                    ZoneLocation::Graveyard(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
        Effect::EachPlayerGraveyardToBattlefield { filter } => {
            for seat in 0..state.players.len() {
                let p = PlayerId::new(seat as u8);
                if state.has_left(p) {
                    continue;
                }
                for &card in &state.zones.list(ZoneLocation::Graveyard(p)).clone() {
                    let matches = state.object(card).is_some_and(|o| {
                        eval::matches_with_context(filter, state, o, you, res.rule_context())
                    });
                    if matches {
                        if let Some(obj) = state.object_mut(card) {
                            obj.kind = ObjectKind::Permanent;
                            obj.set_controller(p);
                        }
                        let _ = state.move_object(
                            card,
                            ZoneLocation::Battlefield,
                            ZonePosition::Top,
                            Cause::Effect,
                        );
                    }
                }
            }
            None
        }
        Effect::TawnossCoffinExile => {
            let &target = res.targets.first()?;
            let creature = state
                .object(target)
                .filter(|o| o.zone == crate::zone::Zone::Battlefield)?;
            let creature_owner = creature.owner;
            let noted_counters: Vec<(baylee_cards_dsl::CounterKind, u16)> =
                creature.counters.iter().filter(|&(_, n)| n > 0).collect();
            let auras: Vec<(ObjectId, PlayerId)> = state
                .battlefield_seen()
                .filter_map(|id| {
                    let obj = state.object(id)?;
                    (obj.attached_to == Some(target)
                        && obj
                            .characteristics()
                            .subtypes
                            .contains(baylee_core::generated::subtypes::enchantment::AURA))
                    .then_some((id, obj.owner))
                })
                .collect();
            let mut objects = Vec::with_capacity(1 + auras.len());
            if let Some(obj) = state.object_mut(target) {
                obj.kind = ObjectKind::Card;
            }
            let _ = state.move_object(
                target,
                ZoneLocation::Exile(creature_owner),
                ZonePosition::Top,
                Cause::Effect,
            );
            if let Some(obj) = state
                .object_mut(target)
                .filter(|o| o.zone == crate::zone::Zone::Exile)
            {
                for (kind, n) in noted_counters {
                    obj.riders.push(crate::object::Rider::NotedCounter(kind, n));
                }
                objects.push(baylee_core::ids::DamageSourceRef {
                    object: target,
                    version: obj.version,
                });
            }
            for (aura, aura_owner) in auras {
                if state
                    .object(aura)
                    .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
                {
                    if let Some(obj) = state.object_mut(aura) {
                        obj.kind = ObjectKind::Card;
                    }
                    let _ = state.move_object(
                        aura,
                        ZoneLocation::Exile(aura_owner),
                        ZonePosition::Top,
                        Cause::Effect,
                    );
                    if let Some(obj) = state
                        .object(aura)
                        .filter(|o| o.zone == crate::zone::Zone::Exile)
                    {
                        objects.push(baylee_core::ids::DamageSourceRef {
                            object: aura,
                            version: obj.version,
                        });
                    }
                }
            }
            if !objects.is_empty()
                && let Some(source) = super::subjects::source(state, res)
                    .filter(|s| super::subjects::on_battlefield(state, *s))
            {
                let after = state.journal.last_seq();
                state.delayed.push(crate::state::DelayedTrigger {
                    controller: res.controller,
                    when: crate::state::DelayedWhen::LeavesBattlefieldOrUntaps {
                        card: source.object,
                        version: source.version,
                        after,
                    },
                    action: crate::state::DelayedAction::ReturnExiledGroup {
                        source,
                        objects,
                        tapped: true,
                    },
                });
            }
            None
        }
        Effect::ReturnTawnossCoffinExile => {
            let objects: Vec<baylee_core::ids::DamageSourceRef> = state
                .object(res.on_stack)
                .into_iter()
                .flat_map(|object| object.riders.iter())
                .filter_map(|rider| match rider {
                    crate::object::Rider::DelayedReturnObject(reference) => Some(*reference),
                    _ => None,
                })
                .collect();
            let &creature_ref = objects.first()?;
            let creature_obj = state.object(creature_ref.object).filter(|o| {
                o.zone == crate::zone::Zone::Exile && o.version == creature_ref.version
            })?;
            let creature_owner = creature_obj.owner;
            if state.has_left(creature_owner) {
                return None;
            }
            let noted: Vec<(baylee_cards_dsl::CounterKind, u16)> = creature_obj
                .riders
                .iter()
                .filter_map(|r| match *r {
                    crate::object::Rider::NotedCounter(kind, n) => Some((kind, n)),
                    _ => None,
                })
                .collect();
            if let Some(obj) = state.object_mut(creature_ref.object) {
                obj.set_controller(creature_owner);
                obj.kind = ObjectKind::Permanent;
            }
            state.set_tapped(creature_ref.object, true);
            let _ = state.move_object(
                creature_ref.object,
                ZoneLocation::Battlefield,
                ZonePosition::Top,
                Cause::Effect,
            );
            if !state
                .object(creature_ref.object)
                .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
            {
                return None;
            }
            for (kind, n) in noted {
                crate::replacement::put_counters_by(state, you, creature_ref.object, kind, n);
            }
            for &aura_ref in &objects[1..] {
                let Some(aura_obj) = state.object(aura_ref.object).filter(|o| {
                    o.zone == crate::zone::Zone::Exile && o.version == aura_ref.version
                }) else {
                    continue;
                };
                let aura_owner = aura_obj.owner;
                if state.has_left(aura_owner) {
                    continue;
                }
                if let Some(obj) = state.object_mut(aura_ref.object) {
                    obj.set_controller(aura_owner);
                }
                if !crate::aura_bindings::can_attach(state, aura_ref.object, creature_ref.object) {
                    continue;
                }
                if let Some(obj) = state.object_mut(aura_ref.object) {
                    obj.kind = ObjectKind::Permanent;
                    obj.attached_to = Some(creature_ref.object);
                }
                let _ = state.move_object(
                    aura_ref.object,
                    ZoneLocation::Battlefield,
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
        Effect::ExileAndReturnAtEndStep | Effect::ExileAndReturnTappedAtEndStep => {
            super::delayed_returns::exile_until_end(
                state,
                res,
                matches!(op, Effect::ExileAndReturnTappedAtEndStep),
            );
            None
        }
        Effect::ReturnExiledGroupToOwners { tapped } => {
            super::delayed_returns::return_exiled(state, res, tapped);
            None
        }
        Effect::ExileLibraryAndShuffleHand { player } => {
            for player in players_of(player, state, you, res) {
                let lib: Vec<ObjectId> = state.zones.list(ZoneLocation::Library(player)).clone();
                for card in lib {
                    let _ = state.move_object(
                        card,
                        ZoneLocation::Exile(player),
                        ZonePosition::Top,
                        Cause::Effect,
                    );
                }
                let hand: Vec<ObjectId> = state.zones.list(ZoneLocation::Hand(player)).clone();
                for card in hand {
                    let _ = state.move_object(
                        card,
                        ZoneLocation::Library(player),
                        ZonePosition::Bottom,
                        Cause::Effect,
                    );
                }
                state.shuffle_library(player);
            }
            None
        }
        Effect::Mill { amount, target } => super::milling::mill(state, res, amount, target),
        Effect::Destroy { target, no_regen } => {
            // Every target and not the first: see [`spec_objects`]. Heliod's
            // Intervention prints "destroy X target artifacts and/or
            // enchantments", and a `first()` reader destroyed one of them.
            // `res.targets` is already narrowed to the legal ones (CR 608.2b).
            for target_id in spec_objects(state, res, target) {
                if no_regen {
                    sba::destroy_no_regen(state, target_id);
                } else {
                    sba::destroy(state, target_id);
                }
            }
            None
        }
        // CR 701.19a: the shield is put on the permanent as this resolves,
        // and does nothing until something would destroy it. `spec_object`
        // answers both spellings from one field — `ThisObject` for
        // "regenerate this creature", the chosen target for "regenerate
        // target creature".
        //
        // Nothing checks that the permanent is a creature: "regenerate" is
        // written about permanents (CR 701.19a) and a shield on an animated
        // land that stops being one is simply a shield nothing spends.
        Effect::Regenerate { target } => {
            if matches!(target, TargetSpec::ThisObject)
                && !super::subjects::source(state, res)
                    .is_some_and(|r| super::subjects::on_battlefield(state, r))
            {
                return None;
            }
            if let Some(target_id) = spec_object(state, res, target)
                && let Some(obj) = state.object_mut(target_id)
            {
                obj.regeneration_shields = obj.regeneration_shields.saturating_add(1);
            }
            None
        }
        Effect::RegenerateAll { filter } => {
            let shielded: Vec<ObjectId> = state
                .battlefield_seen()
                .filter(|id| {
                    state.object(*id).is_some_and(|o| {
                        crate::eval::matches_with_context(filter, state, o, you, res.rule_context())
                    })
                })
                .collect();
            for id in shielded {
                if let Some(obj) = state.object_mut(id) {
                    obj.regeneration_shields = obj.regeneration_shields.saturating_add(1);
                }
            }
            None
        }
        Effect::DestroyChosenForPlayers { who, filter } => {
            let mut players = players_of(who, state, you, res);
            let (player, options) =
                chosen::next_asked(state, &mut players, filter, you, res.source)?;
            res.awaiting = Some(AwaitingOp::DestroyChosen {
                filter,
                remaining: players,
            });
            Some(Pending::ChooseCards {
                player,
                options,
                min: 0,
                max: 1,
                prompt: ChoicePrompt::Generic,
                total: None,
            })
        }
        Effect::SacrificeFilter { who, filter } => {
            let mut players = players_of(who, state, you, res);
            let (player, options) =
                chosen::next_asked(state, &mut players, filter, you, res.source)?;
            res.awaiting = Some(AwaitingOp::SacrificeFilter {
                filter,
                remaining: players,
            });
            Some(Pending::ChooseCards {
                player,
                options,
                min: 1,
                max: 1,
                prompt: ChoicePrompt::Generic,
                total: None,
            })
        }
        Effect::SacrificeOneThen {
            filter,
            if_it_was,
            then,
        } => {
            let options = chosen::options(state, you, filter, you, res.source);
            if options.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::SacrificeOneThen { if_it_was, then });
            Some(Pending::ChooseCards {
                player: you,
                options,
                min: 1,
                max: 1,
                prompt: ChoicePrompt::Generic,
                total: None,
            })
        }
        Effect::ReturnChosenToHand { who, filter } => {
            let mut players = players_of(who, state, you, res);
            // `min: 1` because the printed sentence is an instruction and
            // not an offer; a player with nothing that matches is skipped by
            // `next_asked` rather than shown an empty menu (CR 608.2d: as
            // much as possible, and no question that cannot be answered).
            let (player, options) =
                chosen::next_asked(state, &mut players, filter, you, res.source)?;
            res.awaiting = Some(AwaitingOp::ReturnChosen {
                filter,
                remaining: players,
            });
            Some(Pending::ChooseCards {
                player,
                options,
                min: 1,
                max: 1,
                prompt: ChoicePrompt::Generic,
                total: None,
            })
        }
        // "Choose a creature you control": a choice made as this resolves
        // (CR 608.2d), and one that must be made when it can — the sentence
        // is an instruction, so `min: 1`. Nothing to choose, nothing
        // happens (CR 609.3), and nobody is asked an empty question.
        Effect::ChooseYoursThen { filter, then } => {
            let options = chosen::options(state, you, filter, you, res.source);
            if options.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::ChooseYoursThen { then });
            Some(Pending::ChooseCards {
                player: you,
                options,
                min: 1,
                max: 1,
                prompt: ChoicePrompt::Generic,
                total: None,
            })
        }
        Effect::UntapChosen { filter, count } => {
            // "Up to", so `min: 0`, and only what is tapped is offered: an
            // untapped land is a legal pick that does nothing, and a list
            // with nothing tapped in it is no question at all (CR 608.2d).
            let options: Vec<ObjectId> = state
                .battlefield_seen()
                .filter(|id| {
                    state.object(*id).is_some_and(|o| {
                        o.status.contains(crate::object::Status::TAPPED)
                            && crate::eval::matches_with_context(
                                filter,
                                state,
                                o,
                                you,
                                res.rule_context(),
                            )
                    })
                })
                .collect();
            if options.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::UntapChosen);
            Some(Pending::ChooseCards {
                player: you,
                options,
                min: 0,
                max: count,
                prompt: ChoicePrompt::Generic,
                total: None,
            })
        }
        Effect::EqualizePermanents { filter } => equalize::start(state, res, Some(filter)),
        Effect::EqualizeHands => equalize::start(state, res, None),
        Effect::DiscardAmountForPlayers { who, amount } => {
            equalize::start_amount(state, res, who, None, &amount)
        }
        Effect::SacrificeAmountForPlayers {
            who,
            filter,
            amount,
        } => equalize::start_amount(state, res, who, Some(filter), &amount),
        Effect::DiscardForPlayers { who, count } => {
            discard::start_chain(state, res, who, count, false)
        }
        Effect::DiscardEachOrDraw { who } => discard::start_chain(state, res, who, 1, true),
        Effect::LookAtRandomFromHand { who, count } => {
            private_hand::inspect_random(state, res, who, count)
        }
        Effect::LookHandDiscard {
            player,
            count,
            filter,
        } => private_hand::choose_discard(state, res, player, count, filter),
        Effect::LookAtChosenHand => {
            let player = res.chosen_player?;
            let options = state.zones.list(ZoneLocation::Hand(player)).clone();
            // Only the deciding seat receives these through looking_at.
            // A public Revealed journal event would leak the whole hand.
            res.awaiting = Some(AwaitingOp::InspectHand);
            Some(Pending::ChooseCards {
                player: you,
                options,
                min: 0,
                max: 0,
                prompt: ChoicePrompt::LookAtHand,
                total: None,
            })
        }
        Effect::RevealHandDiscard { filter } => {
            let player = res.chosen_player?;
            let hand = state.zones.list(ZoneLocation::Hand(player)).clone();
            let options: Vec<ObjectId> = hand
                .iter()
                .copied()
                .filter(|id| {
                    state.object(*id).is_some_and(|obj| {
                        eval::matches_with_context(filter, state, obj, you, res.rule_context())
                    })
                })
                .collect();
            // Reveal *all* the cards, including those that cannot be selected.
            // The public log carries their identities to every seat; only the
            // controller receives the pending choice and can answer it.
            state.journal.record(GameEvent::Revealed {
                player,
                cards: hand,
            });
            if options.is_empty() {
                return None;
            }
            res.awaiting = Some(AwaitingOp::DiscardChain {
                player,
                count: 1,
                remaining: Vec::new(),
                selected: Vec::new(),
                draws: 0,
            });
            Some(Pending::ChooseCards {
                player: you,
                options,
                min: 1,
                max: 1,
                prompt: ChoicePrompt::Generic,
                total: None,
            })
        }
        Effect::RevealHand { who } => {
            for player in players_of(who, state, you, res) {
                let hand = state.zones.list(ZoneLocation::Hand(player)).clone();
                state.journal.record(GameEvent::Revealed {
                    player,
                    cards: hand,
                });
            }
            None
        }
        Effect::DiscardMatching { who, filter } => {
            let mut discards: Vec<(ObjectId, PlayerId)> = Vec::new();
            let context = res.rule_context();
            for player in players_of(who, state, you, res) {
                for &card in state.zones.list(ZoneLocation::Hand(player)) {
                    if state.object(card).is_some_and(|obj| {
                        eval::matches_with_context(filter, state, obj, you, context)
                    }) {
                        discards.push((card, player));
                    }
                }
            }
            super::discard_or_ask(state, res, &discards)
        }
        Effect::DiscardRandomMatching { who, filter, count } => {
            let count = amount2(&count, state, you, res) as usize;
            if count == 0 {
                return None;
            }
            let mut discards: Vec<(ObjectId, PlayerId)> = Vec::new();
            let context = res.rule_context();
            for player in players_of(who, state, you, res) {
                let mut matching: Vec<ObjectId> = state
                    .zones
                    .list(ZoneLocation::Hand(player))
                    .iter()
                    .copied()
                    .filter(|&card| {
                        state.object(card).is_some_and(|obj| {
                            eval::matches_with_context(filter, state, obj, you, context)
                        })
                    })
                    .collect();
                state.rng.shuffle(&mut matching);
                discards.extend(matching.into_iter().take(count).map(|card| (card, player)));
            }
            super::discard_or_ask(state, res, &discards)
        }
        Effect::DiscardHand { who } => {
            let mut discards: Vec<(ObjectId, PlayerId)> = Vec::new();
            for player in players_of(who, state, you, res) {
                discards.extend(
                    state
                        .zones
                        .list(ZoneLocation::Hand(player))
                        .iter()
                        .map(|&card| (card, player)),
                );
            }
            super::discard_or_ask(state, res, &discards)
        }
        Effect::ShuffleIntoLibrary {
            who,
            hand,
            graveyard,
        } => {
            let players = players_of(who, state, you, res);
            let mut moves: Vec<(ObjectId, ZoneLocation)> = Vec::new();
            for &player in &players {
                let zones = [
                    (hand, ZoneLocation::Hand(player)),
                    (graveyard, ZoneLocation::Graveyard(player)),
                ];
                for (_, zone) in zones.into_iter().filter(|(on, _)| *on) {
                    moves.extend(
                        state
                            .zones
                            .list(zone)
                            .iter()
                            .map(|&card| (card, ZoneLocation::Library(player))),
                    );
                }
            }
            // CR 903.9b, before anything moves: the last answer re-enters
            // this arm.
            if let Some(pending) = ask_commander_replace(state, res, &moves) {
                return Some(pending);
            }
            for (card, to) in moves {
                let _ = state.move_object(card, to, ZonePosition::Top, Cause::Effect);
            }
            for player in players {
                state.shuffle_library(player);
            }
            None
        }
        Effect::ShuffleLibrary { who } => {
            for player in players_of(who, state, you, res) {
                state.shuffle_library(player);
            }
            None
        }
        Effect::RandomGraveyardToHand { filter } => {
            let options: Vec<_> = state
                .zones
                .list(ZoneLocation::Graveyard(you))
                .iter()
                .copied()
                .filter(|id| {
                    state.object(*id).is_some_and(|object| {
                        eval::matches_with_context(filter, state, object, you, res.rule_context())
                    })
                })
                .collect();
            if !options.is_empty() {
                let chosen = options[state.rng.below(options.len() as u64) as usize];
                return super::graveyard_choice::place(
                    state,
                    res,
                    &[chosen],
                    SearchDest::Hand,
                    false,
                );
            }
            None
        }
        Effect::DiscardRandom { who, count } => {
            let count = amount2(&count, state, you, res) as usize;
            if count == 0 {
                return None;
            }
            // Every card is drawn at random first: Library of Leng's
            // question names them, and the answer discards exactly these
            // rather than drawing again.
            let mut discards: Vec<(ObjectId, PlayerId)> = Vec::new();
            for player in players_of(who, state, you, res) {
                let mut hand = state.zones.list(ZoneLocation::Hand(player)).clone();
                state.rng.shuffle(&mut hand);
                discards.extend(hand.into_iter().take(count).map(|card| (card, player)));
            }
            super::discard_or_ask(state, res, &discards)
        }
        Effect::AllGraveyardCreaturesToBattlefield => {
            // Under `you`'s control, so nowhere once they have left
            // (CR 800.4b).
            if state.has_left(you) {
                return None;
            }
            for seat in 0..state.players.len() {
                let p = PlayerId::new(seat as u8);
                for &card in &state.zones.list(ZoneLocation::Graveyard(p)).clone() {
                    let is_creature = state.object(card).is_some_and(|o| {
                        o.characteristics()
                            .types
                            .contains(baylee_core::types::TypeSet::CREATURE)
                    });
                    if is_creature {
                        let _ = state.move_object(
                            card,
                            ZoneLocation::Battlefield,
                            ZonePosition::Top,
                            Cause::Effect,
                        );
                        if let Some(obj) = state.object_mut(card) {
                            obj.set_controller(you);
                        }
                    }
                }
            }
            None
        }
        Effect::YourGraveyardToBattlefield { filter, tapped } => {
            // Your cards, under your control: nowhere once you have left
            // the game, and your cards left with you (CR 800.4a).
            if state.has_left(you) {
                return None;
            }
            let cards: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Graveyard(you))
                .iter()
                .copied()
                .filter(|id| {
                    state.object(*id).is_some_and(|o| {
                        eval::matches_with_context(filter, state, o, you, res.rule_context())
                    })
                })
                .collect();
            for card in cards {
                if let Some(obj) = state.object_mut(card) {
                    obj.kind = ObjectKind::Permanent;
                    obj.set_controller(you);
                }
                // Before the move, as a search's tapped find does it.
                if tapped {
                    state.set_tapped(card, true);
                }
                let _ = state.move_object(
                    card,
                    ZoneLocation::Battlefield,
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
        Effect::ReturnToBattlefieldTapped { target } => {
            // "Under your control": nowhere once you have left the game
            // (CR 800.4b).
            if state.has_left(you) {
                return None;
            }
            for card in moving_objects(state, res, target) {
                // Where the trigger event put it, or nowhere (CR 603.7c).
                // The zone is asked and not the object's version, which a
                // synthetic trigger does not carry: a card moved from the
                // graveyard into exile in response would come back from
                // exile. No card in the pool does that to its own lands.
                if !state.object(card).is_some_and(|o| {
                    matches!(
                        o.zone,
                        crate::zone::Zone::Graveyard | crate::zone::Zone::Exile
                    )
                }) {
                    continue;
                }
                if let Some(obj) = state.object_mut(card) {
                    obj.kind = ObjectKind::Permanent;
                    obj.set_controller(you);
                }
                // Before the move, as World Shaper's lands and a search's
                // tapped find do it.
                state.set_tapped(card, true);
                let _ = state.move_object(
                    card,
                    ZoneLocation::Battlefield,
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
        // CR 701.27a: turned over where it stands, queued for the engine,
        // which holds the card registry `state.transform` needs and applies
        // it as this resolution completes. A source that has left the
        // battlefield, or is phased out, is no permanent to transform.
        //
        // CR 701.27f: an ability of the permanent transforms it only if it
        // has not transformed since the ability was put on the stack. The
        // ability carries the face it was printed on (CR 113.7a), so a
        // source showing another face has transformed since and the
        // instruction is ignored: two lands entering at once under Twists and
        // Turns trigger it twice, and the second must not turn it back. (Two
        // transforms in between would show the same face again and are not
        // told apart; nothing in the pool reaches that.)
        Effect::TransformSource => {
            let printed = state
                .object(res.on_stack)
                .and_then(|ability| ability.own_origin)
                .and_then(crate::object::AbilityOrigin::printed)
                .map(crate::object::PrintedFace::face);
            if let Some(obj) = state.object_mut(res.source)
                && obj.zone == crate::zone::Zone::Battlefield
                && !obj.status.contains(crate::object::Status::PHASED_OUT)
                && printed.is_none_or(|face| face == obj.face_index)
            {
                obj.pending_face_change = Some(1 - obj.face_index.min(1));
            }
            None
        }
        // The delayed trigger remembers the object and the face it showed
        // as it was created: CR 701.27f ignores the instruction once the
        // permanent has transformed since, and CR 400.7 once it has left.
        Effect::TransformSourceAtNextUpkeep => {
            if let Some(obj) = state.object(res.source)
                && obj.zone == crate::zone::Zone::Battlefield
            {
                let action = crate::state::DelayedAction::Transform {
                    card: res.source,
                    version: obj.version,
                    face: obj.face_index,
                };
                state.delayed.push(crate::state::DelayedTrigger {
                    controller: you,
                    when: crate::state::DelayedWhen::NextUpkeepOfAnyone,
                    action,
                });
            }
            None
        }
        Effect::ReturnLinkedToBattlefield => {
            // Everything exiled with a link to the source, under its owner's
            // control: Endless Sands and Safe Haven print it. The same way
            // back as the "until" exiles take (`GameState::return_linked`).
            let source = res.source;
            state.return_linked(|_, host, _| host == source);
            None
        }
        Effect::ExileTargetsCreateTokens { token } => {
            let targets = res.targets.clone();
            for target_id in targets {
                let Some(obj) = state.object(target_id) else {
                    continue;
                };
                let owner = obj.owner;
                let controller = obj.controller;
                if let Some(obj) = state.object_mut(target_id) {
                    obj.kind = ObjectKind::Card;
                }
                let _ = state.move_object(
                    target_id,
                    ZoneLocation::Exile(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
                // The token goes to the exiled permanent's controller, so
                // CR 614.1 is read off them and not off whoever cast this.
                // The comment here used to say so while the call underneath
                // it consulted no replacement at all.
                tokens::create_tokens(state, controller, token, None, 1);
            }
            None
        }
        Effect::CounterTargetAbility => {
            if let Some(&target_id) = res.targets.first() {
                counter_ability(state, res, target_id);
            }
            None
        }
        Effect::CounterTargetSpellOrAbility => {
            if let Some(&target_id) = res.targets.first()
                && state
                    .object(target_id)
                    .is_none_or(crate::object::GameObject::can_be_countered)
            {
                let kind = state.object(target_id).map(|o| o.kind);
                if kind == Some(ObjectKind::AbilityOnStack) {
                    state
                        .journal
                        .record(GameEvent::SpellCountered { object: target_id });
                    state.remember_damage_source(target_id);
                    state.zones.remove(target_id, ZoneLocation::Stack);
                    let _ = state.arena.remove(target_id);
                } else {
                    state
                        .journal
                        .record(GameEvent::SpellCountered { object: target_id });
                    let owner = state.object(target_id).map_or(you, |o| o.owner);
                    state.remember_damage_source(target_id);
                    if let Some(obj) = state.object_mut(target_id) {
                        obj.kind = ObjectKind::Card;
                    }
                    let _ = state.move_object(
                        target_id,
                        ZoneLocation::Graveyard(owner),
                        ZonePosition::Top,
                        Cause::Effect,
                    );
                }
            }
            None
        }
        Effect::CounterTargetSpellToExile => {
            if let Some(&target_id) = res.targets.first()
                && state
                    .object(target_id)
                    .is_none_or(crate::object::GameObject::can_be_countered)
            {
                state
                    .journal
                    .record(GameEvent::SpellCountered { object: target_id });
                let owner = state.object(target_id).map_or(you, |o| o.owner);
                state.remember_damage_source(target_id);
                if let Some(obj) = state.object_mut(target_id) {
                    obj.kind = ObjectKind::Card;
                }
                let _ = state.move_object(
                    target_id,
                    ZoneLocation::Exile(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
        Effect::CounterTargetSpell => {
            if let Some(&target_id) = res.targets.first()
                && state
                    .object(target_id)
                    .is_none_or(crate::object::GameObject::can_be_countered)
            {
                state
                    .journal
                    .record(GameEvent::SpellCountered { object: target_id });
                let owner = state.object(target_id).map_or(you, |o| o.owner);
                state.remember_damage_source(target_id);
                if let Some(obj) = state.object_mut(target_id) {
                    obj.kind = ObjectKind::Card;
                }
                let _ = state.move_object(
                    target_id,
                    ZoneLocation::Graveyard(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
        _ => unreachable!("not a zone effect"),
    }
}

/// Shared countering door; the caller resolves targeting or event identity first.
pub(super) fn counter_ability(state: &mut GameState, res: &mut Resolution, target: ObjectId) {
    let Some(object) = state.object(target).filter(|object| {
        object.zone == crate::zone::Zone::Stack
            && object.kind == ObjectKind::AbilityOnStack
            && object.can_be_countered()
    }) else {
        return;
    };
    res.countered_source = object.ability.map(|ability| ability.source);
    state
        .journal
        .record(GameEvent::SpellCountered { object: target });
    state.remember_damage_source(target);
    state.zones.remove(target, ZoneLocation::Stack);
    let _ = state.arena.remove(target);
}

/// Who a card put onto the battlefield by an effect comes back under: the
/// controller its sentence names, and only the controller.
///
/// Every case starts from a stolen creature — owned by seat 1 and controlled
/// by seat 0 through a real layer-2 `GainControl` effect — because that is
/// the one board where "under your control" and "under its owner's control"
/// differ, and so the only board that can tell the two apart. The engine
/// used to return every blinked card to its owner, citing CR 610.3c, which
/// is about a card coming back after an "until" event and not about an
/// immediate return. Restoration Angel then handed the creature it flickered
/// back to the opponent it had been stolen from.
///
/// The second half is the arrivals that wrote no controller at all and so
/// inherited whatever default the card last had on the battlefield. Once a
/// blink can leave a card's default with a player who does not own it, a
/// return "under its owner's control" gave the card to that player instead.
#[cfg(test)]
mod arrival_control_tests;
