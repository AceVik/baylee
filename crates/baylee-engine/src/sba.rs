//! State-based actions (CR 704), run as a fixpoint before every priority
//! grant. Each pass returns the actions it took; the engine repeats until
//! no action fires, then offers priority.

use crate::event::{Cause, GameEvent, LossReason};
use crate::object::{CounterKind, ObjectKind};
use crate::state::GameState;
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_core::ids::PlayerId;
use baylee_core::types::{SupertypeSet, TypeSet};

/// What the SBA pass needs from the engine flow.
#[derive(Debug, Default)]
pub struct SbaOutcome {
    /// Whether anything changed (re-run required).
    pub changed: bool,
    /// A pending legend-rule choice interrupts the fixpoint.
    pub legend_choice: Option<(PlayerId, Vec<baylee_core::ids::ObjectId>)>,
    /// A commander in a graveyard or in exile whose owner may send it to
    /// the command zone instead (CR 903.9a) — also an interruption.
    ///
    /// The pass has already recorded that it asked (`Commander::answered`),
    /// so a caller that drops this field does not defer the question, it
    /// *loses* it: the commander stays in the graveyard with nobody asked.
    /// There is exactly one caller outside the tests for that reason.
    pub commander_zone: Option<(PlayerId, baylee_core::ids::ObjectId)>,
}

/// A legend choice held until every choice for the simultaneous SBA event
/// is made (CR 704.3). No permanent moves while these decisions are collected.
#[derive(Clone, Debug, Hash)]
pub(crate) struct LegendDecision {
    player: PlayerId,
    keep: baylee_core::ids::ObjectId,
    options: Vec<baylee_core::ids::ObjectId>,
}

/// The next unanswered legend group, in APNAP order.
fn legend_choice(state: &GameState) -> Option<(PlayerId, Vec<baylee_core::ids::ObjectId>)> {
    for distance in 0..state.players.len() {
        let seat = (usize::from(state.turn.active.get()) + distance) % state.players.len();
        let player = PlayerId::new(seat as u8);
        // Sakashima-style suppression: the legend rule doesn't apply to
        // permanents this player controls.
        let legend_off = state.effects.iter().any(|fx| {
            matches!(fx.modifier, baylee_cards_dsl::Modifier::LegendRuleOff)
                && fx.controller == player
        });
        if legend_off {
            continue;
        }
        // Group by name in battlefield (zone) order — no HashMap: hash
        // iteration order is build/platform-dependent, and this loop
        // decides WHICH choice a player sees first when two legend pairs
        // coexist, so it is part of the determinism contract.
        let mut names: Vec<u32> = Vec::new();
        let mut groups: Vec<Vec<baylee_core::ids::ObjectId>> = Vec::new();
        for id in state.battlefield_seen() {
            let Some(obj) = state.object(id) else {
                continue;
            };
            if obj.controller == player
                && obj
                    .characteristics()
                    .supertypes
                    .contains(SupertypeSet::LEGENDARY)
                && obj.kind == ObjectKind::Permanent
            {
                let name = obj.characteristics().name.get();
                if let Some(i) = names.iter().position(|&n| n == name) {
                    groups[i].push(id);
                } else {
                    names.push(name);
                    groups.push(vec![id]);
                }
            }
        }
        for mut group in groups {
            if group.len() > 1 {
                group.sort(); // deterministic option order (oldest slot first)
                if !state
                    .sba_legend_decisions
                    .iter()
                    .any(|decision| decision.player == player && decision.options == group)
                {
                    return Some((player, group));
                }
            }
        }
    }

    None
}

/// Runs one SBA pass over the state (CR 704.3 list, S2 subset):
/// player losses, lethal damage, loyalty, legend rule, counter
/// annihilation, token cleanup.
pub fn run(state: &mut GameState, lookup: &impl crate::state::CardLookup) -> SbaOutcome {
    run_with_sagas(state, lookup, &[])
}

/// The engine supplies Saga decisions made against the same starting state,
/// since its pending chapter triggers are engine continuation state.
#[allow(clippy::too_many_lines)] // one simultaneous pass over the SBA list
pub(crate) fn run_with_sagas(
    state: &mut GameState,
    lookup: &impl crate::state::CardLookup,
    finished_sagas: &[baylee_core::ids::ObjectId],
) -> SbaOutcome {
    let mut outcome = SbaOutcome::default();

    // --- Player losses (CR 704.5a-c) -----------------------------------
    // Everybody Lives: no losses while the effect is active.
    let cant_lose = players_cant_lose(state);
    for player in 0..state.players.len() {
        if cant_lose {
            break;
        }
        let p = PlayerId::new(player as u8);
        let (life, poison, empty_draw, has_lost, commander_damage) = {
            let pl = &state.players[player];
            (
                pl.life,
                pl.poison,
                pl.tried_empty_draw,
                pl.has_lost(),
                pl.commander_damage
                    .iter()
                    .map(|(_, n)| *n)
                    .max()
                    .unwrap_or(0),
            )
        };
        if has_lost {
            continue;
        }
        let reason = if life <= 0 {
            Some(LossReason::Life)
        } else if poison >= 10 {
            Some(LossReason::Poison)
        } else if empty_draw {
            Some(LossReason::EmptyDraw)
        } else if commander_damage >= 21 {
            // CR 903.10a: twenty-one from *the same* commander, which is
            // why the maximum of the tallies is the number to compare and
            // not their sum. Three commanders at seven apiece is a player
            // in trouble, not a player who has lost.
            Some(LossReason::CommanderDamage)
        } else {
            None
        };
        if let Some(reason) = reason {
            eliminate_player(state, p, reason);
            outcome.changed = true;
        }
    }

    // Conceding while another choice is pending can remove an owner and
    // their cards. Such a group must be asked again if it still exists.
    state.sba_legend_decisions = core::mem::take(&mut state.sba_legend_decisions)
        .into_iter()
        .filter(|decision| {
            decision.options.iter().all(|id| {
                state.object(*id).is_some_and(|o| {
                    o.zone == crate::zone::Zone::Battlefield && o.controller == decision.player
                })
            })
        })
        .collect();
    if let Some(choice) = legend_choice(state) {
        outcome.legend_choice = Some(choice);
        return outcome;
    }

    // All SBAs in a pass see the same starting attachments. A host that
    // dies in this pass only makes its formerly legal Aura fall off in the
    // next pass; those two arrivals are not simultaneous (CR 704.3).
    let attachments = attachment_sbas(state, lookup);
    let departures: Vec<_> = if state.per_turn.permanent_damage.is_empty() {
        Vec::new()
    } else {
        state
            .battlefield_view()
            .into_iter()
            .filter(|id| state.has_damage_history(*id))
            .map(|id| (id, state.departure_snapshot(id)))
            .collect()
    };
    let departure = |id| {
        departures
            .iter()
            .find(|(other, _)| *other == id)
            .and_then(|(_, snapshot)| snapshot.clone())
    };

    // --- Lethal damage / zero toughness (CR 704.5f-h) -------------------
    for id in state.battlefield_view() {
        let Some(obj) = state.object(id) else {
            continue;
        };
        // CR 704.5i also applies to animated planeswalkers; being a
        // creature (even an indestructible one) does not replace this SBA.
        if obj.characteristics().types.contains(TypeSet::PLANESWALKER)
            && obj.counters.get(CounterKind::Loyalty) == 0
            && obj.kind == ObjectKind::Permanent
        {
            put_into_graveyard_with_departure(state, id, departure(id));
            outcome.changed = true;
            continue;
        }
        if !obj.characteristics().types.contains(TypeSet::CREATURE) {
            continue;
        }
        let toughness = obj.characteristics().toughness.unwrap_or(0);
        // CR 704.5f: zero or less toughness puts it in the graveyard. This
        // is not destruction, so indestructible does not save it.
        if toughness <= 0 {
            put_into_graveyard_with_departure(state, id, departure(id));
            outcome.changed = true;
            continue;
        }
        // Indestructible permanents can't be destroyed (CR 702.12b), which
        // covers both lethal damage and deathtouch.
        if obj
            .characteristics()
            .keywords
            .contains(baylee_cards_dsl::KeywordSet::INDESTRUCTIBLE)
        {
            continue;
        }
        // CR 704.5g lethal damage, CR 704.5h deathtouch — one point from a
        // deathtouch source is lethal however big the creature is.
        if obj.damage >= toughness as u16 || obj.deathtouched {
            destroy_with(state, id, true, departure(id));
            outcome.changed = true;
        }
    }
    // The deathtouch window is "since the last time state-based actions
    // were checked" (CR 704.5h), so this pass — which has now judged every
    // marked creature — closes it.
    // phasing: for a phased-out one too, which was not judged and must not
    // be when it phases in.
    for id in state.zones.list(ZoneLocation::Battlefield).clone() {
        if let Some(obj) = state.object_mut(id) {
            obj.deathtouched = false;
        }
    }

    // --- Attachments (CR 704.5m-p) --------------------------------------
    outcome.changed |= apply_attachment_sbas(state, attachments);

    // --- +1/+1 vs -1/-1 annihilation (CR 704.5q) -------------------------
    // That pair and no other. `CounterKind` says every +X/+Y counter in one
    // variant now, so it would be an easy and wrong generalisation to cancel
    // a -0/-1 against a +1/+1: the rule names the two counters by their
    // printed words, and a Wall of Roots wearing both keeps both.
    for id in state.battlefield_view() {
        let Some(obj) = state.object(id) else {
            continue;
        };
        let (plus, minus) = (
            obj.counters.get(CounterKind::P1P1),
            obj.counters.get(CounterKind::M1M1),
        );
        if plus > 0 && minus > 0 {
            let cancel = plus.min(minus);
            if let Some(obj) = state.object_mut(id) {
                obj.counters.set(CounterKind::P1P1, plus - cancel);
                obj.counters.set(CounterKind::M1M1, minus - cancel);
            }
            state.invalidate_projections();
            outcome.changed = true;
        }
    }

    // Every legend decision was made against this pass's starting state.
    // A chosen survivor can also die to a different SBA in this same pass.
    for decision in core::mem::take(&mut state.sba_legend_decisions) {
        for id in decision.options {
            if id != decision.keep
                && state
                    .object(id)
                    .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
            {
                put_into_graveyard_with_departure(state, id, departure(id));
                outcome.changed = true;
            }
        }
    }
    for &id in finished_sagas {
        if state
            .object(id)
            .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
        {
            put_into_graveyard_with_departure(state, id, departure(id));
            outcome.changed = true;
        }
    }

    // --- Tokens outside the battlefield cease to exist (CR 704.5d) ------
    // "Card-less" is not the same as "token": an emblem, an ability on the
    // stack and a *copy of a spell* (CR 707.10 — a copy is not a token)
    // all have no card behind them, and none of them may be swept up here.
    // The spell case mattered: a copy of a token-backed spell was deleted
    // by this pass before it could ever resolve.
    //
    // The candidates come from `GameState::watch_token_cleanup`, recorded
    // when an object leaves for a zone that is neither the battlefield nor
    // the stack. Scanning the whole arena instead — which is what this did
    // — is O(arena) on every round of a fixpoint that runs before every
    // priority grant, and an Ally deck puts six-figure counts of abilities
    // on the stack for it to walk past.
    let mut candidates = state.take_token_cleanup();
    for id in candidates.drain(..) {
        let Some(obj) = state.object(id) else {
            continue; // already gone: queued twice, or removed elsewhere
        };
        let is_token_like = obj.card.is_none()
            && !matches!(
                obj.kind,
                ObjectKind::Emblem | ObjectKind::AbilityOnStack | ObjectKind::Spell
            );
        // CR 704.5e: a copy of a spell anywhere but the stack ceases to
        // exist, and so does a copy of a card anywhere but the stack or the
        // battlefield. Both are the same marker here, because a copy that
        // resolved into a permanent is still the copy it was — Storm of
        // Saruman's own reminder text says so.
        let is_a_copy = obj.riders.contains(&crate::object::Rider::SpellCopy)
            && obj.zone != crate::zone::Zone::Stack;
        if !(is_token_like || is_a_copy) || obj.zone == crate::zone::Zone::Battlefield {
            continue; // an emblem, a spell still on the stack, or it went back
        }
        // Ceasing to exist means leaving the zone list too. Removing it
        // only from the arena leaves a dangling id behind that every later
        // graveyard scan walks over and `snapshot_hash` hashes — which for
        // a deck that makes thousands of tokens is an unbounded leak.
        let loc = ZoneLocation::of(obj.zone, obj.zone_owner.unwrap_or(obj.owner));
        state.zones.remove(id, loc);
        // Kept, not dropped. CR 111.7: "if a token changes zones, applicable
        // triggered abilities will trigger before the token ceases to exist"
        // — and here, they have not fired yet, because this fixpoint runs
        // before `collect_triggers`. `GameState::ceased` is what the trigger
        // scan reads the departed object out of, and the scan is what clears
        // it. Moved rather than cloned: `Arena::remove` hands the object
        // back, and the only reason it used to be thrown away is that nobody
        // had asked for it.
        state.remember_damage_source(id);
        if let Some(gone) = state.arena.remove(id) {
            state.ceased.push(gone);
        }
        // Leaving the zone is a change to it as much as a move out of it
        // is, and `GameState::move_object` invalidates for a move out of a
        // graveyard or exile because a permanent's projection may count
        // what is there (Pyrogoyf, Unlicensed Hearse). This removal does
        // not pass through it, so it says the same itself. The engine's
        // loop refreshes before it checks state-based actions, so a token
        // sacrificed or destroyed by an effect is projected over while it
        // lies in the graveyard, and without this that projection outlived
        // the token: nothing in the effect table moved.
        state.invalidate_projections();
        outcome.changed = true;
    }
    state.return_token_cleanup(candidates);

    // --- Commanders in a graveyard or in exile (CR 903.9a) --------------
    // "If a commander is in a graveyard or in exile and that object was put
    // into that zone since the last time state-based actions were checked,
    // its owner may put it into the command zone." A *may*, so it is a
    // question rather than a move, and it interrupts the fixpoint the way
    // the legend rule does.
    //
    // Last in the pass on purpose. Everything above can be the thing that
    // put it there — lethal damage, the legend rule, an aura falling off a
    // creature that was holding it up — and a commander destroyed by this
    // very pass is caught by this very pass rather than the next one.
    //
    // Whose commander is asked first is decided by seat order and then by
    // position in the seat's list, which is fixed for the whole game: a
    // wrath that kills two seats' commanders asks them in seat order, every
    // replay, and neither question is lost because each commander carries
    // its own `answered` watermark.
    'seats: for seat in 0..state.commanders.len() {
        for i in 0..state.commanders[seat].len() {
            let id = state.commanders[seat][i].object;
            let Some((owner, arrived)) = state.object(id).and_then(|obj| {
                matches!(
                    obj.zone,
                    crate::zone::Zone::Graveyard | crate::zone::Zone::Exile
                )
                .then_some((obj.owner, obj.timestamp))
            }) else {
                // Not in one of the two zones — or gone entirely, which is
                // what `eliminate_player` leaves behind: a marker naming a
                // card that no longer exists. Neither is a question.
                continue;
            };
            if arrived == state.commanders[seat][i].answered {
                continue; // this arrival has been offered already
            }
            if state.players[owner.get() as usize].has_lost() {
                continue; // nobody there to answer
            }
            // Recorded before the question is asked, not after it is
            // answered: a "no" must not be asked again, and neither must a
            // question that never reaches an answer.
            state.commanders[seat][i].answered = arrived;
            outcome.commander_zone = Some((owner, id));
            break 'seats;
        }
    }

    outcome
}

/// What an Aura may legally be attached to, as the card itself says it.
///
/// Enchant is a static ability of the Aura *spell* (CR 702.5b) and the DSL
/// says it where the card says it: the spell ability targets what it will
/// enchant and attaches itself to it. So the restriction is read back out of
/// that same `Effect::AttachSelf`, and there is no second place for a card
/// to state it — which matters, because a second place is a second truth,
/// and the one the targeting used would be the one that could drift.
///
/// `None` means the card states no restriction this can read, and the caller
/// then asks nothing beyond "is the host still there". That is deliberate: a
/// shape nobody has written yet must not make every Aura fall off.
fn enchant_restriction(
    obj: &crate::object::GameObject,
    lookup: &impl crate::state::CardLookup,
) -> Option<&'static baylee_cards_dsl::Filter> {
    use baylee_cards_dsl::{AbilityDef, Effect, TargetSpec};
    obj.abilities(lookup).iter().find_map(|ability| {
        let AbilityDef::Spell { effects, .. } = ability else {
            return None;
        };
        effects.iter().find_map(|effect| match effect {
            Effect::AttachSelf {
                target: TargetSpec::Object(filter),
            } => Some(*filter),
            _ => None,
        })
    })
}

/// Attachment state-based actions (CR 704.5m, 704.5n and both sentences of
/// 704.5p).
///
/// An Aura attached to something illegal — or to nothing — is put into its
/// owner's graveyard; an Equipment or Fortification in the same position
/// simply becomes unattached and stays on the battlefield. Without this,
/// an aura outlived the creature it enchanted and kept granting its
/// effect, and equipment kept pointing at a dead object whose slot a later
/// permanent could reuse.
///
/// **A battle or creature is never attached to anything** (CR 704.5p, first
/// sentence), whatever else it is, so that question comes before the
/// Aura's and the Equipment's: an Equipment an effect turns into a creature
/// while it equips one comes off and stays on the battlefield. For a while
/// only the second sentence was read, which asks about the permanents that
/// are neither an Aura, an Equipment nor a Fortification, and the animated
/// Equipment stayed on its host because it is an Equipment. A reconfigure
/// Equipment is not a creature while it is attached (CR 702.151b), so the
/// sentence leaves it where it is; Rabbit Battery says so as a static of its
/// own.
///
/// **Illegal is not the same as gone**, and for a while this asked only the
/// second question. CR 303.4c makes the Aura's own enchant ability the test:
/// a host that is still a permanent on the battlefield but has stopped being
/// something this Aura may enchant is illegal, and the Aura goes. An
/// Equipment's restriction is the rules' rather than the card's — CR 301.5b
/// attaches it to a creature — so the two are asked differently and answered
/// differently, which is why the counter-test matters more than the test:
/// both sit on the same illegal host, and only one of them is destroyed.
fn attachment_sbas(
    state: &GameState,
    lookup: &impl crate::state::CardLookup,
) -> (
    Vec<baylee_core::ids::ObjectId>,
    Vec<baylee_core::ids::ObjectId>,
) {
    use baylee_core::types::TypeSet as T;
    let mut falling_off = Vec::new();
    let mut unattaching = Vec::new();
    for id in state.battlefield_seen() {
        let Some(obj) = state.object(id) else {
            continue;
        };
        if obj.kind != ObjectKind::Permanent {
            continue;
        }
        let types = obj.characteristics().types;
        // CR 704.5p, first sentence: a battle or creature attached to an
        // object or player becomes unattached and remains on the
        // battlefield. Asked of every permanent, an Aura's and an
        // Equipment's type included, because the sentence excepts none.
        if obj.attached_to.is_some() && types.intersects(T::CREATURE.union(T::BATTLE)) {
            unattaching.push(id);
            continue;
        }
        let is_aura = obj
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::enchantment::AURA);
        let is_equipment = obj
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::artifact::EQUIPMENT);
        if !is_aura && !is_equipment {
            // CR 704.5p, second sentence: any other noncreature,
            // nonbattle permanent attached to something becomes unattached
            // and stays on the battlefield, save a Fortification (CR
            // 704.5n's, which this does not otherwise read). Dowsing Dagger
            // transforms into Lost Vale while it equips a creature, and a
            // land equips nothing. The first sentence took the creatures
            // and battles above.
            let fortification = obj
                .characteristics()
                .subtypes
                .contains(baylee_core::generated::subtypes::artifact::FORTIFICATION);
            if obj.attached_to.is_some() && !fortification {
                unattaching.push(id);
            }
            continue;
        }
        // The host has to be a permanent on the battlefield; anything else
        // (destroyed, exiled, bounced, or never set) is an illegal
        // attachment. And it has to be a permanent this attachment may hold
        // on to, which is the other half of "illegal".
        let restriction = is_aura.then(|| enchant_restriction(obj, lookup)).flatten();
        let controller = obj.controller;
        let host_ok = obj.attached_to.is_some_and(|host| {
            state.object(host).is_some_and(|h| {
                h.zone == crate::zone::Zone::Battlefield
                    && h.kind == ObjectKind::Permanent
                    // CR 303.4d: an Aura can't enchant an Aura it is
                    // attached to being itself; self-attachment is never
                    // legal for either kind.
                    && host != id
                    // CR 301.5b: an Equipment attaches to a creature, and
                    // whose creature it is matters only while the equip
                    // ability is on the stack (CR 301.5d), so control is not
                    // asked here.
                    && (!is_equipment || h.characteristics().types.contains(T::CREATURE))
                    // CR 303.4c: and an Aura, to what its enchant ability
                    // names. `you` is the Aura's controller, so "enchant
                    // creature you control" is read from the side that
                    // controls the Aura rather than the host.
                    && restriction.is_none_or(|filter| {
                        crate::eval::matches(filter, state, h, controller, id)
                    })
                    // CR 702.16c/d: and not one whose protection this
                    // attachment has the quality of. An Aura falls off, an
                    // Equipment stays on the battlefield unattached.
                    && !crate::eval::protected_from(state, host, id)
                    && (!is_aura || crate::eval::permits_enchantment(state, host, id))
            })
        });
        if host_ok {
            continue;
        }
        // Nothing that reaches here and is a creature is attached (the first
        // sentence of CR 704.5p saw to that), and an Equipment creature
        // with no host — a living weapon whose Germ died, an animated
        // Equipment — stays on the battlefield. So does an Aura creature,
        // which stands in for a bestowed Aura that has become unattached
        // and ceased to be bestowed (CR 702.103f); the engine has no bestow,
        // and CR 303.4d and 704.5m would put any other Aura creature into
        // its owner's graveyard.
        if is_aura && !types.contains(T::CREATURE) {
            falling_off.push(id);
        } else if obj.attached_to.is_some() {
            unattaching.push(id);
        }
    }
    (falling_off, unattaching)
}

fn apply_attachment_sbas(
    state: &mut GameState,
    (falling_off, unattaching): (
        Vec<baylee_core::ids::ObjectId>,
        Vec<baylee_core::ids::ObjectId>,
    ),
) -> bool {
    let mut changed = false;
    for id in falling_off {
        if state
            .object(id)
            .is_none_or(|obj| obj.zone != crate::zone::Zone::Battlefield)
        {
            continue;
        }
        put_into_graveyard(state, id);
        changed = true;
    }
    for id in unattaching {
        if let Some(obj) = state.object_mut(id) {
            obj.attached_to = None;
        }
        // The same projection input as the attaching side: an Equipment
        // grants through `Filter::AttachedToBySource`, so a host that is
        // gone has to take the grant with it.
        state.invalidate_projections();
        changed = true;
    }
    changed
}

/// Records a legend-rule choice; the next pass applies every answered
/// group together with all other applicable SBAs (CR 704.3).
pub fn apply_legend_choice(
    state: &mut GameState,
    player: PlayerId,
    keep: baylee_core::ids::ObjectId,
    options: &[baylee_core::ids::ObjectId],
) {
    debug_assert!(options.contains(&keep));
    state.sba_legend_decisions.push(LegendDecision {
        player,
        keep,
        options: options.to_vec(),
    });
}

/// Destroys a permanent (CR 701.8a), unless it can't be.
///
/// Indestructible is a property of the permanent and not of the spell that
/// named it (CR 702.12b), so the question is asked once here rather than at
/// each of the effects that destroy — `Destroy`, `DestroyAll` and the
/// per-player choice `DestroyChosen` makes were three separate places to
/// forget it, and all three did: Darksteel Forge granted a keyword that
/// stopped lethal damage and nothing else, so "destroy target permanent"
/// killed an indestructible artifact outright.
///
/// The lethal-damage state-based action keeps its own check because it
/// decides more than this call does — an indestructible creature is not a
/// state-based action *performed*, and `outcome.changed` is what tells the
/// fixpoint whether to run again.
///
/// A regeneration shield is spent here for the same reason (CR 701.19a):
/// it replaces *destruction*, and destruction is what this function is.
/// Lethal damage routes through here, so a shielded creature survives a
/// Lightning Bolt and a "destroy target creature" by the same line.
pub fn destroy(state: &mut GameState, id: baylee_core::ids::ObjectId) {
    destroy_with(state, id, true, None);
}

/// Destroys a permanent whose destruction a card said it could not be
/// regenerated from (CR 701.19c).
///
/// Not "no shield is created": a shield already standing is simply not
/// applied, and the card that said so is the one that decides, never the
/// permanent. Eight cards in this pool print the clause — Terminate,
/// Oxidize, Pillage, Fissure, Tunnel, Despotic Scepter, Visara and Damn's
/// overload — and every one of them was correct for free while no shield
/// existed at all.
pub fn destroy_no_regen(state: &mut GameState, id: baylee_core::ids::ObjectId) {
    destroy_with(state, id, false, None);
}

/// The two doors above, and the order the rules put their questions in.
///
/// Indestructible first, because a permanent that cannot be destroyed is
/// never destroyed and so never spends a shield on it (CR 702.12b): a
/// Darksteel creature with a regeneration shield still has that shield
/// afterwards. Then the shield, which replaces the destruction
/// (CR 701.19a) rather than preventing it — and the permanent stays on the
/// battlefield tapped, undamaged and out of combat.
///
/// `damage` and `deathtouched` go together and both have to: the marked
/// damage is what the rule removes, and a creature that kept the deathtouch
/// mark would be judged lethal again by the very next state-based check,
/// which would spend the next shield and then kill it.
fn destroy_with(
    state: &mut GameState,
    id: baylee_core::ids::ObjectId,
    regeneratable: bool,
    departure: Option<crate::damage_history::BattlefieldDeparture>,
) {
    // Only a permanent is destroyed (CR 701.8a: "move it from the
    // battlefield"). A delayed "destroy that creature" whose creature went
    // back to its owner's hand names a card that stays there.
    if !state
        .object(id)
        .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
    {
        return;
    }
    if state.object(id).is_some_and(|o| {
        o.characteristics()
            .keywords
            .contains(baylee_cards_dsl::KeywordSet::INDESTRUCTIBLE)
    }) {
        return;
    }
    // "It can't be regenerated this turn" (Disintegrate): the shield
    // stands and is not applied (CR 701.19c), for this object only.
    let regeneratable = regeneratable
        && state
            .object(id)
            .is_none_or(|o| !state.per_turn.cant_regenerate.contains(&(id, o.version)));
    if regeneratable
        && let Some(obj) = state.object_mut(id)
        && obj.regeneration_shields > 0
    {
        obj.regeneration_shields -= 1;
        obj.damage = 0;
        obj.deathtouched = false;
        // `set_tapped` and not the bit: tapping is what the layer
        // projection has to hear about, and
        // `rules::nothing_writes_the_tapped_bit_except_the_one_door` is the
        // test that says so — it caught this line writing the bit directly.
        state.set_tapped(id, true);
        state.combat.remove_from_combat(id);
        return;
    }
    put_into_graveyard_with_departure(state, id, departure);
}

/// Destroy a simultaneous set, retaining attachments before any member leaves.
pub(crate) fn destroy_all(
    state: &mut GameState,
    objects: &[baylee_core::ids::ObjectId],
    no_regen: bool,
) {
    let departures: Vec<_> = objects
        .iter()
        .map(|id| (*id, state.departure_snapshot(*id)))
        .collect();
    for (id, departure) in departures {
        destroy_with(state, id, !no_regen, departure);
    }
}

/// Moves a permanent to its owner's graveyard without destroying it.
///
/// The mechanical half of [`destroy`], and reachable on its own because
/// several state-based actions put a permanent in the graveyard and are
/// explicitly *not* destruction: zero or less toughness (CR 704.5f), a
/// planeswalker at zero loyalty (CR 704.5i), the legend rule (CR 704.5j)
/// and an Aura attached to nothing it could legally be attached to
/// (CR 704.5m). Indestructible saves a permanent from destruction and from
/// none of those.
pub fn put_into_graveyard(state: &mut GameState, id: baylee_core::ids::ObjectId) {
    put_into_graveyard_with_departure(state, id, None);
}

fn put_into_graveyard_with_departure(
    state: &mut GameState,
    id: baylee_core::ids::ObjectId,
    departure: Option<crate::damage_history::BattlefieldDeparture>,
) {
    let owner = state.object(id).map_or(PlayerId::new(0), |o| o.owner);
    // Only the kind. The marked damage and the deathtouch flag used to be
    // cleared here too, which was this one caller doing by hand what every
    // permanent leaving the battlefield needs; `GameState::move_object` is
    // the one door for that now, so the two cannot drift.
    if let Some(obj) = state.object_mut(id) {
        obj.kind = ObjectKind::Card;
    }
    let _ = state.move_object_with_departure(
        id,
        ZoneLocation::Graveyard(owner),
        ZonePosition::Top,
        Cause::StateBased,
        departure,
    );
}

/// Whether an effect says players can't lose the game (Everybody Lives!).
///
/// One reading for every way the game says a player loses other than a
/// concession: the state-based losses above and an effect's
/// [`lose_by_effect`]. A second copy of the scan is how the pact came to
/// ignore it (#237).
#[must_use]
pub fn players_cant_lose(state: &GameState) -> bool {
    state
        .effects
        .iter()
        .any(|fx| matches!(fx.modifier, baylee_cards_dsl::Modifier::PlayersCantLose))
}

/// An effect says `player` loses the game (CR 104.3e), such as a pact left
/// unpaid. They do, unless an effect says players can't lose the game, and
/// then nothing happens: the effect that said so has done all it does.
///
/// Returns whether the player lost. Not for a concession, which "can't lose
/// the game" does not stop (CR 104.3a); that calls [`eliminate_player`].
pub fn lose_by_effect(state: &mut GameState, player: PlayerId) -> bool {
    if players_cant_lose(state) {
        return false;
    }
    eliminate_player(state, player, LossReason::Effect);
    true
}

/// A player leaves the game (CR 800.4a), having lost for `reason`, and
/// returns what of theirs was exiled.
///
/// In the rule's order, which decides where each object ends up:
/// 1. Everything they own leaves the game, and every effect giving them
///    control of an object ends, so what they took goes back to whoever
///    controls it without them. So does every effect giving them control of
///    a player: the one the engine has is Opposition Agent's search takeover
///    (CR 722.2), and `every_card_that_controls_a_player_does_it_by_taking_over_a_search`
///    fails the day a card controls a player some other way. What a
///    permanent of theirs held "until it leaves the battlefield" comes back,
///    and if they were the monarch, the designation passes on as they leave
///    (CR 724.4, `GameState::monarch_leaves`).
/// 2. [`exile_what_the_departed_control`]: an ability or a copy of a spell
///    they control ceases to exist, and what else they still control is
///    exiled. That is what they control by default: a creature they
///    reanimated out of somebody else's graveyard, a spell of somebody
///    else's they cast.
///
/// Not a state-based action: it all happens as they leave.
pub fn eliminate_player(
    state: &mut GameState,
    player: PlayerId,
    reason: LossReason,
) -> Vec<baylee_core::ids::ObjectId> {
    // The first loss is the one that happened: a seat that has already lost
    // and then concedes has not lost a second time for a second reason.
    let _ = state.players[player.get() as usize]
        .loss
        .get_or_insert(reason);
    state
        .journal
        .record(GameEvent::PlayerLost { player, reason });
    let owned: Vec<_> = state
        .arena
        .iter()
        .filter(|(_, o)| o.owner == player)
        .map(|(id, _)| id)
        .collect();
    for id in owned {
        let (zone, owner) = match state.object(id) {
            Some(o) => (o.zone, o.zone_owner.unwrap_or(o.owner)),
            None => continue,
        };
        let loc = ZoneLocation::of(zone, owner);
        state.zones.remove(id, loc);
        state.remember_damage_source(id);
        let _ = state.arena.remove(id);
    }
    // They left without passing through `move_object`, which is what
    // invalidates the projection for everybody else, and what stayed may
    // have counted them: Pyrogoyf counts the card types among cards in all
    // graveyards, theirs until now. The generation compare counts effects,
    // so without this the refresh `exile_what_the_departed_control` runs
    // changed nothing, and a Pyrogoyf kept the size their graveyard gave it.
    state.invalidate_projections();
    state.effects.remove_where(|fx| {
        matches!(
            fx.modifier,
            baylee_cards_dsl::Modifier::GainControl | baylee_cards_dsl::Modifier::SearchTakeover
        ) && fx.controller == player
    });
    // A permanent of theirs that held a card "until it leaves the
    // battlefield" has just left it without passing through `move_object`,
    // and the card comes back to its owner (CR 610.3).
    state.return_what_departed_hosts_held();
    state.monarch_leaves(player);
    let exiled = exile_what_the_departed_control(state);
    let mut gone: Vec<_> = state
        .combat
        .attackers()
        .iter()
        .filter(|a| state.object(a.creature).is_none())
        .map(|a| a.creature)
        .collect();
    gone.sort_unstable();
    let blockers: Vec<_> = state
        .combat
        .blockers
        .iter()
        .filter(|b| state.object(b.blocker).is_some() && state.object(b.attacker).is_some())
        .copied()
        .collect();
    state
        .combat
        .retain_attackers(|a| gone.binary_search(&a.creature).is_err());
    state.combat.blockers = blockers;
    // A delayed trigger they control can never be put on the stack again
    // (CR 800.4d), and one waiting for a turn of theirs would otherwise sit
    // in the hashed state for the rest of the game.
    state.delayed.retain(|d| d.controller != player);
    state.granted_actions.retain(|g| g.player != player);
    exiled
}

/// What players who have left the game still control, gone, and the ids of
/// what was exiled.
///
/// An ability or a copy of a spell on the stack ceases to exist; anything
/// else on the battlefield or the stack is exiled (CR 800.4a). The second
/// reader is CR 800.4c: an object whose last control effect ends while the
/// player who controls it by default has left, such as a creature a player
/// reanimated and left behind that another player had taken until end of
/// turn, is exiled as that effect ends.
///
/// **Not a state-based action**, and not run as one. [`eliminate_player`]
/// calls it as a player leaves. The engine's loop calls it right after it
/// refreshes the projection (step 0a), before it checks whether the game is
/// over and before any state-based action, and the cleanup step calls it as
/// until-end-of-turn effects end. An effect that ends in the middle of a
/// resolution exiles at the next pass, not inside the resolution
/// (`docs/engine-internals.md`).
///
/// It goes through [`GameState::move_object`], so a permanent leaving the
/// battlefield this way triggers what leaving the battlefield triggers.
pub fn exile_what_the_departed_control(state: &mut GameState) -> Vec<baylee_core::ids::ObjectId> {
    state.refresh_characteristics();
    // The projection leaves phased-out permanents as they were; one whose
    // controller left is read again (CR 702.26n).
    state.release_from_the_departed();
    let departed = |state: &GameState, id: baylee_core::ids::ObjectId| {
        state
            .object(id)
            .is_some_and(|o| state.has_left(o.controller))
    };
    let ceased: Vec<_> = state
        .zones
        .list(ZoneLocation::Stack)
        .iter()
        .copied()
        .filter(|&id| {
            departed(state, id)
                && state.object(id).is_some_and(|o| {
                    o.kind == ObjectKind::AbilityOnStack
                        || o.riders.contains(&crate::object::Rider::SpellCopy)
                })
        })
        .collect();
    for id in ceased {
        state.zones.remove(id, ZoneLocation::Stack);
        state.remember_damage_source(id);
        let _ = state.arena.remove(id);
    }
    let exiled: Vec<_> = state
        .zones
        // phasing: a phased-out permanent is exiled too (CR 702.26n).
        .list(ZoneLocation::Battlefield)
        .iter()
        .chain(state.zones.list(ZoneLocation::Stack).iter())
        .copied()
        .filter(|&id| departed(state, id))
        .collect();
    for &id in &exiled {
        let owner = state.object(id).map_or(PlayerId::new(0), |o| o.owner);
        if let Some(obj) = state.object_mut(id) {
            obj.kind = ObjectKind::Card;
        }
        let _ = state.move_object(
            id,
            ZoneLocation::Exile(owner),
            ZonePosition::Top,
            Cause::PlayerLeft,
        );
    }
    exiled
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::Status;
    use crate::state::CardLookup;
    use baylee_core::ids::{CardIndex, PrintRef};
    use baylee_core::preset::{
        AIProfile, DeckEntry, FormatId, GamePreset, HouseRules, PrintInfo, SeatController, SeatSpec,
    };

    struct RegistryLookup;
    impl CardLookup for RegistryLookup {
        fn card(&self, index: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
            baylee_cards::by_index(index)
        }
    }

    fn card_index(oracle_id: &str) -> CardIndex {
        baylee_cards::by_oracle_id(oracle_id)
            .expect("registry contains the card")
            .index
    }

    fn forest() -> CardIndex {
        card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
    }
    fn elesh_norn() -> CardIndex {
        card_index("5ade11c0-41dd-4b6a-9f5b-c5903a3a0d7f")
    }
    fn mox_opal() -> CardIndex {
        card_index("de2440de-e948-4811-903c-0bbe376ff64d")
    }

    fn entry(card: CardIndex) -> DeckEntry {
        DeckEntry {
            card,
            print: PrintRef::new(0),
        }
    }

    /// Seat 0 controls two legend pairs at once: 2× Elesh Norn and
    /// 2× Mox Opal (Elesh Norn enters the battlefield list first).
    fn two_legend_pairs_preset(seed: u64) -> GamePreset {
        let deck: Vec<DeckEntry> = (0..60).map(|_| entry(forest())).collect();
        let mk = |bf: Vec<CardIndex>| SeatSpec {
            controller: SeatController::Ai(AIProfile::default()),
            capabilities: baylee_core::preset::SeatCapabilities::default(),
            deck: deck.clone(),
            sideboard: vec![],
            commanders: vec![],
            starting_life: None,
            starting_hand: None,
            starting_battlefield: bf.into_iter().map(entry).collect(),
            emblems: vec![],
            team: None,
        };
        GamePreset {
            format: FormatId::Freeform,
            seed,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![PrintInfo {
                scryfall_id: uuid::Uuid::nil(),
                lang: "EN".into(),
                finish: baylee_core::preset::Finish::Normal,
            }],
            seats: vec![
                mk(vec![elesh_norn(), elesh_norn(), mox_opal(), mox_opal()]),
                mk(vec![]),
            ],
        }
    }

    /// Two empty boards. Nothing here interrupts the pass, which matters:
    /// the legend rule returns early, so any test of a later SBA needs a
    /// state that gets that far.
    fn empty_boards_preset(seed: u64) -> GamePreset {
        let mut preset = two_legend_pairs_preset(seed);
        for seat in &mut preset.seats {
            seat.starting_battlefield.clear();
        }
        preset
    }

    /// With two legend pairs coexisting, the FIRST choice a player sees
    /// must be deterministic — it used to depend on hash iteration order.
    #[test]
    fn legend_choice_is_deterministic_with_two_pairs() {
        let mut first: Option<Vec<baylee_core::ids::ObjectId>> = None;
        for seed in [7, 42, 1337] {
            let mut state = GameState::from_preset(&two_legend_pairs_preset(seed), &RegistryLookup)
                .expect("game starts");
            let outcome = run(&mut state, &RegistryLookup);
            let (player, group) = outcome.legend_choice.expect("a legend choice is due");
            assert_eq!(player, PlayerId::new(0));
            assert_eq!(group.len(), 2, "one pair is offered, not all four");
            // The offered pair is the Elesh Norns (first in zone order):
            // both objects carry her card index.
            for id in &group {
                let obj = state.object(*id).expect("offered object exists");
                assert_eq!(
                    obj.card.map(|c| c.index),
                    Some(elesh_norn()),
                    "the first pair in battlefield order is offered first"
                );
            }
            if let Some(prev) = &first {
                assert_eq!(*prev, group, "same choice across seeds and runs");
            }
            first = Some(group);
        }
    }

    #[test]
    fn graveyard_order_combines_all_legend_choices_lethal_and_finished_sagas() {
        let mut state = GameState::from_preset(&two_legend_pairs_preset(713), &RegistryLookup)
            .expect("two legend pairs");
        let owner = PlayerId::new(0);
        state.graveyard_order.enabled = true;
        let keep_dying = state.zones.list(ZoneLocation::Battlefield)[0];
        state.object_mut(keep_dying).expect("first legend").damage = u16::MAX;
        let extra: Vec<_> = state.zones.list(ZoneLocation::Library(owner))[..2].to_vec();
        for &card in &extra {
            state
                .move_object(
                    card,
                    ZoneLocation::Battlefield,
                    ZonePosition::Top,
                    Cause::Effect,
                )
                .unwrap();
        }
        let dying = state.object_mut(extra[0]).expect("lethal creature");
        dying.kind = ObjectKind::Permanent;
        dying.base_mut().types = TypeSet::CREATURE;
        dying.base_mut().toughness = Some(0);
        let finished = &[extra[1]];
        let since = state.journal.last_seq();
        let first = run_with_sagas(&mut state, &RegistryLookup, finished);
        let (player, first_group) = first.legend_choice.expect("choose first legend group");
        assert!(
            first_group.contains(&keep_dying),
            "even a dying legend is part of this pass's choice"
        );
        assert!(!first.changed);
        assert!(state.zones.list(ZoneLocation::Graveyard(owner)).is_empty());
        let before = state.snapshot_hash();
        apply_legend_choice(&mut state, player, keep_dying, &first_group);
        assert_ne!(
            state.snapshot_hash(),
            before,
            "a deferred choice is part of the snapshot"
        );
        let second = run_with_sagas(&mut state, &RegistryLookup, finished);
        let (player, second_group) = second.legend_choice.expect("choose second legend group");
        assert!(!second.changed);
        assert!(state.zones.list(ZoneLocation::Graveyard(owner)).is_empty());
        apply_legend_choice(&mut state, player, second_group[0], &second_group);
        let complete = run_with_sagas(&mut state, &RegistryLookup, finished);
        assert!(complete.changed);
        assert!(complete.legend_choice.is_none());
        assert!(state.sba_legend_decisions.is_empty());
        crate::graveyard_order::capture(&mut state, since);
        let Some(crate::choice::Pending::Arrange { cards, .. }) =
            crate::graveyard_order::pending(&mut state)
        else {
            panic!("one simultaneous graveyard order");
        };
        assert_eq!(
            cards.len(),
            5,
            "both first legends, second legend, lethal creature and finished Saga"
        );
        assert!(cards.contains(&keep_dying));
        assert!(cards.contains(&extra[0]));
        assert!(cards.contains(&extra[1]));
    }

    /// CR 704.5d, now driven by a recorded candidate instead of a scan of
    /// the whole arena. The pass has to keep finding the token *and* keep
    /// its hands off the two card-less objects that legitimately live
    /// outside the battlefield.
    #[test]
    fn a_token_that_leaves_the_battlefield_ceases_to_exist() {
        let mut state =
            GameState::from_preset(&empty_boards_preset(3), &RegistryLookup).expect("game starts");
        let owner = PlayerId::new(0);
        let name = state.names.intern("Test Token");

        let token = state.create_bare(
            owner,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        let emblem = state.create_bare(
            owner,
            ObjectKind::Emblem,
            name,
            ZoneLocation::Command(owner),
        );

        // A pass while it is still on the battlefield must not touch it.
        run(&mut state, &RegistryLookup);
        assert!(
            state.object(token).is_some(),
            "a token on the battlefield stays"
        );
        assert!(state.object(emblem).is_some(), "an emblem is not a token");

        let graveyard = ZoneLocation::Graveyard(owner);
        state
            .move_object(token, graveyard, ZonePosition::Top, Cause::StateBased)
            .expect("the token dies");
        assert!(
            state.zones.list(graveyard).contains(&token),
            "it is in the graveyard for exactly as long as it takes SBAs to run"
        );

        run(&mut state, &RegistryLookup);
        assert!(state.object(token).is_none(), "and then it is gone");
        assert!(
            !state.zones.list(graveyard).contains(&token),
            "gone from the zone list too — an id left behind here is walked by \
             every later graveyard scan and hashed by every snapshot, and a deck \
             that makes thousands of tokens leaks one per token"
        );
        assert!(
            state.object(emblem).is_some(),
            "the emblem is still not a token"
        );
    }

    fn flight() -> CardIndex {
        card_index("6a4068b0-fb4f-429c-a94e-47849f3eb7ef")
    }

    /// Seat 0 holds one Flight — "Enchant creature" — on the battlefield,
    /// and nothing else is in play. A real card is needed here rather than a
    /// bare object, because the Aura's enchant restriction is read off its
    /// own `Effect::AttachSelf` (CR 303.4c) and a card-less object states
    /// none.
    fn aura_state() -> (GameState, baylee_core::ids::ObjectId) {
        let mut preset = empty_boards_preset(5);
        preset.seats[0].starting_battlefield = vec![entry(flight())];
        let state = GameState::from_preset(&preset, &RegistryLookup).expect("game starts");
        let aura = *state
            .zones
            .list(ZoneLocation::Battlefield)
            .first()
            .expect("the aura is in play");
        (state, aura)
    }

    fn graveyard_aura_host() -> (
        GameState,
        baylee_core::ids::ObjectId,
        baylee_core::ids::ObjectId,
    ) {
        let mut preset = empty_boards_preset(404);
        preset.seats[0].starting_battlefield = vec![
            entry(flight()),
            entry(card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")),
        ];
        let mut state = GameState::from_preset(&preset, &RegistryLookup).expect("Flight and Bears");
        state.graveyard_order.enabled = true;
        let aura = state.zones.list(ZoneLocation::Battlefield)[0];
        let host = state.zones.list(ZoneLocation::Battlefield)[1];
        attach(&mut state, aura, host);
        state.object_mut(host).expect("Grizzly Bears").damage = 2;
        (state, aura, host)
    }

    #[test]
    fn graveyard_aura_follows_a_dying_host_in_a_later_sba_pass() {
        let (mut state, aura, host) = graveyard_aura_host();
        let owner = PlayerId::new(0);
        let since = state.journal.last_seq();
        assert!(run(&mut state, &RegistryLookup).changed);
        crate::graveyard_order::capture(&mut state, since);
        assert!(
            on_battlefield(&state, aura),
            "the Aura was legally attached at this pass's start"
        );
        assert_eq!(state.zones.list(ZoneLocation::Graveyard(owner)), &[host]);
        assert!(crate::graveyard_order::pending(&mut state).is_none());

        let since = state.journal.last_seq();
        assert!(run(&mut state, &RegistryLookup).changed);
        crate::graveyard_order::capture(&mut state, since);
        assert_eq!(
            state.zones.list(ZoneLocation::Graveyard(owner)),
            &[host, aura],
            "the Aura arrives later, so it is above its former host"
        );
        assert!(
            crate::graveyard_order::pending(&mut state).is_none(),
            "separate SBA passes give no choice to reverse these two cards"
        );
    }

    #[test]
    fn graveyard_an_already_illegal_aura_and_dying_host_can_be_ordered_together() {
        let (mut state, aura, host) = graveyard_aura_host();
        let owner = PlayerId::new(0);
        // Protection already makes Flight illegal when this pass starts;
        // its host independently has lethal damage in that same state.
        let modifier = baylee_cards_dsl::Modifier::ProtectionFrom(&baylee_cards_dsl::Filter::Any);
        let filter = crate::effects::EffectFilter::object(&state, host);
        let timestamp = state.next_timestamp();
        state.effects.register(crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: None,
            controller: owner,
            origin: crate::effects::EffectOrigin::Resolution,
            layer: modifier.layer(),
            timestamp,
            duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
            filter,
            modifier,
        });
        let since = state.journal.last_seq();
        assert!(run(&mut state, &RegistryLookup).changed);
        crate::graveyard_order::capture(&mut state, since);
        let Some(crate::choice::Pending::Arrange {
            player,
            cards,
            piles,
            ..
        }) = crate::graveyard_order::pending(&mut state)
        else {
            panic!("one simultaneous order choice");
        };
        assert_eq!(player, owner);
        assert_eq!(cards.len(), 2);
        assert!(cards.contains(&aura) && cards.contains(&host));
        assert!(piles[0].ordered);
        crate::graveyard_order::answer(&mut state, &[host, aura]);
        assert_eq!(
            state.zones.list(ZoneLocation::Graveyard(owner)),
            &[aura, host],
            "the owner can put the host above the Aura when both died together"
        );
    }

    /// A permanent of exactly these types and subtypes, controlled by seat 0.
    fn bare(
        state: &mut GameState,
        label: &str,
        types: TypeSet,
        subtypes: &[baylee_core::ids::SubtypeId],
    ) -> baylee_core::ids::ObjectId {
        let name = state.names.intern(label);
        let id = state.create_bare(
            PlayerId::new(0),
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        let base = state.object_mut(id).expect("just created").base_mut();
        base.types = types;
        base.subtypes = baylee_core::types::SubtypeSet::from_slice(subtypes);
        if types.contains(TypeSet::CREATURE) {
            base.power = Some(2);
            base.toughness = Some(2);
        }
        state.invalidate_projections();
        id
    }

    fn equipment(state: &mut GameState) -> baylee_core::ids::ObjectId {
        bare(
            state,
            "Test Equipment",
            TypeSet::ARTIFACT,
            &[baylee_core::generated::subtypes::artifact::EQUIPMENT],
        )
    }

    fn creature(state: &mut GameState) -> baylee_core::ids::ObjectId {
        bare(state, "Test Creature", TypeSet::CREATURE, &[])
    }

    fn attach(
        state: &mut GameState,
        what: baylee_core::ids::ObjectId,
        to: baylee_core::ids::ObjectId,
    ) {
        state
            .object_mut(what)
            .expect("on the battlefield")
            .attached_to = Some(to);
        state.invalidate_projections();
    }

    fn on_battlefield(state: &GameState, id: baylee_core::ids::ObjectId) -> bool {
        state.zones.list(ZoneLocation::Battlefield).contains(&id)
    }

    /// CR 704.5m: an Aura attached to nothing, or to something that is no
    /// longer there, is put into its owner's graveyard. Without it an Aura
    /// outlived the creature it enchanted and went on granting its effect.
    #[test]
    fn an_aura_with_nothing_to_enchant_is_put_into_the_graveyard() {
        let (mut state, aura) = aura_state();
        let host = creature(&mut state);
        attach(&mut state, aura, host);

        run(&mut state, &RegistryLookup);
        assert!(
            on_battlefield(&state, aura),
            "an Aura on a legal host stays where it is"
        );

        // The host dies. The Aura is now attached to an object that is not on
        // the battlefield, which is the commonest way this rule fires.
        state
            .move_object(
                host,
                ZoneLocation::Graveyard(PlayerId::new(0)),
                ZonePosition::Top,
                Cause::StateBased,
            )
            .expect("the creature dies");
        run(&mut state, &RegistryLookup);
        assert!(!on_battlefield(&state, aura), "the Aura falls off and dies");
        assert!(
            state
                .zones
                .list(ZoneLocation::Graveyard(PlayerId::new(0)))
                .contains(&aura),
            "into its owner's graveyard, not out of the game"
        );

        // And one that never had a host at all.
        let (mut state, aura) = aura_state();
        assert!(
            state.object(aura).expect("in play").attached_to.is_none(),
            "an Aura put onto the battlefield without being cast enchants \
             nothing"
        );
        run(&mut state, &RegistryLookup);
        assert!(!on_battlefield(&state, aura));
    }

    /// **Illegal is not the same as gone**, and the counter-test is the half
    /// that matters: both sit on the same host, and only the Aura dies.
    ///
    /// CR 303.4c makes the Aura's own enchant ability the test, so a host
    /// that is still a permanent on the battlefield but has stopped being a
    /// creature is illegal for Flight. An Equipment's restriction is the
    /// rules' instead (CR 301.5b), and an Equipment on an illegal host
    /// merely becomes unattached (CR 704.5n) — it is an artifact and stays
    /// in play.
    #[test]
    fn an_illegal_host_kills_an_aura_and_only_unequips_an_equipment() {
        let (mut state, aura) = aura_state();
        let host = creature(&mut state);
        let gear = equipment(&mut state);
        attach(&mut state, aura, host);
        attach(&mut state, gear, host);

        run(&mut state, &RegistryLookup);
        assert!(on_battlefield(&state, aura) && on_battlefield(&state, gear));
        assert_eq!(
            state.object(gear).expect("in play").attached_to,
            Some(host),
            "a creature is a legal host for both"
        );

        // The host stops being a creature without leaving the battlefield.
        state.object_mut(host).expect("in play").base_mut().types = TypeSet::ENCHANTMENT;
        state.invalidate_projections();

        run(&mut state, &RegistryLookup);
        assert!(
            !on_battlefield(&state, aura),
            "the Aura may no longer enchant it (CR 303.4c) and goes"
        );
        assert!(
            on_battlefield(&state, gear),
            "the Equipment is not destroyed by an illegal host"
        );
        assert_eq!(
            state.object(gear).expect("in play").attached_to,
            None,
            "it simply becomes unattached (CR 704.5n)"
        );
        assert!(
            on_battlefield(&state, host),
            "and the host itself is untouched by any of it"
        );
    }

    /// CR 303.4d: an Aura can't enchant itself, and the check is by object
    /// identity rather than by filter.
    ///
    /// The Aura here is card-less, so it states no enchant restriction at
    /// all and *every* permanent on the battlefield is a legal host for it —
    /// which is the point: the only sentence that can refuse is `host != id`,
    /// and an Aura attached to an illegal object is put into its owner's
    /// graveyard (CR 704.5m). A second one attached to a neighbour stays, so
    /// a green run cannot mean the attachment was refused for some other
    /// reason.
    ///
    /// An Equipment can't equip itself either (CR 301.5c), but no Equipment
    /// reaches the identity check any more: it could be its own legal host
    /// only as a creature, and a creature attached to anything comes off
    /// first (CR 704.5p, first sentence). So the Equipment half asks for the
    /// outcome — unattached and still on the battlefield — and its
    /// counter-test is an Equipment that is only an Equipment, on the
    /// neighbour.
    #[test]
    fn nothing_may_be_attached_to_itself() {
        use baylee_core::generated::subtypes::enchantment::AURA;
        let mut state =
            GameState::from_preset(&empty_boards_preset(9), &RegistryLookup).expect("game starts");
        let neighbour = creature(&mut state);
        let aura = bare(
            &mut state,
            "Self-loving Aura",
            TypeSet::ENCHANTMENT,
            &[AURA],
        );
        let kept_aura = bare(
            &mut state,
            "Neighbourly Aura",
            TypeSet::ENCHANTMENT,
            &[AURA],
        );
        // A living weapon: an Equipment creature.
        let gear = bare(
            &mut state,
            "Self-equipping Weapon",
            TypeSet::ARTIFACT.union(TypeSet::CREATURE),
            &[baylee_core::generated::subtypes::artifact::EQUIPMENT],
        );
        let kept_gear = equipment(&mut state);
        for (what, to) in [
            (aura, aura),
            (gear, gear),
            (kept_aura, neighbour),
            (kept_gear, neighbour),
        ] {
            attach(&mut state, what, to);
        }

        run(&mut state, &RegistryLookup);
        assert!(
            !on_battlefield(&state, aura),
            "an Aura attached to itself is attached to nothing legal and goes"
        );
        let gear_obj = state
            .object(gear)
            .expect("an Equipment is never destroyed by this");
        assert_eq!(
            gear_obj.attached_to, None,
            "an Equipment attached to itself comes off"
        );
        assert!(on_battlefield(&state, gear), "and stays on the battlefield");
        for (id, what) in [(kept_aura, "an Aura"), (kept_gear, "an Equipment")] {
            assert_eq!(
                state.object(id).expect("still in play").attached_to,
                Some(neighbour),
                "{what} on a neighbour stays: any permanent is a legal host for \
                 an attachment that names none"
            );
        }
    }

    /// CR 704.5p, first sentence: a battle or creature attached to an object
    /// or player becomes unattached and remains on the battlefield — an
    /// Equipment or a Fortification an effect has made a creature included,
    /// on a host it could otherwise hold on to.
    ///
    /// Only the second sentence used to be read. It asks about permanents
    /// that are neither an Aura, an Equipment nor a Fortification, so the
    /// animated Equipment and the animated Fortification stayed attached.
    /// The counter-test sits on the same creature: an Equipment that is only
    /// an Equipment stays on, so what takes the other one off is being a
    /// creature and not the host.
    #[test]
    fn a_creature_or_battle_attached_to_anything_comes_off() {
        use baylee_core::generated::subtypes::artifact::{EQUIPMENT, FORTIFICATION};
        let mut state =
            GameState::from_preset(&empty_boards_preset(14), &RegistryLookup).expect("game starts");
        let host = creature(&mut state);
        let land = bare(&mut state, "Test Land", TypeSet::LAND, &[]);
        let plain = equipment(&mut state);
        let animated = bare(
            &mut state,
            "Animated Equipment",
            TypeSet::ARTIFACT.union(TypeSet::CREATURE),
            &[EQUIPMENT],
        );
        let fortification = bare(
            &mut state,
            "Animated Fortification",
            TypeSet::ARTIFACT.union(TypeSet::CREATURE),
            &[FORTIFICATION],
        );
        let battle = bare(&mut state, "Test Battle", TypeSet::BATTLE, &[]);
        for (what, to) in [
            (plain, host),
            (animated, host),
            (fortification, land),
            (battle, host),
        ] {
            attach(&mut state, what, to);
        }

        run(&mut state, &RegistryLookup);
        for (id, what) in [
            (animated, "an Equipment that is a creature"),
            (fortification, "a Fortification that is a creature"),
            (battle, "a battle"),
        ] {
            let obj = state.object(id).expect("never destroyed");
            assert_eq!(obj.attached_to, None, "{what} becomes unattached");
            assert!(
                on_battlefield(&state, id),
                "{what} remains on the battlefield"
            );
        }
        assert_eq!(
            state.object(plain).expect("in play").attached_to,
            Some(host),
            "while an Equipment that is only an Equipment stays on the same creature"
        );
    }

    /// An attachment that is *also* a creature needs no host: a living
    /// weapon whose germ has died is an Equipment creature on the
    /// battlefield, and a bestowed Aura that becomes unattached ceases to be
    /// bestowed and stays as the creature it was cast as (CR 702.103f). Only
    /// the non-creature Aura falls into the graveyard, which is what the type
    /// check in front of that arm is for.
    #[test]
    fn an_attachment_that_is_a_creature_stays_without_one() {
        let mut state =
            GameState::from_preset(&empty_boards_preset(13), &RegistryLookup).expect("game starts");
        let living = bare(
            &mut state,
            "Living Weapon",
            TypeSet::ARTIFACT.union(TypeSet::CREATURE),
            &[baylee_core::generated::subtypes::artifact::EQUIPMENT],
        );
        let bestowed = bare(
            &mut state,
            "Bestowed Aura",
            TypeSet::ENCHANTMENT.union(TypeSet::CREATURE),
            &[baylee_core::generated::subtypes::enchantment::AURA],
        );

        // The counterpart, in the same pass and the same state: an
        // ordinary Aura with no host, which is the one thing here that does
        // fall into the graveyard.
        let plain = bare(
            &mut state,
            "Plain Aura",
            TypeSet::ENCHANTMENT,
            &[baylee_core::generated::subtypes::enchantment::AURA],
        );

        run(&mut state, &RegistryLookup);
        assert!(
            on_battlefield(&state, living) && on_battlefield(&state, bestowed),
            "neither is destroyed for having nothing to attach to"
        );
        assert!(
            !on_battlefield(&state, plain),
            "while the Aura that is only an Aura does fall off — the type \
             check in front of that arm is the whole difference"
        );
    }

    /// The candidate queue is drained by every pass, which is what lets
    /// `snapshot_hash` ignore it: two states that played the same game are
    /// equal even though one of them queued and cleared a candidate.
    #[test]
    fn the_cleanup_queue_is_empty_once_the_pass_has_run() {
        let mut state =
            GameState::from_preset(&empty_boards_preset(11), &RegistryLookup).expect("game starts");

        let owner = PlayerId::new(0);
        let name = state.names.intern("Ephemeral");
        let token = state.create_bare(
            owner,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        state
            .move_object(
                token,
                ZoneLocation::Graveyard(owner),
                ZonePosition::Top,
                Cause::StateBased,
            )
            .expect("the token dies");
        run(&mut state, &RegistryLookup);
        assert!(state.object(token).is_none(), "the token is gone");

        // The queue is not hashed, so if a pass left something in it the
        // state would be silently unequal to a replay of the same game.
        // Cloning after the pass and running another one has to be a no-op.
        let mut replay = state.clone();
        run(&mut replay, &RegistryLookup);
        assert_eq!(
            state.snapshot_hash(),
            replay.snapshot_hash(),
            "an empty pass on a settled state changes nothing"
        );
    }

    #[test]
    fn an_indestructible_planeswalker_creature_still_needs_loyalty() {
        let mut state = GameState::from_preset(&empty_boards_preset(17), &RegistryLookup).unwrap();
        let walker = bare(
            &mut state,
            "Animated walker",
            TypeSet::PLANESWALKER.union(TypeSet::CREATURE),
            &[],
        );
        let obj = state.object_mut(walker).unwrap();
        obj.base_mut().keywords = baylee_cards_dsl::KeywordSet::INDESTRUCTIBLE;
        obj.counters.set(CounterKind::Loyalty, 1);
        assert!(!run(&mut state, &RegistryLookup).changed);
        assert!(on_battlefield(&state, walker));
        state
            .object_mut(walker)
            .unwrap()
            .counters
            .set(CounterKind::Loyalty, 0);
        assert!(run(&mut state, &RegistryLookup).changed);
        assert!(
            !on_battlefield(&state, walker),
            "zero loyalty is not destruction"
        );
    }

    #[test]
    fn phased_out_permanents_ignore_lethal_damage_and_zero_loyalty() {
        let mut state = GameState::from_preset(&empty_boards_preset(19), &RegistryLookup).unwrap();
        let creature = creature(&mut state);
        let walker = bare(&mut state, "Absent walker", TypeSet::PLANESWALKER, &[]);
        state.object_mut(creature).unwrap().damage = 2;
        for id in [creature, walker] {
            state
                .object_mut(id)
                .unwrap()
                .status
                .insert(crate::object::Status::PHASED_OUT);
        }
        assert!(!run(&mut state, &RegistryLookup).changed);
        for id in [creature, walker] {
            assert!(on_battlefield(&state, id));
            state
                .object_mut(id)
                .unwrap()
                .status
                .remove(crate::object::Status::PHASED_OUT);
        }
        assert!(run(&mut state, &RegistryLookup).changed);
        assert!(!on_battlefield(&state, creature));
        assert!(!on_battlefield(&state, walker));
    }

    #[test]
    fn phased_out_legend_does_not_conflict_until_it_returns() {
        let mut state = GameState::from_preset(&empty_boards_preset(23), &RegistryLookup).unwrap();
        let a = creature(&mut state);
        let b = creature(&mut state);
        for id in [a, b] {
            state.object_mut(id).unwrap().base_mut().supertypes = SupertypeSet::LEGENDARY;
        }
        state
            .object_mut(b)
            .unwrap()
            .status
            .insert(crate::object::Status::PHASED_OUT);
        assert!(run(&mut state, &RegistryLookup).legend_choice.is_none());
        state
            .object_mut(b)
            .unwrap()
            .status
            .remove(crate::object::Status::PHASED_OUT);
        assert_eq!(
            run(&mut state, &RegistryLookup).legend_choice,
            Some((PlayerId::new(0), vec![a, b]))
        );
    }

    #[test]
    fn phased_out_attachments_and_counters_wait_for_phasing_in() {
        let (mut state, aura) = aura_state();
        let host = creature(&mut state);
        let gear = equipment(&mut state);
        attach(&mut state, gear, host);
        for id in [aura, gear] {
            let obj = state.object_mut(id).unwrap();
            obj.status.insert(crate::object::Status::PHASED_OUT);
            obj.counters.set(CounterKind::P1P1, 2);
            obj.counters.set(CounterKind::M1M1, 1);
        }
        // The host leaves while the equipment is phased out; the aura has no host.
        state
            .move_object(
                host,
                ZoneLocation::Hand(PlayerId::new(0)),
                ZonePosition::Top,
                Cause::Effect,
            )
            .unwrap();
        run(&mut state, &RegistryLookup);
        for id in [aura, gear] {
            assert!(on_battlefield(&state, id));
            let obj = state.object_mut(id).unwrap();
            assert_eq!(obj.counters.get(CounterKind::P1P1), 2);
            assert_eq!(obj.counters.get(CounterKind::M1M1), 1);
            obj.status.remove(crate::object::Status::PHASED_OUT);
        }
        assert_eq!(state.object(gear).unwrap().attached_to, Some(host));
        run(&mut state, &RegistryLookup);
        assert!(!on_battlefield(&state, aura));
        let gear = state.object(gear).unwrap();
        assert_eq!(gear.attached_to, None);
        assert_eq!(gear.counters.get(CounterKind::P1P1), 1);
        assert_eq!(gear.counters.get(CounterKind::M1M1), 0);
    }
    #[test]
    fn phasing_does_not_extend_the_deathtouch_damage_window() {
        let mut state = GameState::from_preset(&empty_boards_preset(29), &RegistryLookup).unwrap();
        let creature = creature(&mut state);
        let obj = state.object_mut(creature).unwrap();
        obj.damage = 1;
        obj.deathtouched = true;
        obj.status.insert(Status::PHASED_OUT);
        run(&mut state, &RegistryLookup);
        assert!(on_battlefield(&state, creature));
        assert!(!state.object(creature).unwrap().deathtouched);
        state
            .object_mut(creature)
            .unwrap()
            .status
            .remove(Status::PHASED_OUT);
        run(&mut state, &RegistryLookup);
        assert!(
            on_battlefield(&state, creature),
            "deathtouch only applies since the last SBA check"
        );
    }

    /// The seat that lost keeps why, for anything reading the state after
    /// the event that said so has scrolled past; the seat that did not lose
    /// carries nothing.
    #[test]
    fn a_seat_that_loses_keeps_why() {
        let mut state =
            GameState::from_preset(&empty_boards_preset(31), &RegistryLookup).expect("game starts");
        state.players[1].poison = 10;
        let _ = run(&mut state, &RegistryLookup);
        assert_eq!(state.players[1].loss, Some(LossReason::Poison));
        assert!(state.players[1].has_lost());
        assert_eq!(state.players[0].loss, None);
        assert!(!state.players[0].has_lost());
    }

    /// `seats` empty boards.
    fn empty_table(seed: u64, seats: usize) -> GameState {
        let mut preset = empty_boards_preset(seed);
        let seat = preset.seats[0].clone();
        preset.seats.resize(seats, seat);
        GameState::from_preset(&preset, &RegistryLookup).expect("game starts")
    }

    /// The monarch leaves the game, and the crown passes at the same time
    /// (CR 724.4). During another player's turn it goes to the active player,
    /// not to the next seat after the leaver; during the monarch's own turn,
    /// to the next player in turn order still in the game; and with nobody
    /// left who can take it, to nobody. A player who is not the monarch
    /// leaving moves nothing.
    ///
    /// The crown stayed with the player who had left.
    #[test]
    fn the_crown_passes_as_the_monarch_leaves_the_game() {
        let seat = PlayerId::new;
        let mut state = empty_table(43, 4);

        state.monarch = Some(seat(1));
        state.turn.active = seat(3);
        eliminate_player(&mut state, seat(1), LossReason::Conceded);
        assert_eq!(
            state.monarch,
            Some(seat(3)),
            "the active player, not seat 2"
        );

        state.monarch = Some(seat(0));
        state.turn.active = seat(0);
        eliminate_player(&mut state, seat(0), LossReason::Conceded);
        assert_eq!(
            state.monarch,
            Some(seat(2)),
            "the next in turn order still in the game: seat 1 has left"
        );

        eliminate_player(&mut state, seat(3), LossReason::Conceded);
        assert_eq!(state.monarch, Some(seat(2)), "seat 3 was not the monarch");

        eliminate_player(&mut state, seat(2), LossReason::Conceded);
        assert_eq!(state.monarch, None, "nobody is left to take it");
    }

    /// A player loses the game once. A seat already out that is eliminated
    /// again (a concession arriving after the SBA) keeps the reason it
    /// actually lost to.
    #[test]
    fn a_second_elimination_does_not_rewrite_the_first() {
        let mut state =
            GameState::from_preset(&empty_boards_preset(37), &RegistryLookup).expect("game starts");
        eliminate_player(&mut state, PlayerId::new(1), LossReason::Life);
        eliminate_player(&mut state, PlayerId::new(1), LossReason::Conceded);
        assert_eq!(state.players[1].loss, Some(LossReason::Life));
    }

    /// Two engines that put the same seat out for different reasons have
    /// diverged, and the determinism hash is what a replay and a
    /// cross-machine comparison compare, so it tells them apart. The loop
    /// signature does not need to: a loss cannot repeat inside a loop.
    #[test]
    fn the_snapshot_hash_tells_two_losses_apart() {
        let base =
            GameState::from_preset(&empty_boards_preset(41), &RegistryLookup).expect("game starts");
        let lost = |reason| {
            let mut state = base.clone();
            eliminate_player(&mut state, PlayerId::new(1), reason);
            state
        };
        let (life, conceded) = (lost(LossReason::Life), lost(LossReason::Conceded));
        assert_ne!(life.snapshot_hash(), conceded.snapshot_hash());
        assert_eq!(life.loop_signature(), conceded.loop_signature());
    }
}
