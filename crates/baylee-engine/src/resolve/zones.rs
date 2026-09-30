//! Zone movements: exile, blink, bounce, destruction, sacrifice,
//! graveyard recursion, mill — and countering spells/abilities (their
//! target's journey from the stack to another zone).

#[allow(clippy::wildcard_imports)] // family modules share the resolve vocabulary
use super::*;

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
    state.object(res.source).is_some_and(|o| {
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
pub(super) fn spec_object(res: &Resolution, target: TargetSpec) -> Option<ObjectId> {
    match target {
        TargetSpec::ThisObject => Some(res.source),
        TargetSpec::EventObject => res.event_object,
        // The one spec that is only ever a second instance of "target"
        // names the object that instance chose.
        TargetSpec::ObjectOfFirstTargetsPlayer(_) => res.second_targets.first().copied(),
        TargetSpec::Object(_)
        | TargetSpec::ObjectOfEachOpponent(_)
        | TargetSpec::OpponentOrObject(_)
        | TargetSpec::ObjectControlledBy(..)
        | TargetSpec::ObjectOfEventPlayer(_)
        | TargetSpec::Spell(_)
        | TargetSpec::StackOrBattlefield(_)
        | TargetSpec::CardInGraveyard(..)
        | TargetSpec::CardInGraveyardBelowEvent(..)
        | TargetSpec::CardInGraveyardBelowValue(..)
        | TargetSpec::AbilityOnStack(_)
        | TargetSpec::SpellOrAbility(_)
        | TargetSpec::Player(_)
        | TargetSpec::AnyPlayer
        | TargetSpec::AnyOpponent
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
pub(super) fn spec_objects(res: &Resolution, target: TargetSpec) -> SmallVec<[ObjectId; 2]> {
    match target {
        TargetSpec::ThisObject
        | TargetSpec::EventObject
        | TargetSpec::ObjectOfFirstTargetsPlayer(_) => {
            spec_object(res, target).into_iter().collect()
        }
        _ => res.targets.clone(),
    }
}

/// Executes one zone-movement effect.
#[allow(clippy::too_many_lines)] // the zone vocabulary is one flat table
pub(super) fn exec(state: &mut GameState, res: &mut Resolution, op: Effect) -> Option<Pending> {
    let you = res.controller;
    match op {
        Effect::Exile { .. } => {
            // Every target and not the first. A card may name more than one
            // — Pit of Offerings prints "exile up to three target cards from
            // graveyards" and carries `TargetReq::up_to(.., 3)` — and a
            // reader that took `first()` exiled one of them and left the
            // rest where they were, with the card still claiming
            // `Coverage::Implemented`. A single-target exile has one entry
            // in `res.targets`, so nothing else in the pool moves.
            for target_id in res.targets.clone() {
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
                // "Under your control" when `you` has left the game: the card
                // stays in exile (CR 800.4b). Under its owner's it comes
                // back, since an owner who had left would have taken the
                // card with them (CR 800.4a).
                if !owner_control && state.has_left(you) {
                    return None;
                }
                if let Some(obj) = state.object_mut(target_id) {
                    obj.kind = ObjectKind::Permanent;
                    // The card that comes back is a new object (CR 400.7):
                    // whatever control effect held the one that left named
                    // that object and reaches this one no more. So it enters
                    // under the control the sentence names, written here
                    // where it arrives, and as its default rather than as a
                    // layer-2 effect: "under its owner's control"
                    // (Ephemerate) or "under your control" (Restoration
                    // Angel), the latter being the resolving ability's
                    // controller (CR 110.2a). Not CR 610.3c, which is about
                    // a card returned after an "until" event and does not
                    // reach an immediate return. Only control is chosen: the
                    // owner stays who it was (CR 108.3).
                    obj.set_controller(if owner_control { owner } else { you });
                }
                let _ = state.move_object(
                    target_id,
                    ZoneLocation::Battlefield,
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
        Effect::ReturnToHand { target } => {
            if let Some(target_id) = spec_object(res, target) {
                let owner = state.object(target_id).map_or(you, |o| o.owner);
                // CR 903.9b, before the kind flips below: this operation
                // re-runs from the top once every owner has answered.
                if let Some(pending) =
                    ask_commander_replace(state, res, &[(target_id, ZoneLocation::Hand(owner))])
                {
                    return Some(pending);
                }
                if let Some(obj) = state.object_mut(target_id) {
                    obj.kind = ObjectKind::Card;
                }
                let _ = state.move_object(
                    target_id,
                    ZoneLocation::Hand(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
        Effect::ReturnAllToHand {
            filter,
            opponents_only,
        } => {
            let all: Vec<ObjectId> = state
                .battlefield_seen()
                .filter(|id| {
                    state.object(*id).is_some_and(|o| {
                        (!opponents_only || state.is_opponent(o.controller, you))
                            && eval::matches(filter, state, o, you, res.source)
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
        Effect::DestroyAll { filter, no_regen } => {
            // Not a phased-out creature: "Destroy all creatures" passes over
            // one (CR 702.26b's example).
            let all: Vec<ObjectId> = state
                .battlefield_seen()
                .filter(|id| {
                    state
                        .object(*id)
                        .is_some_and(|o| eval::matches(filter, state, o, you, res.source))
                })
                .collect();
            for id in all {
                if no_regen {
                    sba::destroy_no_regen(state, id);
                } else {
                    sba::destroy(state, id);
                }
            }
            None
        }
        // Farewell's four sweeps. Nothing is targeted, and a phased-out
        // permanent is treated as though it doesn't exist (CR 702.26b),
        // which `battlefield_seen` is.
        Effect::ExileAll { filter } => {
            let all: Vec<ObjectId> = state
                .battlefield_seen()
                .filter(|id| {
                    state
                        .object(*id)
                        .is_some_and(|o| eval::matches(filter, state, o, you, res.source))
                })
                .collect();
            for id in all {
                let Some(owner) = state.object(id).map(|o| o.owner) else {
                    continue;
                };
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
            let named = spec_object(res, target)?;
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
        Effect::GraveyardToHand { .. } => {
            if let Some(&target_id) = res.targets.first() {
                let owner = state.object(target_id).map_or(you, |o| o.owner);
                // CR 903.9b. A commander is in a graveyard to be returned
                // only because its owner declined 903.9a, and this is a
                // different question about a different destination.
                if let Some(pending) =
                    ask_commander_replace(state, res, &[(target_id, ZoneLocation::Hand(owner))])
                {
                    return Some(pending);
                }
                let _ = state.move_object(
                    target_id,
                    ZoneLocation::Hand(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
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
                    state
                        .object(*id)
                        .is_some_and(|o| eval::matches(filter, state, o, you, res.source))
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
        Effect::GraveyardToTop { .. } => {
            if let Some(&target_id) = res.targets.first() {
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
        Effect::GraveyardToBattlefield {
            target,
            owner_control,
            counters,
        } => {
            // Every target and not the first: see [`spec_objects`].
            //
            // CR 400.7: a card that has moved is a new object with no
            // relation to the one the effect named. A reanimation *spell* is
            // held to that by target legality (CR 608.2b), so the zone check
            // looks redundant beside the fifteen cards that cast one — but
            // undying and persist target nothing at all
            // (`TargetSpec::EventObject` is the creature that died), and
            // without it a card exiled in response to the keyword trigger
            // came back onto the battlefield out of exile.
            // "Under your control" when `you` has left the game: the card
            // stays where it is (CR 800.4b). Under its owner's it goes, since
            // an owner who had left would have taken the card with them.
            if !owner_control && state.has_left(you) {
                return None;
            }
            for target_id in spec_objects(res, target) {
                // The card has to still be in a graveyard, asked per card:
                // one of several targets leaving does not stop the others.
                if !state
                    .object(target_id)
                    .is_some_and(|o| o.zone == crate::zone::Zone::Graveyard)
                {
                    continue;
                }
                if let Some(obj) = state.object_mut(target_id) {
                    obj.kind = ObjectKind::Permanent;
                    let to = if owner_control { obj.owner } else { you };
                    obj.set_controller(to);
                }
                let _ = state.move_object(
                    target_id,
                    ZoneLocation::Battlefield,
                    ZonePosition::Top,
                    Cause::Effect,
                );
                // After the move and not before it: `move_object` clears
                // every field only a permanent can have, counters among
                // them, so a counter written first would be wiped on the
                // way in. `put_counters` and not `Counters::add`, because
                // "returns with a counter on it" is a counter an effect
                // puts on a permanent and a doubler has its say
                // (CR 614.16) — the same door `EnterModifier::WithCounters`
                // goes through.
                if let Some((kind, n)) = counters
                    && n > 0
                    && state
                        .object(target_id)
                        .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
                {
                    crate::replacement::put_counters(state, target_id, kind, n);
                }
            }
            None
        }
        Effect::PutSourceOnTopOfLibrary => {
            let owner = state.object(res.source).map_or(you, |o| o.owner);
            // CR 903.9b: a commander that puts *itself* on top is still
            // being put into a library, and its owner still chooses.
            if let Some(pending) =
                ask_commander_replace(state, res, &[(res.source, ZoneLocation::Library(owner))])
            {
                return Some(pending);
            }
            let _ = state.move_object(
                res.source,
                ZoneLocation::Library(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
            None
        }
        Effect::BottomCardFromHand { player, filter } => {
            let player = players_of(player, state, you, res).first().copied()?;
            let options: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Hand(player))
                .iter()
                .filter(|id| {
                    state
                        .object(**id)
                        .is_some_and(|o| eval::matches(filter, state, o, player, res.source))
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
            res.awaiting = Some(AwaitingOp::BottomFromHand { player });
            Some(Pending::ChooseCards {
                player: you,
                options,
                min: 0,
                max: 1,
                prompt: ChoicePrompt::Generic,
                total: None,
            })
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
            if let Some(&target_id) = res.targets.first() {
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
            }
            None
        }
        // Every target, each exiled with the source (CR 406.6) as the
        // object it is now.
        Effect::ExileTargetsWithSource => {
            let version = state
                .object(res.source)
                .map_or(0, crate::object::Rider::version_of);
            for &target in &res.targets {
                let owner = state.object(target).map_or(you, |o| o.owner);
                if let Some(obj) = state.object_mut(target) {
                    obj.kind = ObjectKind::Card;
                    obj.riders.push(crate::object::Rider::ExiledWith {
                        host: res.source,
                        version,
                    });
                }
                let _ = state.move_object(
                    target,
                    ZoneLocation::Exile(owner),
                    ZonePosition::Top,
                    Cause::Effect,
                );
            }
            None
        }
        Effect::SacrificeSelf => {
            if !can_sacrifice_self(state, res) {
                return None;
            }
            let owner = state.object(res.source).map_or(you, |o| o.owner);
            if let Some(obj) = state.object_mut(res.source) {
                obj.kind = ObjectKind::Card;
            }
            let _ = state.move_object(
                res.source,
                ZoneLocation::Graveyard(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
            None
        }
        // "Sacrifice that creature": by the ability's controller, and only a
        // permanent of theirs that is still the object the spec names
        // (CR 701.21a; an event object that has left is none, CR 603.7c).
        Effect::SacrificeObject { target } => {
            let id = spec_object(res, target)?;
            let owner = state.object(id).and_then(|o| {
                (o.zone == crate::zone::Zone::Battlefield
                    && o.controller == res.controller
                    && !o.status.contains(crate::object::Status::PHASED_OUT))
                .then_some(o.owner)
            })?;
            if let Some(obj) = state.object_mut(id) {
                obj.kind = ObjectKind::Card;
            }
            let _ = state.move_object(
                id,
                ZoneLocation::Graveyard(owner),
                ZonePosition::Top,
                Cause::Effect,
            );
            None
        }
        Effect::PutTargetOnBottomOfLibrary => {
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
                let _ = state.move_object(target, to, ZonePosition::Bottom, Cause::Effect);
            }
            None
        }
        Effect::PutOnBottomOfLibraryFromGraveyard { target } => {
            // CR 400.7, as `GraveyardToBattlefield` asks it: a card that left
            // the graveyard in response is a new object, and "it" is gone.
            let moves: Vec<(ObjectId, ZoneLocation)> = spec_objects(res, target)
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
        Effect::ExileAndReturnAtEndStep => {
            for &target in &res.targets {
                let owner = state.object(target).map_or(you, |o| o.owner);
                let _ = state.move_object(
                    target,
                    ZoneLocation::Exile(owner),
                    ZonePosition::Top,
                    crate::event::Cause::Effect,
                );
                state.delayed.push(crate::state::DelayedTrigger {
                    controller: you,
                    when: crate::state::DelayedWhen::NextEndStep,
                    action: crate::state::DelayedAction::ReturnToBattlefield { card: target },
                });
            }
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
        Effect::Mill { amount, target } => {
            let n = amount2(&amount, state, you, res) as usize;
            for player in super::players_of(target, state, you, res) {
                let top: Vec<ObjectId> = state
                    .zones
                    .list(ZoneLocation::Library(player))
                    .iter()
                    .rev()
                    .take(n)
                    .copied()
                    .collect();
                for card in top {
                    let _ = state.move_object(
                        card,
                        ZoneLocation::Graveyard(player),
                        ZonePosition::Top,
                        Cause::Effect,
                    );
                }
            }
            None
        }
        Effect::Destroy { target, no_regen } => {
            // Every target and not the first: see [`spec_objects`]. Heliod's
            // Intervention prints "destroy X target artifacts and/or
            // enchantments", and a `first()` reader destroyed one of them.
            // `res.targets` is already narrowed to the legal ones (CR 608.2b).
            for target_id in spec_objects(res, target) {
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
            if let Some(target_id) = spec_object(res, target)
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
                    state
                        .object(*id)
                        .is_some_and(|o| crate::eval::matches(filter, state, o, you, res.source))
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
                            && crate::eval::matches(filter, state, o, you, res.source)
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
        Effect::DiscardForPlayers { who, count } => {
            let players = players_of(who, state, you, res);
            let mut remaining: Vec<PlayerId> = players
                .iter()
                .copied()
                .filter(|p| !state.zones.list(ZoneLocation::Hand(*p)).is_empty())
                .collect();
            let player = remaining.first().copied()?;
            remaining.remove(0);
            let hand: Vec<ObjectId> = state.zones.list(ZoneLocation::Hand(player)).clone();
            let n = (count as usize).min(hand.len()) as u8;
            res.awaiting = Some(AwaitingOp::DiscardChain {
                player,
                count,
                remaining,
            });
            Some(Pending::ChooseCards {
                player,
                options: hand,
                min: n,
                max: n,
                prompt: ChoicePrompt::Generic,
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
                    state
                        .object(*id)
                        .is_some_and(|obj| eval::matches(filter, state, obj, you, res.source))
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
        Effect::DiscardHand { who } => {
            for player in players_of(who, state, you, res) {
                let hand = state.zones.list(ZoneLocation::Hand(player)).clone();
                for card in hand {
                    state.journal.record(GameEvent::Discarded {
                        object: card,
                        player,
                    });
                    let _ = state.move_object(
                        card,
                        ZoneLocation::Graveyard(player),
                        ZonePosition::Top,
                        Cause::Effect,
                    );
                }
            }
            None
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
        Effect::DiscardRandom { who, count } => {
            let count = amount2(&count, state, you, res) as usize;
            if count == 0 {
                return None;
            }
            for player in players_of(who, state, you, res) {
                let mut hand = state.zones.list(ZoneLocation::Hand(player)).clone();
                state.rng.shuffle(&mut hand);
                for card in hand.into_iter().take(count) {
                    state.journal.record(GameEvent::Discarded {
                        object: card,
                        player,
                    });
                    let _ = state.move_object(
                        card,
                        ZoneLocation::Graveyard(player),
                        ZonePosition::Top,
                        Cause::Effect,
                    );
                }
            }
            None
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
                    state
                        .object(*id)
                        .is_some_and(|o| eval::matches(filter, state, o, you, res.source))
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
            for card in spec_objects(res, target) {
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
                .and_then(|ability| ability.own_face)
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
        Effect::ExileSelfReturnAsFace {
            face,
            owner_control,
        } => {
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
            // Blink's arrival, for the same reasons: nothing is put onto the
            // battlefield under a player who has left (CR 800.4b), and the
            // card that comes back is a new object (CR 400.7) that enters
            // under the control the sentence names, as its default and
            // written before it arrives. It always returned under its owner,
            // so a stolen Fable of the Mirror-Breaker went home at chapter
            // III where the card says "under your control".
            if !owner_control && state.has_left(you) {
                return None;
            }
            if let Some(obj) = state.object_mut(res.source) {
                obj.kind = ObjectKind::Permanent;
                obj.set_controller(if owner_control { owner } else { you });
            }
            let _ = state.move_object(
                res.source,
                ZoneLocation::Battlefield,
                ZonePosition::Top,
                Cause::Effect,
            );
            if let Some(obj) = state.object_mut(res.source) {
                // The face switch needs the card definition (lookup);
                // finish_resolution applies it.
                obj.pending_face_change = Some(face);
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
                state
                    .journal
                    .record(GameEvent::SpellCountered { object: target_id });
                // Whose ability it was, written down before the object it
                // is written on stops existing. Nothing else in this
                // resolution can answer that afterwards — see
                // [`Resolution::countered_source`].
                res.countered_source = state
                    .object(target_id)
                    .and_then(|o| o.ability.map(|a| a.source));
                // Abilities on the stack cease to exist when countered.
                state.zones.remove(target_id, ZoneLocation::Stack);
                let _ = state.arena.remove(target_id);
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
                    state.zones.remove(target_id, ZoneLocation::Stack);
                    let _ = state.arena.remove(target_id);
                } else {
                    state
                        .journal
                        .record(GameEvent::SpellCountered { object: target_id });
                    let owner = state.object(target_id).map_or(you, |o| o.owner);
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
mod arrival_control_tests {
    use super::*;
    use crate::engine::synthetic::{SyntheticLookup, preset};
    use baylee_cards_dsl::Filter;

    fn me() -> PlayerId {
        PlayerId::new(0)
    }

    fn them() -> PlayerId {
        PlayerId::new(1)
    }

    /// A two-seat game with a creature seat 1 owns on the battlefield.
    fn theirs(seed: u64) -> (GameState, ObjectId) {
        let mut state = GameState::from_preset(&preset(seed, &[]), &SyntheticLookup::new(vec![]))
            .expect("a two-seat game");
        let name = state.names.intern("Creature");
        let it = state.create_bare(
            them(),
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        let obj = state.object_mut(it).expect("fresh");
        let mut base = (*obj.base).clone();
        base.types = baylee_core::types::TypeSet::CREATURE;
        obj.base = std::sync::Arc::new(base);
        state.invalidate_projections();
        (state, it)
    }

    /// A creature seat 1 owns and seat 0 has stolen.
    fn stolen() -> (GameState, ObjectId) {
        let (mut state, it) = theirs(610);
        crate::resolve::gain_control(&mut state, &[(it, me())]);
        let obj = state.object(it).expect("still there");
        assert_eq!(
            (obj.owner, obj.controller, obj.base_controller),
            (them(), me(), them()),
            "the steal is a layer-2 effect over seat 1's own default"
        );
        (state, it)
    }

    /// Seat 0 blinks `it`: an ability of seat 0's, resolving.
    fn blink(state: &mut GameState, it: ObjectId, owner_control: bool) {
        let effect = Effect::Blink {
            target: TargetSpec::Object(&Filter::CREATURE),
            owner_control,
        };
        resolve(state, ObjectId::NO_SOURCE, &[it], effect);
    }

    /// An ability of seat 0's with `source` and `targets`, resolving.
    fn resolve(state: &mut GameState, source: ObjectId, targets: &[ObjectId], effect: Effect) {
        resolve_from(state, source, ObjectId::NO_SOURCE, targets, effect);
    }

    /// [`resolve`], for the ability `on_stack` stands for.
    fn resolve_from(
        state: &mut GameState,
        source: ObjectId,
        on_stack: ObjectId,
        targets: &[ObjectId],
        effect: Effect,
    ) {
        let mut res = Resolution {
            source,
            on_stack,
            controller: me(),
            effects: vec![effect],
            pc: 0,
            targets: SmallVec::from_slice(targets),
            second_targets: SmallVec::new(),
            x: None,
            chosen_player: None,
            target_lki: None,
            retarget_left: None,
            target_players: baylee_core::ids::SeatSet::new(),
            event_object: None,
            awaiting: None,
            targeted: true,
            mana_ability: false,
            countered_source: None,
        };
        assert!(matches!(run(state, &mut res), Flow::Complete));
        state.refresh_characteristics();
    }

    /// "…then return that card to the battlefield under your control"
    /// (Restoration Angel): the new object enters under seat 0's control
    /// (CR 110.2a, CR 400.7) and stays there with no control effect holding
    /// it, while seat 1 still owns it (CR 108.3).
    #[test]
    fn under_your_control_a_stolen_creature_comes_back_yours_and_still_theirs() {
        let (mut state, it) = stolen();
        blink(&mut state, it, false);
        let obj = state.object(it).expect("the same arena handle");
        assert_eq!(obj.zone, crate::zone::Zone::Battlefield, "it came back");
        assert_eq!(
            (obj.controller, obj.base_controller),
            (me(), me()),
            "entered under the blinking player's control, by default and not by an effect"
        );
        assert_eq!(obj.owner, them(), "ownership never changes");

        // The steal effect named the old object (CR 400.7), and ending it
        // hands nothing back: seat 0's control is the new object's own.
        state
            .effects
            .remove_where(|fx| fx.modifier == baylee_cards_dsl::Modifier::GainControl);
        state.refresh_characteristics();
        assert_eq!(state.object(it).map(|o| o.controller), Some(me()));

        // "You own" still reads the owner.
        let obj = state.object(it).expect("still there");
        assert!(!eval::matches(&Filter::OwnedByYou, &state, obj, me(), it));
        assert!(eval::matches(&Filter::OwnedByYou, &state, obj, them(), it));

        // And it dies into its owner's graveyard (CR 400.3).
        sba::destroy(&mut state, it);
        assert_eq!(state.zones.list(ZoneLocation::Graveyard(them()))[..], [it]);
        assert!(state.zones.list(ZoneLocation::Graveyard(me())).is_empty());
    }

    /// "…under its owner's control" (Ephemerate): the other sentence, on the
    /// same board, sends the creature home.
    #[test]
    fn under_its_owners_control_a_stolen_creature_goes_home() {
        let (mut state, it) = stolen();
        blink(&mut state, it, true);
        let obj = state.object(it).expect("the same arena handle");
        assert_eq!(obj.zone, crate::zone::Zone::Battlefield, "it came back");
        assert_eq!(
            (obj.owner, obj.controller, obj.base_controller),
            (them(), them(), them())
        );
    }

    /// A creature kept this way leaves the game with its owner (CR 800.4a:
    /// "all objects … owned by that player leave the game").
    #[test]
    fn a_creature_kept_by_a_blink_leaves_the_game_with_its_owner() {
        let (mut state, it) = stolen();
        blink(&mut state, it, false);
        assert_eq!(state.object(it).map(|o| o.controller), Some(me()));
        crate::sba::eliminate_player(&mut state, them(), crate::event::LossReason::Conceded);
        assert!(state.object(it).is_none());
        assert!(!state.zones.list(ZoneLocation::Battlefield).contains(&it));
    }

    /// Nothing is put onto the battlefield under the control of a player
    /// who has left (CR 800.4b): a blink "under your control" resolving for
    /// one exiles its target and leaves it in exile. Under its owner's it
    /// comes back, since its owner is still in the game.
    #[test]
    fn a_blink_under_your_control_returns_nothing_to_a_player_who_has_left() {
        for (owner_control, back) in [(false, false), (true, true)] {
            let (mut state, it) = theirs(611);
            crate::sba::eliminate_player(&mut state, me(), crate::event::LossReason::Conceded);
            blink(&mut state, it, owner_control);
            let obj = state.object(it).expect("seat 1's own card");
            let want = if back {
                crate::zone::Zone::Battlefield
            } else {
                crate::zone::Zone::Exile
            };
            assert_eq!(obj.zone, want, "owner_control: {owner_control}");
        }
    }

    /// Seat 0's ability of `it`'s own exiles `it` and returns it
    /// (`ExileSelfReturnAsFace`): Fable of the Mirror-Breaker III, Sheoldred's
    /// `{4}{B}`, a transform's stand-in.
    fn self_return(state: &mut GameState, it: ObjectId, owner_control: bool) {
        let effect = Effect::ExileSelfReturnAsFace {
            face: 0,
            owner_control,
        };
        resolve(state, it, &[], effect);
    }

    /// "Exile this Saga, then return it to the battlefield transformed under
    /// your control" on a permanent seat 0 has stolen: the new object enters
    /// under seat 0's control by default, not an effect's (CR 110.2a,
    /// CR 400.7), and stays with seat 0 once the steal is gone, while seat 1
    /// still owns it (CR 108.3). The effect returned every card under its
    /// owner's control, so the thief's Fable went home at chapter III.
    #[test]
    fn a_stolen_permanent_returning_itself_under_your_control_stays_with_the_thief() {
        let (mut state, it) = stolen();
        self_return(&mut state, it, false);
        let obj = state.object(it).expect("the same arena handle");
        assert_eq!(obj.zone, crate::zone::Zone::Battlefield, "it came back");
        assert_eq!(
            (obj.owner, obj.controller, obj.base_controller),
            (them(), me(), me()),
            "under the controller of the ability, by default, still seat 1's card"
        );
        state
            .effects
            .remove_where(|fx| fx.modifier == baylee_cards_dsl::Modifier::GainControl);
        state.refresh_characteristics();
        assert_eq!(state.object(it).map(|o| o.controller), Some(me()));
    }

    /// "…then return it to the battlefield transformed under its owner's
    /// control" (Sheoldred, the Ojers): the other sentence, on the same
    /// board, sends the stolen permanent home.
    #[test]
    fn a_stolen_permanent_returning_itself_under_its_owners_control_goes_home() {
        let (mut state, it) = stolen();
        self_return(&mut state, it, true);
        let obj = state.object(it).expect("the same arena handle");
        assert_eq!(obj.zone, crate::zone::Zone::Battlefield, "it came back");
        assert_eq!(
            (obj.owner, obj.controller, obj.base_controller),
            (them(), them(), them())
        );
    }

    /// A self-return "under your control" for a player who has left leaves
    /// the card in exile (CR 800.4b); under its owner's it comes back.
    #[test]
    fn a_self_return_under_your_control_returns_nothing_to_a_player_who_has_left() {
        for (owner_control, back) in [(false, false), (true, true)] {
            let (mut state, it) = theirs(612);
            crate::sba::eliminate_player(&mut state, me(), crate::event::LossReason::Conceded);
            self_return(&mut state, it, owner_control);
            let obj = state.object(it).expect("seat 1's own card");
            let want = if back {
                crate::zone::Zone::Battlefield
            } else {
                crate::zone::Zone::Exile
            };
            assert_eq!(obj.zone, want, "owner_control: {owner_control}");
        }
    }

    /// Restoration Angel keeps the stolen creature: seat 0 now controls it
    /// by default, and seat 1 still owns it.
    fn kept() -> (GameState, ObjectId) {
        let (mut state, it) = stolen();
        blink(&mut state, it, false);
        let obj = state.object(it).expect("back");
        assert_eq!((obj.owner, obj.base_controller), (them(), me()));
        (state, it)
    }

    /// A permanent of seat 0's that `ExileLinked` names as the host.
    fn host(state: &mut GameState) -> ObjectId {
        let name = state.names.intern("Host");
        state.create_bare(me(), ObjectKind::Permanent, name, ZoneLocation::Battlefield)
    }

    /// Endless Sands and Safe Haven: "Return each creature card exiled with
    /// this land to the battlefield under its owner's control." The kept
    /// creature goes home (CR 610.3c), not back to the default a blink gave
    /// it.
    #[test]
    fn a_linked_exile_returns_under_its_owners_control() {
        let (mut state, it) = kept();
        let host = host(&mut state);
        let exile = Effect::exile_linked(TargetSpec::Object(&Filter::CREATURE));
        resolve(&mut state, host, &[it], exile);
        assert_eq!(
            state.object(it).map(|o| o.zone),
            Some(crate::zone::Zone::Exile)
        );
        resolve(&mut state, host, &[], Effect::ReturnLinkedToBattlefield);
        let obj = state.object(it).expect("back");
        assert_eq!(obj.zone, crate::zone::Zone::Battlefield);
        assert_eq!(
            (obj.owner, obj.controller, obj.base_controller),
            (them(), them(), them())
        );
    }

    /// Palace Jailer: "exile target creature an opponent controls until an
    /// opponent becomes the monarch". The card returns when the "until"
    /// event happens, under its owner's control (CR 610.3c).
    #[test]
    fn a_monarch_linked_exile_returns_under_its_owners_control() {
        let (mut state, it) = kept();
        let host = host(&mut state);
        resolve(&mut state, host, &[it], until_crowned());
        state.set_monarch(them());
        state.refresh_characteristics();
        let obj = state.object(it).expect("back");
        assert_eq!(obj.zone, crate::zone::Zone::Battlefield);
        assert_eq!(
            (obj.owner, obj.controller, obj.base_controller),
            (them(), them(), them())
        );
    }

    // --- "Exile … until …" (CR 610.3) ---------------------------------

    /// Werefox Bodyguard's exile: "until this creature leaves the
    /// battlefield".
    fn until_it_leaves() -> Effect {
        Effect::exile_until(
            TargetSpec::Object(&Filter::CREATURE),
            baylee_cards_dsl::ExileUntil::SourceLeavesBattlefield,
        )
    }

    /// Palace Jailer's exile: "until an opponent becomes the monarch".
    fn until_crowned() -> Effect {
        Effect::exile_until(
            TargetSpec::Object(&Filter::CREATURE),
            baylee_cards_dsl::ExileUntil::OpponentBecomesMonarch,
        )
    }

    fn zone(state: &GameState, id: ObjectId) -> Option<crate::zone::Zone> {
        state.object(id).map(|o| o.zone)
    }

    /// Whether `id` still carries a link to anything.
    fn linked(state: &GameState, id: ObjectId) -> bool {
        state.object(id).is_some_and(|o| {
            o.riders
                .iter()
                .any(|r| matches!(r, crate::object::Rider::Linked { .. }))
        })
    }

    /// A second creature seat 1 owns, beside the one `theirs` made.
    fn another_of_theirs(state: &mut GameState) -> ObjectId {
        let name = state.names.intern("Other Creature");
        let it = state.create_bare(
            them(),
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        let obj = state.object_mut(it).expect("fresh");
        let mut base = (*obj.base).clone();
        base.types = baylee_core::types::TypeSet::CREATURE;
        obj.base = std::sync::Arc::new(base);
        state.invalidate_projections();
        it
    }

    /// The card comes back as the host leaves, inside the move that takes
    /// the host away: nothing is resolved, refreshed or checked in between
    /// (CR 610.3). It comes back under its owner's control (CR 610.3c),
    /// here not the default the blink that kept it gave it, and linked to
    /// nothing.
    #[test]
    fn an_exile_until_its_host_leaves_ends_as_the_host_leaves() {
        let (mut state, it) = kept();
        let host = host(&mut state);
        resolve(&mut state, host, &[it], until_it_leaves());
        assert_eq!(zone(&state, it), Some(crate::zone::Zone::Exile));

        sba::destroy(&mut state, host);
        assert_eq!(zone(&state, host), Some(crate::zone::Zone::Graveyard));
        let obj = state.object(it).expect("back");
        assert_eq!(obj.zone, crate::zone::Zone::Battlefield);
        assert_eq!(
            (obj.owner, obj.controller, obj.base_controller),
            (them(), them(), them())
        );
        assert!(!linked(&state, it));
    }

    /// A host that is blinked leaves the battlefield too, and what comes
    /// back is a new object that holds nothing (CR 400.7).
    #[test]
    fn a_blinked_host_lets_go_of_what_it_held() {
        let (mut state, it) = theirs(613);
        let host = host(&mut state);
        resolve(&mut state, host, &[it], until_it_leaves());
        let blink = Effect::blink_to_owner(TargetSpec::Object(&Filter::Any));
        resolve(&mut state, ObjectId::NO_SOURCE, &[host], blink);
        assert_eq!(zone(&state, host), Some(crate::zone::Zone::Battlefield));
        assert_eq!(zone(&state, it), Some(crate::zone::Zone::Battlefield));
        assert!(!linked(&state, it));
    }

    /// A host that leaves the game with its owner leaves the battlefield
    /// without a move (CR 800.4a), and lets go all the same.
    #[test]
    fn a_host_that_leaves_the_game_lets_go_of_what_it_held() {
        let (mut state, it) = theirs(614);
        let host = host(&mut state);
        resolve(&mut state, host, &[it], until_it_leaves());
        crate::sba::eliminate_player(&mut state, me(), crate::event::LossReason::Conceded);
        assert!(state.object(host).is_none(), "gone with seat 0");
        let obj = state.object(it).expect("seat 1's own creature");
        assert_eq!(obj.zone, crate::zone::Zone::Battlefield);
        assert_eq!((obj.owner, obj.controller), (them(), them()));
    }

    /// If the host has left before the exile would happen, the card does
    /// not move (CR 610.3a, 610.3b).
    #[test]
    fn an_exile_until_the_host_leaves_does_nothing_once_it_has_left() {
        let (mut state, it) = theirs(615);
        let version = state.object(it).expect("there").version;
        let host = host(&mut state);
        sba::destroy(&mut state, host);
        resolve(&mut state, host, &[it], until_it_leaves());
        assert_eq!(zone(&state, it), Some(crate::zone::Zone::Battlefield));
        assert_eq!(state.object(it).map(|o| o.version), Some(version));
    }

    /// Nor when the host left while the ability waited and came back as a
    /// new object (CR 400.7), which the ability on the stack wrote down as
    /// the host went (`source_power_lki`).
    #[test]
    fn an_exile_until_the_host_leaves_does_nothing_once_it_has_been_blinked() {
        let (mut state, it) = theirs(618);
        let version = state.object(it).expect("there").version;
        let host = host(&mut state);
        let name = state.names.intern("Enters Trigger");
        let ability =
            state.create_bare(me(), ObjectKind::AbilityOnStack, name, ZoneLocation::Stack);
        state.object_mut(ability).expect("fresh").ability = Some(crate::object::AbilityLoc {
            card: None,
            index: 0,
            source: host,
        });
        let blink = Effect::blink_to_owner(TargetSpec::Object(&Filter::Any));
        resolve(&mut state, ObjectId::NO_SOURCE, &[host], blink);
        assert_eq!(zone(&state, host), Some(crate::zone::Zone::Battlefield));
        resolve_from(&mut state, host, ability, &[it], until_it_leaves());
        assert_eq!(
            zone(&state, it),
            Some(crate::zone::Zone::Battlefield),
            "the host on the battlefield is not the object whose ability this is"
        );
        assert_eq!(state.object(it).map(|o| o.version), Some(version));

        // And the same ability, whose source never left, does exile.
        state
            .object_mut(ability)
            .expect("still there")
            .source_power_lki = None;
        resolve_from(&mut state, host, ability, &[it], until_it_leaves());
        assert_eq!(zone(&state, it), Some(crate::zone::Zone::Exile));
    }

    /// The monarch ends the exile that waits for an opponent of the player
    /// who exiled, and no other: not an exile with no end of its own
    /// (Skyclave Apparition), and not while the new monarch is that player.
    #[test]
    fn the_monarch_ends_only_the_exile_that_waits_for_it() {
        let (mut state, jailed) = theirs(616);
        let held = another_of_theirs(&mut state);
        let jailer = host(&mut state);
        let apparition = host(&mut state);
        resolve(&mut state, jailer, &[jailed], until_crowned());
        let for_good = Effect::exile_linked(TargetSpec::Object(&Filter::CREATURE));
        resolve(&mut state, apparition, &[held], for_good);

        state.set_monarch(me());
        assert_eq!(zone(&state, jailed), Some(crate::zone::Zone::Exile));
        state.set_monarch(them());
        assert_eq!(zone(&state, jailed), Some(crate::zone::Zone::Battlefield));
        assert_eq!(
            zone(&state, held),
            Some(crate::zone::Zone::Exile),
            "an exile with no \"until\" is not ended by a crown"
        );
    }

    /// "An opponent" is an opponent of the player who controlled the
    /// exiling ability, whoever controls the Jailer by the time the crown
    /// moves. Seat 1 takes the Jailer and then the crown: seat 1 is still
    /// an opponent of seat 0, who exiled, and the card comes back.
    #[test]
    fn a_stolen_jailer_still_waits_for_an_opponent_of_the_player_who_exiled() {
        let (mut state, jailed) = theirs(617);
        let jailer = host(&mut state);
        resolve(&mut state, jailer, &[jailed], until_crowned());
        state.set_monarch(me());
        crate::resolve::gain_control(&mut state, &[(jailer, them())]);
        assert_eq!(state.object(jailer).map(|o| o.controller), Some(them()));
        state.set_monarch(them());
        assert_eq!(zone(&state, jailed), Some(crate::zone::Zone::Battlefield));
    }

    /// Takes `it` out of exile the way a cast does (onto the stack, then the
    /// battlefield as it resolves) and exiles it again with no link, the way
    /// Swords to Plowshares does.
    fn cast_and_exiled_again(state: &mut GameState, it: ObjectId) {
        for to in [
            ZoneLocation::Stack,
            ZoneLocation::Battlefield,
            ZoneLocation::Exile(them()),
        ] {
            state
                .move_object(it, to, ZonePosition::Top, Cause::Effect)
                .expect("the card is there");
        }
    }

    /// A card that leaves exile is a new object with no relation to the exile
    /// it left (CR 400.7), so once it is back in exile some other way it is
    /// not "exiled with" the host that exiled it first, and Safe Haven's
    /// return leaves it where it is.
    ///
    /// The link rode along with every move, so the card that was cast out of
    /// Safe Haven's exile and later hit by Swords to Plowshares came back when
    /// Safe Haven was sacrificed.
    #[test]
    fn a_card_that_left_exile_is_no_longer_exiled_with_its_old_host() {
        let (mut state, it) = theirs(619);
        let haven = host(&mut state);
        resolve(
            &mut state,
            haven,
            &[it],
            Effect::exile_linked(TargetSpec::Object(&Filter::CREATURE)),
        );
        assert!(linked(&state, it), "exiled with Safe Haven");
        cast_and_exiled_again(&mut state, it);
        resolve(&mut state, haven, &[], Effect::ReturnLinkedToBattlefield);
        assert_eq!(zone(&state, it), Some(crate::zone::Zone::Exile));
        assert!(!linked(&state, it), "a new object, exiled by nothing");
    }

    /// The same for an exile that lasted until its host leaves the
    /// battlefield: the Bodyguard leaving lets go of the card it exiled, and
    /// the card in exile now is not that card.
    #[test]
    fn a_card_that_left_exile_does_not_come_back_as_its_old_host_leaves() {
        let (mut state, it) = theirs(620);
        let bodyguard = host(&mut state);
        resolve(&mut state, bodyguard, &[it], until_it_leaves());
        cast_and_exiled_again(&mut state, it);
        sba::destroy(&mut state, bodyguard);
        assert_eq!(zone(&state, it), Some(crate::zone::Zone::Exile));
        assert!(!linked(&state, it));
    }

    /// Every rider that says what a card is *in exile* ends as the card leaves
    /// exile (CR 400.7): exiled with a host, on an adventure (CR 715.3d, "for
    /// as long as that card remains exiled"), castable from exile by a
    /// player, suspended, rebounding, foretold, plotted. A move from exile to
    /// exile is no leaving, and keeps them all.
    #[test]
    fn what_a_card_was_in_exile_ends_as_it_leaves_exile() {
        use crate::object::Rider;
        let (mut state, it) = theirs(621);
        let host = host(&mut state);
        let in_exile = [
            Rider::Linked { host, until: None },
            Rider::ExiledWith { host, version: 0 },
            Rider::Adventure,
            Rider::PlayableFromExileFor(me()),
            Rider::Suspend,
            Rider::Rebound,
            Rider::Foretold,
            Rider::Plotted,
        ];
        let riders = |state: &GameState| state.object(it).expect("there").riders.clone();
        for to in [ZoneLocation::Exile(them()), ZoneLocation::Exile(them())] {
            state
                .move_object(it, to, ZonePosition::Top, Cause::Effect)
                .expect("the card is there");
            if riders(&state).is_empty() {
                state
                    .object_mut(it)
                    .expect("in exile")
                    .riders
                    .extend(in_exile);
            }
        }
        assert_eq!(riders(&state)[..], in_exile, "exile to exile keeps them");
        state
            .move_object(
                it,
                ZoneLocation::Hand(them()),
                ZonePosition::Top,
                Cause::Effect,
            )
            .expect("the card is there");
        assert!(riders(&state).is_empty(), "{:?}", riders(&state));
    }

    /// Coiling Oracle: "Reveal the top card of your library. If it's a land
    /// card, put it onto the battlefield." No control is named, so the land
    /// enters under the player the effect told to put it there (CR 110.2a)
    /// — whatever default the card carried from its last time on the
    /// battlefield, here seat 1's.
    #[test]
    fn a_revealed_land_enters_under_the_player_who_puts_it_there() {
        let (mut state, _) = theirs(612);
        let name = state.names.intern("Land");
        let land = state.create_bare(me(), ObjectKind::Card, name, ZoneLocation::Library(me()));
        state
            .object_mut(land)
            .expect("fresh")
            .set_controller(them());
        let reveal = Effect::RevealTopAndSort {
            filter: &Filter::Any,
            matched: baylee_cards_dsl::effect::SearchDest::Battlefield,
            otherwise: baylee_cards_dsl::effect::SearchDest::Hand,
        };
        resolve(&mut state, ObjectId::NO_SOURCE, &[], reveal);
        let obj = state.object(land).expect("put onto the battlefield");
        assert_eq!(obj.zone, crate::zone::Zone::Battlefield);
        assert_eq!(
            (obj.owner, obj.controller, obj.base_controller),
            (me(), me(), me())
        );
    }
}
