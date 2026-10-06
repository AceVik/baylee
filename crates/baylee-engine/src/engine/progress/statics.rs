//! The static abilities and replacement rules of what is on the battlefield, kept in step with it.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;

impl<L: CardLookup> Engine<L> {
    /// Keeps the effect table in sync with the battlefield: registers
    /// static abilities of permanents, drops effects whose source left.
    #[allow(clippy::too_many_lines)] // Phasing, lapsed rules and provenance-aware registration form one sync pass.
    pub(crate) fn sync_static_effects(&mut self) {
        use baylee_cards_dsl::Duration;
        // A phased-out permanent's statics apply to nothing (CR 702.26b):
        // set aside while it is phased out, back as they were once it has
        // phased in (CR 702.26d). First, so the departure sweep below also
        // drops what a source that left while phased out had set aside.
        // phasing: this walk is the one looking for phased-out permanents.
        let battlefield = self.state.zones.list(ZoneLocation::Battlefield);
        let phased_out: Vec<ObjectId> = battlefield
            .iter()
            .copied()
            .filter(|&id| {
                self.state
                    .object(id)
                    .is_some_and(|o| o.status.contains(crate::object::Status::PHASED_OUT))
            })
            .collect();
        self.state
            .effects
            .follow_phasing(|source| phased_out.contains(&source));
        // Drop effects whose source left the battlefield (structural
        // anthem removal).
        let gone: Vec<ObjectId> = self
            .state
            .effects
            .iter()
            .filter_map(|fx| fx.source)
            .filter(|s| {
                self.state
                    .object(*s)
                    .is_none_or(|o| o.zone != Zone::Battlefield)
            })
            .collect();
        self.state.effects.remove_where(|fx| {
            matches!(fx.duration, Duration::WhileSourceOnBattlefield)
                && fx.source.is_some_and(|s| gone.contains(&s))
        });
        // A blink can leave the same arena object on the battlefield with
        // a new rules identity. Its former printed/copied static must end
        // before the replacement registration affects the projection.
        let stale: Vec<_> = self
            .state
            .effects
            .iter()
            .filter_map(|effect| {
                if effect.origin != crate::effects::EffectOrigin::Static
                    || effect.duration != Duration::WhileSourceOnBattlefield
                {
                    return None;
                }
                let (_, origin) = self
                    .state
                    .effect_text_overrides
                    .iter()
                    .find(|(id, _)| *id == effect.id)?;
                let reference = match *origin {
                    crate::text_changes::TextOrigin::Live(source)
                    | crate::text_changes::TextOrigin::Ability { source, .. } => source,
                    crate::text_changes::TextOrigin::Frozen(_) => return None,
                };
                // A gained static may instead quote a different grantor: that
                // identity cannot be compared with the recipient's incarnation.
                (effect.source == Some(reference.object)
                    && self.state.source_identity(reference.object) != Some(reference))
                .then_some(effect.id)
            })
            .collect();
        self.state
            .effects
            .remove_where(|effect| stale.contains(&effect.id));
        end_control_durations(&mut self.state);
        forget_effects_on_moved_objects(&mut self.state);
        // Collect statics of permanents not yet registered (then apply,
        // so the borrow of `state` ends before mutation).
        //
        // This scan and the replacement-rule scan at the bottom of the
        // function both read `GameObject::abilities` rather than the card
        // behind the permanent. It is the third place a permanent is asked
        // what it can do — the offer and the trigger scan had both already
        // been taught the question — and it is the one nobody notices,
        // because a static ability is something the machine registers
        // rather than something a player is offered and refused.
        //
        // Two kinds of copy were failing here, and only one of them for
        // the obvious reason. A token copy has no card at all, so the
        // whole permanent was skipped. A Glasspool Mimic *has* a card and
        // it is the wrong one: `check_copy_on_enter` writes the copied
        // list into `own_abilities` (CR 707.2), and reading the printed
        // face instead registered the Mimic's own statics, of which it has
        // none.
        //
        // A Mimic reanimated or searched onto the battlefield is registered
        // on the pass after the one it arrived on, because
        // `check_copy_on_enter` runs inside `apply_enter_modifiers`, one
        // step *after* this one: the pass it arrives on scans it before it
        // is a copy of anything, and the next pass picks it up — the
        // question below being whether the effect is already registered and
        // not whether the permanent has been looked at.
        //
        // A Mimic that was *cast* no longer reaches that window at all.
        // CR 614.12a puts the choice in front of the arrival, so
        // `finalize_spell` asks while the card is still on the stack and the
        // permanent enters already a copy — there is no pass on which this
        // scan sees it as itself. That is the half of the ordering the spell
        // door stopped needing, and the reason a permanent's own printed
        // statics are composed into the copiable ability list before entry.
        //
        // For the doors that are left, nothing happens in between, which is
        // the part worth knowing and is not luck. `check_copy_on_enter`
        // asks the controller which creature to copy, so it sets a pending
        // and the machine returns on `awaiting_answer` two lines later; the
        // pass that applies the answer begins again at step 0 and reaches
        // this scan and the one at the bottom before `collect_triggers` at
        // step 3. So a Mirror that
        // entered as a Katara *does* multiply the trigger its own arrival
        // caused, and no player is ever offered priority on a board where a
        // copy is missing half its rules text.
        //
        // A token copy is asked nothing and could not be saved that way:
        // `settle_copied_rules_text` hands it its list at step 0, ahead of
        // every scan in the pass, which is why that function runs where it
        // does.
        //
        // A static with a condition on its source (a station symbol's,
        // CR 721.2a) exists only while the condition holds: its effect is
        // registered on the pass that finds it true and removed on the pass
        // that finds it false. Both move the effect generation, which is the
        // projection's cache key, so no filter has to read the source.
        //
        // A permanent that has lost all its abilities (CR 613.1f) keeps only
        // what its statics do in layers 1, 2, 4 and 5: those effects began
        // before layer 6 took the abilities away and go on applying (CR
        // 613.6). The rest — layer 6 on, and every rules effect parked in
        // layer 3 — is gone with the ability, so it lapses like a static
        // whose condition failed. A modifier spanning multiple layers
        // (AnimateNoncreatureArtifact) is registered at its first layer,
        // so its later parts survive with it (CR 613.6). Separate statics
        // remain separate effects and do not share that continuation.
        // That reads the projection, so it is made current first: the effect
        // that took the abilities may have been registered a moment ago.
        self.state.refresh_characteristics();
        // Not a phased-out permanent: its statics wait parked for it, and a
        // condition on it is not asked while it does not exist.
        let ids: Vec<ObjectId> = self.state.battlefield_view();
        let mut to_register = Vec::new();
        let mut lapsed = Vec::new();
        for id in ids {
            let Some(obj) = self.state.object(id) else {
                continue;
            };
            let lost = obj.characteristics().abilities_lost.is_some();
            let text_lost = obj.characteristics().rules_text_lost;
            let list = obj.printed_ability_list(&self.lookup);
            for (index, ability) in list.abilities.iter().enumerate() {
                let AbilityDef::Static(sa) = ability else {
                    continue;
                };
                let identity = baylee_core::ids::DamageSourceRef {
                    object: id,
                    version: obj.version,
                };
                let origin = crate::text_changes::TextOrigin::Ability {
                    source: identity,
                    index: index as u32,
                    base: list.base_text(index),
                };
                let context = crate::text_changes::RuleContext {
                    source: id,
                    text: origin.resolve(&self.state.text_changes),
                };
                let registered = self
                    .state
                    .effects
                    .iter()
                    .find(|fx| {
                        fx.source == Some(id)
                            && fx.origin == crate::effects::EffectOrigin::Static
                            && fx.modifier == sa.modifier
                            && self
                                .state
                                .effect_text_overrides
                                .iter()
                                .find(|(effect, _)| *effect == fx.id)
                                .is_none_or(|(_, text)| *text == origin)
                    })
                    .map(|fx| fx.id);
                let gone_with_the_ability = (lost && !outlives_its_ability(sa.layer))
                    || (text_lost && !outlives_its_rules_text(sa.layer));
                if gone_with_the_ability
                    || sa.condition.is_some_and(|condition| {
                        !crate::eval::condition_holds_with_context(
                            &self.state,
                            obj.controller,
                            context,
                            condition,
                        )
                    })
                {
                    if let Some(effect) = registered {
                        lapsed.push(effect);
                    }
                    continue;
                }
                if let Some(effect) = registered {
                    // Adopt legacy registrations without an origin sidecar.
                    if !self
                        .state
                        .effect_text_overrides
                        .iter()
                        .any(|(id, _)| *id == effect)
                    {
                        to_register.push((None, effect, Some(origin)));
                    }
                    continue;
                }
                to_register.push((
                    Some(crate::effects::ContinuousEffect {
                        id: baylee_core::ids::EffectId::new(0),
                        source: Some(id),
                        controller: obj.controller,
                        origin: crate::effects::EffectOrigin::Static,
                        layer: sa.layer,
                        timestamp: obj.timestamp,
                        duration: Duration::WhileSourceOnBattlefield,
                        filter: crate::effects::EffectFilter::Dsl(&sa.filter),
                        modifier: sa.modifier,
                    }),
                    baylee_core::ids::EffectId::new(0),
                    Some(origin),
                ));
            }
            for (offset, ability) in list.runtime_statics().iter().enumerate() {
                let layer = ability.modifier.layer();
                let origin = crate::text_changes::TextOrigin::Ability {
                    source: baylee_core::ids::DamageSourceRef {
                        object: id,
                        version: obj.version,
                    },
                    index: u32::try_from(list.abilities.len() + offset)
                        .expect("ability index fits u32"),
                    base: ability.base_text,
                };
                let registered = self
                    .state
                    .effects
                    .iter()
                    .find(|fx| {
                        fx.source == Some(id)
                            && fx.origin == crate::effects::EffectOrigin::Static
                            && fx.modifier == ability.modifier
                            && self
                                .state
                                .effect_text_overrides
                                .iter()
                                .any(|(effect, text)| *effect == fx.id && *text == origin)
                    })
                    .map(|fx| fx.id);
                if (lost && !outlives_its_ability(layer))
                    || (text_lost && !outlives_its_rules_text(layer))
                {
                    if let Some(effect) = registered {
                        lapsed.push(effect);
                    }
                    continue;
                }
                if registered.is_none() {
                    to_register.push((
                        Some(crate::effects::ContinuousEffect {
                            id: baylee_core::ids::EffectId::new(0),
                            source: Some(id),
                            controller: obj.controller,
                            origin: crate::effects::EffectOrigin::Static,
                            layer,
                            timestamp: obj.timestamp,
                            duration: Duration::WhileSourceOnBattlefield,
                            filter: crate::effects::EffectFilter::object(&self.state, id),
                            modifier: ability.modifier,
                        }),
                        baylee_core::ids::EffectId::new(0),
                        Some(origin),
                    ));
                }
            }
        }
        self.state
            .effects
            .remove_where(|effect| lapsed.contains(&effect.id));
        to_register.extend(
            emblem_statics(&self.state)
                .into_iter()
                .map(|effect| (Some(effect), baylee_core::ids::EffectId::new(0), None)),
        );
        for (effect, existing, origin) in to_register {
            let id = effect.map_or(existing, |effect| self.state.effects.register(effect));
            if let Some(origin) = origin {
                self.state.effect_text_overrides.push((id, origin));
            }
        }
        crate::effects::sync_granted_statics(&mut self.state);
        self.sync_replacement_rules();
        #[cfg(test)]
        crate::ability_log::note_sources(&self.state, &self.lookup);
    }

    /// Drops the replacement rules of sources that left the battlefield,
    /// phased out (CR 702.26b) or lost their abilities (CR 613.1f) and
    /// registers the new ones. A permanent that phases in is scanned again
    /// like one that arrived, so its rules come back from its abilities.
    fn sync_replacement_rules(&mut self) {
        let gone_rules: Vec<ObjectId> = self
            .state
            .replacement_rules
            .iter()
            .map(|r| r.source)
            .filter(|s| {
                self.state.object(*s).is_none_or(|o| {
                    o.zone != Zone::Battlefield
                        || o.status.contains(crate::object::Status::PHASED_OUT)
                        || o.characteristics().abilities_lost.is_some()
                        || o.characteristics().rules_text_lost
                })
            })
            .collect();
        self.state
            .replacement_rules
            .retain(|r| !gone_rules.contains(&r.source));
        let mut rules_to_add = Vec::new();
        for id in self.state.battlefield_view() {
            let Some(obj) = self.state.object(id) else {
                continue;
            };
            for ability in obj.abilities(&self.lookup) {
                let AbilityDef::Replacement(rule) = ability else {
                    continue;
                };
                if self
                    .state
                    .replacement_rules
                    .iter()
                    .any(|r| r.source == id && r.rule == *rule)
                {
                    continue;
                }
                rules_to_add.push(crate::state::ReplacementEntry {
                    source: id,
                    controller: obj.controller,
                    rule: *rule,
                });
            }
        }
        self.state.replacement_rules.extend(rules_to_add);
    }
}
