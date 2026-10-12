//! External entry replacement candidates, sharing one order with printed clauses.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_cards_dsl::ReplacementRule;
use baylee_core::types::TypeSet;

fn entry_rule(rule: ReplacementRule) -> bool {
    matches!(
        rule,
        ReplacementRule::LandEquilibrium
            | ReplacementRule::EnterTapped(_)
            | ReplacementRule::EnterUntapped(_)
    )
}

pub(super) fn has_coarrival_statics(state: &GameState) -> bool {
    state.effects.iter().any(|effect| {
        effect.origin == crate::effects::EffectOrigin::Static
            && effect
                .source
                .is_some_and(|source| unavailable(state).contains(&source))
    })
}

fn rules_before_entry(state: &GameState) -> Vec<crate::state::ReplacementEntry> {
    let mut rules: Vec<_> = state
        .replacement_rules
        .iter()
        .copied()
        .filter(|rule| entry_rule(rule.rule))
        .collect();
    // Normally the registered index is complete. Only an internally committed
    // co-arrival's static can have hidden pre-event text during a rules sync.
    if has_coarrival_statics(state) {
        for id in state
            .battlefield_seen()
            .filter(|id| !unavailable(state).contains(id))
        {
            let object = state.object(id).expect("battlefield object");
            for ability in object.printed_abilities(state.bases.as_ref()) {
                if let AbilityDef::Replacement(rule) = ability
                    && entry_rule(*rule)
                    && !rules
                        .iter()
                        .any(|existing| existing.source == id && existing.rule == *rule)
                {
                    rules.push(crate::state::ReplacementEntry {
                        source: id,
                        controller: object.controller,
                        rule: *rule,
                    });
                }
            }
        }
    }
    rules
}

pub(super) fn exists(state: &GameState) -> bool {
    state
        .replacement_rules
        .iter()
        .any(|rule| entry_rule(rule.rule))
        || (has_coarrival_statics(state) && !rules_before_entry(state).is_empty())
}

fn lands(state: &GameState, player: PlayerId, excluded: &BTreeSet<ObjectId>) -> usize {
    state
        .battlefield_seen()
        .filter(|id| {
            !excluded.contains(id)
                && state.object(*id).is_some_and(|o| {
                    o.controller == player
                        && !o.status.contains(crate::object::Status::PHASED_OUT)
                        && o.characteristics().types.contains(TypeSet::LAND)
                })
        })
        .count()
}

fn applicable(
    state: &GameState,
    entry: &mut Attempt,
) -> Vec<(baylee_core::ids::DamageSourceRef, ReplacementRule)> {
    if !exists(state) {
        return Vec::new();
    }
    let Some(object) = state.object(entry.object) else {
        return Vec::new();
    };
    let mut prospective = object.clone();
    prospective.zone = Zone::Battlefield;
    prospective.version = prospective.version.wrapping_add(1);
    let excluded = unavailable(state);
    let plan = crate::layers::LayerPlan::before_entries(&state.effects, excluded);
    let projection = crate::layers::recompute_with(state, &prospective, &plan);
    prospective.controller = projection.controller;
    entry.controller = projection.controller;
    rules_before_entry(state)
        .iter()
        .filter_map(|rule| {
            if excluded.contains(&rule.source) || rule.source == entry.object {
                return None;
            }
            let source = state.object(rule.source)?;
            let source_projection = crate::layers::recompute_with(state, source, &plan);
            if source.zone != Zone::Battlefield
                || source.status.contains(crate::object::Status::PHASED_OUT)
                || source_projection.characteristics.rules_text_lost
                || source_projection.characteristics.abilities_lost.is_some()
            {
                return None;
            }
            let identity = baylee_core::ids::DamageSourceRef {
                object: source.id,
                version: source.version,
            };
            if entry.applied.contains(&(identity, rule.rule)) {
                return None;
            }
            let controller = source_projection.controller;
            let matches = match rule.rule {
                ReplacementRule::LandEquilibrium => {
                    projection.characteristics.types.contains(TypeSet::LAND)
                        && state.is_opponent(entry.putter, controller)
                        && lands(state, entry.putter, excluded)
                            >= lands(state, controller, excluded)
                }
                ReplacementRule::EnterTapped(filter) | ReplacementRule::EnterUntapped(filter) => {
                    crate::eval::matches_projected(
                        filter,
                        state,
                        &prospective,
                        &projection.characteristics,
                        controller,
                        rule.source,
                    )
                }
                _ => false,
            };
            matches.then_some((identity, rule.rule))
        })
        .collect()
}

fn apply(state: &GameState, entry: &mut Attempt, source: ObjectId) {
    // Different relevant rules on one source are offered again after each application.
    let Some((identity, rule)) = applicable(state, entry)
        .into_iter()
        .find(|(identity, _)| identity.object == source)
    else {
        return;
    };
    #[cfg(test)]
    if let Some(registered) = state
        .replacement_rules
        .iter()
        .find(|r| r.source == source && r.rule == rule)
    {
        crate::ability_log::replaced(registered);
    }
    entry.applied.push((identity, rule));
    match rule {
        ReplacementRule::LandEquilibrium => entry.sacrifices_after += 1,
        ReplacementRule::EnterTapped(_) => entry.tapped = true,
        ReplacementRule::EnterUntapped(_) => entry.tapped = false,
        _ => unreachable!("entry replacement candidate"),
    }
}

pub(super) fn replacement_order(state: &GameState, entry: &mut Attempt) -> Option<Pending> {
    if std::mem::take(&mut entry.own_first) {
        return None;
    }
    let candidates = applicable(state, entry);
    if candidates.is_empty() {
        return None;
    }
    let own = entry.modifiers[entry.next..].iter().any(needs_before_entry);
    if !own && candidates.len() == 1 {
        apply(state, entry, candidates[0].0.object);
        return None;
    }
    // Orientation does not change the land counts Equilibrium compares. Its
    // appended sacrifices therefore commute with printed orientation clauses.
    let own_changes_counts = entry.modifiers[entry.next..]
        .iter()
        .any(|modifier| needs_before_entry(modifier) && !tapping::handles(modifier));
    if !own_changes_counts
        && candidates
            .iter()
            .all(|(_, rule)| *rule == ReplacementRule::LandEquilibrium)
    {
        for (identity, _) in candidates {
            apply(state, entry, identity.object);
        }
        return None;
    }
    let mut options = Vec::with_capacity(candidates.len() + usize::from(own));
    if own {
        options.push(entry.object);
    }
    for (identity, _) in candidates {
        if !options.contains(&identity.object) {
            options.push(identity.object);
        }
    }
    Some(Pending::ChooseCards {
        player: entry.controller,
        options,
        min: 1,
        max: 1,
        prompt: ChoicePrompt::EntryReplacement,
        total: None,
    })
}

fn prepare_order(state: &mut GameState) {
    if state.entries.order_prepared {
        return;
    }
    let active = usize::from(state.turn.active.get());
    let seats = state.players.len();
    state.entries.simple_order = state.entries.pending.iter().all(|entry| {
        !entry.modifiers.iter().any(needs_before_entry)
            && copy_clause(state, entry.object).is_none()
    });
    state
        .entries
        .pending
        .make_contiguous()
        .sort_by_key(|entry| (usize::from(entry.controller.get()) + seats - active) % seats);
    state.entries.order_prepared = true;
}

/// Several arrivals may compete for the same payment objects. The affected
/// players choose in APNAP order; within a player's group, no arena order is imposed.
pub(super) fn entry_order(state: &mut GameState) -> Option<Pending> {
    if state
        .entries
        .pending
        .front()
        .is_some_and(|entry| state.entries.chosen_entry == Some(entry.object))
    {
        return None;
    }
    prepare_order(state);
    let first = state.entries.pending.front()?;
    let controller = first.controller;
    if state.entries.simple_order {
        state.entries.chosen_entry = Some(first.object);
        return None;
    }
    let options: Vec<_> = state
        .entries
        .pending
        .iter()
        .take_while(|entry| entry.controller == controller)
        .map(|entry| entry.object)
        .collect();
    let interacting = state
        .entries
        .pending
        .iter()
        .take_while(|entry| entry.controller == controller)
        .any(|entry| {
            entry.modifiers.iter().any(needs_before_entry)
                || copy_clause(state, entry.object).is_some()
        });
    if options.len() < 2 || !interacting {
        state.entries.chosen_entry = Some(first.object);
        return None;
    }
    state.entries.question = Some(Question::EntryOrder);
    Some(Pending::ChooseCards {
        player: controller,
        options,
        min: 1,
        max: 1,
        prompt: ChoicePrompt::EntryOrder,
        total: None,
    })
}

pub(super) fn answer_special(
    state: &mut GameState,
    question: &Question,
    action: &PlayerAction,
) -> bool {
    if !matches!(
        question,
        Question::Replacement | Question::EntryOrder | Question::AfterSacrifice
    ) {
        return false;
    }
    let PlayerAction::ChooseObjects { objects } = action else {
        unreachable!("validated selection")
    };
    let chosen = objects[0];
    match question {
        Question::Replacement => {
            let mut entry = state
                .entries
                .pending
                .pop_front()
                .expect("replacement entry");
            if chosen == entry.object {
                entry.own_first = true;
            } else {
                apply(state, &mut entry, chosen);
            }
            state.entries.pending.push_front(entry);
        }
        Question::EntryOrder => {
            let index = state
                .entries
                .pending
                .iter()
                .position(|entry| entry.object == chosen)
                .expect("offered entry");
            let entry = state.entries.pending.remove(index).expect("entry at index");
            state.entries.pending.push_front(entry);
            state.entries.chosen_entry = Some(chosen);
        }
        Question::AfterSacrifice => {
            sacrifice(state, &[chosen]);
            let job = state.entries.after.front_mut().expect("appended sacrifice");
            job.remaining -= 1;
            if job.remaining == 0 {
                state.entries.after.pop_front();
            }
        }
        _ => unreachable!("handled entry question"),
    }
    true
}

/// Called only after all arrivals' ordinary entry modifiers have settled.
/// CR 614.13 and the card ruling allow sacrificing the newly arrived land;
/// this happens without priority, so it cannot first be tapped for mana.
pub(crate) fn after_entry(state: &mut GameState) -> Option<Pending> {
    while let Some(job) = state.entries.after.front() {
        let player = job.player;
        let options: Vec<_> = state
            .battlefield_seen()
            .filter(|id| {
                state.object(*id).is_some_and(|o| {
                    o.controller == player
                        && o.characteristics().types.contains(TypeSet::LAND)
                        && !o.status.contains(crate::object::Status::PHASED_OUT)
                })
            })
            .collect();
        if options.is_empty() || state.has_left(player) {
            state.entries.after.pop_front();
            continue;
        }
        state.entries.question = Some(Question::AfterSacrifice);
        return Some(Pending::ChooseCards {
            player,
            options,
            min: 1,
            max: 1,
            prompt: ChoicePrompt::SacrificeFor { player },
            total: None,
        });
    }
    None
}
