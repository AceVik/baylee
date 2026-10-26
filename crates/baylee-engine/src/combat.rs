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

mod attack;
mod block;
mod damage;
mod defenders;
mod exchange;
use crate::object::Status;
pub(crate) use defenders::{
    active_damage_recipient, historical_defending_player, recheck_defenders, remember_attack,
};
pub(crate) use exchange::exchange_blockers;

use crate::state::GameState;
use baylee_core::ids::{Defender, ObjectId, PlayerId};
use baylee_core::types::TypeSet;

pub use attack::{
    AttackRules, blocking_player, can_attack, defender_options, defending_player, summoning_sick,
};
pub use block::{
    BlockRules, block_bound, block_options, can_block, controls_land_of_type, menace_satisfiable,
    ready_blockers,
};
pub use damage::divisions_owed;
pub(crate) use damage::{collect_combat_damage, prepare_damage_step, unpreventable};

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

/// A resolved removal suppresses older nonbasic landwalk grants, never later ones.
fn landwalk_removed(state: &GameState, object: &crate::object::GameObject, timestamp: u64) -> bool {
    state.effects.iter().any(|removal| {
        matches!(
            removal.modifier,
            baylee_cards_dsl::Modifier::LoseAllLandwalk
                | baylee_cards_dsl::Modifier::LoseAllAbilities
                | baylee_cards_dsl::Modifier::LoseKeywords
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
