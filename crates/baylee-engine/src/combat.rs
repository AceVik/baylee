//! Structured combat state machine.
//!
//! Implemented: attacker/blocker declaration with the keyword restrictions
//! (flying/reach, menace, unblockable, can't block, protection), first/double strike as
//! a per-creature property, deathtouch, trample, lifelink, and banding
//! (CR 702.22): bands declared with the attack, a block on one member
//! blocking the whole band, and the damage divisions banding hands to the
//! other player. Every division of combat damage among two or more
//! creatures is asked of the player who makes it (`divisions_owed`): the
//! attacker's controller (CR 510.1c), a blocker's (510.1d), or the one
//! banding names (702.22j–k).
//!
//! What effects add to the declarations: [`AttackRules`] (CR 508.1c–d) and
//! [`BlockRules`] (how many attackers a creature may block, CR 509.1a, and
//! the block requirements, CR 509.1c), over the offer [`block_options`]
//! makes.
//!
//! Attacks are aimed at a [`Defender`], so a planeswalker can be attacked
//! and its loyalty comes off (CR 306.8). Battles are the remaining case.
//!
//! Not yet: an attacker with trample blocked by creatures without banding
//! is not asked how to assign its damage (CR 702.19b). The engine assigns
//! lethal damage to each blocker in declaration order and the rest to what
//! it attacks, which is one of the assignments its controller could have
//! chosen.

use crate::event::DamageTarget;
use crate::object::{GameObject, Status};

use crate::state::GameState;
use baylee_cards_dsl::KeywordSet as K;
use baylee_core::color::Color;
use baylee_core::generated::subtypes::land;
use baylee_core::ids::{Defender, ObjectId, PlayerId, SubtypeId};
use baylee_core::types::TypeSet;

/// One declared attacker.
#[derive(Clone, Copy, Hash, Debug)]
pub struct AttackerInfo {
    /// The attacking creature.
    pub creature: ObjectId,
    /// What it attacks: a player, or one of their planeswalkers.
    pub defending: Defender,
    /// Whether a blocker was ever declared against it (CR 509.1h).
    ///
    /// It is set once and never cleared, which is the whole point: a
    /// creature that has been blocked *stays* blocked for the rest of
    /// combat, so "is it blocked" and "what is blocking it" stop being the
    /// same question the moment a blocker leaves the battlefield. Reading
    /// the blocker list for both is what let an attacker whose only blocker
    /// was blinked deal its damage to the player.
    pub blocked: bool,
    /// The band it attacks in (CR 702.22c), by a number shared with its
    /// band mates; `None` for a creature in no band.
    ///
    /// Written once, as the attack is declared, and never read back off
    /// the creature's keywords: a band lasts for the rest of combat even if
    /// something takes banding away (CR 702.22e). A creature removed from
    /// combat leaves its band with its entry (CR 702.22f).
    pub band: Option<u8>,
}

/// How one creature's combat damage is divided among the creatures it
/// deals it to, as a player chose (CR 510.1c–d, 702.22j–k): each recipient
/// with its share, in the order they were asked.
#[derive(Clone, Hash, Debug)]
pub struct Division {
    /// The creature dealing the damage.
    pub source: ObjectId,
    /// Each recipient and the damage assigned to it.
    pub shares: Vec<(ObjectId, i16)>,
}

/// A division of combat damage a player still owes before the damage step
/// can deal it: who chooses, and among what.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwedDivision {
    /// The creature whose damage is divided.
    pub source: ObjectId,
    /// The player who divides it.
    pub chooser: PlayerId,
    /// What it is divided among, in declaration order.
    pub recipients: Vec<ObjectId>,
    /// How much there is to divide: the creature's power (CR 510.1a).
    pub amount: i16,
}

/// One declared blocker.
#[derive(Clone, Copy, Hash, Debug)]
pub struct BlockerInfo {
    /// The blocking creature.
    pub blocker: ObjectId,
    /// The attacker it blocks.
    pub attacker: ObjectId,
}

/// The combat phase's mutable state.
///
/// The attackers are private, and not for tidiness: beside the list, in
/// declaration order (which the damage step and the defender both read),
/// sits the same creatures sorted, which is what "is this attacking?"
/// asks. `Filter::Attacking` is evaluated once per object whenever a filter
/// is walked over the battlefield — every target enumeration, every layer
/// refresh under an "attacking creatures get" effect — and a scan of the
/// list there made each of those walks cost `permanents × attackers`. An
/// Ally deck that attacks with a few thousand tokens against a Kor Haven
/// (whose "target attacking creature" is probed at every priority grant)
/// turned a pass into milliseconds. Every write goes through a method here,
/// so the two lists cannot disagree.
#[derive(Clone, Debug, Default)]
pub struct CombatState {
    /// Incarnations declared as attackers or blockers during this combat.
    /// Removal from combat does not erase that history.
    pub participants: Vec<(ObjectId, u32)>,
    /// Identities that had first or double strike as the first damage step began.
    first_strikers: Option<Vec<(ObjectId, u32)>>,
    /// Damage-step participants fixed before replacement effects reveal creatures.
    prepared_damage: Option<(bool, Vec<(ObjectId, u32)>)>,
    /// Declared attackers, in declaration order.
    attackers: Vec<AttackerInfo>,
    /// The same creatures as `attackers`, sorted, each with what it
    /// attacks, for [`Self::is_attacking`] and [`Self::defender_of`].
    attacking: Vec<(ObjectId, Defender)>,
    /// Declared blockers.
    pub blockers: Vec<BlockerInfo>,
    /// Every block made this combat, `(blocker, attacker)`, kept when the
    /// blocker leaves combat: what "had become blocked by only that
    /// creature this combat" asks (False Orders). Cleared with the combat.
    block_history: Vec<(ObjectId, ObjectId)>,
    /// "That creature can't be blocked this combat except by creatures with
    /// flying and creatures in a pile with the chosen label" (Raging River):
    /// for an attacker, the creatures its chosen pile holds. Several limits
    /// on one attacker all bind (CR 509.1b). Cleared with the combat.
    pile_limits: Vec<(ObjectId, Vec<ObjectId>)>,
    /// The divisions players have chosen for the damage step about to be
    /// dealt, emptied once it is (`deal_combat_damage`).
    divisions: Vec<Division>,
}

/// The declared lists and the chosen divisions: `attacking` is derived
/// from `attackers` and is left out (`GameState::snapshot_hash` hashes
/// this). A division is hashed because it decides where damage lands.
impl std::hash::Hash for CombatState {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.attackers.hash(state);
        self.blockers.hash(state);
        self.block_history.hash(state);
        self.pile_limits.hash(state);
        self.divisions.hash(state);
        self.participants.hash(state);
        self.first_strikers.hash(state);
        self.prepared_damage.hash(state);
    }
}

impl CombatState {
    /// Whether the pending damage step is the first-strike step.
    pub(crate) fn prepared_first_strike(&self) -> Option<bool> {
        self.prepared_damage.as_ref().map(|(first, _)| *first)
    }

    /// Whether combat is underway.
    #[must_use]
    pub fn is_active(&self) -> bool {
        !self.attackers.is_empty()
    }

    /// The declared attackers, in declaration order.
    #[must_use]
    pub fn attackers(&self) -> &[AttackerInfo] {
        &self.attackers
    }

    /// Whether `id` is an attacking creature (CR 506.3), in `O(log n)`.
    #[must_use]
    pub fn is_attacking(&self, id: ObjectId) -> bool {
        self.attacking
            .binary_search_by_key(&id, |(c, _)| *c)
            .is_ok()
    }

    /// What `id` attacks, if it is attacking, in `O(log n)`: the
    /// declare-blockers offer asks it of every pairing (CR 802.4a).
    #[must_use]
    pub fn defender_of(&self, id: ObjectId) -> Option<Defender> {
        self.attacking
            .binary_search_by_key(&id, |(c, _)| *c)
            .ok()
            .map(|at| self.attacking[at].1)
    }

    /// Declares attackers after the ones already declared, in the order
    /// given, sorting the index once rather than inserting into it once
    /// per attacker (a declaration can be tens of thousands of tokens).
    pub fn declare_attackers(&mut self, infos: impl IntoIterator<Item = AttackerInfo>) {
        self.attackers.extend(infos);
        self.reindex();
    }

    /// Keeps the attackers `keep` says to, in their order.
    pub fn retain_attackers(&mut self, mut keep: impl FnMut(&AttackerInfo) -> bool) {
        self.attackers.retain(|a| keep(a));
        self.reindex();
    }

    /// Rebuilds the sorted index from the declaration list.
    fn reindex(&mut self) {
        self.attacking.clear();
        self.attacking
            .extend(self.attackers.iter().map(|a| (a.creature, a.defending)));
        self.attacking.sort_unstable_by_key(|(c, _)| *c);
        self.attacking.dedup_by_key(|(c, _)| *c);
    }

    /// Puts `members` in one band (CR 702.22c), under a number no band has
    /// yet.
    pub fn form_band(&mut self, members: &[ObjectId]) {
        let band = self
            .attackers
            .iter()
            .filter_map(|a| a.band)
            .max()
            .map_or(0, |n| n.saturating_add(1));
        for info in &mut self.attackers {
            if members.contains(&info.creature) {
                info.band = Some(band);
            }
        }
    }

    /// The band an attacker is in, if any.
    #[must_use]
    pub fn band_of(&self, attacker: ObjectId) -> Option<u8> {
        self.attackers
            .iter()
            .find(|a| a.creature == attacker)
            .and_then(|a| a.band)
    }

    /// The other creatures in an attacker's band, in declaration order.
    #[must_use]
    pub fn band_mates(&self, attacker: ObjectId) -> Vec<ObjectId> {
        let Some(band) = self.band_of(attacker) else {
            return Vec::new();
        };
        self.attackers
            .iter()
            .filter(|a| a.band == Some(band) && a.creature != attacker)
            .map(|a| a.creature)
            .collect()
    }

    /// Every attacking band, each its members in declaration order, the
    /// bands in the order their first members were declared.
    #[must_use]
    pub fn bands(&self) -> Vec<Vec<ObjectId>> {
        let mut out: Vec<(u8, Vec<ObjectId>)> = Vec::new();
        for info in &self.attackers {
            let Some(band) = info.band else {
                continue;
            };
            match out.iter_mut().find(|(n, _)| *n == band) {
                Some((_, members)) => members.push(info.creature),
                None => out.push((band, vec![info.creature])),
            }
        }
        out.into_iter().map(|(_, members)| members).collect()
    }

    /// The attackers a blocker blocks, in the order the blocks were made.
    #[must_use]
    pub fn blocked_by(&self, blocker: ObjectId) -> Vec<ObjectId> {
        self.blockers
            .iter()
            .filter(|b| b.blocker == blocker)
            .map(|b| b.attacker)
            .collect()
    }

    /// Whether `blocker` is blocking `attacker`.
    #[must_use]
    pub fn is_blocking(&self, blocker: ObjectId, attacker: ObjectId) -> bool {
        self.blockers
            .iter()
            .any(|b| b.blocker == blocker && b.attacker == attacker)
    }

    /// The division chosen for a creature's damage in this damage step.
    #[must_use]
    pub fn division(&self, source: ObjectId) -> Option<&Division> {
        self.divisions.iter().find(|d| d.source == source)
    }

    /// Records a chosen division for the damage step about to be dealt.
    pub fn record_division(&mut self, division: Division) {
        self.divisions.push(division);
    }

    /// Blockers assigned to an attacker, in declaration order.
    #[must_use]
    pub fn blockers_of(&self, attacker: ObjectId) -> Vec<ObjectId> {
        self.blockers
            .iter()
            .filter(|b| b.attacker == attacker)
            .map(|b| b.blocker)
            .collect()
    }

    /// Whether a creature is blocked (CR 509.1h) — which is not the same as
    /// having a blocker left, and is why the answer is a flag.
    #[must_use]
    pub fn is_blocked(&self, attacker: ObjectId) -> bool {
        self.attackers
            .iter()
            .any(|a| a.creature == attacker && a.blocked)
    }

    /// Records one block, which is two statements and not one: the pairing,
    /// and the fact that the attacker is now blocked (CR 509.1h).
    ///
    /// One door, because the second statement is the easy one to forget —
    /// the flag was set at the call site in `declare_blockers` first, and
    /// every test that built a block by hand went on assigning damage as
    /// though nothing were blocking.
    pub fn declare_block(&mut self, blocker: ObjectId, attacker: ObjectId) {
        for info in &mut self.attackers {
            if info.creature == attacker {
                info.blocked = true;
            }
        }
        self.blockers.push(BlockerInfo { blocker, attacker });
        self.block_history.push((blocker, attacker));
    }

    /// Whether every block `attacker` has met this combat was `blocker`'s:
    /// "had become blocked by only that creature this combat" (False
    /// Orders). A blocker that came and went still counts.
    #[must_use]
    pub fn blocked_only_by(&self, attacker: ObjectId, blocker: ObjectId) -> bool {
        let mut by = self
            .block_history
            .iter()
            .filter(|(_, a)| *a == attacker)
            .map(|(b, _)| *b)
            .peekable();
        by.peek().is_some() && by.all(|b| b == blocker)
    }

    /// Raging River's restriction on `attacker`: from now on this combat,
    /// only a creature with flying or one of `allowed` may block it.
    pub fn limit_blockers_to_pile(&mut self, attacker: ObjectId, allowed: Vec<ObjectId>) {
        self.pile_limits.push((attacker, allowed));
    }

    /// Whether every pile limit on `attacker` lets `blocker` block it: it
    /// has flying, or it is in the pile each limit names.
    #[must_use]
    pub fn pile_limits_allow(&self, attacker: ObjectId, blocker: ObjectId, flying: bool) -> bool {
        flying
            || self
                .pile_limits
                .iter()
                .filter(|(a, _)| *a == attacker)
                .all(|(_, allowed)| allowed.contains(&blocker))
    }

    /// An effect says `attacker` becomes unblocked (CR 509.1h: the flag
    /// changes only by removal from combat, the combat's end, or an effect
    /// that says so).
    pub fn unblock(&mut self, attacker: ObjectId) {
        for info in &mut self.attackers {
            if info.creature == attacker {
                info.blocked = false;
            }
        }
    }

    /// Takes a permanent out of combat (CR 506.4).
    ///
    /// A creature that leaves the battlefield stops being an attacking,
    /// blocking, blocked or unblocked creature — and comes back, if it comes
    /// back at all, as a new object that was never in this combat (CR
    /// 400.7). The `ObjectId` does not say so on its own: it is an arena
    /// handle and survives the round trip, so a Restoration Angel blinking
    /// an attacker left the attacker declared, swinging and being blocked,
    /// with a creature that had not been on the battlefield when blockers
    /// were declared.
    ///
    /// Three entries go, and the third is the one worth naming: the
    /// creature's own attack, every block it was making, and every block
    /// made *against* it, because a blocker with nothing left to block deals
    /// its damage to nothing. What does not go is the `blocked` flag on some
    /// other attacker — that is a fact about the attacker, not about the
    /// blocker that has left.
    pub fn remove_from_combat(&mut self, id: ObjectId) {
        if let Ok(at) = self.attacking.binary_search_by_key(&id, |(c, _)| *c) {
            self.attacking.remove(at);
            self.attackers.retain(|a| a.creature != id);
        }
        self.blockers
            .retain(|b| b.blocker != id && b.attacker != id);
    }
}

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
        // Defender (CR 702.3b): can't attack, however untapped it is —
        // unless it can attack as though it didn't have defender.
        && (!obj.characteristics().keywords.contains(K::DEFENDER)
            || as_though(baylee_cards_dsl::Modifier::AttacksDespiteDefender))
        // "Can't attack" (Wayward Swordtooth, while it lacks the city's
        // blessing): the same rule as defender, from a static.
        && !obj.characteristics().keywords.contains(K::CANT_ATTACK)
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
                })
            })
            .map(Defender::Planeswalker),
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
    except_by: Vec<&'a crate::effects::ContinuousEffect>,
    requirements: Vec<&'a crate::effects::ContinuousEffect>,
}

impl<'a> AttackRules<'a> {
    /// The rules in force now.
    #[must_use]
    pub fn new(state: &'a GameState) -> Self {
        use baylee_cards_dsl::Modifier;
        let mut unless_defender_controls = Vec::new();
        let mut except_by = Vec::new();
        let mut requirements = Vec::new();
        for fx in state.effects.iter() {
            match fx.modifier {
                Modifier::CantAttackUnlessDefenderControls(_) => unless_defender_controls.push(fx),
                Modifier::CantBeAttackedExceptBy { .. } => except_by.push(fx),
                Modifier::AttacksEachCombat => requirements.push(fx),
                _ => {}
            }
        }
        Self {
            state,
            unless_defender_controls,
            except_by,
            requirements,
        }
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

    /// Whether `creature` may attack `defender`, past the restrictions on
    /// the pair. Asked of a creature [`can_attack`] already allows.
    #[must_use]
    pub fn allows(&self, creature: ObjectId, defender: Defender) -> bool {
        if self.unless_defender_controls.is_empty() && self.except_by.is_empty() {
            return true;
        }
        let state = self.state;
        let Some(obj) = state.object(creature) else {
            return false;
        };
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
        if self.unless_defender_controls.is_empty() {
            return true;
        }
        let Some(defending) = defending_player(state, defender) else {
            return false;
        };
        self.unless_defender_controls.iter().all(|fx| {
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

/// Whether `b` could block anything at all for `defending`: the half of
/// [`can_block`] that asks only about the blocker.
fn ready_to_block(b: &GameObject, defending: PlayerId) -> bool {
    b.zone == crate::zone::Zone::Battlefield
        && b.controller == defending
        && b.characteristics().types.contains(TypeSet::CREATURE)
        && !b.status.contains(Status::TAPPED)
        && !b.status.contains(Status::PHASED_OUT)
}

/// The permanents `defending` could declare as blockers at all, in
/// battlefield order: every one [`can_block`] might answer `true` for,
/// whatever the attacker.
///
/// The declare-blockers offer asks [`can_block`] of every candidate against
/// every attacker, so its candidates are worked out once, here, and not by
/// walking the whole battlefield per attacker. Over the battlefield that
/// step cost permanents × attackers — 168,000 permanents against 33,600
/// attackers took 18 s in one self-play game (r001 game 431) for a defender
/// with a handful of creatures — where over these it costs the defender's
/// untapped creatures × attackers.
#[must_use]
pub fn ready_blockers(state: &GameState, defending: PlayerId) -> Vec<ObjectId> {
    state
        .battlefield_seen()
        .filter(|id| {
            state
                .object(*id)
                .is_some_and(|b| ready_to_block(b, defending))
        })
        .collect()
}

/// Each basic landwalk and the land type it names (CR 702.14c, 305.6).
const LANDWALKS: [(K, SubtypeId); 5] = [
    (K::PLAINSWALK, land::PLAINS),
    (K::ISLANDWALK, land::ISLAND),
    (K::SWAMPWALK, land::SWAMP),
    (K::MOUNTAINWALK, land::MOUNTAIN),
    (K::FORESTWALK, land::FOREST),
];

/// Whether `player` controls a land with land type `subtype`, as the
/// battlefield is now: projected types, phased-out permanents not there
/// at all (CR 702.26b).
#[must_use]
pub fn controls_land_of_type(state: &GameState, player: PlayerId, subtype: SubtypeId) -> bool {
    state.battlefield_seen().any(|id| {
        state.object(id).is_some_and(|land| {
            let chars = land.characteristics();
            land.controller == player
                && chars.types.contains(TypeSet::LAND)
                && chars.subtypes.contains(subtype)
        })
    })
}

/// Whether `defending` may block `attacker` with `blocker` (keyword
/// restrictions included): only a creature attacking that player or one of
/// their planeswalkers (CR 509.1a, 802.4a).
#[must_use]
pub fn can_block(
    state: &GameState,
    defending: PlayerId,
    blocker: ObjectId,
    attacker: ObjectId,
) -> bool {
    let (Some(b), Some(a)) = (state.object(blocker), state.object(attacker)) else {
        return false;
    };
    // CR 509.1a, and at a table of several defending players 802.4a: a
    // defending player blocks only "creatures attacking that player, a
    // planeswalker that player controls". A creature not declared as an
    // attacker is asked only about its keywords (the pair's own half).
    if state
        .combat
        .defender_of(attacker)
        .is_some_and(|d| blocking_player(state, d) != Some(defending))
    {
        return false;
    }
    if !ready_to_block(b, defending)
        || a.zone != crate::zone::Zone::Battlefield
        || a.status.contains(Status::PHASED_OUT)
        || !a.characteristics().types.contains(TypeSet::CREATURE)
    {
        return false;
    }
    let kw =
        |o: &GameObject, k: baylee_cards_dsl::KeywordSet| o.characteristics().keywords.contains(k);
    // Shadow restricts both sides of the pair. It does not replace flying
    // or any other evasion restriction (Dauthi Voidwalker's release notes).
    if kw(a, K::SHADOW) != kw(b, K::SHADOW) {
        return false;
    }
    // Flying can only be blocked by flying/reach (CR 702.9).
    if kw(a, K::FLYING) && !kw(b, K::FLYING) && !kw(b, K::REACH) {
        return false;
    }
    // Raging River's piles (a restriction, CR 509.1b): only a creature with
    // flying or one in the pile chosen for this attacker.
    if !state
        .combat
        .pile_limits_allow(attacker, blocker, kw(b, K::FLYING))
    {
        return false;
    }
    // Fear (CR 702.36b): only artifact creatures and/or black creatures.
    if kw(a, K::FEAR) {
        let blocker = b.characteristics();
        if !blocker.types.contains(TypeSet::ARTIFACT) && !blocker.colors.contains(Color::Black) {
            return false;
        }
    }
    // Landwalk (CR 702.14c): unblockable while the *defending* player
    // controls a land of that type — that player's lands and nobody
    // else's, read as they are now (a land an effect made an Island is
    // one).
    if LANDWALKS
        .iter()
        .any(|(walk, land)| kw(a, *walk) && controls_land_of_type(state, defending, *land))
    {
        return false;
    }
    // The rest of the family (desertwalk, CR 702.14c): a land the effect's
    // filter describes, among the defending player's.
    if state.effects.iter().any(|fx| {
        let baylee_cards_dsl::Modifier::LandwalkMatching(filter) = fx.modifier else {
            return false;
        };
        crate::effects::applies_to(state, fx, a)
            && state.battlefield_seen().any(|id| {
                state.object(id).is_some_and(|land| {
                    land.controller == defending
                        && land.characteristics().types.contains(TypeSet::LAND)
                        && crate::eval::matches(
                            filter,
                            state,
                            land,
                            fx.controller,
                            fx.source.unwrap_or(attacker),
                        )
                })
            })
    }) {
        return false;
    }
    // Menace is deliberately *not* asked here. CR 702.111b restricts the
    // whole declaration and CR 509.1b is where that is checked, so a
    // function that sees one pair cannot answer it — and asking it here
    // answered "no" every time, because both callers ask before anything is
    // recorded: `progress_step` while it builds the offer, and
    // `declare_blockers` in a per-pair loop that runs to completion before
    // the first `declare_block`. `state.combat.blockers_of(attacker)` was
    // therefore always empty, which made menace read as plain unblockable
    // (#156). The count lives in `Engine::declare_blockers`, and
    // [`menace_satisfiable`] is what keeps the offer from naming a pairing
    // that count must refuse.
    // Unblockable.
    if kw(a, K::UNBLOCKABLE) {
        return false;
    }
    // "Can't block" (CR 509.1b, the restrictions half): read on the
    // **blocker**, where the line above is read on the attacker. The rule
    // covers both in one sentence and they are still two questions — a
    // creature that can't block and a creature that can't be blocked are
    // different cards, and a reader that asked only the attacker answered
    // one of them.
    if kw(b, K::CANT_BLOCK) {
        return false;
    }
    // Protection (CR 702.16f): can't be blocked by matching creatures.
    if crate::eval::protected_from(state, attacker, blocker) {
        return false;
    }
    // "Can't be blocked by creatures with power 2 or less" (CR 509.1b):
    // the attacker's restriction, asked of this blocker.
    if state.effects.iter().any(|fx| {
        let baylee_cards_dsl::Modifier::CantBeBlockedBy(f) = fx.modifier else {
            return false;
        };
        crate::effects::applies_to(state, fx, a)
            && crate::eval::matches(f, state, b, fx.controller, fx.source.unwrap_or(attacker))
    }) {
        return false;
    }
    true
}

/// Whether `defending` could field the two blockers CR 702.111b demands.
///
/// The half of menace that *is* answerable one attacker at a time. The
/// restriction itself is on the whole declaration, so [`can_block`] does not
/// try — but a defender who has only one creature that may legally block a
/// menace attacker has no legal declaration that blocks it at all, and an
/// offer naming that pairing would name a block `declare_blockers` must
/// refuse. The engine publishes only what a player may actually answer.
///
/// The count is over distinct blockers and never over blocks: CR 702.111b
/// asks for two or more *creatures*, so a single creature that may block an
/// additional creature still answers it once. `take(2)` walks `candidates`,
/// which holds each permanent once: [`ready_blockers`], or any list of
/// permanents that includes them (the whole battlefield answers the same,
/// only slower, since [`can_block`] asks the blocker's half again).
///
/// `true` for an attacker without menace, so callers may ask it of every
/// attacker without asking twice. The count asked for is the one
/// [`block_bound`] states, so the offer and the declaration read one number.
#[must_use]
pub fn menace_satisfiable(
    state: &GameState,
    defending: PlayerId,
    attacker: ObjectId,
    candidates: &[ObjectId],
) -> bool {
    if state.object(attacker).is_none() {
        return false;
    }
    let Some(bound) = block_bound(state, attacker) else {
        return true;
    };
    let need = usize::try_from(bound.min_blockers).unwrap_or(usize::MAX);
    candidates
        .iter()
        .copied()
        .filter(|blocker| can_block(state, defending, *blocker, attacker))
        .take(need)
        .count()
        == need
}

/// How many creatures may block `attacker`, where a rule bounds it: the
/// restriction on the whole declaration that CR 509.1b checks and no pair
/// can answer. Menace is two or more (CR 702.111b); an attacker nobody
/// blocks keeps it.
///
/// `None` for an attacker any number of creatures may block. The one
/// reader of the rule: the offer states what this returns
/// (`Pending::ChooseBlockers::bounds`), and `Engine::declare_blockers`
/// holds a declaration to it.
#[must_use]
pub fn block_bound(state: &GameState, attacker: ObjectId) -> Option<crate::choice::AttackerBound> {
    let a = state.object(attacker)?;
    a.characteristics()
        .keywords
        .contains(K::MENACE)
        .then_some(crate::choice::AttackerBound {
            attacker,
            min_blockers: 2,
            max_blockers: u32::MAX,
        })
}

/// The declare-blockers offer for `defending` (CR 509.1a): each creature
/// that may block, and the attackers it may block.
///
/// CR 702.111b restricts the declaration and not the pair, so [`can_block`]
/// cannot answer it — but an attacker this defender could never field two
/// legal blockers against is one no legal declaration blocks, and offering
/// that pairing would name a block `declare_blockers` has to refuse (#156).
/// Asked once per attacker rather than once per pair, because the answer is
/// the same for every blocker.
///
/// Both walks go over the defender's ready creatures, not the battlefield:
/// the pairs they skip are ones [`can_block`] refuses on the blocker's half
/// alone, and walking the battlefield per attacker made this step cost
/// permanents × attackers ([`ready_blockers`]).
///
/// The one universe the block requirements are measured in: a pair outside
/// it breaks a restriction, and "the maximum possible number of
/// requirements that could be obeyed without disobeying any restrictions"
/// (CR 509.1c) is a maximum over declarations made of these pairs.
#[must_use]
pub fn block_options(state: &GameState, defending: PlayerId) -> Vec<crate::choice::BlockOption> {
    let candidates = ready_blockers(state, defending);
    let blockable: Vec<ObjectId> = state
        .combat
        .attackers()
        .iter()
        .map(|a| a.creature)
        .filter(|a| menace_satisfiable(state, defending, *a, &candidates))
        .collect();
    candidates
        .iter()
        .copied()
        .filter_map(|blocker| {
            let attackers: Vec<ObjectId> = blockable
                .iter()
                .copied()
                .filter(|a| can_block(state, defending, blocker, *a))
                .collect();
            (!attackers.is_empty()).then_some(crate::choice::BlockOption { blocker, attackers })
        })
        .collect()
}

/// The rules about the declaration of blockers that effects add to what
/// [`can_block`] asks of one pair: how many attackers a creature may block
/// (CR 509.1a gives it one; "can block an additional creature each
/// combat" raises that), and the requirements (CR 509.1c: "all creatures
/// able to block enchanted creature do so", "it blocks each attacking
/// creature if able").
///
/// Collected once per declaration, for the reason [`AttackRules`] gives.
///
/// # Counting requirements
///
/// A requirement here is always about one pair, a blocker and an attacker:
/// a lure asks each creature able to block its creature to block it, and
/// "blocks each attacking creature" asks its creature to block each
/// attacker. [`BlockRules::demands`] is how many requirements ask for one
/// pair — two lures on one attacker ask twice, and blocking it obeys both —
/// and the requirements a declaration obeys are the sum over its pairs.
///
/// # The maximum
///
/// CR 509.1c refuses a declaration obeying fewer requirements than the
/// most any declaration could obey without breaking a restriction. The
/// restrictions the engine knows are the pairs [`block_options`] offers,
/// each blocker's limit, and menace's two blockers or none (CR 702.111b).
/// Without menace the blockers do not touch one another, and each obeys
/// most by blocking the attackers most requirements ask of it, up to its
/// limit: [`BlockRules::obeying`] does exactly that, so its count is the
/// maximum. A menace attacker a requirement names ties blockers together —
/// blocking it needs a second blocker, whose own requirements may then go
/// unobeyed — and there the declaration `obeying` builds is one good
/// declaration and not necessarily the best: its count can fall short of
/// the maximum, and a declaration between the two is accepted. It never
/// refuses a legal declaration, since `obeying`'s own is always one.
pub struct BlockRules<'a> {
    state: &'a GameState,
    additional: Vec<&'a crate::effects::ContinuousEffect>,
    any_number: Vec<&'a crate::effects::ContinuousEffect>,
    lures: Vec<&'a crate::effects::ContinuousEffect>,
    each: Vec<&'a crate::effects::ContinuousEffect>,
}

impl<'a> BlockRules<'a> {
    /// The rules in force now.
    #[must_use]
    pub fn new(state: &'a GameState) -> Self {
        use baylee_cards_dsl::Modifier;
        let mut rules = Self {
            state,
            additional: Vec::new(),
            any_number: Vec::new(),
            lures: Vec::new(),
            each: Vec::new(),
        };
        for fx in state.effects.iter() {
            match fx.modifier {
                Modifier::CanBlockAdditional(_) => rules.additional.push(fx),
                Modifier::CanBlockAnyNumber => rules.any_number.push(fx),
                Modifier::MustBeBlockedByAllAble => rules.lures.push(fx),
                Modifier::BlocksEachAttackerIfAble => rules.each.push(fx),
                _ => {}
            }
        }
        rules
    }

    /// Whether any block requirement is in force. Without one, every
    /// declaration obeys the most that can be obeyed, which is none.
    #[must_use]
    pub fn has_requirements(&self) -> bool {
        !self.lures.is_empty() || !self.each.is_empty()
    }

    /// How many attackers `blocker` may block (CR 509.1a): one, one more
    /// for each "additional creature", and `None` for any number.
    #[must_use]
    pub fn capacity(&self, blocker: ObjectId) -> Option<usize> {
        let state = self.state;
        let Some(obj) = state.object(blocker) else {
            return Some(1);
        };
        if self
            .any_number
            .iter()
            .any(|fx| crate::effects::applies_to(state, fx, obj))
        {
            return None;
        }
        let more: usize = self
            .additional
            .iter()
            .filter(|fx| crate::effects::applies_to(state, fx, obj))
            .map(|fx| match fx.modifier {
                baylee_cards_dsl::Modifier::CanBlockAdditional(n) => usize::from(n),
                _ => 0,
            })
            .sum();
        Some(1 + more)
    }

    /// How many requirements ask `blocker` to block `attacker`: each lure
    /// on the attacker and each "blocks each attacking creature" on the
    /// blocker. Asked only of a pair [`block_options`] offers, which is
    /// what "able to block" means.
    #[must_use]
    pub fn demands(&self, blocker: ObjectId, attacker: ObjectId) -> usize {
        let state = self.state;
        let on = |effects: &[&crate::effects::ContinuousEffect], id: ObjectId| {
            state.object(id).map_or(0, |obj| {
                effects
                    .iter()
                    .filter(|fx| crate::effects::applies_to(state, fx, obj))
                    .count()
            })
        };
        on(&self.lures, attacker) + on(&self.each, blocker)
    }

    /// How many requirements `declared` obeys. Each pair counts once, so a
    /// declaration naming a pair twice is refused before it is counted.
    #[must_use]
    pub fn obeyed(&self, declared: &[(ObjectId, ObjectId)]) -> usize {
        if !self.has_requirements() {
            return 0;
        }
        declared.iter().map(|(b, a)| self.demands(*b, *a)).sum()
    }

    /// One legal declaration out of `options` that obeys as many
    /// requirements as the engine holds a declaration to (the header's
    /// "The maximum"): empty when none is in force.
    ///
    /// Each blocker takes the attackers most requirements ask of it, up to
    /// its limit — attackers without menace first, since blocking one
    /// needs nobody else, and within each kind the most asked first. A
    /// menace attacker left with one blocker then gets a second from any
    /// creature that may block it and has room, or loses the one it has
    /// (CR 702.111b: two or none). Every pair is one `options` offers, and
    /// no blocker exceeds its limit, so `declare_blockers` accepts it.
    #[must_use]
    pub fn obeying(&self, options: &[crate::choice::BlockOption]) -> Vec<(ObjectId, ObjectId)> {
        if !self.has_requirements() {
            return Vec::new();
        }
        let menace = |attacker: ObjectId| has_keyword(self.state, attacker, K::MENACE);
        let room = |blocker: ObjectId| self.capacity(blocker).unwrap_or(usize::MAX);
        let mut chosen: Vec<(ObjectId, ObjectId)> = Vec::new();
        for option in options {
            let mut asked: Vec<(bool, std::cmp::Reverse<usize>, ObjectId)> = option
                .attackers
                .iter()
                .filter_map(|attacker| {
                    let demands = self.demands(option.blocker, *attacker);
                    (demands > 0).then_some((
                        menace(*attacker),
                        std::cmp::Reverse(demands),
                        *attacker,
                    ))
                })
                .collect();
            asked.sort_by_key(|(menace, demands, _)| (*menace, *demands));
            chosen.extend(
                asked
                    .into_iter()
                    .take(room(option.blocker))
                    .map(|(_, _, attacker)| (option.blocker, attacker)),
            );
        }
        let menacing: Vec<ObjectId> = chosen
            .iter()
            .map(|(_, attacker)| *attacker)
            .filter(|attacker| menace(*attacker))
            .collect();
        for attacker in menacing {
            let mut on_it = chosen.iter().filter(|(_, a)| *a == attacker);
            let (Some(&(lone, _)), None) = (on_it.next(), on_it.next()) else {
                continue;
            };
            let helper = options.iter().find(|o| {
                o.blocker != lone
                    && o.attackers.contains(&attacker)
                    && chosen.iter().filter(|(b, _)| *b == o.blocker).count() < room(o.blocker)
            });
            match helper {
                Some(helper) => chosen.push((helper.blocker, attacker)),
                None => chosen.retain(|pair| *pair != (lone, attacker)),
            }
        }
        chosen
    }
}

/// Whether a creature deals its combat damage in the given step
/// (CR 510.4): first strikers in the first step, everyone else in the
/// regular one, double strikers in both.
fn strikes_now(state: &GameState, creature: ObjectId, first_strike_step: bool) -> bool {
    let Some(obj) = state.object(creature) else {
        return false;
    };
    if let Some((first, eligible)) = &state.combat.prepared_damage {
        return *first == first_strike_step && eligible.contains(&(creature, obj.version));
    }
    let kw = obj.characteristics().keywords;
    if first_strike_step {
        kw.contains(K::FIRST_STRIKE) || kw.contains(K::DOUBLE_STRIKE)
    } else {
        kw.contains(K::DOUBLE_STRIKE)
            || state
                .combat
                .first_strikers
                .as_ref()
                .is_none_or(|first| !first.contains(&(creature, obj.version)))
    }
}

/// CR 510.4 fixes the strike-step participants before assignments. Revealing
/// a creature during assignment cannot create another damage step or remove
/// its already-required assignment. Returns whether any permanent turned up.
pub(crate) fn prepare_damage_step(state: &mut GameState, first: bool) -> bool {
    if state.combat.prepared_damage.is_some() {
        return false;
    }
    let mut creatures: Vec<_> = state
        .combat
        .attackers()
        .iter()
        .map(|a| a.creature)
        .chain(blocking_creatures(state))
        .collect();
    creatures.sort_unstable();
    creatures.dedup();
    let eligible: Vec<_> = creatures
        .iter()
        .copied()
        .filter(|&id| strikes_now(state, id, first))
        .filter_map(|id| state.object(id).map(|object| (id, object.version)))
        .collect();
    if first {
        state.combat.first_strikers = Some(eligible.clone());
    }
    state.combat.prepared_damage = Some((first, eligible));
    // Preview only establishes which sources would assign nonzero damage;
    // it neither spends existing divisions nor applies damage/prevention.
    let mut assignments = Vec::new();
    for attacker in state.combat.attackers() {
        if strikes_now(state, attacker.creature, first) {
            assign_attacker_damage(
                state,
                attacker.creature,
                attacker.defending,
                attacker.blocked,
                &mut assignments,
            );
        }
    }
    for blocker in blocking_creatures(state) {
        if strikes_now(state, blocker, first) {
            for (attacker, amount) in shares(
                state,
                blocker,
                &live_blocked(state, blocker),
                power_of(state, blocker),
            ) {
                assign(
                    &mut assignments,
                    blocker,
                    DamageTarget::Object(attacker),
                    amount,
                );
            }
        }
    }
    let mut changed = false;
    for assignment in assignments {
        if assignment.amount > 0 {
            changed |= state.reveal_masked(assignment.source);
        }
    }
    if changed {
        state.refresh_characteristics();
    }
    changed
}

/// How much damage from `source` is lethal to `target` right now
/// (CR 702.19b): toughness minus damage already marked, or 1 if the source
/// has deathtouch (CR 702.2b — *any* nonzero damage is lethal).
fn lethal_damage(state: &GameState, source: ObjectId, target: ObjectId) -> i16 {
    if has_keyword(state, source, K::DEATHTOUCH) {
        return 1;
    }
    let Some(obj) = state.object(target) else {
        return 1;
    };
    let toughness = obj.characteristics().toughness.unwrap_or(0).max(0);
    (toughness - obj.damage as i16).max(1)
}

fn has_keyword(state: &GameState, id: ObjectId, kw: baylee_cards_dsl::KeywordSet) -> bool {
    state
        .object(id)
        .is_some_and(|o| o.characteristics().keywords.contains(kw))
}

fn power_of(state: &GameState, id: ObjectId) -> i16 {
    state
        .object(id)
        .and_then(|o| o.characteristics().power)
        .unwrap_or(0)
        .max(0)
}

/// Whether a creature has banding as damage is assigned, which is when
/// CR 702.22j–k ask: a band formed while it had banding is still a band
/// without it (CR 702.22e), but who divides the damage is read now.
fn has_banding(state: &GameState, id: ObjectId) -> bool {
    has_keyword(state, id, K::BANDING)
}

/// The creatures blocking `attacker` that are still there, in declaration
/// order.
fn live_blockers(state: &GameState, attacker: ObjectId) -> Vec<ObjectId> {
    state
        .combat
        .blockers_of(attacker)
        .into_iter()
        .filter(|b| state.object(*b).is_some())
        .collect()
}

/// The attackers `blocker` blocks that are still there, in the order the
/// blocks were made.
fn live_blocked(state: &GameState, blocker: ObjectId) -> Vec<ObjectId> {
    state
        .combat
        .blocked_by(blocker)
        .into_iter()
        .filter(|a| state.object(*a).is_some())
        .collect()
}

/// Every blocking creature once, in the order each first blocked.
///
/// A creature blocking a band blocks each of its members (CR 702.22h), so
/// the declaration holds one pair per member, and it still deals its combat
/// damage once (CR 510.1d): the damage step walks blockers, not pairs.
fn blocking_creatures(state: &GameState) -> Vec<ObjectId> {
    let mut out: Vec<ObjectId> = Vec::new();
    for info in &state.combat.blockers {
        if !out.contains(&info.blocker) {
            out.push(info.blocker);
        }
    }
    out
}

/// The divisions of combat damage players still owe before this strike
/// step's damage can be dealt, attackers first and then blockers, which is
/// the order CR 510.1 has them announced in.
///
/// - an attacker blocked by two or more creatures has its damage divided
///   among them by its controller (CR 510.1c), or by the defending player
///   if one of them has banding (CR 702.22j);
/// - a blocker blocking two or more creatures, which only a band makes it
///   do, has its damage divided by the active player if one of them has
///   banding (CR 702.22k), and by its own controller if none has any more
///   (CR 510.1d).
///
/// An attacker with trample blocked by creatures without banding is not
/// asked: the engine assigns lethal damage to each blocker in declaration
/// order and the rest to what it attacks, which is one of the assignments
/// CR 702.19b lets its controller make (the simplification this module's
/// header names).
#[must_use]
pub fn divisions_owed(state: &GameState, first_strike_step: bool) -> Vec<OwedDivision> {
    let mut owed = Vec::new();
    for info in state.combat.attackers() {
        if !info.blocked || !strikes_now(state, info.creature, first_strike_step) {
            continue;
        }
        let amount = power_of(state, info.creature);
        let blockers = live_blockers(state, info.creature);
        if amount <= 0 || blockers.len() < 2 || state.combat.division(info.creature).is_some() {
            continue;
        }
        // Every blocker is the defending player's: only they declare blocks.
        let banding = blockers
            .iter()
            .find(|b| has_banding(state, **b))
            .and_then(|b| state.object(*b))
            .map(|o| o.controller);
        let chooser = match banding {
            Some(defending) => Some(defending),
            None if has_keyword(state, info.creature, K::TRAMPLE) => None,
            None => state.object(info.creature).map(|o| o.controller),
        };
        if let Some(chooser) = chooser {
            owed.push(OwedDivision {
                source: info.creature,
                chooser,
                recipients: blockers,
                amount,
            });
        }
    }
    for blocker in blocking_creatures(state) {
        if !strikes_now(state, blocker, first_strike_step) {
            continue;
        }
        let amount = power_of(state, blocker);
        let blocked = live_blocked(state, blocker);
        if amount <= 0 || blocked.len() < 2 || state.combat.division(blocker).is_some() {
            continue;
        }
        let chooser = if blocked.iter().any(|a| has_banding(state, *a)) {
            state.turn.active
        } else {
            let Some(controller) = state.object(blocker).map(|o| o.controller) else {
                continue;
            };
            controller
        };
        owed.push(OwedDivision {
            source: blocker,
            chooser,
            recipients: blocked,
            amount,
        });
    }
    owed
}

/// How `source`'s `power` goes to `recipients`: all of it to the one there
/// is, or the division a player chose (CR 510.1c–d, 702.22j–k).
///
/// Two or more recipients and no division is a caller that dealt damage
/// without asking (`divisions_owed`), which the engine never does; the
/// whole of it then goes to the first, which is one of the divisions the
/// player could have chosen.
fn shares(
    state: &GameState,
    source: ObjectId,
    recipients: &[ObjectId],
    power: i16,
) -> Vec<(ObjectId, i16)> {
    match recipients {
        [] => Vec::new(),
        [only] => vec![(*only, power)],
        [first, ..] => state.combat.division(source).map_or_else(
            || vec![(*first, power)],
            |d| {
                d.shares
                    .iter()
                    .copied()
                    .filter(|(r, _)| recipients.contains(r))
                    .collect()
            },
        ),
    }
}

/// Deals combat damage for one strike step.
///
/// `first_strike_step`: only first/double strikers deal damage; the regular
/// step skips first-strikers (double strikers deal in both).
///
/// Attackers and blockers are two separate passes on purpose. Folding the
/// blockers' damage into the attacker loop tied a blocker's strike step to
/// its *attacker's* keywords — a first-striking attacker made its ordinary
/// blocker strike first too, which is precisely the interaction first
/// strike exists to decide — and skipped blockers the attacker had run out
/// of damage to assign to, though CR 510.1d has every blocking creature
/// deal its damage regardless.
///
/// The divisions players chose for this step (`divisions_owed`) are spent
/// here and emptied, so a double striker's second step asks again.
pub(crate) fn collect_combat_damage(
    state: &mut GameState,
    first_strike_step: bool,
) -> Vec<crate::damage::Assignment> {
    prepare_damage_step(state, first_strike_step);
    let mut assignments = Vec::new();
    for info in state.combat.attackers() {
        if strikes_now(state, info.creature, first_strike_step) {
            assign_attacker_damage(
                state,
                info.creature,
                info.defending,
                info.blocked,
                &mut assignments,
            );
        }
    }
    for blocker in blocking_creatures(state) {
        if !strikes_now(state, blocker, first_strike_step) {
            continue;
        }
        let power = power_of(state, blocker);
        for (attacker, amount) in shares(state, blocker, &live_blocked(state, blocker), power) {
            assign(
                &mut assignments,
                blocker,
                DamageTarget::Object(attacker),
                amount,
            );
        }
    }
    state.combat.divisions.clear();
    state.combat.prepared_damage = None;
    assignments
}

/// Assign every source before prevention can change a creature's power.
fn assign_attacker_damage(
    state: &GameState,
    attacker: ObjectId,
    defending: Defender,
    blocked: bool,
    out: &mut Vec<crate::damage::Assignment>,
) {
    let power = power_of(state, attacker);
    if power <= 0 {
        return;
    }
    let trample = has_keyword(state, attacker, K::TRAMPLE);
    let live = live_blockers(state, attacker);
    if blocked && (!trample || live.iter().any(|b| has_banding(state, *b))) {
        for (blocker, amount) in shares(state, attacker, &live, power) {
            assign(out, attacker, DamageTarget::Object(blocker), amount);
        }
    } else if blocked {
        let mut remaining = power;
        for blocker in &live {
            if remaining <= 0 {
                break;
            }
            let assigned = remaining.min(lethal_damage(state, attacker, *blocker));
            assign(out, attacker, DamageTarget::Object(*blocker), assigned);
            remaining -= assigned;
        }
        assign_defender(state, out, attacker, defending, remaining);
    } else {
        assign_defender(state, out, attacker, defending, power);
    }
}

fn assign_defender(
    state: &GameState,
    out: &mut Vec<crate::damage::Assignment>,
    source: ObjectId,
    defender: Defender,
    amount: i16,
) {
    let to = match defender {
        Defender::Player(player) => DamageTarget::Player(player),
        Defender::Planeswalker(id) => {
            if defending_player(state, defender).is_none() {
                return;
            }
            DamageTarget::Object(id)
        }
    };
    assign(out, source, to, amount);
}

fn assign(
    out: &mut Vec<crate::damage::Assignment>,
    source: ObjectId,
    recipient: DamageTarget,
    amount: i16,
) {
    if amount > 0 {
        out.push(crate::damage::Assignment {
            source,
            source_version: None,
            recipient,
            amount: amount as u32,
            is_combat: true,
        });
    }
}

#[cfg(test)]
fn deal_combat_damage(state: &mut GameState, first_strike_step: bool) {
    let assignments = collect_combat_damage(state, first_strike_step);
    assert!(
        crate::damage::DamageWork::new(state, assignments)
            .advance(state)
            .is_none(),
        "fixture needs damage choices"
    );
}

/// True if the damage `source` is dealing can't be prevented
/// (`CombatDamageCantBePrevented`, CR 615.12): combat damage from an object
/// an effect says so of. Every prevention effect then does nothing to it,
/// protection's (CR 702.16e) as much as a shield's.
pub(crate) fn unpreventable(state: &GameState, source: ObjectId, is_combat: bool) -> bool {
    is_combat
        && state.object(source).is_some_and(|obj| {
            state.effects.iter().any(|fx| {
                matches!(
                    fx.modifier,
                    baylee_cards_dsl::Modifier::CombatDamageCantBePrevented
                ) && crate::effects::applies_to(state, fx, obj)
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::ObjectKind;
    use crate::state::CardLookup;
    use crate::zone::ZoneLocation;
    use baylee_cards_dsl::KeywordSet;
    use baylee_core::ids::CardIndex;
    use baylee_core::preset::{FormatId, GamePreset, HouseRules, SeatController, SeatSpec};

    /// A registry with nothing in it: these tests build creatures directly
    /// rather than going through cards, because the interactions under
    /// test are between *keywords*, and picking real cards that happen to
    /// carry them would make the test about those cards.
    struct NoCards;
    impl CardLookup for NoCards {
        fn card(&self, _: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
            None
        }
    }

    fn empty_state() -> GameState {
        seated(2)
    }

    /// An empty board with `seats` players at it.
    fn seated(seats: usize) -> GameState {
        let seat = || SeatSpec {
            controller: SeatController::Open,
            capabilities: baylee_core::preset::SeatCapabilities::default(),
            deck: vec![],
            sideboard: vec![],
            commanders: vec![],
            starting_life: Some(20),
            starting_hand: None,
            starting_battlefield: vec![],
            emblems: vec![],
            team: None,
        };
        GameState::from_preset(
            &GamePreset {
                format: FormatId::Freeform,
                seed: 1,
                house_rules: HouseRules::default(),
                modifiers: vec![],
                prints: vec![],
                seats: (0..seats).map(|_| seat()).collect(),
            },
            &NoCards,
        )
        .expect("an empty board")
    }

    fn creature(
        state: &mut GameState,
        controller: PlayerId,
        power: i16,
        toughness: i16,
        keywords: KeywordSet,
    ) -> ObjectId {
        let name = state.names.intern("Test Creature");
        let id = state.create_bare(
            controller,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        let b = state.object_mut(id).expect("just created").base_mut();
        b.types = TypeSet::CREATURE;
        b.power = Some(power);
        b.toughness = Some(toughness);
        b.keywords = keywords;
        id
    }

    fn attack(state: &mut GameState, creature: ObjectId, defending: PlayerId) {
        state.combat.declare_attackers([AttackerInfo {
            creature,
            defending: Defender::Player(defending),
            blocked: false,
            band: None,
        }]);
    }

    /// Declares `creature` as attacking a planeswalker instead of a seat.
    fn attack_walker(state: &mut GameState, creature: ObjectId, walker: ObjectId) {
        state.combat.declare_attackers([AttackerInfo {
            creature,
            defending: Defender::Planeswalker(walker),
            blocked: false,
            band: None,
        }]);
    }

    /// A planeswalker on the battlefield with `loyalty` counters.
    fn planeswalker(state: &mut GameState, controller: PlayerId, loyalty: u16) -> ObjectId {
        let name = state.names.intern("Test Walker");
        let id = state.create_bare(
            controller,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        let obj = state.object_mut(id).expect("just created");
        let b = obj.base_mut();
        b.types = TypeSet::PLANESWALKER;
        b.loyalty = Some(loyalty);
        obj.counters
            .set(baylee_cards_dsl::CounterKind::Loyalty, loyalty);
        id
    }

    fn loyalty(state: &GameState, id: ObjectId) -> u16 {
        state.object(id).map_or(0, |o| {
            o.counters.get(baylee_cards_dsl::CounterKind::Loyalty)
        })
    }

    fn block(state: &mut GameState, blocker: ObjectId, attacker: ObjectId) {
        state.combat.declare_block(blocker, attacker);
    }

    fn damage(state: &GameState, id: ObjectId) -> u16 {
        state.object(id).map_or(0, |o| o.damage)
    }

    fn on_battlefield(state: &GameState, id: ObjectId) -> bool {
        state
            .object(id)
            .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
    }

    const P0: PlayerId = PlayerId::new(0);
    const P1: PlayerId = PlayerId::new(1);

    /// A blink takes a creature out of combat and brings back a different
    /// one (CR 506.4, CR 400.7) — different to the rules, at any rate; the
    /// `ObjectId` is an arena handle and comes back unchanged, which is why
    /// nothing noticed. Ephemerate on an attacking Solemn Simulacrum left it
    /// declared as an attacker, and a Restoration Angel later took an
    /// attacking Sun Titan and Elesh Norn out of a combat they went on
    /// fighting.
    /// A board of `teams.len()` seats, on the sides it names.
    fn teamed_state(teams: &[Option<u8>]) -> GameState {
        let seat = |team: Option<u8>| SeatSpec {
            controller: SeatController::Open,
            capabilities: baylee_core::preset::SeatCapabilities::default(),
            deck: vec![],
            sideboard: vec![],
            commanders: vec![],
            starting_life: Some(20),
            starting_hand: None,
            starting_battlefield: vec![],
            emblems: vec![],
            team,
        };
        GameState::from_preset(
            &GamePreset {
                format: FormatId::Freeform,
                seed: 1,
                house_rules: HouseRules::default(),
                modifiers: vec![],
                prints: vec![],
                seats: teams.iter().copied().map(seat).collect(),
            },
            &NoCards,
        )
        .expect("a seated board")
    }

    /// What may be attacked is **each surviving opponent and the
    /// planeswalkers those opponents control** (CR 506.2), as one flat list
    /// because the rules make it one choice and a client renders it as one.
    ///
    /// A teammate is not an opponent, and neither is their planeswalker —
    /// the walker half reads the same opponent list, which is the only
    /// reason it cannot answer differently. My own walker is not a defender
    /// either, and a seat that has lost is no longer anybody's opponent.
    #[test]
    fn a_defender_is_an_opponent_or_something_an_opponent_controls() {
        let mut state = teamed_state(&[Some(1), Some(2), Some(1), Some(2)]);
        let me = PlayerId::new(0);
        let ally = PlayerId::new(2);
        let enemy = PlayerId::new(1);
        let other_enemy = PlayerId::new(3);

        let mine = planeswalker(&mut state, me, 4);
        let allys = planeswalker(&mut state, ally, 4);
        let theirs = planeswalker(&mut state, enemy, 4);

        let options = defender_options(&state, me);
        assert_eq!(
            options,
            vec![
                Defender::Player(enemy),
                Defender::Player(other_enemy),
                Defender::Planeswalker(theirs),
            ],
            "the two seats across the table and the one walker they control"
        );
        assert!(!options.contains(&Defender::Player(ally)));
        assert!(
            !options.contains(&Defender::Planeswalker(allys)),
            "a teammate's planeswalker is not a defender"
        );
        assert!(!options.contains(&Defender::Planeswalker(mine)));

        // A seat that has lost is nobody's opponent any more, and its
        // planeswalker goes with it.
        state
            .players
            .iter_mut()
            .find(|p| p.id == enemy)
            .expect("seated")
            .loss = Some(crate::event::LossReason::Conceded);
        assert_eq!(
            defender_options(&state, me),
            vec![Defender::Player(other_enemy)],
            "and the walker it controlled is no longer reachable either"
        );
    }

    /// In a duel every other seat is an opponent, which is the case that
    /// makes the team reading above invisible: with no teams on the table
    /// the two answers are the same list.
    #[test]
    fn with_no_teams_on_the_table_every_other_seat_is_a_defender() {
        let mut state = empty_state();
        let me = PlayerId::new(0);
        let them = PlayerId::new(1);
        let walker = planeswalker(&mut state, them, 3);
        creature(&mut state, them, 2, 2, KeywordSet::EMPTY);

        assert_eq!(
            defender_options(&state, me),
            vec![Defender::Player(them), Defender::Planeswalker(walker)],
            "a creature they control is not something to attack"
        );
        assert_eq!(defender_options(&state, them), vec![Defender::Player(me)]);
    }

    /// The damage aimed at a defender goes to a seat, and which seat is a
    /// second question: a planeswalker's controller rather than the player
    /// it was declared against.
    ///
    /// `None` once the walker has left. The attack stays declared
    /// (CR 506.4c) — this is what says there is nothing left to damage,
    /// which is the difference between a trampling attacker having a
    /// recipient and having none.
    #[test]
    fn the_damage_goes_to_a_seat_and_a_walker_that_left_names_none() {
        let mut state = empty_state();
        let me = PlayerId::new(0);
        let them = PlayerId::new(1);
        let walker = planeswalker(&mut state, them, 3);

        assert_eq!(defending_player(&state, Defender::Player(them)), Some(them));
        assert_eq!(
            defending_player(&state, Defender::Player(me)),
            Some(me),
            "a seat names itself whoever is asking"
        );
        assert_eq!(
            defending_player(&state, Defender::Planeswalker(walker)),
            Some(them),
            "the walker's controller, not whoever was attacked"
        );

        state
            .move_object(
                walker,
                ZoneLocation::Graveyard(them),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::StateBased,
            )
            .expect("it dies");
        assert_eq!(
            defending_player(&state, Defender::Planeswalker(walker)),
            None,
            "the attack is still declared and there is nothing to damage"
        );
        assert_eq!(
            defending_player(&state, Defender::Planeswalker(ObjectId::new(9_999, 0))),
            None,
            "and an id that never was anything answers the same way"
        );
    }

    #[test]
    fn a_blinked_attacker_is_out_of_combat() {
        let mut state = empty_state();
        let titan = creature(&mut state, P0, 6, 6, KeywordSet::EMPTY);
        let wall = creature(&mut state, P1, 0, 8, KeywordSet::EMPTY);
        attack(&mut state, titan, P1);
        block(&mut state, wall, titan);

        // Out and straight back in, which is what a blink is.
        state
            .move_object(
                titan,
                ZoneLocation::Exile(P0),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .expect("exiled");
        state
            .move_object(
                titan,
                ZoneLocation::Battlefield,
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .expect("returned");
        assert!(on_battlefield(&state, titan), "the blink brought it back");
        assert!(
            state.combat.attackers().is_empty(),
            "it is not attacking any more"
        );
        assert!(
            state.combat.blockers.is_empty(),
            "and the wall has nothing left to block"
        );

        deal_combat_damage(&mut state, false);
        assert_eq!(damage(&state, wall), 0, "it dealt no damage");
        assert_eq!(damage(&state, titan), 0, "and took none");
        assert_eq!(state.players[1].life, 20, "nor did anything get through");
    }

    /// "Had become blocked by only that creature this combat" reads every
    /// block made, not the blocks left: a second blocker that has since left
    /// combat still counts, an attacker nothing blocked was blocked by
    /// nobody, and `unblock` clears the flag only of the attacker it names.
    #[test]
    fn blocked_only_by_reads_the_combats_blocks_and_not_the_blocks_left() {
        let mut state = empty_state();
        let attacker = creature(&mut state, P0, 3, 3, KeywordSet::EMPTY);
        let other = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
        let first = creature(&mut state, P1, 1, 1, KeywordSet::EMPTY);
        let second = creature(&mut state, P1, 1, 1, KeywordSet::EMPTY);
        state
            .combat
            .declare_attackers([attacker, other].map(|creature| AttackerInfo {
                creature,
                defending: Defender::Player(P1),
                blocked: false,
                band: None,
            }));
        state.combat.declare_block(first, attacker);
        assert!(state.combat.blocked_only_by(attacker, first));
        assert!(!state.combat.blocked_only_by(other, first), "never blocked");
        state.combat.declare_block(second, attacker);
        state.combat.remove_from_combat(second);
        assert!(
            !state.combat.blocked_only_by(attacker, first),
            "the second blocker left, and still blocked it this combat"
        );
        state.combat.declare_block(first, other);
        state.combat.unblock(other);
        assert!(!state.combat.is_blocked(other));
        assert!(state.combat.is_blocked(attacker));
    }

    /// `is_attacking` answers from a sorted index kept beside the attacker
    /// list, so each door that changes the list must move the index with it:
    /// a declaration out of id order, a creature leaving combat, and the
    /// filter a player leaving the game runs (CR 800.4a).
    #[test]
    fn the_attacking_index_follows_every_change_to_the_attackers() {
        let mut state = empty_state();
        let ids: Vec<ObjectId> = (0..4)
            .map(|_| creature(&mut state, P0, 1, 1, KeywordSet::EMPTY))
            .collect();
        let idle = creature(&mut state, P0, 1, 1, KeywordSet::EMPTY);
        let agree = |state: &GameState| {
            for id in ids.iter().chain([&idle]) {
                assert_eq!(
                    state.combat.is_attacking(*id),
                    state.combat.attackers().iter().any(|a| a.creature == *id),
                    "the index and the list disagree about {id:?}"
                );
            }
        };
        for id in ids.iter().rev() {
            attack(&mut state, *id, P1);
        }
        agree(&state);
        assert!(!state.combat.is_attacking(idle), "it was never declared");
        assert_eq!(
            state.combat.attackers()[0].creature,
            ids[3],
            "the list keeps declaration order; only the index is sorted"
        );

        state.combat.remove_from_combat(ids[1]);
        agree(&state);
        assert!(!state.combat.is_attacking(ids[1]));

        state.combat.retain_attackers(|a| a.creature != ids[2]);
        agree(&state);
        assert!(!state.combat.is_attacking(ids[2]));
        assert!(state.combat.is_attacking(ids[0]) && state.combat.is_attacking(ids[3]));
    }

    /// The other half, and the asymmetry that makes it its own case
    /// (CR 509.1h): an attacker whose blocker leaves stays **blocked**. It
    /// deals its damage to nothing at all — the blocker is gone and the
    /// player is not a legal recipient.
    #[test]
    fn an_attacker_stays_blocked_when_its_blocker_is_blinked() {
        let mut state = empty_state();
        let bear = creature(&mut state, P0, 7, 7, KeywordSet::EMPTY);
        let chump = creature(&mut state, P1, 1, 1, KeywordSet::EMPTY);
        attack(&mut state, bear, P1);
        block(&mut state, chump, bear);

        state
            .move_object(
                chump,
                ZoneLocation::Exile(P1),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .expect("exiled");
        state
            .move_object(
                chump,
                ZoneLocation::Battlefield,
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .expect("returned");
        assert!(
            state.combat.is_blocked(bear),
            "being blocked is a fact about the attacker, not about the blocker"
        );
        assert!(state.combat.blockers_of(bear).is_empty());

        deal_combat_damage(&mut state, false);
        assert_eq!(state.players[1].life, 20, "seven damage went nowhere");
        assert_eq!(
            damage(&state, chump),
            0,
            "the creature that came back was never in this combat"
        );
    }

    /// "Blocking creature" and "unblocked creature" (CR 509.1g, 509.1h): an
    /// attacker is neither blocked nor unblocked before blockers are
    /// declared, and one that was blocked stays blocked with its blocker
    /// gone, while the creature that came back is blocking nothing.
    #[test]
    fn blocking_and_unblocked_read_the_declarations() {
        use baylee_cards_dsl::Filter;
        let is = |state: &GameState, filter: &Filter, id: ObjectId| {
            crate::eval::matches(filter, state, state.object(id).expect("here"), P1, id)
        };
        let mut state = empty_state();
        let blocked = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
        let free = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
        let wall = creature(&mut state, P1, 0, 4, KeywordSet::EMPTY);
        attack(&mut state, blocked, P1);
        attack(&mut state, free, P1);
        state.turn.step = crate::turn::Step::DeclareAttackers;
        assert!(
            !is(&state, &Filter::Unblocked, free),
            "not before blockers are declared"
        );

        state.turn.step = crate::turn::Step::DeclareBlockers;
        block(&mut state, wall, blocked);
        assert!(is(&state, &Filter::Unblocked, free));
        assert!(!is(&state, &Filter::Unblocked, blocked));
        assert!(
            !is(&state, &Filter::Unblocked, wall),
            "a blocker attacks nothing"
        );
        assert!(is(&state, &Filter::Blocking, wall));
        assert!(!is(&state, &Filter::Blocking, free));

        for to in [ZoneLocation::Exile(P1), ZoneLocation::Battlefield] {
            state
                .move_object(
                    wall,
                    to,
                    crate::zone::ZonePosition::Top,
                    crate::event::Cause::Effect,
                )
                .expect("blinked");
        }
        assert!(
            !is(&state, &Filter::Blocking, wall),
            "the creature that came back blocks nothing"
        );
        assert!(
            !is(&state, &Filter::Unblocked, blocked),
            "and what it blocked stays blocked"
        );
    }

    /// …unless it tramples, which is the exception the owner asked for by
    /// name: CR 702.19b assigns everything past the (absent) blockers to
    /// what the creature was attacking.
    #[test]
    fn trample_goes_through_when_the_blocker_is_blinked() {
        let mut state = empty_state();
        let beast = creature(&mut state, P0, 7, 7, KeywordSet::TRAMPLE);
        let chump = creature(&mut state, P1, 1, 1, KeywordSet::EMPTY);
        attack(&mut state, beast, P1);
        block(&mut state, chump, beast);

        state
            .move_object(
                chump,
                ZoneLocation::Exile(P1),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .expect("exiled");
        state
            .move_object(
                chump,
                ZoneLocation::Battlefield,
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .expect("returned");

        deal_combat_damage(&mut state, false);
        assert_eq!(
            state.players[1].life, 13,
            "all seven trample through, nothing having to be assigned first"
        );
    }

    /// Who divides a creature's damage when it blocks a band: the active
    /// player while a creature it blocks has banding (CR 702.22k), and the
    /// blocker's own controller once none has (CR 510.1d) — the band itself
    /// outlives the keyword (CR 702.22e), so the question is asked of the
    /// creatures as they are now.
    #[test]
    fn a_blocker_on_a_band_that_lost_banding_is_divided_by_its_controller() {
        let mut state = empty_state();
        let first = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
        let second = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
        let blocker = creature(&mut state, P1, 3, 3, KeywordSet::EMPTY);
        attack(&mut state, first, P1);
        attack(&mut state, second, P1);
        state.combat.form_band(&[first, second]);
        block(&mut state, blocker, first);
        block(&mut state, blocker, second);

        let owed = divisions_owed(&state, false);
        assert_eq!(
            owed,
            vec![OwedDivision {
                source: blocker,
                chooser: P1,
                recipients: vec![first, second],
                amount: 3,
            }],
            "no creature it blocks has banding: its controller divides"
        );

        state.object_mut(first).expect("there").base_mut().keywords = KeywordSet::BANDING;
        state.refresh_characteristics();
        let owed = divisions_owed(&state, false);
        assert_eq!(owed.len(), 1);
        assert_eq!(
            owed[0].chooser, state.turn.active,
            "banding hands it to the active player"
        );
        assert_ne!(
            state.turn.active, P1,
            "and that is not the blocker's controller"
        );
    }

    /// A creature blocking two deals its combat damage once, as divided
    /// (CR 510.1d): 1 and 2 of its 3, not 3 to each.
    #[test]
    fn a_blocker_on_two_creatures_deals_its_power_once() {
        let mut state = empty_state();
        let first = creature(&mut state, P0, 0, 4, KeywordSet::EMPTY);
        let second = creature(&mut state, P0, 0, 4, KeywordSet::EMPTY);
        let blocker = creature(&mut state, P1, 3, 3, KeywordSet::EMPTY);
        attack(&mut state, first, P1);
        attack(&mut state, second, P1);
        state.combat.form_band(&[first, second]);
        block(&mut state, blocker, first);
        block(&mut state, blocker, second);
        state.combat.record_division(Division {
            source: blocker,
            shares: vec![(first, 1), (second, 2)],
        });

        deal_combat_damage(&mut state, false);
        assert_eq!((damage(&state, first), damage(&state, second)), (1, 2));
        assert!(
            state.combat.division(blocker).is_none(),
            "a division is spent by the step it was chosen for"
        );
    }

    /// CR 702.2b + 704.5h: any nonzero damage from a deathtouch source is
    /// lethal. A 1/1 deathtoucher marks one damage on a 6/6 and the SBA
    /// pass has to destroy it, even though one is nowhere near six.
    #[test]
    fn a_point_of_deathtouch_damage_is_lethal() {
        let mut state = empty_state();
        let biter = creature(&mut state, P0, 1, 1, KeywordSet::DEATHTOUCH);
        let bear = creature(&mut state, P1, 6, 6, KeywordSet::EMPTY);
        attack(&mut state, biter, P1);
        block(&mut state, bear, biter);

        deal_combat_damage(&mut state, false);
        assert_eq!(damage(&state, bear), 1, "one damage, not six");
        assert!(
            state.object(bear).expect("still alive").deathtouched,
            "the deathtouch mark is what the SBA reads"
        );

        crate::sba::run(&mut state, &NoCards);
        // These test creatures are card-less, so dying takes them out of
        // the game entirely (CR 704.5e) rather than to a graveyard.
        assert!(
            !on_battlefield(&state, bear),
            "the 6/6 dies to one point of deathtouch damage"
        );
    }

    /// The deathtouch window is "since the last SBA check" (CR 704.5h), so
    /// an indestructible creature that survived the mark must not die when
    /// the next unrelated SBA pass runs.
    #[test]
    fn deathtouch_does_not_linger_past_the_sba_that_judged_it() {
        let mut state = empty_state();
        let biter = creature(&mut state, P0, 1, 1, KeywordSet::DEATHTOUCH);
        let wall = creature(&mut state, P1, 0, 4, KeywordSet::INDESTRUCTIBLE);
        attack(&mut state, biter, P1);
        block(&mut state, wall, biter);
        deal_combat_damage(&mut state, false);

        crate::sba::run(&mut state, &NoCards);
        assert!(
            on_battlefield(&state, wall),
            "indestructible survives deathtouch (CR 702.12b)"
        );
        assert!(!state.object(wall).expect("alive").deathtouched);

        // Losing indestructibility later must not make it die retroactively.
        state.object_mut(wall).expect("alive").base_mut().keywords = KeywordSet::EMPTY;
        crate::sba::run(&mut state, &NoCards);
        assert!(
            on_battlefield(&state, wall),
            "the mark expired with the SBA pass that judged it"
        );
    }

    /// CR 510.4: first strike is a property of the creature dealing the
    /// damage. A first-striking attacker must not drag its ordinary
    /// blocker into the first-strike step — that is the whole point of
    /// the keyword.
    #[test]
    fn first_strike_is_per_creature_not_per_combat() {
        let mut state = empty_state();
        let knight = creature(&mut state, P0, 3, 3, KeywordSet::FIRST_STRIKE);
        let bear = creature(&mut state, P1, 2, 2, KeywordSet::EMPTY);
        attack(&mut state, knight, P1);
        block(&mut state, bear, knight);

        deal_combat_damage(&mut state, true);
        assert_eq!(damage(&state, bear), 3, "the first striker connects");
        assert_eq!(
            damage(&state, knight),
            0,
            "the ordinary blocker does not strike first"
        );

        // The bear is dead before the regular step, so it never strikes.
        crate::sba::run(&mut state, &NoCards);
        deal_combat_damage(&mut state, false);
        assert_eq!(damage(&state, knight), 0, "the knight takes nothing back");
    }

    /// CR 510.1d: every blocking creature assigns its combat damage,
    /// whether or not the attacker had damage left to assign to it. The
    /// attacker's assignment loop must not gate the blockers' strikes.
    #[test]
    fn every_blocker_strikes_even_when_the_attacker_ran_out() {
        let mut state = empty_state();
        let small = creature(&mut state, P0, 1, 10, KeywordSet::EMPTY);
        let first = creature(&mut state, P1, 2, 2, KeywordSet::EMPTY);
        let second = creature(&mut state, P1, 3, 3, KeywordSet::EMPTY);
        attack(&mut state, small, P1);
        block(&mut state, first, small);
        block(&mut state, second, small);

        deal_combat_damage(&mut state, false);
        assert_eq!(damage(&state, small), 5, "both blockers deal damage: 2 + 3");
    }

    /// A blocker with lifelink gains its controller life (CR 702.15b is
    /// about the damage, not about who is attacking).
    #[test]
    fn a_blocker_with_lifelink_gains_life() {
        let mut state = empty_state();
        let attacker = creature(&mut state, P0, 1, 5, KeywordSet::EMPTY);
        let blocker = creature(&mut state, P1, 4, 4, KeywordSet::LIFELINK);
        attack(&mut state, attacker, P1);
        block(&mut state, blocker, attacker);

        deal_combat_damage(&mut state, false);
        assert_eq!(state.players[1].life, 24, "20 + the blocker's 4 power");
    }

    /// Trample with deathtouch only has to assign one damage per blocker
    /// before the rest tramples over (CR 702.19b + 702.2b).
    #[test]
    fn trample_over_deathtouch_only_owes_one_per_blocker() {
        let mut state = empty_state();
        let beast = creature(
            &mut state,
            P0,
            5,
            5,
            KeywordSet::TRAMPLE.union(KeywordSet::DEATHTOUCH),
        );
        let wall = creature(&mut state, P1, 0, 4, KeywordSet::EMPTY);
        attack(&mut state, beast, P1);
        block(&mut state, wall, beast);

        deal_combat_damage(&mut state, false);
        assert_eq!(damage(&state, wall), 1, "one point is lethal here");
        assert_eq!(state.players[1].life, 16, "the other four trample through");
    }

    /// Double strike (CR 702.4b): damage in *both* steps, and the same
    /// creature deals its full power each time.
    #[test]
    fn a_double_striker_deals_damage_in_both_steps() {
        let mut state = empty_state();
        let hero = creature(&mut state, P0, 2, 2, KeywordSet::DOUBLE_STRIKE);
        let wall = creature(&mut state, P1, 0, 9, KeywordSet::EMPTY);
        attack(&mut state, hero, P1);
        block(&mut state, wall, hero);

        deal_combat_damage(&mut state, true);
        assert_eq!(
            damage(&state, wall),
            2,
            "no damage in the first-strike step"
        );
        deal_combat_damage(&mut state, false);
        assert_eq!(
            damage(&state, wall),
            4,
            "no second helping in the regular step"
        );
    }

    #[test]
    fn assignment_reveal_cannot_retroactively_grant_an_extra_strike() {
        use crate::object::{Rider, Status};
        let mut state = empty_state();
        let first = creature(&mut state, P0, 1, 1, KeywordSet::FIRST_STRIKE);
        let masked = creature(&mut state, P0, 5, 5, KeywordSet::DOUBLE_STRIKE);
        let object = state.object_mut(masked).unwrap();
        object.original_base = Some(std::sync::Arc::clone(&object.base));
        let base = object.base_mut();
        base.power = Some(2);
        base.toughness = Some(2);
        base.keywords = KeywordSet::EMPTY;
        object.status.insert(Status::FACE_DOWN);
        object.riders.push(Rider::Masked);
        attack(&mut state, first, P1);
        attack(&mut state, masked, P1);
        deal_combat_damage(&mut state, true);
        assert_eq!(state.players[1].life, 19);
        assert!(
            state
                .object(masked)
                .unwrap()
                .status
                .contains(Status::FACE_DOWN)
        );
        deal_combat_damage(&mut state, false);
        assert_eq!(
            state.players[1].life, 14,
            "revealed power assigns once in the already-required regular step"
        );
        assert!(
            !state
                .object(masked)
                .unwrap()
                .status
                .contains(Status::FACE_DOWN)
        );
    }

    /// The control: a plain first striker must *not* strike twice, which
    /// is the only thing that makes the test above about double strike
    /// rather than about the step machinery.
    #[test]
    fn a_first_striker_deals_damage_only_once() {
        let mut state = empty_state();
        let knight = creature(&mut state, P0, 2, 2, KeywordSet::FIRST_STRIKE);
        let wall = creature(&mut state, P1, 0, 9, KeywordSet::EMPTY);
        attack(&mut state, knight, P1);
        block(&mut state, wall, knight);

        deal_combat_damage(&mut state, true);
        deal_combat_damage(&mut state, false);
        assert_eq!(damage(&state, wall), 2, "first strike struck twice");
    }

    /// Defender (CR 702.3b): untapped, awake, and still not attacking.
    #[test]
    fn a_creature_with_defender_cannot_attack() {
        let mut state = empty_state();
        let wall = creature(&mut state, P0, 0, 4, KeywordSet::DEFENDER);
        let bear = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
        // Neither is summoning-sick: both were created before this turn.
        for player in &mut state.players {
            player.turn_start_timestamp = u64::MAX;
        }
        assert!(!can_attack(&state, P0, wall), "a wall attacked");
        assert!(can_attack(&state, P0, bear), "the control could not attack");
    }

    /// An "as though" permission on one creature, as an Aura's static line
    /// lands on the effect table.
    fn permit(state: &mut GameState, creature: ObjectId, modifier: baylee_cards_dsl::Modifier) {
        let timestamp = state.next_timestamp();
        let filter = crate::effects::EffectFilter::object(state, creature);
        state.effects.register(crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: None,
            controller: P0,
            origin: crate::effects::EffectOrigin::Resolution,
            layer: baylee_cards_dsl::Layer::Text,
            timestamp,
            duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
            filter,
            modifier,
        });
    }

    /// "Can attack as though it didn't have defender" (Animate Wall): the
    /// wall attacks. An "as though" applies only to what it states (CR
    /// 609.4), so a "can't attack" beside the defender still holds.
    #[test]
    fn a_wall_that_attacks_as_though_it_had_no_defender_attacks() {
        let mut state = empty_state();
        let wall = creature(&mut state, P0, 0, 4, KeywordSet::DEFENDER);
        let barred = creature(
            &mut state,
            P0,
            0,
            4,
            KeywordSet::DEFENDER.union(KeywordSet::CANT_ATTACK),
        );
        for player in &mut state.players {
            player.turn_start_timestamp = u64::MAX;
        }
        permit(
            &mut state,
            wall,
            baylee_cards_dsl::Modifier::AttacksDespiteDefender,
        );
        permit(
            &mut state,
            barred,
            baylee_cards_dsl::Modifier::AttacksDespiteDefender,
        );
        assert!(can_attack(&state, P0, wall), "the wall may attack");
        assert!(
            !can_attack(&state, P0, barred),
            "a \"can't attack\" is not defender, and still holds"
        );
    }

    /// "Can attack as though it had haste" (Instill Energy): the creature
    /// attacks the turn it arrived, and is still summoning-sick for its {T}
    /// abilities (CR 702.10c is not what the effect states).
    #[test]
    fn a_creature_that_attacks_as_though_it_had_haste_is_still_sick_for_its_tap() {
        let mut state = empty_state();
        let bear = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
        let other = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
        permit(
            &mut state,
            bear,
            baylee_cards_dsl::Modifier::AttacksAsThoughHaste,
        );
        assert!(
            can_attack(&state, P0, bear),
            "it attacks as though it had haste"
        );
        assert!(
            !can_attack(&state, P0, other),
            "the creature beside it is still asleep"
        );
        let obj = state.object(bear).expect("on the battlefield");
        assert!(summoning_sick(&state, obj), "and its tap still waits");
    }

    /// CR 302.6 is a rule about creatures, in both of its sentences. The
    /// answer for anything else is no, and it is no *here* rather than at
    /// each call site — a caller that forgot the type test used to get
    /// "did this permanent enter this turn", which is a different question
    /// with the same shape.
    #[test]
    fn nothing_but_a_creature_is_ever_summoning_sick() {
        let mut state = empty_state();
        let name = state.names.intern("Test Land");
        let land = state.create_bare(P0, ObjectKind::Permanent, name, ZoneLocation::Battlefield);
        state
            .object_mut(land)
            .expect("just created")
            .base_mut()
            .types = TypeSet::LAND;
        let bear = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);

        let land_obj = state.object(land).expect("on the battlefield");
        assert!(
            !summoning_sick(&state, land_obj),
            "a land played this turn was called asleep"
        );
        let bear_obj = state.object(bear).expect("on the battlefield");
        assert!(
            summoning_sick(&state, bear_obj),
            "a creature that entered this turn is asleep"
        );
    }

    /// The stamp a turn records is the *last one issued before it began*,
    /// so the comparison against it is strict. Off by one the other way,
    /// the last permanent to enter before a turn started woke up a turn
    /// late — which nothing noticed, because the draw step almost always
    /// issues a stamp in between.
    #[test]
    fn a_creature_that_was_already_there_when_the_turn_began_is_awake() {
        let mut state = empty_state();
        let bear = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
        // Exactly the boundary: the bear is the last thing stamped before
        // the turn began.
        let began = state.timestamp;
        for player in &mut state.players {
            player.turn_start_timestamp = began;
        }
        let obj = state.object(bear).expect("on the battlefield");
        assert!(!summoning_sick(&state, obj));
    }

    /// "Continuously since *their* most recent turn began" (CR 302.6). A
    /// creature cast on your turn is still summoning sick through every
    /// opponent's turn that follows: one shared turn clock woke it as soon
    /// as anybody untapped, which handed its `{T}` to its controller a
    /// whole turn early. Combat never saw the difference, because you only
    /// declare attackers on your own turn.
    #[test]
    fn an_opponents_turn_does_not_wake_your_creature() {
        let mut state = empty_state();
        let mine = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
        // P1's turn has begun since the creature entered; P0's has not.
        state.players[1].turn_start_timestamp = state.timestamp;
        let obj = state.object(mine).expect("on the battlefield");
        assert!(
            summoning_sick(&state, obj),
            "an opponent untapping woke my creature"
        );

        // P0's own next turn is what wakes it.
        state.players[0].turn_start_timestamp = state.timestamp;
        let obj = state.object(mine).expect("on the battlefield");
        assert!(!summoning_sick(&state, obj));
    }

    /// Combat damage to a planeswalker takes loyalty off it (CR 306.8),
    /// and leaves its controller's life alone.
    #[test]
    fn an_attack_on_a_planeswalker_costs_it_loyalty() {
        let mut state = empty_state();
        let bear = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
        let walker = planeswalker(&mut state, P1, 5);
        attack_walker(&mut state, bear, walker);

        deal_combat_damage(&mut state, false);
        assert_eq!(loyalty(&state, walker), 3, "loyalty did not come off");
        assert_eq!(state.players[1].life, 20, "the player took the damage too");
        assert_eq!(damage(&state, walker), 0, "damage was marked on a walker");
    }

    /// Trample goes to whatever the creature is attacking (CR 702.19b) —
    /// a planeswalker here, not past it to the player.
    #[test]
    fn trample_over_a_blocker_hits_the_planeswalker_being_attacked() {
        let mut state = empty_state();
        let beast = creature(&mut state, P0, 5, 5, KeywordSet::TRAMPLE);
        let chump = creature(&mut state, P1, 1, 1, KeywordSet::EMPTY);
        let walker = planeswalker(&mut state, P1, 6);
        attack_walker(&mut state, beast, walker);
        block(&mut state, chump, beast);

        deal_combat_damage(&mut state, false);
        assert_eq!(damage(&state, chump), 1, "the blocker takes lethal");
        assert_eq!(loyalty(&state, walker), 2, "the rest trampled elsewhere");
        assert_eq!(state.players[1].life, 20, "trample skipped the walker");
    }

    /// CR 506.4c: the attack survives the planeswalker leaving, but the
    /// damage has nowhere to land — least of all on its controller.
    #[test]
    fn a_planeswalker_that_left_absorbs_nothing() {
        let mut state = empty_state();
        let bear = creature(&mut state, P0, 2, 2, KeywordSet::LIFELINK);
        let walker = planeswalker(&mut state, P1, 5);
        attack_walker(&mut state, bear, walker);
        state
            .move_object(
                walker,
                ZoneLocation::Graveyard(P1),
                crate::zone::ZonePosition::Top,
                crate::event::Cause::Effect,
            )
            .expect("the walker leaves");

        let life_before = state.players[0].life;
        deal_combat_damage(&mut state, false);
        assert_eq!(state.players[1].life, 20, "the damage found the player");
        assert_eq!(
            state.players[0].life, life_before,
            "lifelink paid out for damage that was never dealt"
        );
    }

    #[test]
    fn phased_out_creatures_cannot_attack_or_block() {
        let mut state = empty_state();
        let p0 = PlayerId::new(0);
        let p1 = PlayerId::new(1);
        let attacker = creature(&mut state, p0, 2, 2, KeywordSet::HASTE);
        let blocker = creature(&mut state, p1, 2, 2, KeywordSet::default());
        assert!(can_attack(&state, p0, attacker));
        assert!(can_block(&state, p1, blocker, attacker));
        state
            .object_mut(blocker)
            .unwrap()
            .status
            .insert(Status::PHASED_OUT);
        assert!(!can_block(&state, p1, blocker, attacker));
        state
            .object_mut(blocker)
            .unwrap()
            .status
            .remove(Status::PHASED_OUT);
        state
            .object_mut(attacker)
            .unwrap()
            .status
            .insert(Status::PHASED_OUT);
        assert!(!can_attack(&state, p0, attacker));
        assert!(!can_block(&state, p1, blocker, attacker));
    }

    /// A land of `controller`'s with the one land type `subtype`.
    fn land_of_type(state: &mut GameState, controller: PlayerId, subtype: SubtypeId) -> ObjectId {
        let name = state.names.intern("Test Land");
        let id = state.create_bare(
            controller,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        );
        let b = state.object_mut(id).expect("just created").base_mut();
        b.types = TypeSet::LAND;
        b.subtypes = baylee_core::types::SubtypeSet::from_slice(&[subtype]);
        id
    }

    /// Fog (a `ShieldKind::AllCombat` shield) prevents the combat damage
    /// both writers here deal, to a player and to a creature; the old
    /// writers dealt all of it.
    #[test]
    fn combat_damage_meets_the_prevention_shields() {
        let mut state = empty_state();
        let attacker = creature(&mut state, P0, 3, 3, KeywordSet::EMPTY);
        let blocked = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
        let blocker = creature(&mut state, P1, 2, 2, KeywordSet::EMPTY);
        state.combat.declare_attackers([
            AttackerInfo {
                creature: attacker,
                defending: Defender::Player(P1),
                blocked: false,
                band: None,
            },
            AttackerInfo {
                creature: blocked,
                defending: Defender::Player(P1),
                blocked: false,
                band: None,
            },
        ]);
        state.combat.declare_block(blocker, blocked);
        state.shields.push(crate::prevention::Shield {
            protects: crate::prevention::Shielded::Everything,
            kind: crate::prevention::ShieldKind::AllCombat,
            controller: P1,
        });
        deal_combat_damage(&mut state, false);
        assert_eq!(state.players[1].life, 20, "nothing got through");
        assert_eq!(state.object(blocker).unwrap().damage, 0);
        assert_eq!(state.object(blocked).unwrap().damage, 0);
        assert_eq!(state.shields.len(), 1, "Fog is never used up");
    }

    /// Fear (CR 702.36b): an artifact creature or a black one may block,
    /// and nothing else — a green creature may not, a black artifact may.
    #[test]
    fn fear_is_blocked_only_by_artifact_or_black_creatures() {
        let mut state = empty_state();
        let attacker = creature(&mut state, P0, 2, 2, KeywordSet::FEAR);
        let plain = creature(&mut state, P1, 2, 2, KeywordSet::EMPTY);
        let artifact = creature(&mut state, P1, 2, 2, KeywordSet::EMPTY);
        state.object_mut(artifact).unwrap().base_mut().types =
            TypeSet::CREATURE.union(TypeSet::ARTIFACT);
        let black = creature(&mut state, P1, 2, 2, KeywordSet::EMPTY);
        state.object_mut(black).unwrap().base_mut().colors =
            baylee_core::color::ColorSet::of(Color::Black);
        let green = creature(&mut state, P1, 2, 2, KeywordSet::EMPTY);
        state.object_mut(green).unwrap().base_mut().colors =
            baylee_core::color::ColorSet::of(Color::Green);

        assert!(
            !can_block(&state, P1, plain, attacker),
            "colourless, not an artifact"
        );
        assert!(!can_block(&state, P1, green, attacker));
        assert!(can_block(&state, P1, artifact, attacker));
        assert!(can_block(&state, P1, black, attacker));

        // And a creature without fear is blocked as ever.
        let ordinary = creature(&mut state, P0, 2, 2, KeywordSet::EMPTY);
        assert!(can_block(&state, P1, green, ordinary));
    }

    /// Landwalk (CR 702.14c) asks the *defending* player's lands: an Island
    /// a third player controls does not make an islandwalker unblockable,
    /// the defender's own Island does, and so does a land an effect made an
    /// Island (the projected type, not the printed one).
    #[test]
    fn landwalk_reads_the_defending_players_lands_as_they_are() {
        const P2: PlayerId = PlayerId::new(2);
        let mut state = seated(3);
        let walker = creature(&mut state, P0, 2, 2, KeywordSet::ISLANDWALK);
        let blocker = creature(&mut state, P1, 2, 2, KeywordSet::EMPTY);
        attack(&mut state, walker, P1);

        land_of_type(&mut state, P2, land::ISLAND);
        land_of_type(&mut state, P1, land::SWAMP);
        assert!(
            can_block(&state, P1, blocker, walker),
            "a third player's Island and the defender's Swamp change nothing"
        );

        let forest = land_of_type(&mut state, P1, land::FOREST);
        let filter = crate::effects::EffectFilter::object(&state, forest);
        let modifier = baylee_cards_dsl::Modifier::AddSubtype(land::ISLAND);
        state.effects.register(crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: Some(forest),
            controller: P1,
            origin: crate::effects::EffectOrigin::Resolution,
            layer: modifier.layer(),
            timestamp: 1,
            duration: baylee_cards_dsl::Duration::Indefinitely,
            filter,
            modifier,
        });
        state.refresh_characteristics();
        assert!(
            !can_block(&state, P1, blocker, walker),
            "a Forest an effect made an Island is an Island"
        );

        // Each walk names its own type: a swampwalker is blocked here.
        let other = creature(&mut state, P0, 2, 2, KeywordSet::MOUNTAINWALK);
        assert!(can_block(&state, P1, blocker, other));
        for (walk, subtype) in LANDWALKS {
            let mut state = empty_state();
            let walker = creature(&mut state, P0, 2, 2, walk);
            let blocker = creature(&mut state, P1, 2, 2, KeywordSet::EMPTY);
            assert!(can_block(&state, P1, blocker, walker));
            land_of_type(&mut state, P1, subtype);
            assert!(!can_block(&state, P1, blocker, walker));
        }
    }

    #[test]
    fn shadow_restricts_both_sides_and_does_not_bypass_flying() {
        for attacker_shadow in [false, true] {
            for blocker_shadow in [false, true] {
                for flying in [false, true] {
                    let mut state = empty_state();
                    let mut attacker_kw = KeywordSet::EMPTY;
                    if attacker_shadow {
                        attacker_kw = attacker_kw.union(KeywordSet::SHADOW);
                    }
                    if flying {
                        attacker_kw = attacker_kw.union(KeywordSet::FLYING);
                    }
                    let blocker_kw = if blocker_shadow {
                        KeywordSet::SHADOW
                    } else {
                        KeywordSet::EMPTY
                    };
                    let attacker = creature(&mut state, P0, 3, 2, attacker_kw);
                    let blocker = creature(&mut state, P1, 2, 2, blocker_kw);
                    assert_eq!(
                        can_block(&state, P1, blocker, attacker),
                        attacker_shadow == blocker_shadow && !flying
                    );
                    let reach = creature(&mut state, P1, 2, 2, blocker_kw.union(KeywordSet::REACH));
                    assert_eq!(
                        can_block(&state, P1, reach, attacker),
                        attacker_shadow == blocker_shadow
                    );
                }
            }
        }
    }

    #[test]
    fn phased_out_planeswalkers_are_not_defenders() {
        let mut state = empty_state();
        let p0 = PlayerId::new(0);
        let p1 = PlayerId::new(1);
        let walker = planeswalker(&mut state, p1, 3);
        let defender = Defender::Planeswalker(walker);
        assert!(defender_options(&state, p0).contains(&defender));
        assert_eq!(defending_player(&state, defender), Some(p1));
        state
            .object_mut(walker)
            .unwrap()
            .status
            .insert(Status::PHASED_OUT);
        assert_eq!(defender_options(&state, p0), vec![Defender::Player(p1)]);
        assert_eq!(defending_player(&state, defender), None);
        state
            .object_mut(walker)
            .unwrap()
            .status
            .remove(Status::PHASED_OUT);
        assert!(defender_options(&state, p0).contains(&defender));
    }

    fn phase_out(state: &mut GameState, id: ObjectId) {
        let mut res = crate::resolve::Resolution {
            source: id,
            on_stack: id,
            controller: state.object(id).unwrap().controller,
            effects: vec![baylee_cards_dsl::Effect::PhaseOut { target: None }],
            pc: 0,
            targets: smallvec::SmallVec::new(),
            second_targets: smallvec::SmallVec::new(),
            x: None,
            chosen_player: None,
            target_players: baylee_core::ids::SeatSet::default(),
            event_object: None,
            awaiting: None,
            targeted: false,
            mana_ability: false,
            countered_source: None,
            target_lki: None,
            subject: crate::resolve::SubjectContext::default(),
            text: crate::text_changes::TextChangeMap::IDENTITY,
            event_mana: None,
            retarget_left: None,
        };
        let _ = crate::resolve::run(state, &mut res);
        assert!(
            state
                .object(id)
                .unwrap()
                .status
                .contains(Status::PHASED_OUT)
        );
    }

    #[test]
    fn phasing_out_an_attacker_removes_it_from_combat() {
        let mut state = empty_state();
        let attacker = creature(&mut state, PlayerId::new(0), 4, 4, KeywordSet::default());
        attack(&mut state, attacker, PlayerId::new(1));
        phase_out(&mut state, attacker);
        assert!(state.combat.attackers().is_empty());
        deal_combat_damage(&mut state, false);
        assert_eq!(state.players[1].life, 20);
        assert!(
            on_battlefield(&state, attacker),
            "phasing is not a zone change"
        );
    }

    #[test]
    fn phasing_out_a_blocker_keeps_the_attacker_blocked() {
        for keywords in [KeywordSet::default(), KeywordSet::TRAMPLE] {
            let mut state = empty_state();
            let attacker = creature(&mut state, PlayerId::new(0), 4, 4, keywords);
            let blocker = creature(&mut state, PlayerId::new(1), 2, 2, KeywordSet::default());
            attack(&mut state, attacker, PlayerId::new(1));
            block(&mut state, blocker, attacker);
            phase_out(&mut state, blocker);
            assert!(state.combat.blockers.is_empty());
            assert!(state.combat.is_blocked(attacker));
            deal_combat_damage(&mut state, false);
            assert_eq!(damage(&state, attacker), 0);
            assert_eq!(damage(&state, blocker), 0);
            assert_eq!(
                state.players[1].life,
                if keywords.contains(KeywordSet::TRAMPLE) {
                    16
                } else {
                    20
                }
            );
        }
    }

    #[test]
    fn phasing_out_an_attacked_planeswalker_prevents_damage_and_lifelink() {
        let mut state = empty_state();
        let attacker = creature(&mut state, PlayerId::new(0), 4, 4, KeywordSet::LIFELINK);
        let walker = planeswalker(&mut state, PlayerId::new(1), 5);
        attack_walker(&mut state, attacker, walker);
        phase_out(&mut state, walker);
        deal_combat_damage(&mut state, false);
        assert_eq!(loyalty(&state, walker), 5);
        assert_eq!(state.players[0].life, 20);
        assert_eq!(state.players[1].life, 20);
    }
}
