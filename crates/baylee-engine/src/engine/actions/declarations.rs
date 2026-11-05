//! Declaring attackers and blockers: the attack tax and its payment window, the blocks made, and what a declaration records (CR 508.1, 509.1).

use super::super::attack_sacrifice::{AttackSacrificeCost, AttackSacrificePayment};
use super::{
    AttackerInfo, CardLookup, Cause, CombatDeclared, Engine, EngineError, GameEvent, ObjectId,
    ObjectKind, Pending, PlayerId, combat,
};

/// What a legal declaration of attackers costs, determined and not yet paid
/// (CR 508.1h).
struct AttackCosts {
    sacrifices: Vec<AttackSacrificeCost>,
    mana: u32,
    life: i32,
}

impl<L: CardLookup> Engine<L> {
    /// The automatic progression machine: SBAs, stack resolution, and
    /// step/turn transitions until a decision is required.
    pub(crate) fn declare_attackers(
        &mut self,
        player: PlayerId,
        attackers: Vec<(ObjectId, baylee_core::ids::Defender)>,
    ) -> Result<(), EngineError> {
        let reach = if combat::AttackRules::new(&self.state).has_mana_tax() {
            self.attack_mana_reach(player)
        } else {
            0
        };
        let costs = self.attack_costs(player, &attackers, reach)?;
        // Before any permanent is chosen: nothing takes a declaration back
        // once its sacrifices are being asked for.
        if !self.attack_tax_within_reach(player, costs.mana, reach) {
            return Err(EngineError::IllegalAction(
                "cannot pay mana required to declare those attackers",
            ));
        }
        if !costs.sacrifices.is_empty() {
            return self.ask_attack_sacrifices(
                player,
                attackers,
                costs.sacrifices,
                costs.mana,
                costs.life,
                reach,
            );
        }
        self.finish_attack_declaration(player, attackers, Vec::new(), costs.mana, costs.life, reach)
    }

    /// Whether the pool, and up to `reach` more mana made in a payment window,
    /// could cover a mana tax.
    fn attack_tax_within_reach(&self, player: PlayerId, tax: u32, reach: u64) -> bool {
        self.state.players[usize::from(player.get())]
            .mana_pool
            .total()
            .saturating_add(reach)
            >= u64::from(tax)
    }

    /// What a mana tax the attacking player's pool does not cover may still
    /// be paid with (CR 508.1i): one unit from each source the payment
    /// window would offer, which is what the gate counts. Zero once a window
    /// for this step has closed short (`CombatDeclared::TaxDeclined`).
    pub(in crate::engine) fn attack_mana_reach(&self, player: PlayerId) -> u64 {
        if matches!(self.combat_declared, CombatDeclared::TaxDeclined { .. }) {
            return 0;
        }
        let mut legal = self.compute_legal(player);
        self.narrow_to_mana(&mut legal);
        let mut sources: Vec<ObjectId> = legal
            .mana_abilities
            .iter()
            .copied()
            .chain(legal.abilities.iter().map(|&(source, _)| source))
            .collect();
        sources.sort_unstable();
        sources.dedup();
        u64::try_from(sources.len() + legal.granted_actions.len()).unwrap_or(u64::MAX)
    }

    /// Whether `attackers` is a legal declaration, and what it costs: CR
    /// 508.1a–d and 508.1h. `reach` is the mana a payment window could still
    /// make for its tax (`Self::attack_mana_reach`), and zero as that window
    /// closes, when the pool alone stands behind it.
    fn attack_costs(
        &self,
        player: PlayerId,
        attackers: &[(ObjectId, baylee_core::ids::Defender)],
        reach: u64,
    ) -> Result<AttackCosts, EngineError> {
        // Checked against the same list the request offered, so a client
        // can never name a defender the engine did not put on the table:
        // an opponent's planeswalker that has since left, a player who has
        // lost, or the attacking player themself.
        let legal = combat::defender_options(&self.state, player);
        // A set and not a list: a declaration can name tens of thousands of
        // tokens, and a `contains` per attacker made it quadratic (33,600
        // attackers cost 190 ms in self-play, r001 game 431).
        let mut seen = std::collections::BTreeSet::new();
        let rules = combat::AttackRules::new(&self.state).reaching(reach);
        if let Some(max) = rules.max_attackers()
            && attackers.len() > max
        {
            return Err(EngineError::IllegalAction("too many attackers declared"));
        }
        if attackers.len() > 1
            && attackers
                .iter()
                .any(|(creature, _)| rules.attacks_alone(*creature))
        {
            return Err(EngineError::IllegalAction(
                "that creature can attack only alone",
            ));
        }
        for (creature, defending) in attackers {
            if !combat::can_attack(&self.state, player, *creature) {
                return Err(EngineError::IllegalAction("creature cannot attack"));
            }
            if !legal.contains(defending) {
                return Err(EngineError::IllegalAction("invalid defender"));
            }
            // CR 508.1c, the restrictions about the pair.
            if !rules.allows(*creature, *defending) {
                return Err(EngineError::IllegalAction(
                    "that creature can't attack that player or planeswalker",
                ));
            }
            if !seen.insert(*creature) {
                return Err(EngineError::IllegalAction("duplicate attacker"));
            }
        }
        // CR 508.1d: every creature that attacks if able, and can, does, up to
        // any cap on how many creatures may attack (CR 508.1c). A player is
        // never required to pay a cost to declare an attacker (CR 508.1d), so
        // only defenders with zero life tax count toward a requirement.
        let required_able: Vec<ObjectId> = self
            .state
            .battlefield_seen()
            .filter(|id| {
                rules.must_attack(*id)
                    && rules.sacrifice_tax(*id).is_empty()
                    && combat::can_attack(&self.state, player, *id)
                    && legal.iter().any(|d| {
                        rules.allows(*id, *d)
                            && rules.mana_tax_for(*id, *d) == 0
                            && rules.life_tax(*id, *d) == 0
                    })
            })
            .collect();
        let required_target = rules.required_maximum(&required_able);
        let required_declared = seen.iter().filter(|id| rules.must_attack(**id)).count();
        if required_declared < required_target {
            return Err(crate::choice::AnswerFault::MustAttack.into());
        }
        let total_life_tax = attackers
            .iter()
            .map(|&(creature, defending)| rules.life_tax(creature, defending))
            .fold(0_i32, i32::saturating_add);
        if total_life_tax > 0 && !self.state.can_pay_life(player, total_life_tax) {
            return Err(EngineError::IllegalAction(
                "cannot pay life required to declare those attackers",
            ));
        }
        let sacrifices: Vec<AttackSacrificeCost> = attackers
            .iter()
            .flat_map(|&(creature, _)| rules.sacrifice_tax(creature))
            .map(AttackSacrificeCost::from)
            .filter(|cost| cost.count > 0)
            .collect();
        // CR 508.1h–i: every cost is now known and none is paid yet. A mana
        // tax the pool does not cover is made in a payment window, and the
        // declaration waits in it with nothing paid.
        let mana = attackers
            .iter()
            .map(|&(creature, defending)| rules.mana_tax_for(creature, defending))
            .fold(0_u32, u32::saturating_add);
        Ok(AttackCosts {
            sacrifices,
            mana,
            life: total_life_tax,
        })
    }

    /// Pays the already determined costs with the permanents their controller
    /// chose, then makes the declaration (CR 508.1h–k).
    pub(in crate::engine) fn finish_attack_declaration(
        &mut self,
        player: PlayerId,
        attackers: Vec<(ObjectId, baylee_core::ids::Defender)>,
        sacrifices: Vec<AttackSacrificePayment>,
        total_mana_tax: u32,
        total_life_tax: i32,
        reach: u64,
    ) -> Result<(), EngineError> {
        if !super::super::attack_sacrifice::valid_payments(&self.state, player, &sacrifices) {
            return Err(EngineError::IllegalAction(
                "chosen permanents no longer pay the attack sacrifice cost",
            ));
        }
        if total_life_tax > 0 && !self.state.can_pay_life(player, total_life_tax) {
            return Err(EngineError::IllegalAction(
                "cannot pay life required to declare those attackers",
            ));
        }
        let tax = baylee_core::mana::ManaCost::ZERO.with_more_generic(total_mana_tax);
        let pool = &self.state.players[usize::from(player.get())].mana_pool;
        if total_mana_tax > 0 && !crate::mana_pay::can_pay(pool, &tax) {
            if !self.attack_tax_within_reach(player, total_mana_tax, reach) {
                return Err(EngineError::IllegalAction(
                    "cannot pay mana required to declare those attackers",
                ));
            }
            self.open_attack_tax_window(player, attackers, sacrifices, total_life_tax, tax);
            return Ok(());
        }
        if total_life_tax > 0 {
            self.state
                .change_life(player, -total_life_tax, Cause::TurnBased);
        }
        if total_mana_tax > 0 {
            let paid = crate::mana_pay::pay(
                &mut self.state.players[usize::from(player.get())].mana_pool,
                &tax,
            );
            debug_assert!(paid, "mana tax was validated");
        }
        for payment in sacrifices {
            for sac_id in payment.objects {
                if let Some(obj) = self.state.object_mut(sac_id) {
                    obj.kind = ObjectKind::Card;
                }
                let _ = self.state.sacrifice_object(sac_id, Cause::TurnBased);
            }
        }
        // CR 508.1k: only a creature its cost left under this player's control
        // becomes an attacker. A green land creature may pay with itself.
        let attackers: Vec<_> = attackers
            .into_iter()
            .filter(|(creature, _)| {
                self.state.object(*creature).is_some_and(|object| {
                    object.zone == crate::zone::Zone::Battlefield && object.controller == player
                })
            })
            .collect();
        for &(creature, defending) in &attackers {
            if combat::attack_taps(&self.state, creature) {
                self.state.set_tapped(creature, true);
                self.state.journal.record(GameEvent::ObjectTapped {
                    object: creature,
                    cause: Cause::TurnBased,
                });
            }
            self.state.journal.record(GameEvent::BecameAttacker {
                object: creature,
                defending,
            });
            // "If it attacked this turn" (CR 508.1): this declaration is the
            // one way a creature attacks, so it is the one writer.
            if let Some(version) = self.state.object(creature).map(|o| o.version) {
                self.state
                    .per_turn
                    .attacked
                    .push((creature, version, player));
                self.state.combat.participants.push((creature, version));
                crate::combat::remember_attack(&mut self.state, creature, defending);
            }
        }
        self.state
            .combat
            .declare_attackers(
                attackers
                    .into_iter()
                    .map(|(creature, defending)| AttackerInfo {
                        creature,
                        defending,
                        blocked: false,
                        band: None,
                    }),
            );
        // `Filter::Attacking` is read by the layer system (Orcish Oriflamme's
        // "attacking creatures you control get +1/+0"), and which permanents
        // match it just changed without the effect table moving — the same
        // shape as a tap, and the same door.
        self.state.board_state_changed();
        self.combat_declared = CombatDeclared::Attackers;
        self.passes = 0;
        self.priority_holder = None;
        // The bands, which the declaration announces (CR 508.1e): asked
        // here, before the machine runs, so they stand before anything
        // triggers on the attack. The machine is also what refreshes the
        // projection, so it is refreshed first: the attack's taps
        // (CR 508.1f) already took Giant Tortoise's untapped-only +0/+3, and
        // the band question showed it at 1/4 (CR 611.3a).
        self.state.refresh_characteristics();
        self.ask_band(player, None);
        Ok(())
    }

    /// Opens the CR 508.1i window for a declaration whose mana tax the pool
    /// does not cover: the attacking player makes mana in it, and passing
    /// settles the declaration ([`Self::settle_attack_tax`]).
    fn open_attack_tax_window(
        &mut self,
        player: PlayerId,
        attackers: Vec<(ObjectId, baylee_core::ids::Defender)>,
        sacrifices: Vec<AttackSacrificePayment>,
        life: i32,
        cost: baylee_core::mana::ManaCost,
    ) {
        let mut legal = self.compute_legal(player);
        self.narrow_to_mana(&mut legal);
        self.mana_window = Some(super::PaymentWindow {
            player,
            suspended: super::PaymentContinuation::AttackTax {
                attackers,
                sacrifices,
                life,
                cost,
                opened: Box::new(self.window_start(player)),
            },
        });
        self.pending = Pending::Priority {
            player,
            legal: Box::new(legal),
        };
        self.awaiting_answer = true;
    }

    /// The window of [`Self::open_attack_tax_window`] closing: the
    /// declaration is made out of the pool if it now pays, and otherwise it
    /// was never made, and what the window made is given back (CR 508.1i,
    /// CR 733.1). Returns whether it was made.
    pub(in crate::engine) fn settle_attack_tax(
        &mut self,
        player: PlayerId,
        attackers: Vec<(ObjectId, baylee_core::ids::Defender)>,
        sacrifices: Vec<AttackSacrificePayment>,
        life: i32,
        cost: baylee_core::mana::ManaCost,
        opened: &super::WindowStart,
    ) -> bool {
        self.awaiting_answer = false;
        // The window's mana abilities ran after the declaration was checked:
        // a creature they tapped or sacrificed is no longer one that may
        // attack. The costs stay the ones determined before it (CR 508.1h).
        if self.attack_costs(player, &attackers, 0).is_ok()
            && self
                .finish_attack_declaration(
                    player,
                    attackers,
                    sacrifices,
                    cost.generic_total(),
                    life,
                    0,
                )
                .is_ok()
        {
            return true;
        }
        self.give_back_window(player, opened);
        self.combat_declared = CombatDeclared::TaxDeclined { after: None };
        false
    }

    pub(crate) fn declare_blockers(
        &mut self,
        defending: PlayerId,
        blockers: &[(ObjectId, ObjectId)],
    ) -> Result<(), EngineError> {
        self.declare_blockers_paying(defending, blockers, true)
    }

    /// The block declaration, with a payment window only on the outer call.
    fn declare_blockers_paying(
        &mut self,
        defending: PlayerId,
        blockers: &[(ObjectId, ObjectId)],
        window: bool,
    ) -> Result<(), EngineError> {
        // Sets for the reason `declare_attackers` keeps one.
        let rules = combat::BlockRules::new(&self.state);
        let mut seen = std::collections::BTreeSet::new();
        let mut blocks = std::collections::BTreeMap::<ObjectId, usize>::new();
        for (blocker, attacker) in blockers {
            if !self.state.combat.is_attacking(*attacker) {
                return Err(EngineError::IllegalAction("no such attacker"));
            }
            if !combat::can_block(&self.state, defending, *blocker, *attacker) {
                return Err(EngineError::IllegalAction("creature cannot block"));
            }
            if !seen.insert((*blocker, *attacker)) {
                return Err(EngineError::IllegalAction("duplicate block"));
            }
            // CR 509.1a: one attacker for each blocker, unless an effect
            // lets it block more.
            let count = blocks.entry(*blocker).or_default();
            *count += 1;
            if rules.capacity(*blocker).is_some_and(|most| *count > most) {
                return Err(EngineError::IllegalAction(
                    "creature cannot block that many attackers",
                ));
            }
        }
        if let Some(max) = rules.max_blockers() {
            let mut distinct_blockers: std::collections::BTreeSet<ObjectId> = self
                .state
                .combat
                .blockers
                .iter()
                .map(|b| b.blocker)
                .collect();
            distinct_blockers.extend(blocks.keys().copied());
            if distinct_blockers.len() > max {
                return Err(EngineError::IllegalAction("too many blockers declared"));
            }
        }
        // The counts the declaration is held to as a whole (CR 509.1b),
        // menace's two or more (CR 702.111b) among them. `apply` refused a
        // declaration short of one already, because the question states
        // them (`Pending::ChooseBlockers::bounds`, from the same
        // `combat::block_bound`); this is the rule where the declaration is
        // made, for a caller that did not come through `apply`.
        //
        // Zero is legal and one is not: "can't be blocked except by two or
        // more creatures" says nothing about a creature nobody blocks.
        for attacker in self.state.combat.attackers() {
            if let Some(bound) = combat::block_bound(&self.state, attacker.creature) {
                let count = blockers
                    .iter()
                    .filter(|(_, a)| *a == attacker.creature)
                    .count();
                let count = u32::try_from(count).unwrap_or(u32::MAX);
                if count > 0 && count < bound.min_blockers {
                    return Err(crate::choice::AnswerFault::TooFewBlockers.into());
                }
                if count > bound.max_blockers {
                    return Err(crate::choice::AnswerFault::TooManyBlockers.into());
                }
            }
        }
        // CR 509.1c: as many requirements obeyed as the most a declaration
        // could obey without breaking a restriction. `BlockRules` says how
        // the most is found, and where it is only a good declaration's. A
        // blocker that costs mana counts on neither side, since nobody has
        // to pay to obey (`BlockRules::demands`); the options still offer it.
        if rules.has_requirements() {
            let mana_reach = if window && rules.has_mana_tax() {
                self.attack_mana_reach(defending)
            } else {
                0
            };
            let options = combat::block_options(&self.state, defending, mana_reach);
            let most = rules.obeyed(&rules.obeying(&options));
            if rules.obeyed(blockers) < most {
                return Err(crate::choice::AnswerFault::MustBlock.into());
            }
        }
        let total_mana_tax = blockers
            .iter()
            .map(|&(blocker, attacker)| rules.mana_tax(blocker, attacker))
            .fold(0_u32, u32::saturating_add);
        let tax = baylee_core::mana::ManaCost::ZERO.with_more_generic(total_mana_tax);
        let can_pay = crate::mana_pay::can_pay(
            &self.state.players[usize::from(defending.get())].mana_pool,
            &tax,
        );
        drop(rules);
        if total_mana_tax > 0 && !can_pay {
            if !window {
                return Err(EngineError::IllegalAction(
                    "cannot pay mana required to declare those blockers",
                ));
            }
            let reach = self.attack_mana_reach(defending);
            let available = self.state.players[usize::from(defending.get())]
                .mana_pool
                .total()
                .saturating_add(reach);
            if available < u64::from(total_mana_tax) {
                return Err(EngineError::IllegalAction(
                    "cannot pay mana required to declare those blockers",
                ));
            }
            self.open_block_tax_window(defending, blockers.to_vec(), tax);
            return Ok(());
        }
        if total_mana_tax > 0 {
            let paid = crate::mana_pay::pay(
                &mut self.state.players[usize::from(defending.get())].mana_pool,
                &tax,
            );
            debug_assert!(paid, "block mana tax was validated");
        }
        self.make_blocks(defending, blockers);
        Ok(())
    }

    /// Opens the CR 509.1d–f payment window for a block declaration.
    fn open_block_tax_window(
        &mut self,
        defending: PlayerId,
        blockers: Vec<(ObjectId, ObjectId)>,
        cost: baylee_core::mana::ManaCost,
    ) {
        let mut legal = self.compute_legal(defending);
        self.narrow_to_mana(&mut legal);
        self.mana_window = Some(super::PaymentWindow {
            player: defending,
            suspended: super::PaymentContinuation::BlockTax {
                defending,
                blockers,
                cost,
                opened: Box::new(self.window_start(defending)),
            },
        });
        self.pending = Pending::Priority {
            player: defending,
            legal: Box::new(legal),
        };
        self.awaiting_answer = true;
    }

    /// Settles a block declaration, or reverses the window without declaring.
    pub(in crate::engine) fn settle_block_tax(
        &mut self,
        defending: PlayerId,
        blockers: &[(ObjectId, ObjectId)],
        opened: &super::WindowStart,
    ) -> bool {
        self.awaiting_answer = false;
        if self
            .declare_blockers_paying(defending, blockers, false)
            .is_ok()
        {
            return true;
        }
        self.give_back_window(defending, opened);
        // The defending players before this one have declared, and their
        // blocks stand (CR 802.4): it is this one who is asked again.
        let after = match self.combat_declared {
            CombatDeclared::BlockersBy(seat) => Some(seat),
            CombatDeclared::TaxDeclined { after } => after,
            _ => None,
        };
        self.combat_declared = CombatDeclared::TaxDeclined { after };
        false
    }

    /// Makes `defending`'s blocks (CR 509.1h), spreads them through bands
    /// (CR 702.22h), and asks the next defending player or ends the
    /// declarations: the half of declaring blockers after the legality
    /// checks, which Camouflage's piles share (`answer_camouflage`).
    pub(crate) fn make_blocks(&mut self, defending: PlayerId, blockers: &[(ObjectId, ObjectId)]) {
        for &(blocker, attacker) in blockers {
            if let Some(version) = self.state.object(blocker).map(|o| o.version) {
                self.state.combat.participants.push((blocker, version));
            }
            self.state.declare_block(blocker, attacker);
        }
        self.spread_blocks_through_bands(blockers);
        // `Filter::Blocking` and `Filter::Unblocked` just changed for the
        // reason `declare_attackers` gives for `Filter::Attacking`.
        self.state.board_state_changed();
        self.passes = 0;
        self.priority_holder = None;
        // CR 802.4: the next defending player in APNAP order declares all
        // their blocks. Asked here and not by the machine, so no ability
        // that triggered on a block reaches the stack before the last
        // defending player has declared (CR 509.2a: they are put onto the
        // stack "before the active player gets priority").
        if let Some(next) = self.next_defending_player(Some(defending)) {
            self.combat_declared = CombatDeclared::BlockersBy(defending);
            // "A player knows the choices made by the previous players"
            // (CR 101.4b): what the next one may block with, and how many,
            // is read from the board with these blocks on it.
            self.sync_static_effects();
            self.state.refresh_characteristics();
            self.ask_blockers(next);
            return;
        }
        self.blockers_declared();
    }

    /// Every defending player has declared: the declaration is over, and
    /// each attacker nothing blocks is unblocked (CR 509.1h), which "attacks
    /// and isn't blocked" hears.
    pub(in crate::engine) fn blockers_declared(&mut self) {
        self.combat_declared = CombatDeclared::Blockers;
        let unblocked: Vec<ObjectId> = self
            .state
            .combat
            .attackers()
            .iter()
            .filter(|a| !a.blocked)
            .map(|a| a.creature)
            .collect();
        for object in unblocked {
            self.state
                .journal
                .record(GameEvent::AttackerUnblocked { object });
        }
    }
}
