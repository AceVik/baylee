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
    if_defender_controls: Vec<&'a crate::effects::ContinuousEffect>,
    except_by: Vec<&'a crate::effects::ContinuousEffect>,
    requirements: Vec<&'a crate::effects::ContinuousEffect>,
    alone: Vec<&'a crate::effects::ContinuousEffect>,
    idle_defenders: Vec<&'a crate::effects::ContinuousEffect>,
    cant_attack_you: Vec<&'a crate::effects::ContinuousEffect>,
    attack_tax_life: Vec<&'a crate::effects::ContinuousEffect>,
    attack_tax_mana: Vec<&'a crate::effects::ContinuousEffect>,
    unless_pay_mana: Vec<&'a crate::effects::ContinuousEffect>,
    unless_sacrifice: Vec<&'a crate::effects::ContinuousEffect>,
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
        let mut if_defender_controls = Vec::new();
        let mut except_by = Vec::new();
        let mut requirements = Vec::new();
        let mut alone = Vec::new();
        let mut idle_defenders = Vec::new();
        let mut cant_attack_you = Vec::new();
        let mut attack_tax_life = Vec::new();
        let mut attack_tax_mana = Vec::new();
        let mut unless_pay_mana = Vec::new();
        let mut unless_sacrifice = Vec::new();
        let mut max_attackers: Option<usize> = None;
        for fx in state.effects.iter() {
            match fx.modifier {
                Modifier::CantAttackUnlessDefenderControls(_) => unless_defender_controls.push(fx),
                Modifier::CantAttackIfDefenderControls(_) => if_defender_controls.push(fx),
                Modifier::CantAttackPlayerIdleLastTurn => idle_defenders.push(fx),
                Modifier::CantBeAttackedExceptBy { .. } => except_by.push(fx),
                Modifier::AttacksEachCombat => requirements.push(fx),
                Modifier::AttacksAloneOnly => alone.push(fx),
                Modifier::CantAttackYou => cant_attack_you.push(fx),
                Modifier::AttackTaxLife { .. } => attack_tax_life.push(fx),
                Modifier::AttackTaxMana { .. } => attack_tax_mana.push(fx),
                Modifier::CantAttackUnlessPayMana(_)
                | Modifier::CantAttackUnlessPayManaPerCounter(_) => unless_pay_mana.push(fx),
                Modifier::CantAttackUnlessYouSacrifice { .. } => unless_sacrifice.push(fx),
                Modifier::MaxAttackersPerCombat(n)
                    if crate::effects::source_is_present(state, fx) =>
                {
                    let cap = usize::from(n);
                    max_attackers = Some(max_attackers.map_or(cap, |cur| cur.min(cap)));
                }
                _ => {}
            }
        }
        Self {
            state,
            unless_defender_controls,
            if_defender_controls,
            except_by,
            requirements,
            alone,
            idle_defenders,
            cant_attack_you,
            attack_tax_life,
            attack_tax_mana,
            unless_pay_mana,
            unless_sacrifice,
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

    /// Whether any creature's attack costs mana now (CR 508.1h: Brainwash),
    /// so that asking what mana its controller could make is worth it.
    #[must_use]
    pub fn has_mana_tax(&self) -> bool {
        !self.unless_pay_mana.is_empty() || !self.attack_tax_mana.is_empty()
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
    /// that protect only their own controller and planeswalkers.
    #[must_use]
    pub fn mana_tax_for(&self, creature: ObjectId, defender: Defender) -> u32 {
        let base = self.mana_tax(creature);
        if self.attack_tax_mana.is_empty()
            || !matches!(defender, Defender::Player(_) | Defender::Planeswalker(_))
        {
            return base;
        }
        let Some(obj) = self.state.object(creature) else {
            return base;
        };
        let Some(defending) = defending_player(self.state, defender) else {
            return base;
        };
        self.attack_tax_mana
            .iter()
            .filter_map(|fx| {
                let baylee_cards_dsl::Modifier::AttackTaxMana { mana } = fx.modifier else {
                    return None;
                };
                (fx.controller == defending && crate::effects::applies_to(self.state, fx, obj))
                    .then_some(mana)
            })
            .fold(base, u32::saturating_add)
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
            && self.if_defender_controls.is_empty()
            && self.except_by.is_empty()
            && self.idle_defenders.is_empty()
            && self.cant_attack_you.is_empty()
            && self.attack_tax_life.is_empty()
            && self.attack_tax_mana.is_empty()
            && self.unless_pay_mana.is_empty()
            && self.unless_sacrifice.is_empty()
        {
            return true;
        }
        let state = self.state;
        let Some(obj) = state.object(creature) else {
            return false;
        };
        let mana_tax = self.mana_tax_for(creature, defender);
        if mana_tax > 0 {
            let pool = &state.players[usize::from(obj.controller.get())].mana_pool;
            let pays = crate::mana_pay::can_pay(
                pool,
                &baylee_core::mana::ManaCost::ZERO.with_more_generic(mana_tax),
            );
            if !pays && pool.total().saturating_add(self.mana_reach) < u64::from(mana_tax) {
                return false;
            }
        }
        for (count, filter, you, source) in self.sacrifice_tax(creature) {
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
        let tax = self.life_tax(creature, defender);
        if tax > 0 && !state.can_pay_life(obj.controller, tax) {
            return false;
        }
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
        if self.unless_defender_controls.is_empty() && self.if_defender_controls.is_empty() {
            return true;
        }
        let Some(defending) = defending_player(state, defender) else {
            return false;
        };
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
