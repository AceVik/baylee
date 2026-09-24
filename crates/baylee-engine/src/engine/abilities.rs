use super::{
    AbilityDef, AbilityLoc, ActivationTiming, CardLookup, Cause, Cost, CostPart, Engine,
    EngineError, GameEvent, GameObject, LegalActions, NameRef, ObjectId, ObjectKind, Pending,
    Phase, PlanKind, PlayerId, Resolution, SmallVec, Status, TypeSet, Zone, ZoneLocation,
    ZonePosition, casting, cost_wizard, eval, resolve,
};
use crate::choice::TargetPrompt;
use baylee_cards_dsl::{ActivationLimit, ActivationZone};

/// The cost parts an activation would leave unpaid, and think it had paid.
///
/// These are paid in `cast_wizard` — the pitch of Force of Will, the X of
/// Toxic Deluge — which is a path `pay_cost` is never shown, so it walks past
/// both. For a spell's alternative or additional cost that is exactly right.
/// For an **activated** ability there is no wizard, and the ability is
/// offered, pressed, and its part silently not paid — a pitch cost that
/// exiles nothing.
///
/// This used to have a twin, `choice_cost_unpayable`, which named the parts an
/// activation could not pay *at all* — a sacrifice or a discard, because
/// nothing could ask which one. `cost_wizard` asks now, so that half is gone
/// and what is left here is the opposite failure: a part that would be
/// *accepted and then skipped*. `can_afford` is no longer the other half of
/// the sentence either. It reads `ExileFromHand` for real (there has to be a
/// card to exile, or the offer is dead), and the wizard caps `PayLifeX` at
/// the caster's life.
///
/// That makes this the invisible one of the two failures. A cost that cannot
/// be paid ends in an error; this is offered and then granted for free, and
/// the activation *succeeds* — no test of an action can fail on it, and the
/// only evidence is the card still sitting in a hand that should have paid
/// it. Which is why
/// the reader that matters is a pool-wide guard,
/// `offer_tests::nothing_in_the_pool_carries_an_activated_cost_the_engine_would_skip`,
/// and why nothing here refuses: the parts are correct where they are used,
/// and the guard is what keeps them from being used anywhere else.
///
/// Exhaustive on purpose, and this is the enum where that matters most: a
/// `matches!` answers a *new* cost part with `false`, which here means "pay
/// it for real" and is the safe direction — but the same shape one function
/// down in `can_afford` answered `CostPart::TapOther` with silence and lost
/// the house AI a card. A part added to this enum should have to say which
/// side of this line it is on, in one word, at compile time.
pub(crate) const fn paid_by_the_casting_wizard(part: &CostPart) -> bool {
    match part {
        CostPart::ExileFromHand(_) | CostPart::PayLifeX => true,
        CostPart::TapSelf
        | CostPart::UntapSelf
        | CostPart::SacrificeSelf
        | CostPart::Sacrifice(_)
        | CostPart::PayLife(_)
        | CostPart::Discard(_)
        | CostPart::TapOther(_)
        | CostPart::ReturnToHand(_)
        | CostPart::ExileFromGraveyard(_)
        | CostPart::DiscardSelf
        | CostPart::ExileSelf
        | CostPart::ReturnSelfToHand
        | CostPart::RemoveCounterSelf { .. }
        | CostPart::RemoveCounterSelfX { .. }
        | CostPart::PutCounterSelf { .. } => false,
    }
}

/// The counter a cost asks the player for a *number* of, if it asks at all.
///
/// A finder rather than a classifier, which is why the `_` arm is honest
/// here and is not in `paid_by_the_casting_wizard` above: the question is
/// "is this that one variant", and a part added tomorrow is not it.
///
/// One part at most. A cost with two of these would be two numbers and one
/// answer, and `activation_x` holds one — the pool prints seventeen such
/// costs and every one of them removes a single kind.
fn counter_x_part(cost: &Cost) -> Option<baylee_cards_dsl::CounterKind> {
    cost.parts.iter().find_map(|part| match part {
        CostPart::RemoveCounterSelfX { kind } => Some(*kind),
        _ => None,
    })
}

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
    fn has_a_legal_target(&self, player: PlayerId, card: ObjectId) -> bool {
        // The front face's own line, which is what this gate has always
        // asked. The other faces a card may be cast as are asked in
        // `casting::can_cast`, beside the probe that says whether each of
        // them can be paid for — the two belong together, because a face
        // that cannot be paid for is not a way of casting the card whatever
        // it targets.
        casting::face_has_a_legal_target(&self.state, &self.lookup, player, card, 0)
    }
}

impl<L: CardLookup> Engine<L> {
    #[allow(clippy::too_many_lines)]
    pub(crate) fn compute_legal(&self, player: PlayerId) -> LegalActions {
        let mut legal = LegalActions {
            can_pass: true,
            ..LegalActions::default()
        };
        let main_phase = matches!(self.state.turn.phase, Phase::FirstMain | Phase::SecondMain);
        let sorcery_timing =
            main_phase && self.state.turn.active == player && self.state.zones.stack_is_empty();
        // The graveyard is walked beside the hand, and only for the lands
        // in it: Crucible of Worlds is a permission to *play a land* from
        // there and says nothing about casting anything, so the loop below
        // asks `can_cast` of a hand card and never of this one. Which zones
        // are open is `casting::land_zone_open`, the same reader
        // `casting::play_land` refuses with — an offer and a refusal that
        // disagreed would be a land this engine lists and then will not
        // let go of.
        let graveyard: &[ObjectId] =
            if casting::land_zone_open(&self.state, player, Zone::Graveyard) {
                self.state.zones.list(ZoneLocation::Graveyard(player))
            } else {
                &[]
            };
        for (&card, from_hand) in self
            .state
            .zones
            .list(ZoneLocation::Hand(player))
            .iter()
            .map(|c| (c, true))
            .chain(graveyard.iter().map(|c| (c, false)))
        {
            let Some(obj) = self.state.object(card) else {
                continue;
            };
            let any_face_is_land = if obj.characteristics().types.contains(TypeSet::LAND) {
                true
            } else {
                // MDFC: a back land face is playable (CR 712.12).
                obj.card
                    .and_then(|c| self.lookup.card(c.index))
                    .is_some_and(|def| def.faces.iter().any(|f| f.types.contains(TypeSet::LAND)))
            };
            if any_face_is_land
                && sorcery_timing
                && casting::has_a_land_drop_left(&self.state, player)
            {
                legal.lands.push(card);
            }
            if !from_hand {
                continue;
            }
            if casting::can_cast(&self.state, &self.lookup, player, card).is_ok()
                && self.has_a_legal_target(player, card)
            {
                legal.castable.push(card);
            }
        }
        // Adventure + Opposition Agent: cards castable from exile.
        for &card in self.state.zones.list(ZoneLocation::Exile(player)) {
            let exiled_castable = self.state.object(card).is_some_and(|o| {
                o.riders.contains(&crate::object::Rider::Adventure)
                    || o.riders
                        .iter()
                        .any(|r| matches!(r, crate::object::Rider::PlayableFromExileFor(p) if *p == player))
            });
            if exiled_castable
                && casting::can_cast(&self.state, &self.lookup, player, card).is_ok()
                && self.has_a_legal_target(player, card)
            {
                legal.castable.push(card);
            }
        }
        // Commander (CR 903.8): your own commanders in the command zone.
        // The emblems sharing that zone are filtered out by `can_cast`,
        // which asks the marker list rather than the zone.
        for &card in self.state.zones.list(ZoneLocation::Command(player)) {
            if casting::can_cast(&self.state, &self.lookup, player, card).is_ok()
                && self.has_a_legal_target(player, card)
            {
                legal.castable.push(card);
            }
        }
        // Flashback (CR 702.34) and disturb (CR 702.146): a card in your own
        // graveyard is castable when a grant or a printed face says so, and
        // `can_cast` asks both of those itself — for a graveyard card it is
        // exactly `flashback_ok || disturb_ok` that keeps it out of
        // `CastError::NotInHand`. So this sweep asks nothing. The pre-check
        // it used to carry was not a fast path, it was a second opinion:
        // it recognised only the `EffectFilter::ObjectIs` shape of a grant
        // and hid every filtered one from the offer.
        for &card in self.state.zones.list(ZoneLocation::Graveyard(player)) {
            if casting::can_cast(&self.state, &self.lookup, player, card).is_ok()
                && self.has_a_legal_target(player, card)
            {
                legal.castable.push(card);
            }
        }
        for &id in self.state.zones.list(ZoneLocation::Battlefield) {
            // Karn's lock, asked on the offering side too. It stops every
            // activated ability of the permanent, a mana ability included —
            // CR 605.1 makes a mana ability a kind of activated ability, not
            // an exception to one. What it does *not* stop is a cast, which
            // is why this is two guards and not a `continue`: the Prepared
            // rider below offers a spell.
            let locked = self.artifact_activations_are_locked(id);
            if !locked
                && casting::can_activate_mana(&self.state, player, id)
                && !casting::intrinsic_mana_offer(&self.state, &self.lookup, id).is_empty()
            {
                legal.mana_abilities.push(id);
            }
            // Activated abilities of controlled permanents.
            let Some(obj) = self.state.object(id) else {
                continue;
            };
            if obj.controller != player {
                continue;
            }
            // A token's abilities come from its definition rather than from a
            // card; everything below reads the same `AbilityDef`s either way.
            let offered: &[AbilityDef] = if locked {
                &[]
            } else {
                obj.abilities(&self.lookup)
            };
            for (i, ability) in offered.iter().enumerate() {
                match ability {
                    AbilityDef::Activated {
                        cost,
                        timing,
                        zone,
                        targets,
                        second_targets,
                        limit,
                        ..
                    } => {
                        if *zone != ActivationZone::Battlefield {
                            continue; // hand-zone abilities are scanned below
                        }
                        if *timing == ActivationTiming::SorcerySpeed && !sorcery_timing {
                            continue;
                        }
                        if self.activation_limit_spent(id, i as u32, *limit) {
                            continue;
                        }
                        if !self.ability_has_a_target(player, id, *targets)
                            || !self.ability_has_a_target(player, id, *second_targets)
                        {
                            continue;
                        }
                        if self.can_afford(player, id, cost, casting::SpendFor::Ability(id)) {
                            legal.abilities.push((id, i as u32));
                        }
                    }
                    AbilityDef::ActivatedConditional {
                        cost,
                        timing,
                        zone,
                        condition,
                        targets,
                        second_targets,
                        limit,
                        ..
                    } => {
                        if *zone != ActivationZone::Battlefield {
                            continue;
                        }
                        if *timing == ActivationTiming::SorcerySpeed && !sorcery_timing {
                            continue;
                        }
                        if self.activation_limit_spent(id, i as u32, *limit) {
                            continue;
                        }
                        if !crate::eval::condition_holds(&self.state, player, id, *condition) {
                            continue;
                        }
                        if !self.ability_has_a_target(player, id, *targets)
                            || !self.ability_has_a_target(player, id, *second_targets)
                        {
                            continue;
                        }
                        if self.can_afford(player, id, cost, casting::SpendFor::Ability(id)) {
                            legal.abilities.push((id, i as u32));
                        }
                    }
                    AbilityDef::Loyalty { cost, targets, .. } => {
                        // Loyalty abilities: sorcery timing, once per turn
                        // per walker, enough loyalty for negative costs.
                        if !sorcery_timing || self.loyalty_used_this_turn.contains(&id) {
                            continue;
                        }
                        let loyalty = obj.counters.get(baylee_cards_dsl::CounterKind::Loyalty);
                        if *cost < 0 && loyalty < (-*cost) as u16 {
                            continue;
                        }
                        if !self.ability_has_a_target(player, id, *targets) {
                            continue;
                        }
                        legal.abilities.push((id, i as u32));
                    }
                    _ => {}
                }
            }
            // Prepared (Emeritus of Woe): a prepared permanent may cast
            // a copy of its linked spell — synthetic index `choice::PREPARED_CAST`.
            if obj.riders.contains(&crate::object::Rider::Prepared) {
                let linked = obj.card.and_then(|c| {
                    let face = obj.face_index as usize;
                    self.lookup.card(c.index).and_then(|def| {
                        def.abilities_for_face(face).iter().find_map(|a| match a {
                            AbilityDef::Prepared { card } => Some(*card),
                            _ => None,
                        })
                    })
                });
                if let Some(linked_card) = linked {
                    let castable = self.lookup.card(linked_card).is_some_and(|spell_def| {
                        let face = &spell_def.faces[0];
                        self.prepared_cast_is_timely(player, spell_def)
                            && casting::affordable(
                                &self.state,
                                &self.state.players[player.get() as usize].mana_pool,
                                &face.mana_cost,
                            )
                    });
                    if castable {
                        legal.abilities.push((id, crate::choice::PREPARED_CAST));
                    }
                }
            }
            // Karn's lock reaches the grants below as well: an ability a
            // continuous effect gave an artifact is an activated ability of
            // that artifact like any other. This is the third door onto the
            // same list — printed, intrinsic, granted — and the one that made
            // moving the check to the top of `start_activation` necessary,
            // because `start_granted_activation` returns before the branch it
            // used to sit in. (Prepared, in between, is the door the lock
            // leaves open: that one offers a spell.)
            if locked {
                continue;
            }
            // Granted abilities (Urza's Saga chapters, a Chromatic Lantern's
            // lands): each surfaces under its own synthetic index. The slot
            // is the *position among the grants that apply*, affordable or
            // not — an index that shifted when a cost became payable would
            // name a different ability from one priority window to the next.
            for (n, granted) in crate::effects::granted_activated(&self.state, id)
                .take(crate::choice::GRANTED_SLOTS as usize)
                .enumerate()
            {
                if !self.can_afford(player, id, &granted.cost, casting::SpendFor::Ability(id)) {
                    continue;
                }
                legal
                    .abilities
                    .push((id, crate::choice::granted_ability(n as u32)));
                // Once per permanent, not once per ability.
                // `PlayerAction::ActivateManaAbility` names a *source* and no
                // index, so a second entry for the same permanent is an offer
                // nothing can accept: the first press takes whichever ability
                // `activate_mana` prefers, the permanent is tapped, and the
                // duplicate is refused by the list that put it there. A
                // Chromatic Lantern entered every land on the board twice.
                //
                // Which one the single entry stands for is settled in
                // `apply`: intrinsic first, and the granted ability is
                // reached by naming `GRANTED_ABILITY` in `legal.abilities`,
                // where it also appears.
                if granted.mana_ability && !legal.mana_abilities.contains(&id) {
                    legal.mana_abilities.push(id);
                }
            }
        }
        // Hand-zone activations (cycling) and suspensions.
        for &card in self.state.zones.list(ZoneLocation::Hand(player)) {
            let Some(obj) = self.state.object(card) else {
                continue;
            };
            let Some(card_ref) = obj.card else { continue };
            let Some(def) = self.lookup.card(card_ref.index) else {
                continue;
            };
            for (i, ability) in def
                .abilities_for_face(obj.face_index as usize)
                .iter()
                .enumerate()
            {
                match ability {
                    AbilityDef::Activated {
                        cost,
                        timing,
                        zone,
                        targets,
                        second_targets,
                        limit,
                        ..
                    } => {
                        if *zone != ActivationZone::Hand {
                            continue;
                        }
                        if *timing == ActivationTiming::SorcerySpeed && !sorcery_timing {
                            continue;
                        }
                        if self.activation_limit_spent(card, i as u32, *limit) {
                            continue;
                        }
                        // The same probe as the battlefield arms, for the
                        // reason `ability_has_a_target` gives. This arm once
                        // asked only about the turn and the price, so
                        // Rustic Clachan's reinforce was offered on a board
                        // with no creature and then refused with "no legal
                        // targets". The source is the card in hand, so a
                        // filter saying "another" still reads it right.
                        if !self.ability_has_a_target(player, card, *targets)
                            || !self.ability_has_a_target(player, card, *second_targets)
                        {
                            continue;
                        }
                        if self.can_afford(player, card, cost, casting::SpendFor::Ability(card)) {
                            legal.abilities.push((card, i as u32));
                        }
                    }
                    AbilityDef::ActivatedConditional {
                        cost,
                        timing,
                        zone,
                        condition,
                        targets,
                        second_targets,
                        limit,
                        ..
                    } => {
                        // The same ability with a precondition on it — the
                        // battlefield scan above has both arms, and this one
                        // had only the first, so a cycling ability behind an
                        // "activate only if…" clause would never be offered
                        // at all. No card in the pool prints one today; the
                        // hole is closed rather than recorded, because the
                        // arm is four lines longer than the note would be.
                        if *zone != ActivationZone::Hand {
                            continue;
                        }
                        if *timing == ActivationTiming::SorcerySpeed && !sorcery_timing {
                            continue;
                        }
                        if self.activation_limit_spent(card, i as u32, *limit) {
                            continue;
                        }
                        if !crate::eval::condition_holds(&self.state, player, card, *condition) {
                            continue;
                        }
                        if !self.ability_has_a_target(player, card, *targets)
                            || !self.ability_has_a_target(player, card, *second_targets)
                        {
                            continue;
                        }
                        if self.can_afford(player, card, cost, casting::SpendFor::Ability(card)) {
                            legal.abilities.push((card, i as u32));
                        }
                    }
                    // Suspend's first ability is an activated one with a cost
                    // and "activate only as a sorcery" on it (CR 702.62a) —
                    // "rather than cast this card from your hand, **pay
                    // {U}** and exile it", as Ancestral Vision's reminder
                    // text puts it. So it is offered on the same two
                    // conditions as every other activation above, and this
                    // was the one branch here that asked only about the
                    // turn: a card with suspend was offered as suspendable
                    // off an empty pool, and `actions.rs` — which *does* pay
                    // the cost — then refused it. An offer the answer
                    // disagrees with is worse than no offer, because the
                    // client draws it as something to click.
                    AbilityDef::Suspend { cost, .. }
                        if sorcery_timing
                            && self.can_pay_mana(player, casting::SpendFor::Other, cost) =>
                    {
                        legal.suspendable.push(card);
                    }
                    _ => {}
                }
            }
        }
        self.narrow_to_mana_window(player, &mut legal);
        legal
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
    fn narrow_to_mana_window(&self, player: PlayerId, legal: &mut LegalActions) {
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
        legal.lands.clear();
        legal.castable.clear();
        legal.suspendable.clear();
        legal.abilities.retain(|&(source, index)| {
            let Some(obj) = self.state.object(source) else {
                return false;
            };
            if let Some(slot) = crate::choice::granted_slot(index) {
                return crate::effects::granted_activated(&self.state, source)
                    .nth(slot as usize)
                    .is_some_and(|granted| granted.mana_ability);
            }
            obj.abilities(&self.lookup)
                .get(index as usize)
                .is_some_and(AbilityDef::is_mana_ability)
        });
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
    fn ability_has_a_target(
        &self,
        player: PlayerId,
        source: ObjectId,
        targets: Option<baylee_cards_dsl::TargetReq>,
    ) -> bool {
        let Some(req) = targets else {
            return true;
        };
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
        let objects = eval::target_options(&req.spec, &self.state, player, source).len();
        let players = eval::target_player_options(&self.state, &req.spec, player).len();
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
    /// [`Engine::ability_has_a_target`] gives. The lock lived only in `apply`,
    /// which is invisible while the walker is yours — the abilities it stops
    /// are on the *other* seat's board, and that seat was still being offered
    /// every one of them.
    pub(crate) fn artifact_activations_are_locked(&self, id: ObjectId) -> bool {
        let Some(obj) = self.state.object(id) else {
            return false;
        };
        obj.characteristics()
            .types
            .contains(baylee_core::types::TypeSet::ARTIFACT)
            && self.state.effects.iter().any(|fx| {
                matches!(
                    fx.modifier,
                    baylee_cards_dsl::Modifier::CantActivateArtifacts
                ) && self.state.is_opponent(obj.controller, fx.controller)
            })
    }

    /// Whether a printed "activate only once each turn" has already been
    /// spent on this ability this turn.
    ///
    /// The tally is [`GameState::ability_fires`], shared with once-per-turn
    /// triggers: both are counts of *this ability of this object* within a
    /// turn, both are keyed the same way, and both are cleared as a turn
    /// begins. The key carries the object, so a permanent that leaves and
    /// comes back is a new object with a fresh count — which is CR 400.7
    /// rather than a convenience.
    ///
    /// Asked on the offering side, so an exhausted ability is simply not in
    /// `legal.abilities`; `apply`'s offer guard is then the whole of the
    /// refusal, the way it is for every other activation restriction.
    fn activation_limit_spent(&self, source: ObjectId, index: u32, limit: ActivationLimit) -> bool {
        match limit {
            ActivationLimit::Unlimited => false,
            ActivationLimit::PerTurn(n) => {
                self.state
                    .ability_fires
                    .get(&(source, index))
                    .copied()
                    .unwrap_or(0)
                    >= u32::from(n)
            }
        }
    }

    /// Whether `player`'s pool covers a bare mana cost paid for `what`.
    ///
    /// Split out of [`Self::can_afford`] for suspend, whose cost is a
    /// `ManaCost` and not a `Cost` — it has no parts to check. An offer that
    /// asked the same question a second way would be free to answer it
    /// differently, and this one is asked against the pool
    /// [`casting::pay_mana_for`] then spends: the plain counters plus the
    /// restricted mana `what` may use.
    pub(crate) fn can_pay_mana(
        &self,
        player: PlayerId,
        what: casting::SpendFor,
        cost: &baylee_core::mana::ManaCost,
    ) -> bool {
        let with_restricted = casting::spendable_pool(&self.state, player, what);
        let pool = with_restricted
            .as_ref()
            .unwrap_or(&self.state.players[player.get() as usize].mana_pool);
        // Mycosynth Lattice: mana spends as though it were any colour, so a
        // five-colour activation cost is payable off five Islands.
        casting::affordable(&self.state, pool, cost)
    }

    pub(crate) fn can_afford(
        &self,
        player: PlayerId,
        source: ObjectId,
        cost: &Cost,
        what: casting::SpendFor,
    ) -> bool {
        if !self.can_pay_mana(player, what, &cost.mana) {
            return false;
        }
        for part in cost.parts {
            match part {
                // CR 302.6, second sentence: a creature's activated ability
                // with the tap or the untap symbol in its cost cannot be
                // activated unless the creature has been under its
                // controller's control since their most recent turn began.
                // The check belongs here and not in the cost payment: a
                // creature tapped to pay someone *else's* cost (convoke,
                // crew) is not activating an ability of its own, and
                // summoning sickness has never stopped that.
                CostPart::TapSelf | CostPart::UntapSelf => {
                    let Some(obj) = self.state.object(source) else {
                        return false;
                    };
                    if crate::combat::summoning_sick(&self.state, obj) {
                        return false;
                    }
                    // {T} needs it untapped. {Q} ought to need it tapped and
                    // does not — `docs/observed-faults.md` entry 53.
                    if matches!(part, CostPart::TapSelf) && obj.status.contains(Status::TAPPED) {
                        return false;
                    }
                }
                CostPart::PayLife(n) => {
                    if !self.state.can_pay_life(player, i32::from(*n)) {
                        return false;
                    }
                }
                // A pitch cost with nothing to pitch is not an offer. The
                // list is `casting::pitchable`, the same one the wizard's
                // `PitchChoice` stage puts in front of the player, because a
                // mode this accepts and that stage then refuses is a cast
                // that reverses itself under the player's hands.
                CostPart::ExileFromHand(filter) => {
                    if casting::pitchable(&self.state, player, source, filter).is_empty() {
                        return false;
                    }
                }
                // A cost that has to ask a question, asked of the board
                // instead of refused outright. `cost_wizard::options` is the
                // one reader of "what may pay this", and it is the same list
                // the player is shown a moment later — an offer whose
                // payment then finds nothing is the contradiction
                // `offer_tests` exists to catch.
                CostPart::Sacrifice(_)
                | CostPart::Discard(_)
                | CostPart::TapOther(_)
                | CostPart::ReturnToHand(_)
                | CostPart::ExileFromGraveyard(_) => {
                    // **As many candidates as the cost asks questions.** A
                    // cost may print the same one more than once — Time
                    // Sieve's "Sacrifice five artifacts" is five
                    // `CostPart::Sacrifice` parts, one permanent and one
                    // question each — and each answer is a *different*
                    // permanent, because `cost_wizard` takes what was chosen
                    // out of the list before it asks again. Asking each
                    // occurrence whether the list is non-empty said yes to
                    // five sacrifices with one artifact on the table, and the
                    // payment then refused the second: the engine offered a
                    // thing and took it back, which is exactly the
                    // contradiction `offer_tests` exists to catch.
                    //
                    // Parts that are *equal* are counted, not parts of the
                    // same kind: two `Sacrifice` parts over different filters
                    // are two questions with two lists, and the overlap
                    // between them is a matching problem this does not
                    // pretend to solve. Nothing in the pool prints one.
                    let asked = cost.parts.iter().filter(|other| *other == part).count();
                    if cost_wizard::menu(&self.state, player, source, cost, part).len() < asked {
                        return false;
                    }
                }
                // Arithmetic on the source, and nobody is asked anything:
                // 39 of the pool's 40 counter costs take the counters off
                // the permanent whose ability it is, so there is exactly one
                // legal answer and it is not a question. See
                // `CostPart::RemoveCounterSelf`.
                CostPart::RemoveCounterSelf { kind, n } => {
                    let Some(obj) = self.state.object(source) else {
                        return false;
                    };
                    if obj.counters.get(*kind) < *n {
                        return false;
                    }
                }
                // The first four are paid off the source alone, so there is
                // nothing about the board to ask. `PayLifeX` is
                // [`paid_by_the_casting_wizard`]: accepted here and skipped
                // by `pay_cost`, which is right for a spell — the wizard caps
                // it at the caster's life — and would hand an activation half
                // its cost for free. Nothing in the pool prints one on an
                // activated ability, and
                // `offer_tests::nothing_in_the_pool_carries_an_activated_cost_the_engine_would_skip`
                // is what keeps it that way.
                //
                // `RemoveCounterSelfX` is in the same list for a different
                // reason and is **always affordable, deliberately**: zero is
                // a legal answer to "remove any number of storage counters",
                // so a storage land with nothing stored may still be tapped
                // for no mana at all — that is the card, not a hole. Nothing
                // loops on it because every cost that prints it prints `{T}`
                // or a mana part beside it, which is what
                // `baylee_ai::activate::consumes` reads.
                //
                // `PutCounterSelf` is the third reason in the same list: a
                // permanent can always take a counter, so there is nothing
                // to check. What bounds it is the counter, not the offer —
                // Devoted Druid's second -1/-1 leaves a 0/0 that CR 704.5f
                // puts in the graveyard, and a permanent in a graveyard is
                // offered nothing.
                CostPart::SacrificeSelf
                | CostPart::DiscardSelf
                | CostPart::ExileSelf
                | CostPart::ReturnSelfToHand
                | CostPart::RemoveCounterSelfX { .. }
                | CostPart::PutCounterSelf { .. }
                | CostPart::PayLifeX => {}
            }
        }
        true
    }

    // ---------------------------------------------------- S3: abilities

    /// Whether `player` could begin casting a copy of `spell_def` right now.
    ///
    /// A prepared permanent says "you may **cast** a copy of its spell", and
    /// grants no exception to when that spell may be cast — so the copy is
    /// held to the linked card's own timing, through the same
    /// [`casting::timing_allows`] every other cast goes through. Emeritus of
    /// Woe's spell is Demonic Tutor, a sorcery (CR 307.1), and the offer
    /// asked about the mana and nothing else: a prepared Warlock was a tutor
    /// at instant speed on anybody's turn.
    ///
    /// The linked card's own timing rather than a flat sorcery speed,
    /// because nothing about being prepared slows a spell down — a prepared
    /// permanent whose spell were an instant would rightly offer it at
    /// instant speed. Sharing the function is what carries the two
    /// player-scoped effects a hand-rolled copy of the rule would have lost:
    /// Teferi's +1 gives that player's sorceries flash, and his static pulls
    /// every opponent's spell back to sorcery speed.
    ///
    /// `keywords_for_face(0)` and not `faces[0].keywords`: a single-faced
    /// card states its keywords once at card level and leaves the face's own
    /// list empty, so reading the face directly would lose the flash on
    /// every card that has one.
    fn prepared_cast_is_timely(
        &self,
        player: PlayerId,
        spell_def: &baylee_cards_dsl::CardDef,
    ) -> bool {
        casting::timing_allows(
            &self.state,
            player,
            spell_def.faces[0].types,
            spell_def.keywords_for_face(0),
        )
    }

    /// Prepared cast (Emeritus of Woe): pays the linked spell's cost,
    /// puts a copy of it on the stack, and removes the prepared marker.
    fn start_prepared_cast(
        &mut self,
        player: PlayerId,
        source: ObjectId,
    ) -> Result<(), EngineError> {
        let linked_card = self
            .state
            .object(source)
            .and_then(|o| o.card)
            .and_then(|c| {
                let face = self
                    .state
                    .object(source)
                    .map_or(0, |o| o.face_index as usize);
                self.lookup.card(c.index).and_then(|def| {
                    def.abilities_for_face(face).iter().find_map(|a| match a {
                        AbilityDef::Prepared { card } => Some(*card),
                        _ => None,
                    })
                })
            })
            .ok_or(EngineError::IllegalAction("no prepared spell"))?;
        let spell_def = self
            .lookup
            .card(linked_card)
            .ok_or(EngineError::IllegalAction("unknown linked card"))?;
        let face = &spell_def.faces[0];
        // The same question the offer asked, asked again here so the two
        // probes cannot disagree: a client that named this action out of a
        // stale `LegalActions` is refused rather than handed a sorcery on
        // the opponent's turn.
        if !self.prepared_cast_is_timely(player, spell_def) {
            return Err(EngineError::IllegalAction(
                "the prepared spell cannot be cast right now",
            ));
        }
        let wild = casting::mana_is_wild(&self.state);
        if !casting::pay_with(
            wild,
            &mut self.state.players[player.get() as usize].mana_pool,
            &face.mana_cost,
        ) {
            return Err(EngineError::IllegalAction("cannot pay the spell's cost"));
        }
        // Unprepare the source.
        if let Some(obj) = self.state.object_mut(source) {
            obj.riders
                .retain(|r| !matches!(r, crate::object::Rider::Prepared));
        }
        // The copy of the linked spell, built the way `Effect::CopyTargetSpell`
        // builds one (CR 707.10): a fresh object carrying the copied card.
        let name = self.state.names.intern(face.name);
        let base = crate::object::Characteristics::from_face(spell_def, 0, name);
        let ts = self.state.next_timestamp();
        let id = self.state.arena.insert_with(|oid| {
            let mut obj = GameObject::new_bare(oid, player, ObjectKind::Spell, base);
            obj.timestamp = ts;
            obj.cast_from_hand = false;
            // A fresh object starts in its owner's library, and putting it
            // somewhere with `Zones::insert` does not say otherwise —
            // `move_object` reads the object, not the zone lists. Left at
            // the default, the spell resolved *out of the library*: the
            // stack kept the id, the spell ceased to exist beneath it, and
            // `stack_projectable` was still pointing at nothing.
            obj.zone = Zone::Stack;
            // And it has to *be* the linked card. `resolve_stack_top` reads a
            // spell's effects off `obj.card` rather than through
            // `GameObject::abilities`, so a card-less spell resolves to
            // nothing at all — which is what this one did: Demonic Tutor went
            // on the stack, both seats passed, and no library was ever
            // searched.
            obj.card = Some(crate::object::CardRef {
                index: linked_card,
                // The rules identity is the linked card; the *printing* is
                // one nobody brought to the table, and `PrintRef::new(0)`
                // here would be another card's art under this one's name —
                // and would hand the opponent a row of the print table out
                // of a deck they have never seen.
                print: baylee_core::ids::PrintRef::UNKNOWN,
            });
            // And it is still a copy, however it got here (CR 704.5e): the
            // spell was never a card anyone owns, so a graveyard is the one
            // place it must not end up.
            obj.riders.push(crate::object::Rider::SpellCopy);
            obj
        });
        self.state
            .zones
            .insert(id, ZoneLocation::Stack, ZonePosition::Top, true);
        // Per-turn tracking, exactly as an ordinary cast keeps it. The card
        // says "you may **cast** a copy of its spell", so this is a cast and
        // the turn has to count it: without these two the prepared spell was
        // invisible to Storm of Saruman's "your second spell each turn" and
        // handed Esper Sentinel's "first noncreature spell" to whatever was
        // cast next.
        if !face.types.contains(baylee_core::types::TypeSet::CREATURE)
            && let Some(v) = self
                .state
                .per_turn
                .noncreature_spells
                .get_mut(player.get() as usize)
        {
            *v = v.saturating_add(1);
        }
        if let Some(v) = self
            .state
            .per_turn
            .spells_cast
            .get_mut(player.get() as usize)
        {
            *v = v.saturating_add(1);
        }
        self.state
            .journal
            .record(GameEvent::SpellCast { object: id, player });
        self.after_action(player);
        Ok(())
    }

    /// Activates a granted ability (Urza's Saga's Construct chapter):
    /// pays the cost, then pushes the granted effects via the synthetic
    /// side map (or resolves immediately for mana abilities).
    fn start_granted_activation(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        slot: u32,
        targets: SmallVec<[ObjectId; 2]>,
    ) -> Result<(), EngineError> {
        // `slot` counts the grants that *apply*, in the order the offer
        // counted them. Reading it back through the same iterator is what
        // makes the two agree; a mismatch would run the ability next to the
        // one the player pressed.
        let granted = crate::effects::granted_activated(&self.state, source)
            .nth(slot as usize)
            .ok_or(EngineError::IllegalAction("no granted ability"))?;
        let (cost, effects, mana_ability) = (granted.cost, granted.effects, granted.mana_ability);
        // A granted ability has no stage that asks, so a granted cost that
        // needs an answer is refused *here* rather than by the payer — which
        // is the difference between an activation that does not happen and
        // one whose mana is gone. Nothing in the pool grants such a cost
        // today; `offer_tests`' pool-wide sweep is what keeps it that way.
        if cost.parts.iter().any(cost_wizard::needs_an_answer) {
            return Err(EngineError::IllegalAction(
                "a granted ability cannot ask for its cost yet",
            ));
        }
        // A granted ability carries no counter cost — nothing in the pool
        // grants one — so there is no number to announce and none to pay.
        self.pay_cost(player, source, &cost, &[], 0)?;
        if mana_ability {
            let mut res = crate::resolve::Resolution {
                source,
                on_stack: source,
                controller: player,
                effects: crate::resolve::flatten(effects),
                pc: 0,
                targets,
                second_targets: SmallVec::new(),
                x: None,
                chosen_player: None,
                target_players: baylee_core::ids::SeatSet::new(),
                event_object: None,
                targeted: false,
                awaiting: None,
                mana_ability: true,
                countered_source: None,
                target_lki: None,
            };
            match crate::resolve::run(&mut self.state, &mut res) {
                crate::resolve::Flow::Complete => {}
                crate::resolve::Flow::Wait(pending) => {
                    self.resolution = Some(res);
                    self.pending = pending;
                    self.awaiting_answer = true;
                    return Ok(());
                }
            }
        } else {
            let name = self
                .state
                .object(source)
                .map_or(NameRef::new(0), |o| o.base.name);
            let base = self.state.bare_base(name);
            // An ability a continuous effect *granted* belongs to whatever
            // it was granted to, which need not have a card. It records
            // whichever it is, as `push_ability_to_stack` does; there is no
            // sentinel to pick any more.
            let card = self
                .state
                .object(source)
                .and_then(|o| o.card)
                .map(|c| c.index);
            let id = self.state.arena.insert_with(|id| {
                GameObject::new_ability_on_stack(
                    id,
                    player,
                    crate::object::AbilityLoc {
                        card,
                        index: baylee_core::ids::AbilityRef::SYNTHETIC,
                        source,
                    },
                    targets,
                    base,
                )
            });
            self.synthetic_fx.insert(id, effects);
            self.state
                .zones
                .insert(id, ZoneLocation::Stack, ZonePosition::Top, false);
            self.state.journal.record(GameEvent::AbilityTriggered {
                object: id,
                source,
                ability_index: baylee_core::ids::AbilityRef::SYNTHETIC,
                controller: player,
            });
        }
        self.after_action(player);
        Ok(())
    }

    #[allow(clippy::too_many_lines)] // activation is a staged checklist; extraction would obscure it
    pub(crate) fn start_activation(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        targets: SmallVec<[ObjectId; 2]>,
    ) -> Result<(), EngineError> {
        // The player half of a choice this activation has already answered,
        // put here by `apply`. Read out before anything at all can return,
        // so that an activation refused further down cannot leave it
        // standing for the next one to inherit.
        let chosen_players = std::mem::take(&mut self.activation_target_players);
        // Prepared cast (`choice::PREPARED_CAST`): pay the linked
        // spell's cost, put a copy on the stack, unprepare the source.
        if ability_index == crate::choice::PREPARED_CAST {
            return self.start_prepared_cast(player, source);
        }
        // Karn's lock, ahead of every route below it. It used to sit inside
        // the printed-ability branch, which the granted and loyalty branches
        // return before reaching — so a locked artifact's *granted* ability
        // passed under it. Structural rather than observed: the offer guard
        // above refuses first, and no test gets this far.
        if self.artifact_activations_are_locked(source) {
            return Err(EngineError::IllegalAction(
                "activated abilities of artifacts can't be activated (Karn)",
            ));
        }
        // Granted abilities (synthetic index): resolve via the side map.
        if let Some(slot) = crate::choice::granted_slot(ability_index) {
            return self.start_granted_activation(player, source, slot, targets);
        }
        // Loyalty abilities route to their own activation path first.
        if let Some(AbilityDef::Loyalty { cost, .. }) = self
            .state
            .object(source)
            .map(|o| o.abilities(&self.lookup))
            .and_then(|abilities| abilities.get(ability_index as usize))
        {
            return self.start_loyalty_activation(player, source, ability_index, targets, *cost);
        }
        let (cost, effects, (first, second), mana_ability, zone, limit) = {
            let obj = self
                .state
                .object(source)
                .ok_or(EngineError::IllegalAction("no such permanent"))?;
            // Deliberately no `obj.card` here. The list this ability was
            // offered from is the object's *own* (`GameObject::abilities`),
            // which answers for a Treasure out of its definition and for a
            // token copy out of the rules text it copied — so demanding a
            // card refused an ability the engine had just offered, which is
            // the failure mode this engine treats as worse than either half
            // of it alone. The card index it used to read was never used
            // for anything: it was discarded again further down.
            //
            // The object's own list, which is what `legal_actions` indexed
            // when it offered this. Reading the card's instead was the
            // "two probes must agree" bug once more: a Glasspool Mimic
            // copying Werefox Bodyguard was offered the sacrifice ability
            // at index 1 and refused it with "no such ability", because
            // Glasspool Mimic's printed list is one entry long.
            match obj
                .abilities(&self.lookup)
                .get(ability_index as usize)
                .ok_or(EngineError::IllegalAction("no such ability"))?
            {
                AbilityDef::Activated {
                    cost,
                    effects,
                    targets: first,
                    second_targets,
                    mana_ability,
                    zone,
                    limit,
                    ..
                } => (
                    *cost,
                    *effects,
                    (*first, *second_targets),
                    *mana_ability,
                    *zone,
                    *limit,
                ),
                AbilityDef::ActivatedConditional {
                    cost,
                    effects,
                    targets: first,
                    second_targets,
                    mana_ability,
                    zone,
                    condition,
                    limit,
                    ..
                } => {
                    if !crate::eval::condition_holds(&self.state, player, source, *condition) {
                        return Err(EngineError::IllegalAction("activation condition not met"));
                    }
                    (
                        *cost,
                        *effects,
                        (*first, *second_targets),
                        *mana_ability,
                        *zone,
                        *limit,
                    )
                }
                _ => return Err(EngineError::IllegalAction("not an activated ability")),
            }
        };
        // Read before any cost is paid, because a cost may move the source
        // and a moved copy is no longer one — see `Engine::activating_abilities`.
        self.activating_abilities = self
            .state
            .object(source)
            .map(|o| (source, o.ability_list(&self.lookup)));
        // Zone validation (battlefield abilities vs. hand abilities).
        let in_right_zone = match zone {
            ActivationZone::Battlefield => self
                .state
                .object(source)
                .is_some_and(|o| o.zone == Zone::Battlefield),
            ActivationZone::Hand => self
                .state
                .object(source)
                .is_some_and(|o| o.zone == Zone::Hand && o.zone_owner == Some(player)),
        };
        if !in_right_zone {
            return Err(EngineError::IllegalAction(
                "ability not usable from this zone",
            ));
        }
        let _ = zone;
        // CR 601.2b, and it sits *above* the targets below because that is
        // the order the rule puts them in: a number announced with the
        // ability, then the targets, then the payment. "Remove any number of
        // storage counters" names no number and neither does "Remove X
        // storage counters", so the player is asked for one, bounded by what
        // is actually on the source — an answer above that bound would be a
        // cost nothing could pay, and the bound is the whole of the
        // legality here because zero is allowed.
        //
        // Reached only through the offer guard in `apply`, which refuses an
        // ability `legal.abilities` does not carry, so nobody is asked a
        // number for an activation `can_afford` has already ruled out.
        if let Some(kind) = counter_x_part(&cost)
            && self.activation_x.is_none()
        {
            let max = u32::from(
                self.state
                    .object(source)
                    .map_or(0, |o| o.counters.get(kind)),
            );
            self.pending_plan = Some(PlanKind::ChooseActivationX {
                source,
                ability_index,
            });
            self.pending = Pending::ChooseNumber {
                player,
                min: 0,
                max,
            };
            self.awaiting_answer = true;
            return Ok(());
        }
        // The *mana* {X}, and it sits beside the counter question above
        // because CR 602.2b announces both in the same breath. Until this
        // was written only the counter half existed, so `{X}{G}` was paid
        // as `{G}` with nobody asked: Lair of the Hydra became a 0/0 and
        // died to a state-based action, and Treasure Vault, Kessig Wolf Run
        // and Blast Zone made the same silent zero.
        //
        // One answer, so one question: `activation_x` is a single field and
        // the guard is the same `is_none()`. A cost carrying *both* kinds of
        // X would take the counter bound and pay the mana with it — the
        // right *number* by CR 107.3i, under which all instances of X on an
        // object normally have one value, but bounded by the counters alone,
        // so the payment below could fail after the ability had been
        // announced. No cost in the pool does, and
        // `lints::no_cost_announces_two_different_xs` is what keeps it that
        // way.
        if cost.mana.has_variable() && self.activation_x.is_none() {
            // Bounded by what the pool can actually pay, which is where an
            // activation differs from a cast. The cast wizard offers
            // `X_CEILING` and validates at the end, because a cast that
            // cannot pay unwinds back to the player; an activation has no
            // wizard to unwind to — `pay_cost` below would return
            // `IllegalAction` and the ability would already have been
            // announced. So the question is the legality, which is where
            // this engine puts every other one.
            let max = (0..=crate::engine::cast_wizard::X_CEILING)
                .take_while(|x| {
                    self.can_pay_mana(
                        player,
                        casting::SpendFor::Ability(source),
                        &cost.mana.with_x(*x),
                    )
                })
                .last()
                .unwrap_or(0);
            self.pending_plan = Some(PlanKind::ChooseActivationX {
                source,
                ability_index,
            });
            self.pending = Pending::ChooseNumber {
                player,
                min: 0,
                max,
            };
            self.awaiting_answer = true;
            return Ok(());
        }
        // Targets, unless this activation has already answered them. That is
        // a flag and not a look at the lists, because an empty list is an
        // answer too: "up to one" answered with nothing re-enters here with
        // no objects and no players (CR 115.6). Reading the lists asked the
        // same question again forever. Before that it sent an ability whose
        // targets are all players back to the same question, because only
        // the objects were read.
        //
        // This used to be written twice — once here and once after the cost
        // was paid, identically — and the second copy was unreachable:
        // nothing between them touches `targets` or `target`. What it left
        // behind is a comment claiming the engine chooses targets after
        // paying, which is neither what it does nor what the rules say
        // (CR 601.2c chooses targets, CR 601.2h pays; an activation follows
        // the same order by CR 602.2b).
        if !self.activation_targets_answered
            && let Some(req) = first
        {
            // The count is read, and is no longer assumed to be one: "tap
            // two target lands" is two, and "up to one" is none or one
            // (CR 601.2c, by CR 602.2b). X is answered by now (CR 601.2b
            // comes first), so an X count is a number here.
            let (min, max) = req.bounds(self.activation_x.unwrap_or(0));
            let options = eval::target_options(&req.spec, &self.state, player, source);
            // Players are the other half of the same choice, and asking for
            // objects alone made three implemented lands dead: Nephalia
            // Drownyard, Duskmantle and Orzhova all say "target player",
            // whose object list is empty by construction — so each was
            // offered in `LegalActions` (which does add the two counts) and
            // then refused here with "no legal targets", the two-probes
            // disagreement this engine treats as the worst kind.
            let player_options = eval::target_player_options(&self.state, &req.spec, player);
            if options.len() + player_options.len() < min as usize {
                // The number was answered before this question was asked, so
                // an activation that dies here has one to throw away.
                self.activation_x = None;
                return Err(EngineError::IllegalAction("no legal targets"));
            }
            if max == 0 || (options.is_empty() && player_options.is_empty()) {
                // The only answer is none at all, which is a legal one
                // (CR 115.6), so nobody is asked for it. The cast wizard
                // skips the same question the same way.
                self.activation_targets_answered = true;
            } else {
                self.pending_plan = Some(PlanKind::ActivateAbility {
                    source,
                    ability_index,
                });
                self.pending = Pending::ChooseTargets {
                    player,
                    options,
                    player_options,
                    min,
                    max,
                    reason: TargetPrompt::Targets,
                };
                self.awaiting_answer = true;
                return Ok(());
            }
        }
        // The second instance of the word "target" (Contested Cliffs), asked
        // once the first has its answer and before anything is paid — both
        // are CR 601.2c, and the cost is CR 601.2h. Its options leave in what
        // the first instance chose, because CR 115.3 lets one object be
        // chosen once for each instance.
        if let Some(req) = second
            && self.activation_second_targets.is_none()
        {
            let options = eval::target_options(&req.spec, &self.state, player, source);
            if options.len() < req.min as usize {
                self.activation_x = None;
                return Err(EngineError::IllegalAction("no legal targets"));
            }
            if options.is_empty() || req.max == 0 {
                self.activation_second_targets = Some(SmallVec::new());
            } else {
                self.pending_plan = Some(PlanKind::ActivateAbilitySecondTargets {
                    source,
                    ability_index,
                    targets,
                    target_players: chosen_players,
                });
                self.pending = Pending::ChooseTargets {
                    player,
                    options,
                    player_options: Vec::new(),
                    min: req.min,
                    max: req.max,
                    reason: TargetPrompt::Targets,
                };
                self.awaiting_answer = true;
                return Ok(());
            }
        }
        if !self.can_afford(player, source, &cost, casting::SpendFor::Ability(source)) {
            self.activation_cost_choices.clear();
            self.activation_x = None;
            return Err(EngineError::IllegalAction("cannot pay the cost"));
        }
        // CR 601.2h, and the one step of it the player has to take: a cost
        // that says "sacrifice a creature" names no creature. Asked after
        // `can_afford` and never before it, so nobody is made to choose what
        // to give up for an activation that cannot happen — and asked one
        // part at a time, because a cost with two asking parts is two
        // questions and answering them together would lose which answer
        // belongs to which.
        let wanted = cost_wizard::answers_wanted(&cost);
        if self.activation_cost_choices.len() < wanted {
            let asked = self.activation_cost_choices.len();
            let part = cost_wizard::asking_parts(&cost)
                .nth(asked)
                .expect("fewer answers in hand than the cost has asking parts");
            let mut options = cost_wizard::menu(&self.state, player, source, &cost, part);
            // Nothing is paid until every question has an answer (CR 601.2h
            // pays the whole cost at once), so the board the second question
            // is asked of still holds the object the first one named. Taking
            // the answers so far off the menu is what stops a cost with two
            // sacrifice parts from eating one permanent twice. No card in
            // the pool prints such a cost today; this is one line and is
            // right for the one that does.
            options.retain(|id| !self.activation_cost_choices.contains(id));
            let prompt = cost_wizard::prompt(part);
            // `can_afford` asked the same question of the same function a
            // moment ago, so an empty list here is not a board this engine
            // can be in. Refusing rather than asserting all the same: this
            // is a rules path, and one process per game is the panic
            // boundary.
            if options.is_empty() {
                self.activation_cost_choices.clear();
                self.activation_x = None;
                return Err(EngineError::IllegalAction("nothing can pay this cost"));
            }
            self.pending_plan = Some(PlanKind::PayActivationCost {
                source,
                ability_index,
                targets,
                target_players: chosen_players,
            });
            self.pending = Pending::ChooseCards {
                player,
                options,
                min: 1,
                max: 1,
                prompt,
            };
            self.awaiting_answer = true;
            return Ok(());
        }
        let answers = std::mem::take(&mut self.activation_cost_choices);
        // Taken rather than read, for `activation_cost_choices`' reason one
        // line up: the number belongs to this activation and to no other.
        let x = self.activation_x.take().unwrap_or(0);
        self.activation_targets_answered = false;
        self.pay_cost(player, source, &cost, &answers, x)?;
        // "Activate only once each turn" is spent *here* and not at the
        // offer, because this is the line the rules count: CR 602.2 makes
        // activating an ability putting it on the stack and paying its
        // costs, and every path above this one still ends in a refusal or a
        // question. An activation abandoned over a target choice has
        // announced nothing.
        if let ActivationLimit::PerTurn(_) = limit {
            *self
                .state
                .ability_fires
                .entry((source, ability_index))
                .or_insert(0) += 1;
        }
        if mana_ability {
            // Mana abilities resolve immediately, without the stack
            // (CR 605.3b). Choice-mana abilities (any-color lands, Command
            // Tower) suspend on the color choice like any resolution.
            let mut res = Resolution {
                source,
                on_stack: source,
                controller: player,
                effects: resolve::flatten(effects),
                pc: 0,
                targets,
                // CR 605.1a: a mana ability has no target, so it has no
                // second one either — `lints::mana_ability_fault` refuses
                // either list on one.
                second_targets: SmallVec::new(),
                // The number this activation announced, which is what every
                // `Amount::X` in its effects reads: "Add {W} for each storage
                // counter removed this way" is the counters that just came
                // off as a cost.
                x: Some(x),
                chosen_player: None,
                target_players: baylee_core::ids::SeatSet::new(),
                event_object: None,
                targeted: false,
                awaiting: None,
                mana_ability: true,
                countered_source: None,
                target_lki: None,
            };
            match resolve::run(&mut self.state, &mut res) {
                resolve::Flow::Complete => {}
                resolve::Flow::Wait(pending) => {
                    self.resolution = Some(res);
                    self.pending = pending;
                    self.awaiting_answer = true;
                    return Ok(());
                }
            }
        } else {
            let ability = self.push_ability_to_stack(player, source, ability_index, targets);
            // The second instance's answer, taken so that it belongs to this
            // activation and to no other.
            if let Some(second) = self.activation_second_targets.take()
                && let Some(obj) = self.state.object_mut(ability)
            {
                obj.set_second(second, None);
            }
            // The number this activation announced, carried on the ability
            // the way a spell carries its own X (CR 601.2b). No card in the
            // pool prints a counter-X cost on an ability that uses the stack
            // — all seventeen are mana abilities, which resolve without one
            // — so this line has no card behind it and is here because the
            // alternative is an ability that pays for X and resolves with
            // nought. That is a claim about the pool rather than about this
            // function, so it is held by a scan and not by this comment:
            // `offer_tests::every_counter_x_cost_in_the_pool_is_on_a_mana_ability`.
            if x > 0
                && let Some(obj) = self.state.object_mut(ability)
            {
                obj.x_value = x;
            }
            // The seats that were targeted, written onto the ability now
            // that there is one — the same two fields the trigger path
            // writes, and for the same reason: `target_players` is the set
            // that was named, `chosen_player` the single seat
            // `PlayerRel::Chosen` reads back at resolution. It cannot wait
            // until `apply` has this call's answer, because `after_action`
            // below stacks whatever a sacrifice cost triggered, and that
            // lands above this ability.
            if !chosen_players.is_empty()
                && let Some(obj) = self.state.object_mut(ability)
            {
                obj.target_players = chosen_players.iter().copied().collect();
                if let [only] = chosen_players[..] {
                    obj.chosen_player = Some(only);
                }
            }
        }
        self.after_action(player);
        Ok(())
    }

    /// Completes a loyalty activation after targeting: pushes the ability
    /// to the stack without re-paying (cost was paid at activation).
    ///
    /// The one thing it could refuse — a source with no card — was never
    /// its to refuse: the ability came off the object's own list, which is
    /// also the list `legal_actions` offered from. With that gone there is
    /// nothing left here that can fail.
    pub(crate) fn finish_loyalty_activation(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        targets: SmallVec<[ObjectId; 2]>,
    ) {
        // A loyalty ability of a card-less permanent is a token copy of a
        // planeswalker: nothing in the pool makes one, but `legal_actions`
        // offers whatever the object's own list holds, so refusing here
        // would be the engine taking back an offer it had just made.
        let card_index = self
            .state
            .object(source)
            .and_then(|o| o.card)
            .map(|c| c.index);
        let loc = AbilityLoc {
            card: card_index,
            index: ability_index,
            source,
        };
        let name = self
            .state
            .object(source)
            .map_or(NameRef::new(0), |o| o.base.name);
        let base = self.state.bare_base(name);
        // CR 608.2, as in `push_ability_to_stack`. A loyalty cost never moves
        // the walker, so the source is still there to be read.
        let abilities = self
            .state
            .object(source)
            .map_or(crate::object::AbilityList::NONE, |o| {
                o.ability_list(&self.lookup)
            });
        let id = self.state.arena.insert_with(|id| {
            let mut obj = GameObject::new_ability_on_stack(id, player, loc, targets, base);
            obj.take_abilities(abilities);
            obj.chosen_player = self.loyalty_player_choice.take();
            obj
        });
        self.state
            .zones
            .insert(id, ZoneLocation::Stack, ZonePosition::Top, false);
        self.state.journal.record(GameEvent::AbilityTriggered {
            object: id,
            source,
            ability_index,
            controller: player,
        });
        self.after_action(player);
    }

    /// Activates a planeswalker loyalty ability: applies the loyalty cost
    /// and puts the ability on the stack (CR 606.2-606.4).
    #[allow(clippy::too_many_lines)] // loyalty activation is a staged checklist
    pub(crate) fn start_loyalty_activation(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        ability_index: u32,
        targets: SmallVec<[ObjectId; 2]>,
        cost: i8,
    ) -> Result<(), EngineError> {
        if self.loyalty_used_this_turn.contains(&source) {
            return Err(EngineError::IllegalAction("loyalty already used this turn"));
        }
        let (card_index, effects, wanted) = {
            let obj = self
                .state
                .object(source)
                .ok_or(EngineError::IllegalAction("no such permanent"))?;
            // Identity only, as everywhere else — the ability comes off the
            // object's own list, and a token copy of a planeswalker has one
            // without having a card.
            let card = obj.card.map(|c| c.index);
            let AbilityDef::Loyalty {
                effects, targets, ..
            } = obj
                .abilities(&self.lookup)
                .get(ability_index as usize)
                .ok_or(EngineError::IllegalAction("no such ability"))?
            else {
                return Err(EngineError::IllegalAction("not a loyalty ability"));
            };
            (card, *effects, *targets)
        };
        // Loyalty cost is paid at activation (CR 606.4) — after checking
        // that required targets exist, before targeting.
        let old = self.state.object(source).map_or(0, |o| {
            o.counters.get(baylee_cards_dsl::CounterKind::Loyalty)
        });
        if cost < 0 && old < (-cost) as u16 {
            return Err(EngineError::IllegalAction("not enough loyalty"));
        }
        if let Some(req) = wanted
            && req.min > 0
            && !matches!(
                req.spec,
                baylee_cards_dsl::TargetSpec::AnyPlayer | baylee_cards_dsl::TargetSpec::AnyOpponent
            )
        {
            let options = eval::target_options(&req.spec, &self.state, player, source);
            if options.len() < req.min as usize {
                return Err(EngineError::IllegalAction("no legal targets"));
            }
        }
        let new = if cost >= 0 {
            old.saturating_add(cost as u16)
        } else {
            old - (-cost) as u16
        };
        {
            let obj = self.state.object_mut(source).expect("walker exists");
            obj.counters
                .set(baylee_cards_dsl::CounterKind::Loyalty, new);
        }
        self.state.journal.record(GameEvent::CounterChanged {
            object: source,
            kind: baylee_cards_dsl::CounterKind::Loyalty,
            old,
            new,
        });
        self.loyalty_used_this_turn.push(source);
        // Targets first if required.
        if targets.is_empty()
            && let Some(req) = wanted
        {
            if matches!(
                req.spec,
                baylee_cards_dsl::TargetSpec::AnyPlayer | baylee_cards_dsl::TargetSpec::AnyOpponent
            ) {
                let options = eval::target_player_options(&self.state, &req.spec, player);
                if options.len() < req.min as usize {
                    return Err(EngineError::IllegalAction("no legal targets"));
                }
                self.pending_plan = Some(PlanKind::LoyaltyPlayer {
                    source,
                    ability_index,
                });
                self.pending = Pending::ChoosePlayer { player, options };
                self.awaiting_answer = true;
                return Ok(());
            }
            let options = eval::target_options(&req.spec, &self.state, player, source);
            if options.len() < req.min as usize {
                return Err(EngineError::IllegalAction("no legal targets"));
            }
            // "Up to one target" with nothing on the board to point at is not
            // a question: there would be one answer to it. The ability goes
            // on the stack targeting nothing, which is what the printing says
            // it may do.
            if !options.is_empty() {
                self.pending_plan = Some(PlanKind::ActivateAbility {
                    source,
                    ability_index,
                });
                self.pending = Pending::ChooseTargets {
                    player,
                    options,
                    player_options: Vec::new(),
                    min: req.min,
                    max: req.max,
                    reason: TargetPrompt::Targets,
                };
                self.awaiting_answer = true;
                return Ok(());
            }
        }
        // The ability goes on the stack.
        let loc = AbilityLoc {
            card: card_index,
            index: ability_index,
            source,
        };
        let name = self
            .state
            .object(source)
            .map_or(NameRef::new(0), |o| o.base.name);
        let base = self.state.bare_base(name);
        // CR 608.2, as in `push_ability_to_stack`. A loyalty cost never moves
        // the walker, so the source is still there to be read.
        let abilities = self
            .state
            .object(source)
            .map_or(crate::object::AbilityList::NONE, |o| {
                o.ability_list(&self.lookup)
            });
        let id = self.state.arena.insert_with(|id| {
            let mut obj = GameObject::new_ability_on_stack(id, player, loc, targets, base);
            obj.take_abilities(abilities);
            obj.chosen_player = self.loyalty_player_choice.take();
            obj
        });
        self.state
            .zones
            .insert(id, ZoneLocation::Stack, ZonePosition::Top, false);
        self.state.journal.record(GameEvent::AbilityTriggered {
            object: id,
            source,
            ability_index,
            controller: player,
        });
        self.after_action(player);
        let _ = effects;
        Ok(())
    }

    /// Pays a cost in the order the card prints it.
    ///
    /// `chosen` carries the answers to the parts that had to ask — a
    /// sacrifice, a discard — one per asking part and in the same order, put
    /// there by the stage in [`cost_wizard`] that asked. Two of the three
    /// callers pass an empty slice, and threading it is still the cheaper
    /// shape than the alternatives: paying those parts outside this function
    /// would either pay them out of the printed order or need a second payer
    /// beside this one, and a cost paid in two places is a cost that can be
    /// paid twice.
    ///
    /// # Errors
    /// [`EngineError::IllegalAction`] when the mana is not there, when a
    /// move refuses, or when an asking part has no answer left — the last of
    /// which means a caller paid without going through the stage that asks,
    /// and refusing is the point: the failure mode
    /// [`paid_by_the_casting_wizard`] documents is a part silently skipped,
    /// and a free sacrifice is worse than a refused activation.
    #[allow(clippy::too_many_lines)] // one arm per `CostPart`, and the list is the point
    pub(crate) fn pay_cost(
        &mut self,
        player: PlayerId,
        source: ObjectId,
        cost: &Cost,
        chosen: &[ObjectId],
        x: u32,
    ) -> Result<(), EngineError> {
        let mut answers = chosen.iter().copied();
        if !cost.mana.is_empty() {
            // CR 107.3a, second half: while an activated ability is on the
            // stack, any X in its activation cost equals the announced
            // value. Without `with_x` the variable pip is worth nothing and
            // every X in an activation cost was free — a no-op for the cast
            // path, whose caller substitutes before it gets here, and the
            // whole of the cost for an activated one.
            let mana = cost.mana.with_x(x);
            // Out of the pool the offer read, restricted mana this ability
            // may spend included, and before any other part. A part that
            // sacrifices, discards or exiles the source would otherwise move
            // it before a restriction naming "abilities of creatures" reads
            // what it is. Nothing that admits an ability carries a rider, so
            // what was spent needs no second look.
            casting::pay_mana_for(
                &mut self.state,
                player,
                casting::SpendFor::Ability(source),
                &mana,
            )
            .ok_or(EngineError::IllegalAction("not enough mana"))?;
        }
        for part in cost.parts {
            if paid_by_the_casting_wizard(part) {
                continue;
            }
            match part {
                CostPart::TapSelf => {
                    self.state.set_tapped(source, true);
                    self.state.journal.record(GameEvent::ObjectTapped {
                        object: source,
                        cause: Cause::Cost,
                    });
                }
                CostPart::UntapSelf => {
                    self.state.set_tapped(source, false);
                }
                CostPart::DiscardSelf => {
                    let owner = self.state.object(source).map_or(player, |o| o.owner);
                    self.state.move_object(
                        source,
                        ZoneLocation::Graveyard(owner),
                        ZonePosition::Top,
                        Cause::Cost,
                    )?;
                }
                CostPart::SacrificeSelf => {
                    let owner = self.state.object(source).map_or(player, |o| o.owner);
                    self.state.move_object(
                        source,
                        ZoneLocation::Graveyard(owner),
                        ZonePosition::Top,
                        Cause::Cost,
                    )?;
                }
                CostPart::ReturnSelfToHand => {
                    let owner = self.state.object(source).map_or(player, |o| o.owner);
                    self.state.move_object(
                        source,
                        ZoneLocation::Hand(owner),
                        ZonePosition::Top,
                        Cause::Cost,
                    )?;
                }
                CostPart::PayLife(n) => {
                    let p = &mut self.state.players[player.get() as usize];
                    let old = p.life;
                    p.life -= i32::from(*n);
                    let new = p.life;
                    self.state.journal.record(GameEvent::LifeChanged {
                        player,
                        old,
                        new,
                        cause: Cause::Cost,
                    });
                }
                CostPart::ExileSelf => {
                    let owner = self.state.object(source).map_or(player, |o| o.owner);
                    self.state.move_object(
                        source,
                        ZoneLocation::Exile(owner),
                        ZonePosition::Top,
                        Cause::Cost,
                    )?;
                }
                // Already skipped, by [`paid_by_the_casting_wizard`] above —
                // and *skipped* is the word, because an activation has no
                // wizard to pay them anywhere else. Named here rather than
                // swept into a `_` so that a new `CostPart` is still a
                // compile error in this match.
                CostPart::ExileFromHand(_) | CostPart::PayLifeX => {}
                // Through the removal door rather than off the object by
                // hand, because the journal entry and the projection
                // invalidation live there: a +1/+1 counter spent on Walking
                // Ballista's ping makes the creature smaller, and the
                // state-based action that then kills it reads a projection
                // this is what refreshes.
                //
                // Refusing an object with too few is `can_afford`'s job and
                // already done — but a cost is paid part by part, and a
                // part that *empties* the source (a sacrifice, an exile, a
                // bounce) may sit before this one in the printed order. So
                // the refusal is here too, after the door has said how many
                // it actually found, and it is a refusal rather than a
                // saturating shrug: paying two counters out of a permanent
                // holding one is not a discount.
                CostPart::RemoveCounterSelf { kind, n } => {
                    if crate::replacement::remove_counters(&mut self.state, source, *kind, *n) < *n
                    {
                        return Err(EngineError::IllegalAction(
                            "not enough counters to pay the cost",
                        ));
                    }
                }
                // The number the player announced, through the same door as
                // the fixed one above — a removal takes no multiplier either
                // way (CR 614.16 is about counters being *put* on). The
                // refusal is the same too, and is not dead code: the bound
                // was read when the question was asked, and a counter can
                // leave in between (Thief of Blood in response).
                // `record_counters` and not `put_counters`, which is the
                // door that applies the doubling replacements. CR 614.16:
                // "if an effect would put one or more counters on a
                // permanent" applies to what the effect of a resolving spell
                // or ability puts there and to what another replacement puts
                // there — a cost is neither. Doubling Season is the pool's
                // only such replacement and prints exactly that wording, so
                // this is the measured answer and not a cautious one.
                CostPart::PutCounterSelf { kind, n } => {
                    crate::replacement::record_counters(&mut self.state, source, *kind, *n);
                }
                CostPart::RemoveCounterSelfX { kind } => {
                    let want = u16::try_from(x).unwrap_or(u16::MAX);
                    if crate::replacement::remove_counters(&mut self.state, source, *kind, want)
                        < want
                    {
                        return Err(EngineError::IllegalAction(
                            "not enough counters to pay the cost",
                        ));
                    }
                }
                // The parts that had to ask, paid with the answers in the
                // order they were asked for, through the same doors as the
                // `SacrificeSelf`, `DiscardSelf` and `TapSelf` arms above —
                // so a sacrifice a player chose and one the card named
                // cannot come out as two different events, and neither can
                // a tap.
                CostPart::Sacrifice(_)
                | CostPart::Discard(_)
                | CostPart::TapOther(_)
                | CostPart::ReturnToHand(_)
                | CostPart::ExileFromGraveyard(_) => {
                    let Some(card) = answers.next() else {
                        return Err(EngineError::IllegalAction(
                            "a cost that has to ask reached the payer unanswered",
                        ));
                    };
                    cost_wizard::pay(&mut self.state, player, part, card)?;
                }
            }
        }
        Ok(())
    }

    /// Puts one of `source`'s abilities on the stack (CR 603.3 for a
    /// trigger, CR 602.2a for an activation).
    ///
    /// It cannot fail, and that is the point. It used to refuse a source
    /// with no card — and both of its trigger callers discarded the refusal
    /// with `let _ =`, so a token's triggered ability was thrown away
    /// silently and the line after the call then wrote the trigger's
    /// `event_object` onto whatever happened to be on top of the stack
    /// instead, which is a bystander's target context taking a stranger's.
    /// The emblem case was walked around rather than fixed: a second,
    /// almost identical function existed for it and the callers chose
    /// between them on `ObjectKind::Emblem`, which is the wrong question —
    /// what this needs to know is whether the source has a card, and an
    /// emblem is only one of the things that has none.
    ///
    /// Returns the ability object it made, because two callers need it back
    /// and both used to find it by reading the top of the stack — true, and
    /// true only because nothing runs in between.
    pub(crate) fn push_ability_to_stack(
        &mut self,
        controller: PlayerId,
        source: ObjectId,
        ability_index: u32,
        targets: SmallVec<[ObjectId; 2]>,
    ) -> ObjectId {
        // An emblem (CR 114.2), a token and a token copy have no card, and
        // the handle a client is given says so. What resolves is the list
        // captured below (CR 608.2), as for any ability.
        let card = self
            .state
            .object(source)
            .and_then(|o| o.card)
            .map(|c| c.index);
        let name = self
            .state
            .object(source)
            .map_or(NameRef::new(0), |o| o.base.name);
        let base = self.state.bare_base(name);
        // CR 113.7a: the ability on the stack exists independently of the
        // permanent it came from, so it takes the list `index` points into
        // with it. Reading it back off the source at resolution time was
        // right only while a source's abilities could not change under it —
        // a copy that dies with its ability on the stack stops being a copy
        // (CR 400.7), and the index would then be read against the printed
        // card, which is a different ability or none at all.
        //
        // The slot holds the list for the *next* push, and two callers fill
        // it: an activation, which captured its list before paying a cost
        // that may already have moved the source, and a look-back trigger
        // (CR 603.10a), whose source stopped being a copy on the way off the
        // battlefield. That second one is the hazard the paragraph above
        // describes, and it happened — Phyrexian Metamorph copying Solemn
        // Simulacrum died and drew nobody a card, because the list read back
        // here was the printed Metamorph's and has no dies trigger in it.
        // Everything else still reads the source, which is right by
        // definition while it is still standing there.
        let captured = self
            .activating_abilities
            .take()
            .and_then(|(id, abilities)| (id == source).then_some(abilities));
        let list = captured.unwrap_or_else(|| {
            self.state
                .object(source)
                .map_or(crate::object::AbilityList::NONE, |o| {
                    o.ability_list(&self.lookup)
                })
        });
        let abilities = list.abilities;
        // CR 107.3m: an object's **own** enters-the-battlefield triggered
        // ability that refers to X uses the X chosen for the spell that
        // became that object, although X for the permanent itself is 0. The
        // Meathook Massacre is the card that reads it — `{X}{B}{B}`, "when
        // this enters, each creature gets -X/-X" — and without this the
        // trigger resolved at nought: a `Coverage::Implemented` enchantment
        // that swept no board. `Trigger::ETB` and not any entering trigger,
        // because the rule says *its* enter trigger: a landfall
        // `EntersBattlefield(&Filter::YOUR_LAND)` is about another permanent
        // and announces nothing. Nothing is guarded here: a permanent's
        // `x_value` is normalised the moment it enters
        // (`progress::apply_enter_modifiers`), so on the battlefield the
        // field already means "the X of the spell that became this, or
        // nothing" and a reanimated body announces a 0.
        let announced_x = matches!(
            abilities.get(ability_index as usize),
            Some(
                baylee_cards_dsl::AbilityDef::Triggered {
                    trigger: baylee_cards_dsl::Trigger::ETB,
                    ..
                } | baylee_cards_dsl::AbilityDef::ModalTriggered {
                    trigger: baylee_cards_dsl::Trigger::ETB,
                    ..
                }
            )
        )
        .then(|| self.state.object(source).map_or(0, |o| o.x_value))
        .unwrap_or(0);
        let id = self.state.arena.insert_with(|id| {
            let mut obj = GameObject::new_ability_on_stack(
                id,
                controller,
                AbilityLoc {
                    card,
                    index: ability_index,
                    source,
                },
                targets,
                base,
            );
            obj.take_abilities(list);
            obj.x_value = announced_x;
            obj
        });
        self.state
            .zones
            .insert(id, ZoneLocation::Stack, ZonePosition::Top, false);
        self.state.journal.record(GameEvent::AbilityTriggered {
            object: id,
            source,
            ability_index,
            controller,
        });
        id
    }
}
