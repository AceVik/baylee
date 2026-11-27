//! Declaring attackers: who may attack, what they may attack, and the rules effects add (CR 508.1).

use crate::object::{GameObject, Status};
use crate::state::GameState;
use baylee_cards_dsl::KeywordSet as K;
use baylee_core::ids::{Defender, ObjectId, PlayerId};
use baylee_core::types::TypeSet;

/// Whether `creature` may attack at all (untapped, a creature, not
/// summoning-sick, no defender, nothing saying it can't attack).
#[must_use]
pub fn can_attack(state: &GameState, player: PlayerId, creature: ObjectId) -> bool {
    let Some(obj) = state.object(creature) else {
        return false;
    };
    // "Can attack as though …" (CR 609.4): a permission read only where
    // its own rule would stop the attack, so a creature with neither
    // defender nor summoning sickness never walks the effect table.
    let as_though = |modifier: baylee_cards_dsl::Modifier| {
        state
            .effects
            .iter()
            .any(|fx| fx.modifier == modifier && crate::effects::applies_to(state, fx, obj))
    };
    obj.zone == crate::zone::Zone::Battlefield
        && obj.controller == player
        && obj.characteristics().types.contains(TypeSet::CREATURE)
        && !obj.characteristics().types.contains(TypeSet::BATTLE)
        // Defender (CR 702.3b): can't attack, however untapped it is —
        // unless it can attack as though it didn't have defender.
        && (!obj.characteristics().keywords.contains(K::DEFENDER)
            || as_though(baylee_cards_dsl::Modifier::AttacksDespiteDefender))
        // "Can't attack" (Wayward Swordtooth, while it lacks the city's
        // blessing): the same rule as defender, from a static.
        && !obj.characteristics().keywords.contains(K::CANT_ATTACK)
        && (!state.effects.may_forbid_attacking() || !as_though(baylee_cards_dsl::Modifier::CantAttack))
        && !obj.status.contains(Status::TAPPED)
        && !obj.status.contains(Status::PHASED_OUT)
        // Summoning sickness (CR 302.6), unless it can attack as though it
        // had haste (CR 702.10b).
        && (!summoning_sick(state, obj)
            || as_though(baylee_cards_dsl::Modifier::AttacksAsThoughHaste))
}

/// Everything `player` may declare an attack against right now: each
/// surviving opponent, and every planeswalker those opponents control
/// (CR 506.2).
///
/// An opponent, not another player: a teammate cannot be attacked, and
/// neither can a planeswalker they control, because the walker filter reads
/// the same opponent list.
///
/// One list rather than "pick a player, then pick one of their walkers":
/// the choice is a single one in the rules, and a flat list is also what
/// a client needs to render the choice.
#[must_use]
pub fn defender_options(state: &GameState, player: PlayerId) -> Vec<Defender> {
    let opponents: Vec<PlayerId> = state
        .players
        .iter()
        .filter(|p| state.is_opponent(p.id, player) && !p.has_lost())
        .map(|p| p.id)
        .collect();
    let mut options: Vec<Defender> = opponents.iter().copied().map(Defender::Player).collect();
    options.extend(
        state
            .battlefield_seen()
            .filter(|id| {
                state.object(*id).is_some_and(|o| {
                    opponents.contains(&o.controller)
                        && o.characteristics().types.contains(TypeSet::PLANESWALKER)
                        && !o.characteristics().types.contains(TypeSet::BATTLE)
                })
            })
            .map(Defender::Planeswalker),
    );
    options.extend(
        state
            .battlefield_seen()
            .filter(|&id| {
                crate::battle::protector(state, id)
                    .is_some_and(|protector| opponents.contains(&protector))
            })
            .map(Defender::Battle),
    );
    options
}

/// The player who would take the damage aimed at `defender` — the
/// defending player themself, or a planeswalker's controller.
///
/// `None` once a planeswalker has left the battlefield: the attack stays
/// declared (CR 506.4c) but there is nothing left to damage.
#[must_use]
pub fn defending_player(state: &GameState, defender: Defender) -> Option<PlayerId> {
    match defender {
        Defender::Player(p) => Some(p),
        Defender::Battle(id) => crate::battle::protector(state, id),
        Defender::Planeswalker(id) => state
            .object(id)
            .filter(|o| {
                o.zone == crate::zone::Zone::Battlefield && !o.status.contains(Status::PHASED_OUT)
            })
            .map(|o| o.controller),
    }
}

/// The defending player whose creatures may block a creature attacking
/// `defender` (CR 802.4a: "those creatures can block only creatures
/// attacking that player, a planeswalker that player controls").
///
/// Unlike [`defending_player`], a planeswalker that has left the battlefield
/// still names somebody: its attacker "may be blocked" (CR 506.4c), and the
/// defending player an attacker refers to is then "the controller of the
/// planeswalker that creature was attacking before it was removed from
/// combat" (CR 802.2a), which is the walker's last known controller.
#[must_use]
pub fn blocking_player(state: &GameState, defender: Defender) -> Option<PlayerId> {
    match defender {
        Defender::Player(p) => Some(p),
        Defender::Planeswalker(id) => state.last_known_controller(id),
        Defender::Battle(id) => crate::battle::designated_protector(state, id),
    }
}

/// The rules about the declaration of attackers that effects add to what
/// [`can_attack`] asks of a creature alone: the restrictions about the pair,
/// a creature and what it attacks (CR 508.1c, "can't attack unless defending
/// player controls an Island"), and the requirements (CR 508.1d, "attacks
/// each combat if able").
///
/// Collected once per declaration: the effects are walked here and not per
/// creature, because a declaration may name tens of thousands of tokens and
/// almost never meets one of these.
pub struct AttackRules<'a> {
    state: &'a GameState,
    unless_defender_controls: Vec<&'a crate::effects::ContinuousEffect>,
    unless_more_creatures: Vec<&'a crate::effects::ContinuousEffect>,
    if_defender_controls: Vec<&'a crate::effects::ContinuousEffect>,
    except_by: Vec<&'a crate::effects::ContinuousEffect>,
    requirements: Vec<&'a crate::effects::ContinuousEffect>,
    must_attack_player: Vec<&'a crate::effects::ContinuousEffect>,
    alone: Vec<&'a crate::effects::ContinuousEffect>,
    idle_defenders: Vec<&'a crate::effects::ContinuousEffect>,
    cant_attack_you: Vec<&'a crate::effects::ContinuousEffect>,
    cant_attack_you_or_pws: Vec<&'a crate::effects::ContinuousEffect>,
    cant_be_attacked_by: Vec<&'a crate::effects::ContinuousEffect>,
    attack_tax_life: Vec<&'a crate::effects::ContinuousEffect>,
    attack_tax_mana: Vec<&'a crate::effects::ContinuousEffect>,
    attack_phyrexian_tax: Vec<&'a crate::effects::ContinuousEffect>,
    unless_pay_mana: Vec<&'a crate::effects::ContinuousEffect>,
    unless_sacrifice: Vec<&'a crate::effects::ContinuousEffect>,
    unless_return: Vec<&'a crate::effects::ContinuousEffect>,
    unless_tap: Vec<&'a crate::effects::ContinuousEffect>,
    creature_counts: Option<super::CreatureCounts>,
    max_attackers: Option<usize>,
    /// Mana the attacking player could still make before paying a mana tax
    /// ([`Self::reaching`]); zero, so the pool alone pays, unless set.
    mana_reach: u64,
}

impl<'a> AttackRules<'a> {
    /// The rules in force now.
    #[must_use]
    pub fn new(state: &'a GameState) -> Self {
        use baylee_cards_dsl::Modifier;
        let mut unless_defender_controls = Vec::new();
        let mut unless_more_creatures = Vec::new();
        let mut if_defender_controls = Vec::new();
        let mut except_by = Vec::new();
        let mut requirements = Vec::new();
        let mut must_attack_player = Vec::new();
        let mut alone = Vec::new();
        let mut idle_defenders = Vec::new();
        let mut cant_attack_you = Vec::new();
        let mut cant_attack_you_or_pws = Vec::new();
        let mut cant_be_attacked_by = Vec::new();
        let mut attack_tax_life = Vec::new();
        let mut attack_tax_mana = Vec::new();
        let mut attack_phyrexian_tax = Vec::new();
        let mut unless_pay_mana = Vec::new();
        let mut unless_sacrifice = Vec::new();
        let mut unless_return = Vec::new();
        let mut unless_tap = Vec::new();
        let mut max_attackers: Option<usize> = None;
        for fx in state.effects.iter() {
            match fx.modifier {
                Modifier::CantAttackUnlessDefenderControls(_) => unless_defender_controls.push(fx),
                Modifier::CantAttackUnlessYouControlMoreCreaturesThanDefender => {
                    unless_more_creatures.push(fx);
                }
                Modifier::CantAttackIfDefenderControls(_) => if_defender_controls.push(fx),
                Modifier::CantAttackPlayerIdleLastTurn => idle_defenders.push(fx),
                Modifier::CantBeAttackedExceptBy { .. } => except_by.push(fx),
                Modifier::AttacksEachCombat => requirements.push(fx),
                Modifier::MustAttackPlayer(_) => must_attack_player.push(fx),
                Modifier::AttacksAloneOnly => alone.push(fx),
                Modifier::CantAttackYou => cant_attack_you.push(fx),
                Modifier::CantAttackYouOrYourPlaneswalkers => cant_attack_you_or_pws.push(fx),
                Modifier::CantBeAttackedBy(_) => cant_be_attacked_by.push(fx),
                Modifier::AttackTaxLife { .. } => attack_tax_life.push(fx),
                Modifier::AttackTaxMana { .. } | Modifier::AttackManaTax { .. } => {
                    attack_tax_mana.push(fx);
                }
                Modifier::AttackPhyrexianTax { .. } => attack_phyrexian_tax.push(fx),
                Modifier::CantAttackUnlessPayMana(_)
                | Modifier::CantAttackUnlessPayManaPerCounter(_) => unless_pay_mana.push(fx),
                Modifier::CantAttackUnlessYouSacrifice { .. } => unless_sacrifice.push(fx),
                Modifier::CantAttackUnlessYouReturn { .. } => unless_return.push(fx),
                Modifier::CantAttackUnlessYouTap { .. } => unless_tap.push(fx),
                Modifier::MaxAttackersPerCombat(n)
                    if crate::effects::source_is_present(state, fx) =>
                {
                    let cap = usize::from(n);
                    max_attackers = Some(max_attackers.map_or(cap, |cur| cur.min(cap)));
                }
                _ => {}
            }
        }
        let creature_counts =
            (!unless_more_creatures.is_empty()).then(|| super::CreatureCounts::new(state));
        Self {
            state,
            unless_defender_controls,
            unless_more_creatures,
            if_defender_controls,
            except_by,
            requirements,
            must_attack_player,
            alone,
            idle_defenders,
            cant_attack_you,
            cant_attack_you_or_pws,
            cant_be_attacked_by,
            attack_tax_life,
            attack_tax_mana,
            attack_phyrexian_tax,
            unless_pay_mana,
            unless_sacrifice,
            unless_return,
            unless_tap,
            creature_counts,
            max_attackers,
            mana_reach: 0,
        }
    }

    /// An individual restriction, not a cap on unrelated attackers.
    #[must_use]
    pub fn attacks_alone(&self, creature: ObjectId) -> bool {
        self.state.object(creature).is_some_and(|object| {
            self.alone
                .iter()
                .any(|effect| crate::effects::applies_to(self.state, effect, object))
        })
    }

    /// Maximum simultaneously satisfiable requirements among legal free attackers.
    #[must_use]
    pub fn required_maximum(&self, required: &[ObjectId]) -> usize {
        let together = required
            .iter()
            .filter(|id| !self.attacks_alone(**id))
            .count();
        let maximum = together.max(usize::from(!required.is_empty()));
        self.max_attackers.map_or(maximum, |cap| maximum.min(cap))
    }

    /// Number of attack requirements on `creature` obeyed when it attacks
    /// `defender` (CR 508.1d, CR 701.15b–d).
    #[must_use]
    pub fn requirement_score(&self, creature: ObjectId, defender: Defender) -> u32 {
        let Some(obj) = self.state.object(creature) else {
            return 0;
        };
        let mut score = 0_u32;
        for fx in &self.requirements {
            if crate::effects::applies_to(self.state, fx, obj) {
                score = score.saturating_add(1);
            }
        }
        for fx in &self.must_attack_player {
            if let baylee_cards_dsl::Modifier::MustAttackPlayer(who) = fx.modifier
                && crate::effects::applies_to(self.state, fx, obj)
                && let Defender::Player(attacked) = defender
                && crate::eval::players_for_source(who, self.state, fx.controller, fx.source)
                    .is_some_and(|players| players.contains(&attacked))
            {
                score = score.saturating_add(1);
            }
        }
        for rider in &obj.riders {
            if let crate::object::Rider::MustAttackPlayerThisTurn(opp)
            | crate::object::Rider::MustAttackPlayerThisCombat(opp) = rider
                && defender == Defender::Player(*opp)
            {
                score = score.saturating_add(1);
            }
        }
        for goader in super::goad::goaders_of(self.state, creature) {
            score = score.saturating_add(1);
            if matches!(defender, Defender::Player(p) if p != goader) {
                score = score.saturating_add(1);
            }
        }
        score
    }

    /// Whether `creature` can legally attack `defender` without paying any
    /// mana, life, Phyrexian, sacrifice, return, or tap attack tax (CR 508.1d).
    #[must_use]
    pub fn untaxed_allows(&self, creature: ObjectId, defender: Defender) -> bool {
        self.allows(creature, defender)
            && self.sacrifice_tax(creature).is_empty()
            && self.return_tax(creature).is_empty()
            && self.tap_tax(creature).is_empty()
            && self.mana_tax_for(creature, defender) == 0
            && self.life_tax(creature, defender) == 0
            && self.phyrexian_tax_for(creature, defender).is_empty()
    }

    /// Maximum number of requirements `creature` can obey across `legal`
    /// defenders without paying any attack cost (CR 508.1d).
    #[must_use]
    pub fn max_free_requirement_score(&self, creature: ObjectId, legal: &[Defender]) -> u32 {
        legal
            .iter()
            .copied()
            .filter(|&d| self.untaxed_allows(creature, d))
            .map(|d| self.requirement_score(creature, d))
            .max()
            .unwrap_or(0)
    }

    /// Maximum total number of untaxed attack requirements `player` can obey
    /// simultaneously across a legal declaration (CR 508.1d).
    #[must_use]
    pub fn max_total_requirement_score(&self, player: PlayerId, legal: &[Defender]) -> u32 {
        if self.max_attackers == Some(0) || legal.is_empty() {
            return 0;
        }
        let mut together_scores = Vec::new();
        let mut best_alone = 0_u32;
        for id in self.state.battlefield_seen() {
            if !can_attack(self.state, player, id) {
                continue;
            }
            let score = self.max_free_requirement_score(id, legal);
            if score == 0 {
                continue;
            }
            if self.attacks_alone(id) {
                best_alone = best_alone.max(score);
            } else {
                together_scores.push(score);
            }
        }
        together_scores.sort_unstable_by(|a, b| b.cmp(a));
        let take = together_scores
            .len()
            .min(self.max_attackers.unwrap_or(usize::MAX));
        let group_sum: u32 = together_scores[..take].iter().copied().sum();
        group_sum.max(best_alone)
    }

    /// Builds `(required, minimum_required)` for `Pending::ChooseAttackers`
    /// from `(creature, max_free_score)` pairs with `score > 0`.
    #[must_use]
    pub fn required_plan(&self, scored: &[(ObjectId, u32)]) -> (Vec<ObjectId>, u32) {
        if scored.is_empty() || self.max_attackers == Some(0) {
            return (Vec::new(), 0);
        }
        let best_alone = scored
            .iter()
            .filter(|&&(id, _)| self.attacks_alone(id))
            .map(|&(_, s)| s)
            .max()
            .unwrap_or(0);
        let mut together_sorted: Vec<(ObjectId, u32)> = scored
            .iter()
            .copied()
            .filter(|&(id, _)| !self.attacks_alone(id))
            .collect();
        together_sorted.sort_by_key(|a| std::cmp::Reverse(a.1));
        let take = together_sorted
            .len()
            .min(self.max_attackers.unwrap_or(usize::MAX));
        let group_sum: u32 = together_sorted[..take].iter().map(|&(_, s)| s).sum();

        if best_alone > group_sum {
            let required = scored
                .iter()
                .filter(|&&(id, s)| self.attacks_alone(id) && s == best_alone)
                .map(|&(id, _)| id)
                .collect();
            return (required, 1);
        }

        let minimum = u32::try_from(take).unwrap_or(u32::MAX);
        if take < together_sorted.len() {
            let threshold = together_sorted[take - 1].1;
            if together_sorted[0].1 > threshold {
                let required = together_sorted[..take].iter().map(|&(id, _)| id).collect();
                return (required, minimum);
            }
            let required = scored
                .iter()
                .filter(|&&(id, s)| {
                    if self.attacks_alone(id) {
                        take >= 2 || s == group_sum
                    } else {
                        s == threshold
                    }
                })
                .map(|&(id, _)| id)
                .collect();
            return (required, minimum);
        }

        let required = scored
            .iter()
            .filter(|&&(id, s)| !self.attacks_alone(id) || take >= 2 || s == group_sum)
            .map(|&(id, _)| id)
            .collect();
        (required, minimum)
    }

    /// Whether any creature's attack costs mana now (CR 508.1h: Brainwash),
    /// so that asking what mana its controller could make is worth it.
    #[must_use]
    pub fn has_mana_tax(&self) -> bool {
        !self.unless_pay_mana.is_empty()
            || !self.attack_tax_mana.is_empty()
            || !self.attack_phyrexian_tax.is_empty()
    }

    /// The same rules for a declaration whose mana tax may also be paid with
    /// up to `mana` more mana, made in the payment window the declaration
    /// opens (CR 508.1i). An estimate from the sources on offer, never a
    /// promise: a window that closes short undoes the declaration.
    #[must_use]
    pub const fn reaching(mut self, mana: u64) -> Self {
        self.mana_reach = mana;
        self
    }

    /// Maximum number of creatures that may attack this combat, if capped.
    #[must_use]
    pub const fn max_attackers(&self) -> Option<usize> {
        self.max_attackers
    }

    /// Whether `creature` attacks each combat if able (CR 508.1d).
    #[must_use]
    pub fn must_attack(&self, creature: ObjectId) -> bool {
        if self.requirements.is_empty() {
            return false;
        }
        self.state.object(creature).is_some_and(|obj| {
            self.requirements
                .iter()
                .any(|fx| crate::effects::applies_to(self.state, fx, obj))
        })
    }

    /// Total generic mana cost `creature`'s controller must pay to declare
    /// `creature` attacking (CR 508.1h: Brainwash).
    #[must_use]
    pub fn mana_tax(&self, creature: ObjectId) -> u32 {
        if self.unless_pay_mana.is_empty() {
            return 0;
        }
        let Some(obj) = self.state.object(creature) else {
            return 0;
        };
        self.unless_pay_mana
            .iter()
            .filter_map(|fx| {
                let amount = match fx.modifier {
                    baylee_cards_dsl::Modifier::CantAttackUnlessPayMana(n) => n,
                    baylee_cards_dsl::Modifier::CantAttackUnlessPayManaPerCounter(kind) => {
                        u32::from(obj.counters.get(kind))
                    }
                    _ => return None,
                };
                crate::effects::applies_to(self.state, fx, obj).then_some(amount)
            })
            .fold(0_u32, u32::saturating_add)
    }

    /// Total generic mana for this particular defender, including effects
    /// that protect only their own controller, planeswalkers, or any defender.
    #[must_use]
    pub fn mana_tax_for(&self, creature: ObjectId, defender: Defender) -> u32 {
        let base = self.mana_tax(creature);
        if self.attack_tax_mana.is_empty() {
            return base;
        }
        let Some(obj) = self.state.object(creature) else {
            return base;
        };
        let defending = defending_player(self.state, defender);
        self.attack_tax_mana
            .iter()
            .filter_map(|fx| match fx.modifier {
                baylee_cards_dsl::Modifier::AttackTaxMana { mana } => {
                    if !matches!(defender, Defender::Player(_) | Defender::Planeswalker(_)) {
                        return None;
                    }
                    (defending == Some(fx.controller)
                        && crate::effects::applies_to(self.state, fx, obj))
                    .then_some(mana)
                }
                baylee_cards_dsl::Modifier::AttackManaTax { target, amount } => {
                    (matches_attack_tax_target(self.state, target, fx.controller, defender)
                        && crate::effects::applies_to(self.state, fx, obj))
                    .then(|| attack_mana_tax_amount(self.state, fx, obj, amount))
                }
                _ => None,
            })
            .fold(base, u32::saturating_add)
    }

    /// Phyrexian mana symbols required for `creature` to attack `defender`
    /// (Norn's Annex, CR 508.1h, CR 107.4f).
    #[must_use]
    pub fn phyrexian_tax_for(
        &self,
        creature: ObjectId,
        defender: Defender,
    ) -> Vec<(baylee_core::mana::ManaColor, ObjectId)> {
        if self.attack_phyrexian_tax.is_empty() {
            return Vec::new();
        }
        let Some(obj) = self.state.object(creature) else {
            return Vec::new();
        };
        self.attack_phyrexian_tax
            .iter()
            .filter_map(|fx| {
                let baylee_cards_dsl::Modifier::AttackPhyrexianTax { target, color } = fx.modifier
                else {
                    return None;
                };
                (matches_attack_tax_target(self.state, target, fx.controller, defender)
                    && crate::effects::applies_to(self.state, fx, obj))
                .then_some((color, fx.source.unwrap_or(creature)))
            })
            .collect()
    }

    /// Return-to-hand costs `creature`'s controller must pay to declare
    /// `creature` attacking (Floodtide Serpent, CR 508.1h).
    #[must_use]
    pub fn return_tax(
        &self,
        creature: ObjectId,
    ) -> Vec<(u8, &'static baylee_cards_dsl::Filter, PlayerId, ObjectId)> {
        if self.unless_return.is_empty() {
            return Vec::new();
        }
        let Some(obj) = self.state.object(creature) else {
            return Vec::new();
        };
        self.unless_return
            .iter()
            .filter_map(|fx| {
                let baylee_cards_dsl::Modifier::CantAttackUnlessYouReturn { count, filter } =
                    fx.modifier
                else {
                    return None;
                };
                crate::effects::applies_to(self.state, fx, obj).then_some((
                    count,
                    filter,
                    fx.controller,
                    fx.source.unwrap_or(creature),
                ))
            })
            .collect()
    }

    /// Tap-untapped-permanent costs `creature`'s controller must pay to declare
    /// `creature` attacking (Hollow Warrior, CR 508.1h).
    #[must_use]
    pub fn tap_tax(
        &self,
        creature: ObjectId,
    ) -> Vec<(u8, &'static baylee_cards_dsl::Filter, PlayerId, ObjectId)> {
        if self.unless_tap.is_empty() {
            return Vec::new();
        }
        let Some(obj) = self.state.object(creature) else {
            return Vec::new();
        };
        self.unless_tap
            .iter()
            .filter_map(|fx| {
                let baylee_cards_dsl::Modifier::CantAttackUnlessYouTap { count, filter } =
                    fx.modifier
                else {
                    return None;
                };
                crate::effects::applies_to(self.state, fx, obj).then_some((
                    count,
                    filter,
                    fx.controller,
                    fx.source.unwrap_or(creature),
                ))
            })
            .collect()
    }

    /// Sacrifice costs `creature`'s controller must pay to declare `creature`
    /// attacking (CR 508.1h: Leviathan).
    #[must_use]
    pub fn sacrifice_tax(
        &self,
        creature: ObjectId,
    ) -> Vec<(u8, &'static baylee_cards_dsl::Filter, PlayerId, ObjectId)> {
        if self.unless_sacrifice.is_empty() {
            return Vec::new();
        }
        let Some(obj) = self.state.object(creature) else {
            return Vec::new();
        };
        self.unless_sacrifice
            .iter()
            .filter_map(|fx| {
                let baylee_cards_dsl::Modifier::CantAttackUnlessYouSacrifice { count, filter } =
                    fx.modifier
                else {
                    return None;
                };
                crate::effects::applies_to(self.state, fx, obj).then_some((
                    count,
                    filter,
                    fx.controller,
                    fx.source.unwrap_or(creature),
                ))
            })
            .collect()
    }

    /// Total life cost `creature`'s controller must pay to declare `creature`
    /// attacking `defender` (CR 508.1h: Sivitri, Dragon Master's +1).
    #[must_use]
    pub fn life_tax(&self, creature: ObjectId, defender: Defender) -> i32 {
        if self.attack_tax_life.is_empty()
            || !matches!(defender, Defender::Player(_) | Defender::Planeswalker(_))
        {
            return 0;
        }
        let Some(obj) = self.state.object(creature) else {
            return 0;
        };
        let Some(defending) = defending_player(self.state, defender) else {
            return 0;
        };
        self.attack_tax_life
            .iter()
            .filter_map(|fx| {
                let baylee_cards_dsl::Modifier::AttackTaxLife { life } = fx.modifier else {
                    return None;
                };
                (fx.controller == defending && crate::effects::applies_to(self.state, fx, obj))
                    .then_some(i32::from(life))
            })
            .fold(0_i32, i32::saturating_add)
    }

    /// Whether `creature` may attack `defender`, past the restrictions on
    /// the pair. Asked of a creature [`can_attack`] already allows.
    #[allow(clippy::too_many_lines)]
    #[must_use]
    pub fn allows(&self, creature: ObjectId, defender: Defender) -> bool {
        if self.unless_defender_controls.is_empty()
            && self.unless_more_creatures.is_empty()
            && self.if_defender_controls.is_empty()
            && self.except_by.is_empty()
            && self.idle_defenders.is_empty()
            && self.cant_attack_you.is_empty()
            && self.cant_attack_you_or_pws.is_empty()
            && self.cant_be_attacked_by.is_empty()
            && self.attack_tax_life.is_empty()
            && self.attack_tax_mana.is_empty()
            && self.attack_phyrexian_tax.is_empty()
            && self.unless_pay_mana.is_empty()
            && self.unless_sacrifice.is_empty()
            && self.unless_return.is_empty()
            && self.unless_tap.is_empty()
        {
            return true;
        }
        let state = self.state;
        let Some(obj) = state.object(creature) else {
            return false;
        };
        let mana_tax = self.mana_tax_for(creature, defender);
        let life_tax = self.life_tax(creature, defender);
        let phyrexian: Vec<baylee_core::mana::ManaColor> = self
            .phyrexian_tax_for(creature, defender)
            .into_iter()
            .map(|(color, _)| color)
            .collect();
        if !can_afford_attack_mana_and_phyrexian(
            state,
            obj.controller,
            baylee_core::mana::ManaCost::ZERO.with_more_generic(mana_tax),
            life_tax,
            &phyrexian,
            self.mana_reach,
        ) {
            return false;
        }
        for (count, filter, you, source) in self.sacrifice_tax(creature) {
            let have = state
                .battlefield_seen()
                .filter(|&id| {
                    state.object(id).is_some_and(|p| {
                        p.controller == obj.controller
                            && state.can_be_sacrificed(id, None)
                            && crate::eval::matches(filter, state, p, you, source)
                    })
                })
                .count();
            if have < usize::from(count) {
                return false;
            }
        }
        for (count, filter, you, source) in self.return_tax(creature) {
            let have = state
                .battlefield_seen()
                .filter(|&id| {
                    state.object(id).is_some_and(|p| {
                        p.controller == obj.controller
                            && crate::eval::matches(filter, state, p, you, source)
                    })
                })
                .count();
            if have < usize::from(count) {
                return false;
            }
        }
        for (count, filter, you, source) in self.tap_tax(creature) {
            let have = state
                .battlefield_seen()
                .filter(|&id| {
                    id != creature
                        && state.object(id).is_some_and(|p| {
                            p.controller == obj.controller
                                && !p.status.contains(Status::TAPPED)
                                && crate::eval::matches(filter, state, p, you, source)
                        })
                })
                .count();
            if have < usize::from(count) {
                return false;
            }
        }
        // Arboria: a player who neither cast a spell nor put a nontoken
        // permanent onto the battlefield during their last turn can't be
        // attacked; their planeswalkers can (ruling 2022-12-08).
        if let Defender::Player(attacked) = defender
            && !state.players[usize::from(attacked.get())].last_turn_acted
            && self
                .idle_defenders
                .iter()
                .any(|fx| crate::effects::applies_to(state, fx, obj))
        {
            return false;
        }
        // "Creatures can't attack you" (Web of Inertia) protects the player
        // themself, while a planeswalker they control remains a legal defender
        // (CR 506.3).
        if let Defender::Player(attacked) = defender
            && self
                .cant_attack_you
                .iter()
                .any(|fx| fx.controller == attacked && crate::effects::applies_to(state, fx, obj))
        {
            return false;
        }
        // "can't attack you or planeswalkers you control" (Assault Suit,
        // Vow cycle, Eriette, Sandwurm Convergence, etc.).
        if !self.cant_attack_you_or_pws.is_empty()
            && let Some(protected_player) = match defender {
                Defender::Player(p) => Some(p),
                Defender::Planeswalker(id) => state.object(id).map(|o| o.controller),
                Defender::Battle(_) => None,
            }
            && self.cant_attack_you_or_pws.iter().any(|fx| {
                fx.controller == protected_player && crate::effects::applies_to(state, fx, obj)
            })
        {
            return false;
        }
        // "<Defender> can't be attacked [by <filter>]" (The Aetherspark).
        if !self.cant_be_attacked_by.is_empty()
            && let Defender::Planeswalker(id) | Defender::Battle(id) = defender
            && let Some(def_obj) = state.object(id)
            && self.cant_be_attacked_by.iter().any(|fx| {
                let baylee_cards_dsl::Modifier::CantBeAttackedBy(attacker_filter) = fx.modifier
                else {
                    return false;
                };
                crate::effects::applies_to(state, fx, def_obj)
                    && crate::eval::matches(
                        attacker_filter,
                        state,
                        obj,
                        fx.controller,
                        fx.source.unwrap_or(creature),
                    )
            })
        {
            return false;
        }
        // Life tax was validated jointly with mana and Phyrexian taxes above.
        // "You can't be attacked except by …" protects the player, and a
        // planeswalker they control is a defender of its own (CR 506.3).
        if let Defender::Player(attacked) = defender
            && !self.except_by.iter().all(|fx| {
                let baylee_cards_dsl::Modifier::CantBeAttackedExceptBy { who, by } = fx.modifier
                else {
                    return true;
                };
                !crate::eval::players(who, state, fx.controller)
                    .is_some_and(|p| p.contains(&attacked))
                    || crate::eval::matches(
                        by,
                        state,
                        obj,
                        fx.controller,
                        fx.source.unwrap_or(creature),
                    )
            })
        {
            return false;
        }
        if self.unless_defender_controls.is_empty()
            && self.unless_more_creatures.is_empty()
            && self.if_defender_controls.is_empty()
        {
            return true;
        }
        let Some(defending) = defending_player(state, defender) else {
            return false;
        };
        if self.creature_counts.as_ref().is_some_and(|counts| {
            !counts.satisfies(state, &self.unless_more_creatures, obj, defending)
        }) {
            return false;
        }
        if !self.unless_defender_controls.iter().all(|fx| {
            let baylee_cards_dsl::Modifier::CantAttackUnlessDefenderControls(filter) = fx.modifier
            else {
                return true;
            };
            !crate::effects::applies_to(state, fx, obj)
                || state.battlefield_seen().any(|id| {
                    state.object(id).is_some_and(|p| {
                        p.controller == defending
                            && crate::eval::matches(
                                filter,
                                state,
                                p,
                                fx.controller,
                                fx.source.unwrap_or(creature),
                            )
                    })
                })
        }) {
            return false;
        }
        self.if_defender_controls.iter().all(|fx| {
            let baylee_cards_dsl::Modifier::CantAttackIfDefenderControls(filter) = fx.modifier
            else {
                return true;
            };
            !crate::effects::applies_to(state, fx, obj)
                || !state.battlefield_seen().any(|id| {
                    state.object(id).is_some_and(|p| {
                        p.controller == defending
                            && crate::eval::matches(
                                filter,
                                state,
                                p,
                                fx.controller,
                                fx.source.unwrap_or(creature),
                            )
                    })
                })
        })
    }

    /// Of `defenders`, those `creature` may attack.
    #[must_use]
    pub fn defenders_for(&self, creature: ObjectId, defenders: &[Defender]) -> Vec<Defender> {
        defenders
            .iter()
            .copied()
            .filter(|d| self.allows(creature, *d))
            .collect()
    }
}
fn matches_attack_tax_target(
    state: &GameState,
    target: baylee_cards_dsl::AttackTaxTarget,
    fx_controller: PlayerId,
    defender: Defender,
) -> bool {
    use baylee_cards_dsl::AttackTaxTarget;
    let is_you = matches!(defender, Defender::Player(p) if p == fx_controller);
    let is_your_pw = matches!(defender, Defender::Planeswalker(id) if state.object(id).is_some_and(|o| {
        o.zone == crate::zone::Zone::Battlefield
            && !o.status.contains(Status::PHASED_OUT)
            && o.controller == fx_controller
    }));
    match target {
        AttackTaxTarget::You => is_you,
        AttackTaxTarget::PlaneswalkersYouControl => is_your_pw,
        AttackTaxTarget::YouOrYourPlaneswalkers => is_you || is_your_pw,
        AttackTaxTarget::AnyDefender => true,
    }
}

fn attack_mana_tax_amount(
    state: &GameState,
    fx: &crate::effects::ContinuousEffect,
    obj: &GameObject,
    amount: baylee_cards_dsl::Amount,
) -> u32 {
    if matches!(amount, baylee_cards_dsl::Amount::TotalCountersOnSubject) {
        return obj
            .counters
            .iter()
            .map(|(_, n)| u32::from(n))
            .fold(0_u32, u32::saturating_add);
    }
    crate::eval::amount(
        &amount,
        state,
        fx.controller,
        fx.source.unwrap_or(obj.id),
        None,
    )
}

pub(crate) fn mana_symbol_of(color: baylee_core::mana::ManaColor) -> baylee_core::mana::ManaSymbol {
    use baylee_core::mana::{ManaColor, ManaSymbol};
    match color {
        ManaColor::White => ManaSymbol::White,
        ManaColor::Blue => ManaSymbol::Blue,
        ManaColor::Black => ManaSymbol::Black,
        ManaColor::Red => ManaSymbol::Red,
        ManaColor::Green => ManaSymbol::Green,
        ManaColor::Colorless => ManaSymbol::Colorless,
    }
}

/// Whether `player` can pay `mana_cost`, `life_tax`, and every Phyrexian
/// attack tax symbol in `phyrexian` (each as 2 life or one mana of its color),
/// given up to `mana_reach` additional mana producible in a payment window.
#[must_use]
pub(crate) fn can_afford_attack_mana_and_phyrexian(
    state: &GameState,
    player: PlayerId,
    mana_cost: baylee_core::mana::ManaCost,
    life_tax: i32,
    phyrexian: &[baylee_core::mana::ManaColor],
    mana_reach: u64,
) -> bool {
    if life_tax > 0 && !state.can_pay_life(player, life_tax) {
        return false;
    }
    let pool = &state.players[usize::from(player.get())].mana_pool;
    if phyrexian.is_empty() {
        if mana_cost.is_empty() || crate::mana_pay::can_pay(pool, &mana_cost) {
            return true;
        }
        return mana_reach > 0
            && pool.total().saturating_add(mana_reach) >= u64::from(mana_cost.cmc());
    }
    let mut max_life_pips = 0_usize;
    for j in 1..=phyrexian.len() {
        let extra = i32::try_from(j).unwrap_or(i32::MAX).saturating_mul(2);
        let total_life = life_tax.saturating_add(extra);
        if state.can_pay_life(player, total_life) {
            max_life_pips = j;
        } else {
            break;
        }
    }
    let min_mana_pips = phyrexian.len().saturating_sub(max_life_pips);
    if mana_reach > 0 {
        let min_mv = u64::from(mana_cost.cmc())
            .saturating_add(u64::try_from(min_mana_pips).unwrap_or(u64::MAX));
        if pool.total().saturating_add(mana_reach) >= min_mv {
            return true;
        }
    }
    // Exact check against pool alone: convert colored deficits to life first,
    // then any remaining pips up to `max_life_pips`, checking `can_pay` along
    // the way.
    let mut rem: Vec<baylee_core::mana::ManaColor> = phyrexian.to_vec();
    let cost_of = |pips: &[baylee_core::mana::ManaColor]| {
        pips.iter()
            .fold(mana_cost, |acc, &c| acc.with_symbol(mana_symbol_of(c), 1))
    };
    if crate::mana_pay::can_pay(pool, &cost_of(&rem)) {
        return true;
    }
    for _ in 0..max_life_pips {
        if rem.is_empty() {
            break;
        }
        // Prefer removing a pip whose color demand exceeds pool availability.
        let deficit_idx = rem.iter().position(|&c| {
            let demand = rem.iter().filter(|&&x| x == c).count() as u32;
            demand > pool.available(c)
        });
        let idx = deficit_idx.unwrap_or(0);
        rem.swap_remove(idx);
        if crate::mana_pay::can_pay(pool, &cost_of(&rem)) {
            return true;
        }
    }
    false
}

/// Summoning sickness (CR 302.6): a creature must be controlled
/// continuously since the beginning of its controller's most recent turn
/// (haste excepted).
///
/// A *creature*, and the test is here rather than at each call site: the
/// rule is about creatures in both of its sentences, and a caller that
/// forgot the type check got an answer that was true of every fresh
/// permanent. That is what put a land the player had just played on the
/// wrong side of the field, and it is what the view projected to clients.
/// A Vehicle answers this the moment it is crewed and not before, because
/// the type comes off the *projected* characteristics.
///
/// It reads [`GameObject::controlled_since`], never the object's
/// timestamp: CR 613.7g gives a permanent a new timestamp as it transforms,
/// and it has been under its controller's control no less for that.
///
/// Measured against the controller's own turn clock, so the answer holds
/// through an opponent's turn, and strictly, because
/// [`Player::turn_start_timestamp`] holds the last stamp issued before the
/// turn began rather than the first one issued during it.
///
/// [`Player::turn_start_timestamp`]: crate::state::Player::turn_start_timestamp
#[must_use]
pub fn summoning_sick(state: &GameState, obj: &GameObject) -> bool {
    let chars = obj.characteristics();
    if !chars.types.contains(TypeSet::CREATURE) {
        return false;
    }
    if chars.keywords.contains(baylee_cards_dsl::KeywordSet::HASTE) {
        return false;
    }
    let began = state
        .players
        .get(obj.controller.get() as usize)
        .map_or(0, |p| p.turn_start_timestamp);
    obj.controlled_since > began
}
