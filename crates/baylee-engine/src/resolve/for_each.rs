//! Instructions run once for each of several objects: each permanent a
//! filter matches (`Effect::ForEachMatching`) or each target
//! (`Effect::ForEachTarget`).
//!
//! "For each creature, its controller …" is answered once, as the
//! instruction begins (CR 608.2h): the permanents are counted then, each
//! with the player who controls it. Their controllers take their turns in
//! APNAP order (CR 101.4), the active player's permanents first and then
//! each other player's in turn order, one player's in battlefield order.
//!
//! When the instructions sacrifice (`Effect::SacrificeFilter`, as in Fade
//! Away: "its controller sacrifices a permanent of their choice unless they
//! pay {1}"), the iteration is a [`Count`]. Each player makes all of their
//! choices on their turn — every payment first, then each permanent owed —
//! and nothing is sacrificed until every player has chosen; then everything
//! chosen goes at once (CR 101.4, and Fade Away's ruling of 2004-10-04).
//! So a permanent chosen once is not offered again, and a counted creature
//! that has left meanwhile — chosen for another, or sacrificed for mana to
//! pay for another (CR 605.3a) — still counts: its controller cannot
//! sacrifice one creature to pay for another and so avoid paying for the
//! first. The payments come first so that a permanent chosen to be
//! sacrificed cannot also be sacrificed for mana afterwards.
//!
//! Instructions that do not sacrifice act on the permanent itself (Heroism,
//! Tidal Flats, Dread Wight), and there a permanent that has left before its
//! turn is passed over: what it was is a new object now (CR 400.7). Those
//! iterate as they always did, through `SubjectContext::for_each_remaining`,
//! and hash as they always did.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;
use baylee_cards_dsl::Filter;
use baylee_core::ids::DamageSourceRef;

#[cfg(test)]
thread_local! {
    /// Set, `ForEachMatching` runs as it did before the count: in
    /// battlefield order, each sacrifice made as it is chosen, and a
    /// permanent gone before its turn skipped. The engine as it was, for
    /// `for_each_count_tests`, which holds the change to that and nothing
    /// else, and `recorded_lockstep_tests`.
    pub(crate) static ONE_AT_A_TIME: std::cell::Cell<bool> =
        const { std::cell::Cell::new(false) };
}

/// Whether the engine runs as it did before the count (tests only).
fn one_at_a_time() -> bool {
    #[cfg(test)]
    {
        ONE_AT_A_TIME.with(std::cell::Cell::get)
    }
    #[cfg(not(test))]
    {
        false
    }
}

/// A permanent as it was counted.
#[derive(Clone, Copy, Debug, Hash)]
struct Counted {
    identity: DamageSourceRef,
    /// Who controlled it as the count was taken: the player its instructions
    /// name, even after it has left.
    controller: PlayerId,
    power: i16,
}

/// A `ForEachMatching` whose instructions sacrifice, from its count to the
/// sacrifices it ends with. Carried in `SubjectContext::counted` and hashed
/// with it while it lasts.
#[derive(Clone, Debug, Hash)]
pub(crate) struct Count {
    /// The counted permanents still to come, the next one last.
    remaining: Vec<Counted>,
    /// The player whose turn it is: the controller of the permanent whose
    /// instructions ran last.
    current: Option<PlayerId>,
    /// Sacrifices owed so far on this turn and not chosen yet: who sacrifices
    /// and what they may choose.
    owed: Vec<(PlayerId, &'static Filter)>,
    /// What has been chosen, and by whom: sacrificed together once every
    /// player has chosen.
    held: Vec<(PlayerId, DamageSourceRef)>,
}

/// One turn of the iteration: the instructions for the next object are
/// spliced in after `op`, followed by `op` again while there is more to do.
/// `Some` is a question this turn of a [`Count`] asks.
pub(super) fn step(
    state: &mut GameState,
    res: &mut Resolution,
    effects: &'static [Effect],
    op: Effect,
) -> Option<Pending> {
    if let Effect::ForEachMatching { filter, .. } = op
        && !one_at_a_time()
        && (res.subject.counted.is_some() || sacrifices(effects))
    {
        if res.subject.counted.is_none() {
            let mut remaining = count(state, res, filter);
            remaining.reverse();
            res.subject.counted = Some(Box::new(Count {
                remaining,
                current: None,
                owed: Vec::new(),
                held: Vec::new(),
            }));
        }
        return advance(state, res, effects, op);
    }
    one_by_one(state, res, effects, op);
    None
}

/// Whether `effects` sacrifice through `Effect::SacrificeFilter`, which a
/// [`Count`] holds until every player has chosen.
fn sacrifices(effects: &'static [Effect]) -> bool {
    let mut found = false;
    Effect::walk(effects, &mut 0, &mut |effect| {
        found |= matches!(effect, Effect::SacrificeFilter { .. });
    });
    found
}

/// The permanents `filter` matches, in the order their controllers take
/// their turns (CR 101.4); one player's in battlefield order.
fn count(state: &GameState, res: &Resolution, filter: &'static Filter) -> Vec<Counted> {
    let context = res.rule_context();
    let mut counted: Vec<Counted> = state
        .battlefield_seen()
        .filter_map(|id| {
            let o = state.object(id)?;
            eval::matches_with_context(filter, state, o, res.controller, context).then(|| Counted {
                identity: DamageSourceRef {
                    object: id,
                    version: o.version,
                },
                controller: o.controller,
                power: o.characteristics().power.unwrap_or(0),
            })
        })
        .collect();
    counted.sort_by_key(|c| crate::battle::sba::apnap_distance(state, c.controller));
    counted
}

/// The next thing a [`Count`] does: ask what the turn that just ended owes,
/// run the next permanent's instructions, or, with nothing left, sacrifice
/// everything chosen at once.
fn advance(
    state: &mut GameState,
    res: &mut Resolution,
    effects: &'static [Effect],
    op: Effect,
) -> Option<Pending> {
    loop {
        let count = res.subject.counted.as_deref_mut()?;
        let next = count.remaining.last().copied();
        let turn_over = next.is_none_or(|next| Some(next.controller) != count.current);
        if turn_over && !count.owed.is_empty() {
            let (player, filter) = count.owed.remove(0);
            let mut options = chosen::options(state, player, filter, res.controller, res.source);
            options.retain(|id| !count.held.iter().any(|(_, held)| held.object == *id));
            // As much as can be done (CR 609.3): with nothing left to give
            // up, nothing more is asked.
            if options.is_empty() || state.has_left(player) {
                continue;
            }
            res.effects.insert(res.pc + 1, op);
            res.awaiting = Some(AwaitingOp::HeldSacrifice { player });
            return Some(Pending::ChooseCards {
                player,
                options,
                min: 1,
                max: 1,
                prompt: ChoicePrompt::Generic,
                total: None,
            });
        }
        if let Some(next) = count.remaining.pop() {
            count.current = Some(next.controller);
            if state.has_left(next.controller) {
                continue;
            }
            res.event_object = Some(next.identity.object);
            res.subject
                .set_event_identity(Some((next.identity, next.power)));
            res.subject.set_event_player(Some(next.controller));
            let mut program = flatten(effects);
            program.push(op);
            let at = res.pc + 1;
            res.effects.splice(at..at, program);
            return None;
        }
        let count = res.subject.counted.take()?;
        // A permanent chosen is sacrificed by the player who chose it, and
        // only while it is still theirs (CR 701.21a).
        let victims: Vec<ObjectId> = count
            .held
            .iter()
            .filter(|(player, held)| {
                state.object(held.object).is_some_and(|o| {
                    o.zone == crate::zone::Zone::Battlefield
                        && o.version == held.version
                        && o.controller == *player
                })
            })
            .map(|(_, held)| held.object)
            .collect();
        sacrifice_together(state, &victims);
        return None;
    }
}

/// `Effect::SacrificeFilter` during a [`Count`]: each player `who` names owes
/// one permanent matching `filter`, chosen when that player's turn ends and
/// sacrificed with everything else chosen. `false` when no count is open.
pub(super) fn hold(
    state: &GameState,
    res: &mut Resolution,
    who: PlayerRel,
    filter: &'static Filter,
) -> bool {
    if res.subject.counted.is_none() {
        return false;
    }
    let players = players_of(who, state, res.controller, res);
    let count = res.subject.counted.as_deref_mut().expect("checked above");
    count
        .owed
        .extend(players.into_iter().map(|player| (player, filter)));
    true
}

/// The answer to a held sacrifice's question: `player` gives up the chosen
/// permanent, once everyone has chosen.
pub(super) fn held(state: &GameState, res: &mut Resolution, player: PlayerId, chosen: &[ObjectId]) {
    let Some(identity) = chosen.first().and_then(|&id| {
        state.object(id).map(|o| DamageSourceRef {
            object: id,
            version: o.version,
        })
    }) else {
        return;
    };
    if let Some(count) = res.subject.counted.as_deref_mut() {
        count.held.push((player, identity));
    }
}

/// The iteration as it always ran, for targets and for instructions that do
/// not sacrifice: the objects are listed as it begins and one is taken per
/// turn, an object that has left in the meantime passed over.
fn one_by_one(state: &GameState, res: &mut Resolution, effects: &'static [Effect], op: Effect) {
    let you = res.controller;
    let mut remaining = res.subject.for_each_remaining.take().unwrap_or_else(|| {
        let context = res.rule_context();
        if matches!(op, Effect::ForEachTarget { .. }) {
            let mut targets: Vec<_> = res
                .targets
                .iter()
                .filter_map(|id| subjects::target(state, res, *id))
                .collect();
            targets.reverse();
            return targets;
        }
        let Effect::ForEachMatching { filter, .. } = op else {
            unreachable!("iteration instruction");
        };
        let mut matched: Vec<(DamageSourceRef, PlayerId)> = state
            .battlefield_seen()
            .filter_map(|id| {
                let o = state.object(id)?;
                eval::matches_with_context(filter, state, o, you, context).then_some((
                    DamageSourceRef {
                        object: id,
                        version: o.version,
                    },
                    o.controller,
                ))
            })
            .collect();
        // CR 101.4 for whatever the instructions ask. No permanent this
        // reaches in the pool is matched under two controllers at a table
        // of two, so the order there is the battlefield's, as it was.
        if !one_at_a_time() {
            matched.sort_by_key(|(_, controller)| {
                crate::battle::sba::apnap_distance(state, *controller)
            });
        }
        matched.reverse();
        matched.into_iter().map(|(identity, _)| identity).collect()
    });
    while let Some(next_ref) = remaining.pop() {
        let Some(obj) = state
            .object(next_ref.object)
            .filter(|o| o.zone == crate::zone::Zone::Battlefield && o.version == next_ref.version)
        else {
            continue;
        };
        let controller = obj.controller;
        let power = obj.characteristics().power.unwrap_or(0);
        res.event_object = Some(next_ref.object);
        res.subject.set_event_identity(Some((next_ref, power)));
        res.subject.set_event_player(Some(controller));
        let mut spliced = flatten(effects);
        if !remaining.is_empty() {
            res.subject.for_each_remaining = Some(remaining);
            spliced.push(op);
        }
        let at = res.pc + 1;
        res.effects.splice(at..at, spliced);
        return;
    }
}
