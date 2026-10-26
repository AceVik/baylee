//! The untap step: day/night, restrictions, choices and completion.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary
use super::*;

/// One `Modifier::UntapAtMost` binding the active player's untap step.
pub(super) struct UntapLimit {
    /// What it counts.
    of: &'static Filter,
    /// The effect's controller, the filter's "you".
    you: PlayerId,
    /// The effect's source, the filter's "this".
    this: Option<ObjectId>,
    /// How many of them may untap.
    count: u8,
}

impl<L: CardLookup> Engine<L> {
    /// The untap step's second turn-based action (CR 502.2): the game looks
    /// at the previous turn and decides whether the designation flips.
    ///
    /// Both halves are one-sided on purpose. Day becomes night when the
    /// previous turn's active player cast **no** spells; night becomes day
    /// when they cast **two or more**. One spell holds the designation
    /// where it is, in either direction.
    ///
    /// A game with neither designation skips the check entirely and keeps
    /// having neither (CR 730.2c) — which is every game in this pool that
    /// has no daybound card in it, so the common case costs two compares.
    ///
    /// CR 502.2a states a variant for the shared team turns option, which
    /// is CR 805 and something this engine does not offer: `--teams` puts
    /// chairs on sides, and each of them still takes its own turn. The
    /// plain rule is therefore the whole rule here.
    fn check_day_night(&mut self) {
        let (Some(now), Some(previous)) = (self.state.day_night, self.state.previous_turn) else {
            return;
        };
        match now {
            DayNight::Day if previous.spells_cast == 0 => self.state.become_night(),
            DayNight::Night if previous.spells_cast >= 2 => self.state.become_day(),
            _ => {}
        }
    }

    /// Whether an effect keeps `id` from untapping this untap step
    /// (CR 502.3, and CR 613.11 for what kind of effect that is).
    ///
    /// An ordinary restriction follows the affected permanent's controller
    /// (Paralyze). "Your next untap step" instead refers to the resolving
    /// ability's controller (CR 109.5), even after the permanent changes
    /// hands. Its duration supplies that fixed seat as well as its deadline.
    fn keeps_tapped(&self, id: ObjectId) -> bool {
        let Some(obj) = self.state.object(id) else {
            return false;
        };
        self.state.effects.iter().any(|fx| {
            matches!(fx.modifier, baylee_cards_dsl::Modifier::DoesNotUntap)
                && (fx.duration != baylee_cards_dsl::Duration::UntilYourNextUntapStep
                    || fx.controller == self.state.turn.active)
                && crate::effects::applies_to(&self.state, fx, obj)
        })
    }

    /// The permanents CR 502.3 lets the active player decide about.
    ///
    /// Three conditions, and the third is the one that is easy to leave out.
    /// Tapped, because an untapped permanent has nothing to determine.
    /// Controlled by the active player, because CR 502.3 is only ever about
    /// their permanents. And **not already kept from untapping** — a Basalt
    /// Monolith that also said "you may choose not to untap" would otherwise
    /// be offered a question whose two answers do the same thing, which is
    /// the offer that contradicts its own apply.
    fn untap_optional(&self) -> Vec<ObjectId> {
        let active = self.state.turn.active;
        // What phased in a moment ago untaps with the rest (CR 502.1 before
        // 502.3); what is still phased out does not.
        self.state
            .battlefield_seen()
            .filter(|id| {
                let Some(obj) = self.state.object(*id) else {
                    return false;
                };
                obj.controller == active
                    && obj.status.contains(Status::TAPPED)
                    && !self.keeps_tapped(*id)
                    && self.state.effects.iter().any(|fx| {
                        let applies =
                            matches!(fx.modifier, baylee_cards_dsl::Modifier::MayChooseNotToUntap)
                                && crate::effects::applies_to(&self.state, fx, obj);
                        // Read here and never by the projection, so this is
                        // where the static is applied: the recorder's door.
                        #[cfg(test)]
                        if applies {
                            crate::ability_log::static_applied(fx);
                        }
                        applies
                    })
            })
            .collect()
    }

    /// The untap step, up to the point where it may have to ask.
    ///
    /// Returns `true` when it suspended on a question, which is the
    /// contract [`Self::progress_step`] has with every other step.
    ///
    /// The split is where CR 502.3's two halves already are. Phasing
    /// (CR 702.26a) and the day/night check (CR 502.2) come first and
    /// happen exactly once; then "the active player **determines** which
    /// permanents they control will untap", which is a question whenever a
    /// permanent gives it a second answer; then "they untap them all
    /// simultaneously", which is [`Self::finish_untap_step`] and is
    /// reachable from either side of the question. Nothing before the
    /// suspension may run again on the way back, which is why the resume
    /// path enters at the second function rather than re-entering this one.
    ///
    /// **No priority is granted here.** CR 502.4 says no player receives
    /// priority during the untap step; it does not say the turn-based
    /// action may not take the answer its own rule asks a player for.
    pub(crate) fn untap_step(&mut self) -> bool {
        let active = self.state.turn.active;
        // "Players skip their untap steps" (Stasis) replaces the step with
        // nothing (CR 614.1b, 614.10): no phasing, no day/night check, no
        // untap, and an effect waiting for the player's *next* untap step
        // keeps waiting for one that is not skipped (CR 614.10a) — which is
        // why this goes straight on and not through `finish_untap_step`.
        if self.state.skips_untap_step(active) {
            self.advance_step();
            return false;
        }
        let arena = &self.state.arena;
        self.state.effects.remove_where(|effect| {
            effect.duration == baylee_cards_dsl::Duration::UntilStartOfAffectedNextUntap
                && match effect.filter {
                    crate::effects::EffectFilter::ObjectIs(id, version) => arena
                        .get(id)
                        .filter(|object| {
                            object.version == version && object.zone == Zone::Battlefield
                        })
                        .is_none_or(|object| object.controller == active),
                    _ => false,
                }
        });
        self.state.refresh_characteristics();
        // "All phased-out permanents that the active player controlled when
        // they phased out phase in" (CR 502.1): a phased-out permanent is
        // not projected, so its controller is still that one. One that
        // phased out indirectly phases in with its host and never by itself
        // (CR 702.26g).
        let coming: Vec<ObjectId> = self
            .state
            .zones
            // phasing: the walk is for the permanents that are phased out.
            .list(ZoneLocation::Battlefield)
            .iter()
            .copied()
            .filter(|&id| {
                self.state.object(id).is_some_and(|o| {
                    o.controller == active
                        && o.status.contains(Status::PHASED_OUT)
                        && !o.status.contains(Status::PHASED_OUT_INDIRECTLY)
                        && !o.riders.iter().any(|r| {
                            matches!(r, crate::object::Rider::PhasedOutUntilHostLeaves { .. })
                        })
                })
            })
            .collect();
        let going: Vec<_> = self
            .state
            .battlefield_seen()
            .filter(|&id| {
                self.state.object(id).is_some_and(|object| {
                    object.controller == active
                        && object
                            .characteristics()
                            .keywords
                            .contains(baylee_cards_dsl::KeywordSet::PHASING)
                })
            })
            .collect();
        if !going.is_empty() {
            self.state.phase_out(&going);
        }
        for id in coming {
            self.state.phase_in(id);
        }
        self.check_day_night();
        self.return_during_untap();
        // CR 502.3, the third turn-based action: "the active player
        // determines which permanents they control will untap. Then they
        // untap them all simultaneously."
        let optional = self.untap_optional();
        if optional.is_empty() {
            return self.untap_under_limits(Vec::new(), Vec::new());
        }
        // The answer names the permanents that stay tapped, so an empty one
        // is "untap everything" — the determination every board without
        // such a permanent makes, and the one an automated seat produces by
        // answering the minimum.
        let max = u8::try_from(optional.len()).unwrap_or(u8::MAX);
        self.pending_plan = Some(PlanKind::UntapChoice);
        self.pending = Pending::ChooseCards {
            player: active,
            options: optional,
            min: 0,
            max,
            prompt: crate::choice::ChoicePrompt::LeaveTapped,
            total: None,
        };
        self.awaiting_answer = true;
        true
    }

    /// The limits on the active player's untap step
    /// (`Modifier::UntapAtMost`), each with the "you" and "this" its filter
    /// is read against.
    fn untap_limits(&self) -> Vec<UntapLimit> {
        let active = self.state.turn.active;
        self.state
            .effects
            .iter()
            .filter_map(|fx| {
                let baylee_cards_dsl::Modifier::UntapAtMost { who, of, count } = fx.modifier else {
                    return None;
                };
                crate::eval::players(who, &self.state, fx.controller)
                    .is_some_and(|p| p.contains(&active))
                    .then_some(UntapLimit {
                        of,
                        you: fx.controller,
                        this: fx.source,
                        count,
                    })
            })
            .collect()
    }

    /// Whether `limit` counts the permanent `id`.
    fn counts(&self, limit: &UntapLimit, id: ObjectId) -> bool {
        self.state.object(id).is_some_and(|obj| {
            crate::eval::matches(
                limit.of,
                &self.state,
                obj,
                limit.you,
                limit.this.unwrap_or(id),
            )
        })
    }

    /// CR 502.3's determination under untap limits (Smoke, Winter Orb):
    /// of the permanents that would untap, the ones a limit counts untap as
    /// the active player names them, and the rest of them stay tapped once
    /// no limit has room left for any of them.
    ///
    /// "Can't untap more than one" keeps a permanent tapped only when the
    /// limit is full: the default is still that everything untaps (CR
    /// 502.3), so the player chooses *which*, not *whether*. The question
    /// is asked again after every answer, and each answer counts against
    /// every limit the permanents in it match — an animated land under
    /// Smoke and Winter Orb is the one creature and the one land (the Smoke
    /// ruling), and Static Orb beside Winter Moon lets two permanents untap,
    /// at most one of them a nonbasic land (the Winter Moon ruling).
    ///
    /// `max` is the most a single answer can name without breaking a limit
    /// whatever it names: the smallest room among the limits that cannot
    /// take everything still on the menu. Anything from one to that is
    /// legal, so the menu never offers an answer the apply would refuse.
    /// When every limit can take everything on the menu, nothing is asked.
    ///
    /// `kept` is what the player already chose to leave tapped
    /// (`ChoicePrompt::LeaveTapped`), asked first so that a permanent the
    /// player keeps by choice does not take a limit's room. Returns `true`
    /// when a question was asked.
    pub(crate) fn untap_under_limits(
        &mut self,
        kept: Vec<ObjectId>,
        chosen: Vec<ObjectId>,
    ) -> bool {
        let active = self.state.turn.active;
        let limits = self.untap_limits();
        let counted = |id: ObjectId| limits.iter().any(|l| self.counts(l, id));
        // What would untap if no limit applied. A phased-out permanent is
        // treated as though it does not exist (CR 702.26b): no limit counts
        // it and no menu offers it.
        let would: Vec<ObjectId> =
            self.state
                .battlefield_seen()
                .filter(|id| {
                    self.state.object(*id).is_some_and(|o| {
                        o.controller == active && o.status.contains(Status::TAPPED)
                    }) && !kept.contains(id)
                        && !self.keeps_tapped(*id)
                })
                .collect();
        let room: Vec<usize> = limits
            .iter()
            .map(|l| {
                let used = chosen.iter().filter(|c| self.counts(l, **c)).count();
                usize::from(l.count).saturating_sub(used)
            })
            .collect();
        // Counted, not yet named, and every limit counting it has room.
        let open: Vec<ObjectId> = would
            .iter()
            .copied()
            .filter(|id| {
                !chosen.contains(id)
                    && counted(*id)
                    && limits
                        .iter()
                        .zip(&room)
                        .all(|(l, r)| *r > 0 || !self.counts(l, *id))
            })
            .collect();
        let most = limits
            .iter()
            .zip(&room)
            .filter(|(l, r)| open.iter().filter(|id| self.counts(l, **id)).count() > **r)
            .map(|(_, r)| *r)
            .min();
        let Some(most) = most else {
            // Everything still open fits: it untaps with what was named, and
            // what the limits shut out stays tapped.
            let mut stays = kept;
            stays.extend(
                would
                    .iter()
                    .filter(|id| counted(**id) && !chosen.contains(id) && !open.contains(id)),
            );
            return self.finish_untap_step(&stays);
        };
        self.pending_plan = Some(PlanKind::UntapLimit { kept, chosen });
        self.pending = Pending::ChooseCards {
            player: active,
            options: open,
            min: 1,
            max: u8::try_from(most).unwrap_or(u8::MAX),
            prompt: crate::choice::ChoicePrompt::Untap,
            total: None,
        };
        self.awaiting_answer = true;
        true
    }

    /// "Then they untap them all simultaneously" (CR 502.3), with `kept`
    /// left out of it.
    ///
    /// `kept` has been checked against the very list [`Self::untap_optional`]
    /// produced before this is reached, so it is read and not re-validated —
    /// the same arrangement `cost_wizard::pay` has with its own menu.
    ///
    /// The `keeps_tapped` half is read off the effect table rather than off
    /// a projection: what a "doesn't untap" effect modifies is a rule and
    /// not a characteristic (CR 613.11), so there is nothing on the
    /// permanent to look at.
    pub(crate) fn finish_untap_step(&mut self, kept: &[ObjectId]) -> bool {
        let active = self.state.turn.active;
        let determined: Vec<_> = self
            .state
            .battlefield_view()
            .into_iter()
            .filter(|id| {
                self.state.object(*id).is_some_and(|object| {
                    object.controller == active && object.status.contains(Status::TAPPED)
                }) && !kept.contains(id)
                    && !self.keeps_tapped(*id)
            })
            .collect();
        let batch = super::untap_replacements::UntapBatch::new(&self.state, determined);
        self.commit_untap_batch(batch)
    }

    pub(crate) fn commit_untap_batch(
        &mut self,
        batch: super::untap_replacements::UntapBatch,
    ) -> bool {
        if let Some(pending) = batch.pending(&self.state) {
            self.pending_plan = Some(PlanKind::UntapReplacements(batch));
            self.pending = pending;
            self.awaiting_answer = true;
            return true;
        }
        batch.commit(&mut self.state);
        let active = self.state.turn.active;
        let arena = &self.state.arena;
        self.state
            .effects
            .finish_bound_untap(active, |id, version| {
                arena
                    .get(id)
                    .filter(|object| object.version == version && object.zone == Zone::Battlefield)
                    .map(|object| object.controller)
            });
        // "…doesn't untap during your **next** untap step": the step it was
        // waiting for has now happened, so the effect is spent.
        //
        // **After the loop above and not before it.** This is the fourth
        // duration that expires at a point in the turn structure — the other
        // three are `UntilYourNextTurn` in `start_turn`, `UntilEndOfCombat`
        // in `end_of_combat` and `UntilEndOfTurn` in
        // `cleanup_ends_the_turns_effects` — and it
        // is the first that ends inside the step it is about rather than at
        // a boundary between two. An expiry written at the top of the untap
        // step would let the land untap on schedule and leave a card that
        // compiles, claims `Implemented` and does nothing at all; one
        // written at the turn boundary instead would take the *following*
        // untap step with it and cost the land a second turn. Both
        // directions are pinned in `untap_tests`.
        //
        // Both expiry and `keeps_tapped` bind "your" to the ability's
        // controller, independently of who now controls the permanent.
        self.state.effects.remove_where(|fx| {
            matches!(
                fx.duration,
                baylee_cards_dsl::Duration::UntilYourNextUntapStep
            ) && fx.controller == active
        });
        self.advance_step();
        false
    }
}
