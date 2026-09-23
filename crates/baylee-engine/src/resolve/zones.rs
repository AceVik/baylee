//! Zone movements: exile, blink, bounce, destruction, sacrifice,
//! graveyard recursion, mill — and countering spells/abilities (their
//! target's journey from the stack to another zone).

#[allow(clippy::wildcard_imports)] // family modules share the resolve vocabulary
use super::*;

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
fn spec_object(res: &Resolution, target: TargetSpec) -> Option<ObjectId> {
    match target {
        TargetSpec::ThisObject => Some(res.source),
        TargetSpec::EventObject => res.event_object,
        TargetSpec::Object(_)
        | TargetSpec::Spell(_)
        | TargetSpec::StackOrBattlefield(_)
        | TargetSpec::CardInGraveyard(..)
        | TargetSpec::AbilityOnStack(_)
        | TargetSpec::SpellOrAbility(_)
        | TargetSpec::Player(_)
        | TargetSpec::AnyPlayer
        | TargetSpec::AnyOpponent
        | TargetSpec::AnyTarget => res.targets.first().copied(),
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
        Effect::Blink { .. } => {
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
                if let Some(obj) = state.object_mut(target_id) {
                    obj.kind = ObjectKind::Permanent;
                    // Blink returns under its OWNER's control (Eerie
                    // Interlude, Momentary Blink family, CR 610.3c note:
                    // "return … under its owner's control").
                    obj.set_controller(owner);
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
                .zones
                .list(ZoneLocation::Battlefield)
                .iter()
                .filter(|id| {
                    state.object(**id).is_some_and(|o| {
                        (!opponents_only || state.is_opponent(o.controller, you))
                            && eval::matches(filter, state, o, you, res.source)
                    })
                })
                .copied()
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
            let all: Vec<ObjectId> = state
                .zones
                .list(ZoneLocation::Battlefield)
                .iter()
                .filter(|id| {
                    state
                        .object(**id)
                        .is_some_and(|o| eval::matches(filter, state, o, you, res.source))
                })
                .copied()
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
        Effect::GraveyardToBattlefield { target } => {
            if let Some(target_id) = spec_object(res, target) {
                if let Some(obj) = state.object_mut(target_id) {
                    obj.kind = ObjectKind::Permanent;
                    obj.set_controller(you);
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
            let target_id = match target {
                Some(_) => res.targets.first().copied(),
                None => Some(res.source),
            };
            if let Some(id) = target_id {
                if let Some(obj) = state.object_mut(id) {
                    obj.status.insert(Status::PHASED_OUT);
                }
                // CR 702.26b: phasing removes a permanent from combat
                // without changing zones. Blocked attackers stay blocked.
                state.combat.remove_from_combat(id);
                state.journal.record(GameEvent::PhaseChanged {
                    object: id,
                    phased_out: true,
                });
            }
            None
        }
        Effect::ExileLinked { .. } => {
            if let Some(&target_id) = res.targets.first() {
                let owner = state.object(target_id).map_or(you, |o| o.owner);
                if let Some(obj) = state.object_mut(target_id) {
                    obj.kind = ObjectKind::Card;
                    obj.riders
                        .push(crate::object::Rider::Linked { host: res.source });
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
        Effect::SacrificeSelf => {
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
            if let Some(target_id) = spec_object(res, target) {
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
            })
        }
        Effect::AllGraveyardCreaturesToBattlefield => {
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
        Effect::ExileSelfReturnAsFace { face } => {
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
            let _ = state.move_object(
                res.source,
                ZoneLocation::Battlefield,
                ZonePosition::Top,
                Cause::Effect,
            );
            if let Some(obj) = state.object_mut(res.source) {
                obj.kind = ObjectKind::Permanent;
                obj.set_controller(owner);
                // The face switch needs the card definition (lookup);
                // finish_resolution applies it.
                obj.pending_face_change = Some(face);
            }
            None
        }
        Effect::ReturnLinkedToBattlefield => {
            // Everything exiled with a link to the source returns under its
            // owner's control (Skyclave Apparition & co.).
            let mut returning = Vec::new();
            for seat in 0..state.players.len() {
                let p = PlayerId::new(seat as u8);
                for &card in state.zones.list(ZoneLocation::Exile(p)) {
                    if state.object(card).is_some_and(|o| {
                        o.riders
                            .iter()
                            .any(|r| matches!(r, crate::object::Rider::Linked { host } if *host == res.source))
                    }) {
                        returning.push(card);
                    }
                }
            }
            for card in returning {
                if let Some(obj) = state.object_mut(card) {
                    obj.kind = ObjectKind::Permanent;
                    obj.riders
                        .retain(|r| !matches!(r, crate::object::Rider::Linked { host } if *host == res.source));
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
                && !state
                    .object(target_id)
                    .is_some_and(|o| o.riders.contains(&crate::object::Rider::Uncounterable))
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
                && !state
                    .object(target_id)
                    .is_some_and(|o| o.riders.contains(&crate::object::Rider::Uncounterable))
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
                && !state
                    .object(target_id)
                    .is_some_and(|o| o.riders.contains(&crate::object::Rider::Uncounterable))
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
