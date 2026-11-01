//! Target reachability, activation locks and offer restrictions.
#[allow(clippy::wildcard_imports)] // shared ability vocabulary
use super::*;

impl<L: CardLookup> Engine<L> {
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
        // The front face's own line, which is what this gate has always
        // asked. The other faces a card may be cast as are asked in
        // `casting::can_cast`, beside the probe that says whether each of
        // them can be paid for — the two belong together, because a face
        // that cannot be paid for is not a way of casting the card whatever
        // it targets.
        casting::face_has_a_legal_target(&self.state, &self.lookup, player, card, 0)
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
    pub(super) fn narrow_under_chosen_names(&self, legal: &mut LegalActions) {
        let locked = self.names_locked_from_activating();
        let has_non_mana_lock =
            self.state.effects.may_change_or_forbid_abilities()
                && self.state.effects.iter().any(|fx| {
                    matches!(fx.modifier, baylee_cards_dsl::Modifier::CantActivateNonMana)
                });
        if locked.is_empty() && !has_non_mana_lock {
            return;
        }
        legal.abilities.retain(|&(source, index)| {
            crate::choice::is_special_action(index)
                || index == crate::choice::PREPARED_CAST
                || self.is_mana_offer(source, index)
                || (!self
                    .state
                    .object(source)
                    .is_some_and(|obj| locked.contains(&obj.characteristics().name))
                    && !self.non_mana_activations_are_locked(source))
        });
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
    /// Read off the projected keywords of every spell on the stack, so a
    /// copy of a split-second spell locks it too: a copy is a spell.
    pub(super) fn narrow_under_split_second(&self, legal: &mut LegalActions) {
        let locked = self.state.zones.list(ZoneLocation::Stack).iter().any(|id| {
            self.state.object(*id).is_some_and(|o| {
                o.kind == ObjectKind::Spell
                    && o.characteristics()
                        .keywords
                        .contains(baylee_cards_dsl::KeywordSet::SPLIT_SECOND)
            })
        });
        if !locked {
            return;
        }
        legal.castable.clear();
        legal.payable.clear();
        legal.suspendable.clear();
        // Foretell is a special action, not casting or activating an ability.
        legal.abilities.retain(|&(source, index)| {
            crate::choice::is_special_action(index) || self.is_mana_offer(source, index)
        });
    }

    /// Keeps of the cards the pool could not pay for (`legal.payable`, the
    /// candidates the sweep collected) those that may be cast and paid for
    /// while casting (CR 601.2g): only while the seat has a mana ability on
    /// offer, and only a card with something to point at, as `castable`.
    pub(super) fn settle_payable(&self, player: PlayerId, legal: &mut LegalActions) {
        if legal.payable.is_empty() {
            return;
        }
        let makes_mana = !legal.mana_abilities.is_empty()
            || legal
                .abilities
                .iter()
                .any(|&(source, index)| self.is_mana_offer(source, index))
            || legal.granted_actions.iter().any(|offer| {
                matches!(
                    offer.effect,
                    crate::choice::GrantedActionKind::AddMana { .. }
                )
            });
        if !makes_mana {
            legal.payable.clear();
            return;
        }
        legal.payable.retain(|&card| {
            casting::can_cast_paying_later(&self.state, &self.lookup, player, card).is_ok()
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
    pub(crate) fn narrow_to_mana(&self, legal: &mut LegalActions) {
        legal
            .granted_actions
            .retain(|offer| offer.timing == baylee_cards_dsl::SpecialActionTiming::ManaAbility);
        legal.lands.clear();
        legal.castable.clear();
        legal.payable.clear();
        legal.suspendable.clear();
        legal.foretellable.clear();
        legal
            .abilities
            .retain(|&(source, index)| self.is_payment_mana_offer(source, index));
    }

    /// Target permissions of the actual activation, including runtime grants.
    pub(super) fn activation_wall_only(&self, source: ObjectId, index: u32) -> bool {
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
    pub(super) fn ability_rule_context(
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
    ) -> bool {
        let wall_only = eval::target_domain::walls(first, second);
        if !self.ability_has_target_in_domain(player, source, ability_index, first, wall_only) {
            return false;
        }
        let (Some(first), Some(second_req)) = (first, second) else {
            return self.ability_has_target_in_domain(
                player,
                source,
                ability_index,
                second,
                wall_only,
            );
        };
        if second_req.min == 0 || !crate::casting::spec_reads_reference(second_req.spec) {
            return self.ability_has_target_in_domain(
                player,
                source,
                ability_index,
                second,
                wall_only,
            );
        }
        let context = self.ability_rule_context(source, ability_index);
        eval::target_options_in_domain(&first.spec, &self.state, player, context, None, wall_only)
            .into_iter()
            .any(|reference| {
                eval::target_options_in_domain(
                    &second_req.spec,
                    &self.state,
                    player,
                    crate::text_changes::RuleContext {
                        resolved_name: None,

                        reference: Some(reference),
                        ..context
                    },
                    None,
                    wall_only,
                )
                .len()
                    >= usize::from(second_req.min)
            })
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
        // Both halves of one choice (CR 115.4): "any target" is a creature,
        // a planeswalker, a battle *or* a player, so only the sum says
        // whether the ability can be pointed anywhere. Splitting on the spec
        // instead — players for `AnyPlayer`, objects for everything else —
        // read Blighted Gorge's "deal 2 damage to any target" as objects
        // alone, and withheld it at a table with an empty board although a
        // player is always there to point at. `target_player_options`
        // answers empty for every object-only spec, which is what lets one
        // sum serve every spec; the cast wizard adds them the same way.
        let objects = eval::target_options_in_domain(
            &req.spec,
            &self.state,
            player,
            self.ability_rule_context(source, ability_index),
            None,
            wall_only,
        )
        .len();
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
