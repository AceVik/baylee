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
            | ReplacementRule::EnterTappedIf { .. }
            | ReplacementRule::EnterUntappedIf { .. }
            | ReplacementRule::EnterAsAdditionalTypesAndPt { .. }
    )
}

fn entry_modifier(modifier: baylee_cards_dsl::Modifier) -> bool {
    matches!(
        modifier,
        baylee_cards_dsl::Modifier::EnterUnderYourControl(_)
            | baylee_cards_dsl::Modifier::ReplaceBattlefieldEntry { .. }
            | baylee_cards_dsl::Modifier::PermanentsEnterTapped(_)
    )
}

fn has_self_entry_replace(state: &GameState, id: ObjectId) -> bool {
    state.object(id).is_some_and(|object| {
        object
            .printed_abilities(state.bases.as_ref())
            .iter()
            .any(|ability| {
                matches!(
                    ability,
                    AbilityDef::Static(sa)
                        if matches!(
                            sa.modifier,
                            baylee_cards_dsl::Modifier::ReplaceBattlefieldEntry { filter, .. }
                                if *filter == Filter::This
                        )
                )
            })
    })
}

fn entry_predicate_holds(
    entry: &Attempt,
    object: &crate::object::GameObject,
    predicate: baylee_cards_dsl::EntryPredicate,
) -> bool {
    // CR 707.10f: A copy of a spell on the stack wasn't cast (even after
    // `a_copy_becomes_a_token` strips `Rider::SpellCopy`, its `cast_history`
    // has `caster() == None`), while a cast copy of a card (CR 707.12) has
    // `cast_history.caster().is_some()`.
    let was_cast = entry.cause == Cause::Spell
        && object.cast_history.caster().is_some()
        && !object.riders.contains(&crate::object::Rider::SpellCopy);
    match predicate {
        baylee_cards_dsl::EntryPredicate::NotCast => !was_cast,
        baylee_cards_dsl::EntryPredicate::NotCastOrNoManaSpent => {
            !was_cast || object.paid.as_ref().map_or(0, |p| p.mana_spent) == 0
        }
        baylee_cards_dsl::EntryPredicate::FromExileOrCastFromExile => {
            object.zone == Zone::Exile
                || (was_cast
                    && crate::eval::object_was_cast_from_zone(
                        object,
                        baylee_cards_dsl::ZoneRef::Exile,
                    ))
        }
    }
}

#[derive(Clone, Copy)]
enum EntryCandidate {
    Rule(baylee_core::ids::DamageSourceRef, ReplacementRule),
    Effect {
        effect_id: baylee_core::ids::EffectId,
        source: ObjectId,
        controller: PlayerId,
        modifier: baylee_cards_dsl::Modifier,
    },
    SelfReplace {
        object: ObjectId,
        destination: baylee_cards_dsl::EntryInsteadZone,
    },
}

impl EntryCandidate {
    const fn option_object(self) -> ObjectId {
        match self {
            Self::Rule(identity, _) => identity.object,
            Self::Effect { source, .. } => source,
            Self::SelfReplace { object, .. } => object,
        }
    }

    const fn is_land_equilibrium(self) -> bool {
        matches!(self, Self::Rule(_, ReplacementRule::LandEquilibrium))
    }

    const fn is_control_changing(self) -> bool {
        matches!(
            self,
            Self::Effect {
                modifier: baylee_cards_dsl::Modifier::EnterUnderYourControl(_),
                ..
            }
        )
    }
}

fn has_pending_own_control(entry: &Attempt) -> bool {
    !entry.applied_own_control
        && entry.modifiers[entry.next..]
            .iter()
            .any(|m| matches!(m, EnterModifier::EntersUnderChosenOpponentControl))
}

pub(super) fn apply_own_control(
    state: &GameState,
    entry: &mut Attempt,
) -> Option<(Question, Pending)> {
    entry.applied_own_control = true;
    let opponents = crate::eval::players(
        baylee_cards_dsl::PlayerRel::Opponent,
        state,
        entry.controller,
    )
    .unwrap_or_default();
    match opponents.as_slice() {
        [] => None,
        [opp] => {
            entry.controller_override = Some(*opp);
            entry.controller = *opp;
            None
        }
        _ => Some((
            Question::EntryControllerOpponent,
            Pending::ChoosePlayer {
                player: entry.controller,
                options: opponents,
            },
        )),
    }
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

pub(super) fn exists_for(state: &GameState, id: ObjectId) -> bool {
    exists(state)
        || state.effects.iter().any(|fx| entry_modifier(fx.modifier))
        || has_self_entry_replace(state, id)
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

#[allow(clippy::too_many_lines)]
fn applicable(state: &GameState, entry: &mut Attempt) -> Vec<EntryCandidate> {
    if entry.is_cancelled() || !exists_for(state, entry.object) {
        return Vec::new();
    }
    let Some(object) = state.object(entry.object) else {
        return Vec::new();
    };
    let mut prospective = super::prospective_battlefield(object);
    if let Some(ctrl) = entry.controller_override {
        prospective.set_controller(ctrl);
    }
    if !entry.entry_type_pt.is_empty() {
        let mut chars = (*prospective.base).clone();
        for &(add_types, add_subtypes, power, toughness) in &entry.entry_type_pt {
            chars.types = chars.types.union(add_types);
            for &st in add_subtypes {
                chars.subtypes.insert(st);
            }
            chars.power = Some(power);
            chars.toughness = Some(toughness);
        }
        prospective.base = std::sync::Arc::new(chars);
    }
    let excluded = unavailable(state);
    let plan = crate::layers::LayerPlan::before_entries(&state.effects, excluded);
    let projection = crate::layers::recompute_with(state, &prospective, &plan);
    if let Some(ctrl) = entry.controller_override {
        prospective.controller = ctrl;
        entry.controller = ctrl;
    } else {
        prospective.controller = projection.controller;
        entry.controller = projection.controller;
    }
    let mut out = Vec::new();
    for rule in rules_before_entry(state) {
        if excluded.contains(&rule.source) || rule.source == entry.object {
            continue;
        }
        let Some(source) = state.object(rule.source) else {
            continue;
        };
        let source_projection = crate::layers::recompute_with(state, source, &plan);
        if source.zone != Zone::Battlefield
            || source.status.contains(crate::object::Status::PHASED_OUT)
            || source_projection.characteristics.rules_text_lost
            || source_projection.characteristics.abilities_lost.is_some()
        {
            continue;
        }
        let identity = baylee_core::ids::DamageSourceRef {
            object: source.id,
            version: source.version,
        };
        if entry.applied.contains(&(identity, rule.rule)) {
            continue;
        }
        let controller = source_projection.controller;
        let matches = match rule.rule {
            ReplacementRule::LandEquilibrium => {
                projection.characteristics.types.contains(TypeSet::LAND)
                    && state.is_opponent(entry.putter, controller)
                    && lands(state, entry.putter, excluded) >= lands(state, controller, excluded)
            }
            ReplacementRule::EnterTapped(filter)
            | ReplacementRule::EnterUntapped(filter)
            | ReplacementRule::EnterAsAdditionalTypesAndPt { filter, .. } => {
                crate::eval::matches_projected(
                    filter,
                    state,
                    &prospective,
                    &projection.characteristics,
                    controller,
                    rule.source,
                )
            }
            ReplacementRule::EnterTappedIf { filter, condition }
            | ReplacementRule::EnterUntappedIf { filter, condition } => {
                crate::eval::condition_holds(state, controller, rule.source, condition)
                    && crate::eval::matches_projected(
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
        if matches {
            out.push(EntryCandidate::Rule(identity, rule.rule));
        }
    }
    for fx in state.effects.iter() {
        if !entry_modifier(fx.modifier) || entry.applied_effects.contains(&fx.id) {
            continue;
        }
        if let Some(src) = fx.source
            && (excluded.contains(&src) || src == entry.object)
        {
            continue;
        }
        if fx.origin == crate::effects::EffectOrigin::Static
            && let Some(src) = fx.source
        {
            let Some(source) = state.object(src) else {
                continue;
            };
            let source_projection = crate::layers::recompute_with(state, source, &plan);
            if source.zone != Zone::Battlefield
                || source.status.contains(crate::object::Status::PHASED_OUT)
                || source_projection.characteristics.rules_text_lost
                || source_projection.characteristics.abilities_lost.is_some()
            {
                continue;
            }
        }
        let source_id = fx.source.unwrap_or(entry.object);
        let matches = match fx.modifier {
            baylee_cards_dsl::Modifier::PermanentsEnterTapped(filter) => {
                crate::eval::matches_projected(
                    filter,
                    state,
                    &prospective,
                    &projection.characteristics,
                    fx.controller,
                    source_id,
                )
            }
            baylee_cards_dsl::Modifier::EnterUnderYourControl(filter) => {
                !state.has_left(fx.controller)
                    && state.is_opponent(prospective.controller, fx.controller)
                    && crate::eval::matches_projected(
                        filter,
                        state,
                        &prospective,
                        &projection.characteristics,
                        fx.controller,
                        source_id,
                    )
            }
            baylee_cards_dsl::Modifier::ReplaceBattlefieldEntry {
                filter, predicate, ..
            } => {
                entry_predicate_holds(entry, object, predicate)
                    && crate::eval::matches_projected(
                        filter,
                        state,
                        &prospective,
                        &projection.characteristics,
                        fx.controller,
                        source_id,
                    )
            }
            _ => false,
        };
        if matches {
            out.push(EntryCandidate::Effect {
                effect_id: fx.id,
                source: source_id,
                controller: fx.controller,
                modifier: fx.modifier,
            });
        }
    }
    if !entry.applied_self_replace
        && !projection.characteristics.rules_text_lost
        && projection.characteristics.abilities_lost.is_none()
    {
        for ability in object.printed_abilities(state.bases.as_ref()) {
            if let AbilityDef::Static(sa) = ability
                && let baylee_cards_dsl::Modifier::ReplaceBattlefieldEntry {
                    filter,
                    predicate,
                    destination,
                } = sa.modifier
                && *filter == Filter::This
                && entry_predicate_holds(entry, object, predicate)
            {
                out.push(EntryCandidate::SelfReplace {
                    object: entry.object,
                    destination,
                });
                break;
            }
        }
    }
    out
}

fn apply_candidate(state: &GameState, entry: &mut Attempt, candidate: EntryCandidate) {
    #[cfg(not(any(test, feature = "test-support")))]
    let _ = state;
    match candidate {
        EntryCandidate::Rule(identity, rule) => {
            #[cfg(any(test, feature = "test-support"))]
            if let Some(registered) = state
                .replacement_rules
                .iter()
                .find(|r| r.source == identity.object && r.rule == rule)
            {
                crate::ability_log::replaced(registered);
            }
            entry.applied.push((identity, rule));
            match rule {
                ReplacementRule::LandEquilibrium => entry.sacrifices_after += 1,
                ReplacementRule::EnterTapped(_) | ReplacementRule::EnterTappedIf { .. } => {
                    entry.tapped = true;
                }
                ReplacementRule::EnterUntapped(_) | ReplacementRule::EnterUntappedIf { .. } => {
                    entry.tapped = false;
                }
                ReplacementRule::EnterAsAdditionalTypesAndPt {
                    add_types,
                    add_subtypes,
                    power,
                    toughness,
                    ..
                } => {
                    entry
                        .entry_type_pt
                        .push((add_types, add_subtypes, power, toughness));
                }
                _ => unreachable!("entry replacement candidate"),
            }
        }
        EntryCandidate::Effect {
            effect_id,
            controller,
            modifier,
            ..
        } => {
            entry.applied_effects.push(effect_id);
            match modifier {
                baylee_cards_dsl::Modifier::PermanentsEnterTapped(_) => {
                    entry.tapped = true;
                }
                baylee_cards_dsl::Modifier::EnterUnderYourControl(_) => {
                    entry.controller_override = Some(controller);
                    entry.controller = controller;
                }
                baylee_cards_dsl::Modifier::ReplaceBattlefieldEntry { destination, .. } => {
                    entry.instead_destination = Some(destination);
                }
                _ => unreachable!("entry modifier candidate"),
            }
        }
        EntryCandidate::SelfReplace { destination, .. } => {
            entry.applied_self_replace = true;
            entry.instead_destination = Some(destination);
        }
    }
}

fn apply(state: &GameState, entry: &mut Attempt, source: ObjectId) {
    // Different relevant rules on one source are offered again after each application.
    let candidates = applicable(state, entry);
    let control_tier =
        has_pending_own_control(entry) || candidates.iter().any(|c| c.is_control_changing());
    let Some(candidate) = candidates
        .into_iter()
        .filter(|c| !control_tier || c.is_control_changing())
        .find(|candidate| candidate.option_object() == source)
    else {
        return;
    };
    apply_candidate(state, entry, candidate);
}

#[allow(clippy::too_many_lines)]
pub(super) fn replacement_order(
    state: &GameState,
    entry: &mut Attempt,
) -> Option<(Question, Pending)> {
    if has_pending_own_control(entry) && entry.own_first {
        entry.own_first = false;
        if let Some(question) = apply_own_control(state, entry) {
            return Some(question);
        }
    } else if std::mem::take(&mut entry.own_first) {
        return None;
    }
    loop {
        if entry.is_cancelled() {
            return None;
        }
        let candidates = applicable(state, entry);
        // CR 616.1b: Control-changing entry replacements MUST be chosen and
        // applied before non-control entry replacements so subsequent
        // controller-dependent replacements (e.g. Kismet / EnterTapped) see
        // the permanent's updated entering controller.
        let own_control = has_pending_own_control(entry);
        let control_candidates: Vec<_> = candidates
            .iter()
            .copied()
            .filter(|c| c.is_control_changing())
            .collect();
        if own_control || !control_candidates.is_empty() {
            if !own_control && control_candidates.len() == 1 {
                apply_candidate(state, entry, control_candidates[0]);
                continue;
            }
            if own_control && control_candidates.is_empty() {
                if let Some(question) = apply_own_control(state, entry) {
                    return Some(question);
                }
                continue;
            }
            let mut options =
                Vec::with_capacity(control_candidates.len() + usize::from(own_control));
            if own_control {
                options.push(entry.object);
            }
            for candidate in &control_candidates {
                let obj = candidate.option_object();
                if !options.contains(&obj) {
                    options.push(obj);
                }
            }
            if options.len() == 1 {
                if own_control {
                    if let Some(question) = apply_own_control(state, entry) {
                        return Some(question);
                    }
                } else {
                    apply_candidate(state, entry, control_candidates[0]);
                }
                continue;
            }
            return Some((
                Question::Replacement,
                Pending::ChooseCards {
                    player: entry.controller,
                    options,
                    min: 1,
                    max: 1,
                    prompt: ChoicePrompt::EntryReplacement,
                    total: None,
                },
            ));
        }

        if candidates.is_empty() {
            return None;
        }
        let own = entry.modifiers[entry.next..].iter().any(|modifier| {
            needs_before_entry(modifier)
                && !matches!(modifier, EnterModifier::EntersUnderChosenOpponentControl)
        });
        if !own && candidates.len() == 1 {
            apply_candidate(state, entry, candidates[0]);
            continue;
        }
        // Orientation does not change the land counts Equilibrium compares. Its
        // appended sacrifices therefore commute with printed orientation clauses.
        let own_changes_counts = entry.modifiers[entry.next..].iter().any(|modifier| {
            needs_before_entry(modifier)
                && !tapping::handles(modifier)
                && !matches!(modifier, EnterModifier::EntersUnderChosenOpponentControl)
        });
        if !own_changes_counts && candidates.iter().all(|c| c.is_land_equilibrium()) {
            for candidate in candidates {
                apply_candidate(state, entry, candidate);
            }
            return None;
        }
        let mut options = Vec::with_capacity(candidates.len() + usize::from(own));
        if own {
            options.push(entry.object);
        }
        for candidate in &candidates {
            let obj = candidate.option_object();
            if !options.contains(&obj) {
                options.push(obj);
            }
        }
        if options.len() == 1 {
            // Only one distinct option (e.g., own + SelfReplace on entry.object).
            if let Some(self_rep) = candidates
                .iter()
                .copied()
                .find(|c| matches!(c, EntryCandidate::SelfReplace { .. }))
            {
                apply_candidate(state, entry, self_rep);
                continue;
            }
            apply_candidate(state, entry, candidates[0]);
            continue;
        }
        return Some((
            Question::Replacement,
            Pending::ChooseCards {
                player: entry.controller,
                options,
                min: 1,
                max: 1,
                prompt: ChoicePrompt::EntryReplacement,
                total: None,
            },
        ));
    }
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
            let own = has_pending_own_control(&entry)
                || entry.modifiers[entry.next..].iter().any(|modifier| {
                    needs_before_entry(modifier)
                        && !matches!(modifier, EnterModifier::EntersUnderChosenOpponentControl)
                });
            if chosen == entry.object && own {
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
