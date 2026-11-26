//! Target reachability, activation locks and offer restrictions.
#[allow(clippy::wildcard_imports)] // shared ability vocabulary
use super::*;

impl<L: CardLookup, D: crate::engine::LoopDetection> Engine<L, D> {
    /// Whether a spell has something it could legally be cast at.
    ///
    /// CR 601.2c: a spell with a mandatory target and no legal target cannot
    /// be cast at all. Offering it anyway is not a harmless over-approximation
    /// — the cast wizard aborts at the targeting step, so a human sees a
    /// button that only ever errors, and an agent picks the same illegal cast
    /// on every pass because nothing about the state has changed.
    ///
    /// Deliberately conservative, and returns `true` whenever it cannot be
    /// sure: modal spells choose targets per mode, X-counted targets depend on
    /// a value nobody has picked yet, and player targets are not objects. All
    /// three stay the wizard's problem.
    pub(super) fn has_a_legal_target(&self, player: PlayerId, card: ObjectId) -> bool {
        casting::card_has_a_legal_target(&self.state, &self.lookup, player, card)
    }

    /// Pithing Needle: "Activated abilities of sources with the chosen name
    /// can't be activated unless they're mana abilities" (CR 602.5).
    ///
    /// Narrowed here, where the offer is built, because `apply` refuses any
    /// activation the offer does not hold: one probe read from both sides,
    /// for the reason [`Engine::ability_has_targets`] gives. Every door
    /// onto `legal.abilities` is covered by it, printed, loyalty, granted and
    /// a card's in hand alike. What stays is what the sentence does not
    /// reach: a mana ability (CR 605.1a), turning a permanent face up and
    /// unlocking a door (special actions, CR 116.2b, 116.2m), and a
    /// prepared cast, which casts a spell.
    /// The intrinsic CR 305.6 mana of `legal.mana_abilities` is mana too.
    pub(super) fn narrow_under_chosen_names(&self, player: PlayerId, legal: &mut LegalActions) {
        let locked = self.names_locked_from_activating();
        let player_locked = self.player_non_mana_activations_locked(player);
        let has_non_mana_lock =
            self.state.effects.may_change_or_forbid_abilities()
                && self.state.effects.iter().any(|fx| {
                    matches!(fx.modifier, baylee_cards_dsl::Modifier::CantActivateNonMana)
                });
        if locked.is_empty() && !has_non_mana_lock && !player_locked {
            return;
        }
        legal.abilities.retain(|&(source, index)| {
            crate::choice::is_special_action(index)
                || index == crate::choice::PREPARED_CAST
                || self.is_mana_offer(source, index)
                || (!player_locked
                    && !self
                        .state
                        .object(source)
                        .is_some_and(|obj| locked.contains(&obj.characteristics().name))
                    && !self.non_mana_activations_are_locked(source))
        });
    }

    pub(crate) fn player_non_mana_activations_locked(&self, player: PlayerId) -> bool {
        self.state.effects.may_change_or_forbid_abilities()
            && self.state.effects.iter().any(|fx| {
                let baylee_cards_dsl::Modifier::PlayerCantActivateNonMana { who } = fx.modifier
                else {
                    return false;
                };
                crate::casting::player_locks::affects_player(&self.state, fx, who, player)
            })
    }

    pub(crate) fn non_mana_activations_are_locked(&self, id: ObjectId) -> bool {
        self.state.effects.may_change_or_forbid_abilities()
            && self.state.object(id).is_some_and(|obj| {
                self.state.effects.iter().any(|fx| {
                    matches!(fx.modifier, baylee_cards_dsl::Modifier::CantActivateNonMana)
                        && crate::effects::applies_to(&self.state, fx, obj)
                })
            })
    }

    /// The names every `Modifier::ChosenNameCantActivate` in force has
    /// locked, each the name chosen as its source entered.
    ///
    /// Compared as interned names, so a source's projected name is what is
    /// read (a copy is named what it copies, CR 707.2) and a chosen name no
    /// object of the game has carried locks nothing, having never been
    /// interned. A source that has no chosen name locks nothing either.
    pub(super) fn names_locked_from_activating(&self) -> SmallVec<[NameRef; 2]> {
        let mut locked = SmallVec::new();
        if !self.state.effects.may_change_or_forbid_abilities() {
            return locked;
        }
        for fx in self.state.effects.iter() {
            if !matches!(
                fx.modifier,
                baylee_cards_dsl::Modifier::ChosenNameCantActivate
            ) {
                continue;
            }
            let Some(chosen) = fx
                .source
                .and_then(|source| self.state.object(source))
                .and_then(|source| source.chosen_name)
            else {
                continue;
            };
            let Some(name) = self
                .lookup
                .card(chosen.card())
                .and_then(|def| def.faces.get(usize::from(chosen.face())))
                .and_then(|face| self.state.names.find(face.name))
            else {
                continue;
            };
            if !locked.contains(&name) {
                locked.push(name);
            }
        }
        locked
    }

    /// Split second (CR 702.61a): "As long as this spell is on the stack,
    /// players can't cast other spells or activate abilities that aren't
    /// mana abilities."
    ///
    /// What stays is what CR 702.61b names: mana abilities, and special
    /// actions — turning a face-down permanent face up, which its controller
    /// may do any time they have priority (CR 116.2b). Suspending a card is a
    /// special action too, but one taken "only if they could begin to cast
    /// that card" (CR 116.2f), and under split second nobody can, so it goes
    /// with the spells. A land is never offered with a spell on the stack.
    ///
    /// This is the offer's half. The casts a resolving ability makes are
    /// refused by `casting::may_begin_casting`, on the same reader.
    pub(super) fn narrow_under_split_second(&self, legal: &mut LegalActions) {
        if !casting::split_second_on_stack(&self.state) {
            return;
        }
        legal.castable.clear();
        legal.payable.clear();
        legal.suspendable.clear();
        legal.unpaid_suspend.clear();
        // Foretell is a special action, not casting or activating an ability.
        legal.abilities.retain(|&(source, index)| {
            crate::choice::is_special_action(index) || self.is_mana_offer(source, index)
        });
    }

    /// Whether `legal` offers anything that makes mana: a CR 305.6 tap, a
    /// mana ability, or a granted action that adds mana.
    pub(in crate::engine) fn offers_mana(&self, legal: &LegalActions) -> bool {
        !legal.mana_abilities.is_empty()
            || legal
                .abilities
                .iter()
                .any(|&(source, index)| self.is_mana_offer(source, index))
            || legal.granted_actions.iter().any(|offer| {
                matches!(
                    offer.effect,
                    crate::choice::GrantedActionKind::AddMana { .. }
                )
            })
    }

    /// Keeps of the cards the pool could not pay for (`legal.payable`, the
    /// candidates the sweep collected) those that may be cast and paid for
    /// while casting (CR 601.2g): only while the seat has a mana ability on
    /// offer, and only a card with something to point at, as `castable`.
    pub(super) fn settle_payable(
        &self,
        player: PlayerId,
        legal: &mut LegalActions,
        check_assist: bool,
    ) {
        if legal.payable.is_empty() {
            return;
        }
        let own_mana = self.offers_mana(legal);
        let card_has_assist = |card: ObjectId| {
            self.state
                .object(card)
                .and_then(|o| o.card)
                .and_then(|c| self.lookup.card(c.index))
                .and_then(|d| d.faces.first())
                .is_some_and(|f| f.keywords.contains(baylee_cards_dsl::KeywordSet::ASSIST))
        };
        let assist_mana = check_assist
            && !own_mana
            && legal.payable.iter().copied().any(card_has_assist)
            && self.state.players.iter().any(|p| {
                p.loss.is_none() && p.id != player && self.mana_window_legal(p.id).is_some()
            });
        if !own_mana && !assist_mana {
            legal.payable.clear();
            return;
        }
        legal.payable.retain(|&card| {
            (own_mana || (assist_mana && card_has_assist(card)))
                && casting::can_cast_paying_later(&self.state, &self.lookup, player, card).is_ok()
                && self.has_a_legal_target(player, card)
        });
    }

    /// Inside a CR 605.3a payment window, the only thing a player may do is
    /// make mana.
    ///
    /// Applied here, at the single place a `LegalActions` is built, rather
    /// than where the window is opened: the machine re-publishes priority to
    /// the same player after every mana ability they activate, through
    /// `regrant_priority`, and a restriction written at the opening would
    /// have lasted exactly one tap.
    ///
    /// `abilities` is narrowed rather than emptied, which is the half that
    /// is easy to get wrong. `mana_abilities` is only the CR 305.6 shortcut
    /// — a basic land type, or a granted ability with no printed index —
    /// while a nonbasic that prints its own `{T}: Add …` is an ordinary
    /// entry here. Emptying the list would leave Rupture Spire unable to tap
    /// for the very payment Rupture Spire is asking for, and would put every
    /// filter land in this pool out of reach of its own window.
    pub(super) fn narrow_to_mana_window(&self, player: PlayerId, legal: &mut LegalActions) {
        if self.mana_window.as_ref().map(|w| w.player) != Some(player) {
            return;
        }
        self.narrow_to_mana(legal);
    }

    /// The narrowing itself, with no window to ask about.
    ///
    /// Split out because the opener has to know what a window *would* offer
    /// before deciding to open one — a window with nothing in it to press is
    /// not worth opening — and that question cannot be put to the un-narrowed
    /// list, where `abilities` is every activation this seat could make and
    /// not the mana ones. Idempotent, so narrowing an already narrowed list
    /// on the regrant path costs a `retain` over what is left.
    ///
    /// **What an entry is, is asked of the object and not of its card.** The
    /// offer is built from `GameObject::abilities` — the face that is up, a
    /// copy's copied list, a token's definition — and the narrowing has to
    /// read the same list, or it judges a different ability from the one it
    /// is keeping or dropping. It used to read the card's own list, which
    /// was wrong both ways: a Treasure (no card) and the back face of a
    /// modal land were taken out of the window they could pay, and a Cursed
    /// Mirror copying Bottle Gnomes kept the Gnomes' sacrifice in, because
    /// the Mirror's card prints a mana ability at that index. A granted
    /// ability is looked up among the grants by its slot, the same way the
    /// offer numbers it.
    ///
    /// The planning hints (`unpaid_abilities`) are narrowed by the same
    /// test. `compute_legal` moves an activation whose mana is not floating
    /// yet out of `abilities` and into the hints, so a window opened from a
    /// fresh list (an upkeep tax, a miracle, an attack tax) kept every such
    /// activation as a hint: a seat's planner read Energy Flux's {2} window
    /// as one where Aeolipile could be activated, tapped a Forest for it,
    /// and the pass that followed left the tax unpaid (LLM report, 3 Nov
    /// 2026).
    pub(crate) fn narrow_to_mana(&self, legal: &mut LegalActions) {
        legal
            .granted_actions
            .retain(|offer| offer.timing == baylee_cards_dsl::SpecialActionTiming::ManaAbility);
        legal.lands.clear();
        legal.castable.clear();
        legal.payable.clear();
        legal.cast_ways.clear();
        legal.suspendable.clear();
        legal.foretellable.clear();
        legal.unpaid_foretell.clear();
        legal.unpaid_suspend.clear();
        // What restricted mana pays for is stated of the narrowed offer
        // (`offer_restricted_mana`), the payment a window is open for.
        legal.restricted_taps.clear();
        legal.restricted_floating.clear();
        legal
            .abilities
            .retain(|&(source, index)| self.is_payment_mana_offer(source, index));
        legal
            .unpaid_abilities
            .retain(|&(source, index, _)| self.is_payment_mana_offer(source, index));
    }

    /// Target permissions of the actual activation, including runtime grants.
    pub(in crate::engine) fn activation_wall_only(&self, source: ObjectId, index: u32) -> bool {
        let frozen = self
            .activating_abilities
            .as_ref()
            .filter(|(id, _)| *id == source)
            .and_then(|(_, list)| list.abilities.get(index as usize).copied());
        frozen
            .or_else(|| self.activation_definition(source, index))
            .is_some_and(|definition| {
                eval::target_domain::ability_walls(
                    &definition,
                    self.activation_mode_index(source, index),
                )
            })
    }

    /// Effective words captured for an activation, or live words before it starts.
    pub(in crate::engine) fn ability_rule_context(
        &self,
        source: ObjectId,
        index: u32,
    ) -> crate::text_changes::RuleContext {
        if crate::choice::granted_slot(index).is_some()
            && let Some(granted) = self.granted_definition(source, index)
        {
            return crate::text_changes::RuleContext {
                source,
                text: granted.text,
                resolved_name: None,
                resolved_subtype: None,
                reference: None,
            };
        }
        let text = self
            .activating_abilities
            .as_ref()
            .filter(|(id, _)| *id == source)
            .map_or_else(
                || self.state.ability_text(source, index),
                |(_, list)| list.base_text(index as usize),
            );
        crate::text_changes::RuleContext {
            source,
            text,
            resolved_name: None,
            resolved_subtype: None,
            reference: None,
        }
    }

    /// Whether a targeting ability has anything legal to point at
    /// (CR 601.2c, applied to activations by CR 602.2b).
    ///
    /// `apply` already refuses such an activation with "no legal targets";
    /// this is the same probe on the offering side, because the two have to
    /// be one probe or the client lights a permanent up and the click is
    /// refused. Riptide Laboratory is the case that found it — "{1}{U}, {T}:
    /// Return target Wizard you control" was offered at a table whose only
    /// Wizard was the opponent's.
    /// Whether an ability can point at what it has to point at.
    ///
    /// The *count* is half the question and used not to be asked at all: a
    /// bare spec reads as "exactly one", so "up to one target" (`min: 0`) was
    /// an ability nobody could activate with an empty board — which on Karn,
    /// the Great Creator hides a loyalty tick and on Teferi, Time Raveler
    /// hides a drawn card.
    /// Whether both instances of "target" can be satisfied together
    /// (CR 601.2c, 602.2b): a second instance whose requirement names the
    /// first (`Filter::Reference` and its family, Gauntlets of Chaos) needs a
    /// legal first target that leaves enough second ones, read with that
    /// first target as the reference.
    pub(super) fn ability_has_targets(
        &self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        first: Option<baylee_cards_dsl::TargetReq>,
        second: Option<baylee_cards_dsl::TargetReq>,
        third: Option<baylee_cards_dsl::TargetReq>,
    ) -> bool {
        let wall_only = eval::target_domain::walls3(first, second, third);
        if !self.ability_has_target_in_domain(player, source, ability_index, first, wall_only)
            || !self.ability_has_target_in_domain(player, source, ability_index, third, wall_only)
        {
            return false;
        }
        let context = self.ability_rule_context(source, ability_index);
        let second_ok = match (first, second) {
            (Some(first_req), Some(second_req))
                if second_req.min > 0 && crate::casting::spec_reads_reference(second_req.spec) =>
            {
                eval::target_options_in_domain(
                    &first_req.spec,
                    &self.state,
                    player,
                    context,
                    None,
                    wall_only,
                )
                .into_iter()
                .any(|reference| {
                    let second_objs = eval::target_options_in_domain(
                        &second_req.spec,
                        &self.state,
                        player,
                        crate::text_changes::RuleContext {
                            resolved_name: None,
                            resolved_subtype: None,
                            reference: Some(reference),
                            ..context
                        },
                        None,
                        wall_only,
                    );
                    if second_req.constraint()
                        == Some(baylee_cards_dsl::TargetConstraint::SameController)
                    {
                        !eval::eligible_same_controller_groups(
                            &self.state,
                            &second_objs,
                            u32::from(second_req.min),
                        )
                        .is_empty()
                    } else if second_req.constraint()
                        == Some(baylee_cards_dsl::TargetConstraint::OnePerPlayer)
                    {
                        eval::one_per_player_satisfied(
                            &self.state,
                            &second_req.spec,
                            player,
                            &second_objs,
                            second_req.min,
                        )
                    } else {
                        second_objs.len() >= usize::from(second_req.min)
                    }
                })
            }
            _ => {
                self.ability_has_target_in_domain(player, source, ability_index, second, wall_only)
            }
        };
        if !second_ok {
            return false;
        }
        if [second, third].into_iter().flatten().any(|r| {
            r.min > 0
                && !r.count_is_x
                && r.constraint()
                    == Some(baylee_cards_dsl::TargetConstraint::DistinctFromPriorTargetGroups)
        }) {
            let pools: Vec<_> = [first, second, third]
                .into_iter()
                .flatten()
                .map(|r| self.ability_clause_distinct_pool(player, context, r, None, wall_only))
                .collect();
            if !eval::can_complete_target_groups(&[], baylee_core::ids::SeatSet::new(), &pools) {
                return false;
            }
        }
        true
    }

    pub(super) fn ability_clause_distinct_pool(
        &self,
        player: PlayerId,
        context: crate::text_changes::RuleContext,
        req: baylee_cards_dsl::TargetReq,
        x: Option<u32>,
        wall_only: bool,
    ) -> eval::DistinctGroupPool {
        let distinct_from_prior = req.constraint()
            == Some(baylee_cards_dsl::TargetConstraint::DistinctFromPriorTargetGroups);
        let min = if req.is_up_to_x() {
            0
        } else if req.count_is_x {
            x.unwrap_or(0)
        } else {
            u32::from(req.min)
        };
        if min == 0
            || matches!(
                req.spec,
                baylee_cards_dsl::TargetSpec::Player(rel) if rel != baylee_cards_dsl::PlayerRel::You
            )
        {
            return eval::DistinctGroupPool {
                needed: 0,
                distinct_from_prior,
                objects: Vec::new(),
                players: Vec::new(),
            };
        }
        let objects =
            eval::target_options_in_domain(&req.spec, &self.state, player, context, x, wall_only);
        let players = eval::target_player_options_from(&self.state, &req.spec, player, context);
        eval::DistinctGroupPool {
            needed: usize::try_from(min).unwrap_or(usize::MAX),
            distinct_from_prior,
            objects,
            players,
        }
    }

    fn ability_has_target_in_domain(
        &self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        targets: Option<baylee_cards_dsl::TargetReq>,
        wall_only: bool,
    ) -> bool {
        let Some(req) = targets else {
            return true;
        };
        if req.chooser == baylee_cards_dsl::TargetChooser::EventPlayer {
            return false;
        }
        if req.chooser == baylee_cards_dsl::TargetChooser::Opponent
            && self.opponents_in_turn_order(player).is_empty()
        {
            return false;
        }
        if req.min == 0 {
            return true;
        }
        let wanted = req.min as usize;
        if let Some(def) = self
            .granted_definition(source, ability_index)
            .map(|g| g.definition)
            .or_else(|| self.activation_definition(source, ability_index))
        {
            let (effects, has_var) = match def {
                AbilityDef::Activated { cost, effects, .. }
                | AbilityDef::ActivatedConditional { cost, effects, .. } => {
                    (Some(effects), cost.mana.has_variable())
                }
                AbilityDef::Loyalty { effects, .. } => (Some(effects), false),
                _ => (None, false),
            };
            if let Some(effects) = effects
                && !has_var
            {
                let ctx = self.ability_rule_context(source, ability_index);
                let ver = self.state.object(source).map(|o| o.version);
                if let Some((_, total)) = crate::engine::division::effects_divided_spec(
                    effects,
                    &self.state,
                    player,
                    ctx,
                    ver,
                    0,
                ) && (total as usize) < wanted
                {
                    return false;
                }
            }
        }
        // Both halves of one choice (CR 115.4): "any target" is a creature,
        // a planeswalker, a battle *or* a player, so only the sum says
        // whether the ability can be pointed anywhere. Splitting on the spec
        // instead — players for `AnyPlayer`, objects for everything else —
        // read Blighted Gorge's "deal 2 damage to any target" as objects
        // alone, and withheld it at a table with an empty board although a
        // player is always there to point at. `target_player_options`
        // answers empty for every object-only spec, which is what lets one
        // sum serve every spec; the cast wizard adds them the same way.
        let object_list = eval::target_options_in_domain(
            &req.spec,
            &self.state,
            player,
            self.ability_rule_context(source, ability_index),
            None,
            wall_only,
        );
        if req.constraint() == Some(baylee_cards_dsl::TargetConstraint::SameController) {
            return !eval::eligible_same_controller_groups(
                &self.state,
                &object_list,
                u32::from(req.min),
            )
            .is_empty();
        }
        if req.constraint() == Some(baylee_cards_dsl::TargetConstraint::OnePerPlayer) {
            return eval::one_per_player_satisfied(
                &self.state,
                &req.spec,
                player,
                &object_list,
                req.min,
            );
        }
        let objects = object_list.len();
        let players = eval::target_player_options_from(
            &self.state,
            &req.spec,
            player,
            self.ability_rule_context(source, ability_index),
        )
        .len();
        objects + players >= wanted
    }

    /// Whether Karn's lock covers this permanent: "activated abilities of
    /// artifacts your *opponents* control can't be activated".
    ///
    /// Not "everyone but me" — a teammate is neither, and at a two-headed
    /// table the two readings differ by every artifact on Karn's own side of
    /// the table but Karn's controller's.
    ///
    /// One probe read from both sides, for the reason
    /// [`Engine::ability_has_targets`] gives. The lock lived only in `apply`,
    /// which is invisible while the walker is yours — the abilities it stops
    /// are on the *other* seat's board, and that seat was still being offered
    /// every one of them.
    pub(crate) fn artifact_activations_are_locked(&self, id: ObjectId) -> bool {
        self.state.effects.may_change_or_forbid_abilities()
            && self.activations_are_locked_by(id, self.state.effects.iter())
    }

    pub(super) fn activations_are_locked_by<'a>(
        &self,
        id: ObjectId,
        mut effects: impl Iterator<Item = &'a crate::effects::ContinuousEffect>,
    ) -> bool {
        self.state.object(id).is_some_and(|obj| {
            effects.any(|fx| match fx.modifier {
                baylee_cards_dsl::Modifier::CantActivateArtifacts => {
                    obj.characteristics().types.contains(TypeSet::ARTIFACT)
                        && self.state.is_opponent(obj.controller, fx.controller)
                }
                baylee_cards_dsl::Modifier::CantActivate => {
                    crate::effects::applies_to(&self.state, fx, obj)
                }
                _ => false,
            })
        })
    }
}
