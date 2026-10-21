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
//! and its loyalty comes off (CR 306.8); battles use their protector and defense.
//!
//! Not yet: an attacker with trample blocked by creatures without banding
//! is not asked how to assign its damage (CR 702.19b). The engine assigns
//! lethal damage to each blocker in declaration order and the rest to what
//! it attacks, which is one of the assignments its controller could have
//! chosen.

use crate::event::DamageTarget;

mod defenders;
use crate::object::{GameObject, Status};
pub(crate) use defenders::{
    active_damage_recipient, historical_defending_player, recheck_defenders, remember_attack,
};

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
    /// Per exact attacker: its sole historical blocker, or None once different
    /// incarnations have blocked it. Idempotent repeated blocks need no extra
    /// storage. Leaving combat does not erase history; combat end clears it.
    block_history: std::collections::BTreeMap<
        baylee_core::ids::DamageSourceRef,
        Option<baylee_core::ids::DamageSourceRef>,
    >,
    /// "That creature can't be blocked this combat except by creatures with
    /// flying and creatures in a pile with the chosen label" (Raging River):
    /// for an attacker, the creatures its chosen pile holds. Several limits
    /// on one attacker all bind (CR 509.1b). Cleared with the combat.
    pile_limits: Vec<(ObjectId, Vec<ObjectId>)>,
    /// The divisions players have chosen for the damage step about to be
    /// dealt, emptied once it is (`deal_combat_damage`).
    divisions: Vec<Division>,
    /// What each creature that attacked this combat attacked, as the object
    /// it was then, kept after it is removed from combat: "the player that
    /// creature was attacking before it was removed from combat" (CR 508.5,
    /// [`Self::remembered_defender`]). Written by
    /// [`Self::remember_defender`] for declared attackers and creatures put
    /// onto the battlefield attacking; the latest entry for a creature
    /// wins. Cleared with the combat.
    defended: Vec<(ObjectId, u32, Defender)>,
    defender_contexts: std::collections::BTreeMap<ObjectId, defenders::DefenderContext>,
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
        self.defended.hash(state);
        self.defender_contexts.hash(state);
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

    /// Notes that `id`, as the object it is at `version`, attacks
    /// `defender` this combat (CR 508.5).
    pub fn remember_defender(&mut self, id: ObjectId, version: u32, defender: Defender) {
        self.defended.retain(|(c, _, _)| *c != id);
        self.defended.push((id, version, defender));
    }

    /// What `id` attacked this combat, and as which version of itself,
    /// whether or not it is still attacking (`crate::eval::last_defender`
    /// decides whether that is still the same creature).
    #[must_use]
    pub fn remembered_defender(&self, id: ObjectId) -> Option<(u32, Defender)> {
        self.defended
            .iter()
            .find(|(c, _, _)| *c == id)
            .map(|(_, v, d)| (*v, *d))
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

    /// Number of current blockers, without allocating the list of their ids.
    #[must_use]
    pub fn blocker_count(&self, attacker: ObjectId) -> usize {
        self.blockers
            .iter()
            .filter(|b| b.attacker == attacker)
            .count()
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
    pub fn declare_block(
        &mut self,
        blocker: baylee_core::ids::DamageSourceRef,
        attacker: baylee_core::ids::DamageSourceRef,
    ) {
        self.block_history
            .entry(attacker)
            .and_modify(|sole| {
                if *sole != Some(blocker) {
                    *sole = None;
                }
            })
            .or_insert(Some(blocker));
        let (blocker, attacker) = (blocker.object, attacker.object);
        for info in &mut self.attackers {
            if info.creature == attacker {
                info.blocked = true;
            }
        }
        self.blockers.push(BlockerInfo { blocker, attacker });
    }

    /// Whether every block `attacker` has met this combat was `blocker`'s:
    /// "had become blocked by only that creature this combat" (False
    /// Orders). A blocker that came and went still counts.
    #[must_use]
    pub fn blocked_only_by(
        &self,
        attacker: baylee_core::ids::DamageSourceRef,
        blocker: baylee_core::ids::DamageSourceRef,
    ) -> bool {
        self.sole_blocker(attacker) == Some(blocker)
    }

    /// One exact blocker, only if every historical block used that incarnation.
    pub(crate) fn sole_blocker(
        &self,
        attacker: baylee_core::ids::DamageSourceRef,
    ) -> Option<baylee_core::ids::DamageSourceRef> {
        self.block_history.get(&attacker).copied().flatten()
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
    idle_defenders: Vec<&'a crate::effects::ContinuousEffect>,
    cant_attack_you: Vec<&'a crate::effects::ContinuousEffect>,
    attack_tax_life: Vec<&'a crate::effects::ContinuousEffect>,
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
        let mut idle_defenders = Vec::new();
        let mut cant_attack_you = Vec::new();
        let mut attack_tax_life = Vec::new();
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
                Modifier::CantAttackYou => cant_attack_you.push(fx),
                Modifier::AttackTaxLife { .. } => attack_tax_life.push(fx),
                Modifier::CantAttackUnlessPayMana(_) => unless_pay_mana.push(fx),
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
            idle_defenders,
            cant_attack_you,
            attack_tax_life,
            unless_pay_mana,
            unless_sacrifice,
            max_attackers,
            mana_reach: 0,
        }
    }

    /// Whether any creature's attack costs mana now (CR 508.1h: Brainwash),
    /// so that asking what mana its controller could make is worth it.
    #[must_use]
    pub fn has_mana_tax(&self) -> bool {
        !self.unless_pay_mana.is_empty()
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
                let baylee_cards_dsl::Modifier::CantAttackUnlessPayMana(n) = fx.modifier else {
                    return None;
                };
                crate::effects::applies_to(self.state, fx, obj).then_some(n)
            })
            .fold(0_u32, u32::saturating_add)
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
            && self.unless_pay_mana.is_empty()
            && self.unless_sacrifice.is_empty()
        {
            return true;
        }
        let state = self.state;
        let Some(obj) = state.object(creature) else {
            return false;
        };
        let mana_tax = self.mana_tax(creature);
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

/// Whether `b` could block anything at all for `defending`: the half of
/// [`can_block`] that asks only about the blocker.
fn ready_to_block(b: &GameObject, defending: PlayerId) -> bool {
    b.zone == crate::zone::Zone::Battlefield
        && b.controller == defending
        && b.characteristics().types.contains(TypeSet::CREATURE)
        && !b.characteristics().types.contains(TypeSet::BATTLE)
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

/// Landwalk is read against this defender's lands; an as-though permission
/// ignores only its named walks while deciding whether the creature can block.
fn basic_landwalk_prevents_blocking(
    state: &GameState,
    attacker: &GameObject,
    defending: PlayerId,
) -> bool {
    let keywords = attacker.characteristics().keywords;
    let mut walks = LANDWALKS.iter().fold(K::EMPTY, |all, (walk, land)| {
        if keywords.contains(*walk) && controls_land_of_type(state, defending, *land) {
            all.union(*walk)
        } else {
            all
        }
    });
    if walks.is_empty() {
        return false;
    }
    for effect in state.effects.iter() {
        if let baylee_cards_dsl::Modifier::BlockableWithoutLandwalk(ignored) = effect.modifier
            && crate::effects::applies_to(state, effect, attacker)
        {
            walks = walks.difference(ignored);
        }
    }
    !walks.is_empty()
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
        .is_some_and(|_| historical_defending_player(state, attacker) != Some(defending))
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
    // Intimidate (CR 702.13b): any shared color works; colorless is not
    // a color and therefore only an artifact creature can block it.
    if kw(a, K::INTIMIDATE) {
        let blocker = b.characteristics();
        if !blocker.types.contains(TypeSet::ARTIFACT)
            && !blocker.colors.intersects(a.characteristics().colors)
        {
            return false;
        }
    }
    // Landwalk (CR 702.14c): unblockable while the *defending* player
    // controls a land of that type — that player's lands and nobody
    // else's, read as they are now (a land an effect made an Island is
    // one).
    if basic_landwalk_prevents_blocking(state, a, defending) {
        return false;
    }
    // The rest of the family (desertwalk, CR 702.14c): a land the effect's
    // filter describes, among the defending player's.
    if state.effects.iter().any(|fx| {
        let baylee_cards_dsl::Modifier::LandwalkMatching(filter) = fx.modifier else {
            return false;
        };
        crate::effects::applies_to(state, fx, a)
            && !landwalk_removed(state, a, fx.timestamp)
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
        let (filter, affected, other, source) = match fx.modifier {
            // The Black Gate: a player, bound as its ability resolved.
            baylee_cards_dsl::Modifier::CantBeBlockedByPlayer(player) => {
                return b.controller == player && crate::effects::applies_to(state, fx, a);
            }
            baylee_cards_dsl::Modifier::CantBeBlockedBy(filter) => (filter, a, b, attacker),
            baylee_cards_dsl::Modifier::CantBlock(filter) => (filter, b, a, blocker),
            _ => return false,
        };
        crate::effects::applies_to(state, fx, affected)
            && crate::eval::matches(
                filter,
                state,
                other,
                fx.controller,
                fx.source.unwrap_or(source),
            )
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
    max_blockers: Option<usize>,
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
            max_blockers: None,
        };
        for fx in state.effects.iter() {
            match fx.modifier {
                Modifier::CanBlockAdditional(_) => rules.additional.push(fx),
                Modifier::CanBlockAnyNumber => rules.any_number.push(fx),
                Modifier::MustBeBlockedByAllAble | Modifier::MustBeBlockedByAllAbleMatching(_) => {
                    rules.lures.push(fx);
                }
                Modifier::BlocksEachAttackerIfAble => rules.each.push(fx),
                Modifier::MaxBlockersPerCombat(n)
                    if crate::effects::source_is_present(state, fx) =>
                {
                    let cap = usize::from(n);
                    rules.max_blockers = Some(rules.max_blockers.map_or(cap, |cur| cur.min(cap)));
                }
                _ => {}
            }
        }
        rules
    }

    /// Maximum number of creatures that may block this combat, if capped.
    #[must_use]
    pub const fn max_blockers(&self) -> Option<usize> {
        self.max_blockers
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
                    // Marble Priest's lure binds only the blockers its
                    // filter names (Walls), read with the effect's
                    // controller as "you".
                    .filter(|fx| match fx.modifier {
                        baylee_cards_dsl::Modifier::MustBeBlockedByAllAbleMatching(filter) => {
                            state.object(blocker).is_some_and(|b| {
                                crate::eval::matches(
                                    filter,
                                    state,
                                    b,
                                    fx.controller,
                                    fx.source.unwrap_or(attacker),
                                )
                            })
                        }
                        _ => true,
                    })
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
        let mut distinct_blockers: std::collections::BTreeSet<ObjectId> = self
            .state
            .combat
            .blockers
            .iter()
            .map(|b| b.blocker)
            .collect();
        let can_add_blocker =
            |set: &std::collections::BTreeSet<ObjectId>, b: ObjectId| match self.max_blockers {
                Some(max) => set.contains(&b) || set.len() < max,
                None => true,
            };
        let mut chosen: Vec<(ObjectId, ObjectId)> = Vec::new();
        for option in options {
            if !can_add_blocker(&distinct_blockers, option.blocker) {
                continue;
            }
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
            let before = chosen.len();
            chosen.extend(
                asked
                    .into_iter()
                    .take(room(option.blocker))
                    .map(|(_, _, attacker)| (option.blocker, attacker)),
            );
            if chosen.len() > before {
                distinct_blockers.insert(option.blocker);
            }
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
                    && can_add_blocker(&distinct_blockers, o.blocker)
                    && o.attackers.contains(&attacker)
                    && chosen.iter().filter(|(b, _)| *b == o.blocker).count() < room(o.blocker)
            });
            match helper {
                Some(helper) => {
                    distinct_blockers.insert(helper.blocker);
                    chosen.push((helper.blocker, attacker));
                }
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
            assign_attacker_damage(state, attacker.creature, attacker.blocked, &mut assignments);
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

/// Whether a creature has banding or "bands with other [quality]" paired with
/// a matching creature in `group` as damage is assigned (CR 702.22j–k).
fn has_banding_in_group(state: &GameState, id: ObjectId, group: &[ObjectId]) -> bool {
    if has_keyword(state, id, K::BANDING) {
        return true;
    }
    let filters = bands_with_other_filters(state, id);
    if filters.is_empty() {
        return false;
    }
    group.iter().any(|&other| {
        other != id
            && state.object(other).is_some_and(|obj| {
                filters
                    .iter()
                    .any(|f| crate::eval::matches(f, state, obj, obj.controller, id))
            })
    })
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
        if !info.blocked
            || !strikes_now(state, info.creature, first_strike_step)
            || assigns_no_combat_damage(state, info.creature)
        {
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
            .find(|b| has_banding_in_group(state, **b, &blockers))
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
        if !strikes_now(state, blocker, first_strike_step)
            || assigns_no_combat_damage(state, blocker)
        {
            continue;
        }
        let amount = power_of(state, blocker);
        let blocked = live_blocked(state, blocker);
        if amount <= 0 || blocked.len() < 2 || state.combat.division(blocker).is_some() {
            continue;
        }
        let chooser = if blocked
            .iter()
            .any(|a| has_banding_in_group(state, *a, &blocked))
        {
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
        if strikes_now(state, info.creature, first_strike_step)
            && !assigns_no_combat_damage(state, info.creature)
        {
            assign_attacker_damage(state, info.creature, info.blocked, &mut assignments);
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

/// "This creature assigns no combat damage this turn" (Floral Spuzzem; the
/// card overrides CR 510.1a's "assigns combat damage equal to its power",
/// CR 101.1): it is left out of the step's assignments altogether, so there
/// is nothing to deal, prevent or redirect.
fn assigns_no_combat_damage(state: &GameState, creature: ObjectId) -> bool {
    state.object(creature).is_some_and(|obj| {
        state.effects.iter().any(|fx| {
            fx.modifier == baylee_cards_dsl::Modifier::AssignsNoCombatDamage
                && crate::effects::applies_to(state, fx, obj)
        })
    })
}

/// Assign every source before prevention can change a creature's power.
fn assign_attacker_damage(
    state: &GameState,
    attacker: ObjectId,
    blocked: bool,
    out: &mut Vec<crate::damage::Assignment>,
) {
    let power = power_of(state, attacker);
    if power <= 0 {
        return;
    }
    let trample = has_keyword(state, attacker, K::TRAMPLE);
    let live = live_blockers(state, attacker);
    if blocked && (!trample || live.iter().any(|b| has_banding_in_group(state, *b, &live))) {
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
        assign_defender(state, out, attacker, remaining);
    } else {
        assign_defender(state, out, attacker, power);
    }
}

fn assign_defender(
    state: &GameState,
    out: &mut Vec<crate::damage::Assignment>,
    source: ObjectId,
    amount: i16,
) {
    let Some(to) = active_damage_recipient(state, source) else {
        return;
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

impl GameState {
    pub(crate) fn blocked_only_by(&self, attacker: ObjectId, blocker: ObjectId) -> bool {
        self.source_identity(attacker)
            .zip(self.source_identity(blocker))
            .is_some_and(|(attacker, blocker)| self.combat.blocked_only_by(attacker, blocker))
    }

    /// Records a block pair and the first blocked transition separately.
    pub(crate) fn declare_block(&mut self, blocker: ObjectId, attacker: ObjectId) {
        let became_blocked = !self.combat.is_blocked(attacker);
        let (Some(blocker_ref), Some(attacker_ref)) = (
            self.source_identity(blocker),
            self.source_identity(attacker),
        ) else {
            return;
        };
        self.combat.declare_block(blocker_ref, attacker_ref);
        if let Some(controller) = self.object(attacker).map(|o| o.controller) {
            self.per_turn.blocks.push(crate::state::BlockRecord {
                blocker: blocker_ref,
                attacker: attacker_ref,
                attacker_controller: controller,
            });
        }
        self.journal.record(crate::event::GameEvent::BecameBlocker {
            object: blocker,
            attacker,
        });
        if became_blocked {
            self.journal
                .record(crate::event::GameEvent::AttackerBlocked { object: attacker });
        }
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

/// A resolved removal suppresses older nonbasic landwalk grants, never later ones.
fn landwalk_removed(state: &GameState, object: &crate::object::GameObject, timestamp: u64) -> bool {
    state.effects.iter().any(|removal| {
        matches!(
            removal.modifier,
            baylee_cards_dsl::Modifier::LoseAllLandwalk
        ) && removal.timestamp >= timestamp
            && crate::effects::applies_to(state, removal, object)
    })
}

/// A resolved removal suppresses older "bands with other" abilities, never later ones.
fn bands_with_other_removed(
    state: &GameState,
    object: &crate::object::GameObject,
    timestamp: u64,
) -> bool {
    object.characteristics().abilities_lost.is_some()
        || state.effects.iter().any(|removal| {
            matches!(
                removal.modifier,
                baylee_cards_dsl::Modifier::LoseAllBandsWithOther
                    | baylee_cards_dsl::Modifier::LoseAllAbilities
            ) && removal.timestamp >= timestamp
                && crate::effects::applies_to(state, removal, object)
        })
}

/// Active "bands with other [quality]" filters on `id` (CR 702.22c).
pub(crate) fn bands_with_other_filters(
    state: &GameState,
    id: ObjectId,
) -> Vec<&'static baylee_cards_dsl::Filter> {
    let Some(object) = state.object(id) else {
        return Vec::new();
    };
    state
        .effects
        .iter()
        .filter_map(|fx| {
            let baylee_cards_dsl::Modifier::BandsWithOther(filter) = fx.modifier else {
                return None;
            };
            (crate::effects::applies_to(state, fx, object)
                && !bands_with_other_removed(state, object, fx.timestamp))
            .then_some(filter)
        })
        .collect()
}

/// Whether `leader` has an active "bands with other [quality]" ability that
/// matches `mate` (CR 702.22c).
pub(crate) fn can_band_with_other(state: &GameState, leader: ObjectId, mate: ObjectId) -> bool {
    if leader == mate {
        return false;
    }
    let Some(mate_obj) = state.object(mate) else {
        return false;
    };
    let filters = bands_with_other_filters(state, leader);
    filters
        .into_iter()
        .any(|filter| crate::eval::matches(filter, state, mate_obj, mate_obj.controller, leader))
}

/// Whether declaring this creature as an attacker taps it.
#[must_use]
pub fn attack_taps(state: &GameState, creature: ObjectId) -> bool {
    let Some(object) = state.object(creature) else {
        return false;
    };
    if object
        .characteristics()
        .keywords
        .contains(baylee_cards_dsl::KeywordSet::VIGILANCE)
    {
        return false;
    }
    if !state.effects.may_change_attack_tapping() {
        return true;
    }
    !state.effects.iter().any(|effect| {
        let baylee_cards_dsl::Modifier::AttackWithoutTappingWhileSourceUntapped { source_version } =
            effect.modifier
        else {
            return false;
        };
        effect.controller == object.controller
            && effect
                .source
                .and_then(|id| state.object(id))
                .is_some_and(|source| {
                    source.version == source_version
                        && source.zone == crate::zone::Zone::Battlefield
                        && !source.status.contains(crate::object::Status::TAPPED)
                        && !source.status.contains(crate::object::Status::PHASED_OUT)
                })
    })
}

#[cfg(test)]
mod tests;
