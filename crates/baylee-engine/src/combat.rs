//! Structured combat state machine.
//!
//! Implemented: attacker/blocker declaration with the keyword restrictions
//! (flying/reach, menace, unblockable, protection), first/double strike as
//! a per-creature property, deathtouch, trample, lifelink, and damage
//! assignment in declaration order.
//!
//! Not yet: the attacking player's *choice* of damage assignment order
//! among multiple blockers (CR 509.2) — the declaration order stands in
//! for it — and attacking planeswalkers or battles, which need
//! [`AttackerInfo::defending`] to become a defender handle rather than a
//! [`PlayerId`].

use crate::event::{DamageTarget, GameEvent};
use crate::object::{GameObject, Status};
use crate::state::GameState;
use baylee_cards_dsl::KeywordSet as K;
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::types::TypeSet;

/// One declared attacker.
#[derive(Clone, Copy, Debug)]
pub struct AttackerInfo {
    /// The attacking creature.
    pub creature: ObjectId,
    /// The player it attacks.
    pub defending: PlayerId,
}

/// One declared blocker.
#[derive(Clone, Copy, Debug)]
pub struct BlockerInfo {
    /// The blocking creature.
    pub blocker: ObjectId,
    /// The attacker it blocks.
    pub attacker: ObjectId,
}

/// The combat phase's mutable state.
#[derive(Clone, Debug, Default)]
pub struct CombatState {
    /// Declared attackers.
    pub attackers: Vec<AttackerInfo>,
    /// Declared blockers.
    pub blockers: Vec<BlockerInfo>,
}

impl CombatState {
    /// Whether combat is underway.
    #[must_use]
    pub fn is_active(&self) -> bool {
        !self.attackers.is_empty()
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

    /// Whether a creature is currently blocked.
    #[must_use]
    pub fn is_blocked(&self, attacker: ObjectId) -> bool {
        self.blockers.iter().any(|b| b.attacker == attacker)
    }
}

/// Whether `creature` may attack at all (untapped, a creature, not
/// summoning-sick).
#[must_use]
pub fn can_attack(state: &GameState, player: PlayerId, creature: ObjectId) -> bool {
    let Some(obj) = state.object(creature) else {
        return false;
    };
    obj.zone == crate::zone::Zone::Battlefield
        && obj.controller == player
        && obj.characteristics().types.contains(TypeSet::CREATURE)
        && !obj.status.contains(Status::TAPPED)
        && !summoning_sick(state, obj)
}

/// Summoning sickness (CR 302.6): a creature must be controlled
/// continuously since the beginning of its controller's most recent turn
/// (haste excepted).
#[must_use]
pub fn summoning_sick(state: &GameState, obj: &GameObject) -> bool {
    if obj
        .characteristics()
        .keywords
        .contains(baylee_cards_dsl::KeywordSet::HASTE)
    {
        return false;
    }
    obj.timestamp >= state.turn_start_timestamp
}

/// Whether `blocker` may block `attacker` (keyword restrictions included).
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
    if b.zone != crate::zone::Zone::Battlefield
        || b.controller != defending
        || !b.characteristics().types.contains(TypeSet::CREATURE)
        || b.status.contains(Status::TAPPED)
    {
        return false;
    }
    let kw =
        |o: &GameObject, k: baylee_cards_dsl::KeywordSet| o.characteristics().keywords.contains(k);
    // Flying can only be blocked by flying/reach (CR 702.9).
    if kw(a, K::FLYING) && !kw(b, K::FLYING) && !kw(b, K::REACH) {
        return false;
    }
    // Menace requires two or more blockers (checked at declaration: a
    // single blocker is never legal).
    if kw(a, K::MENACE) {
        let blockers = state.combat.blockers_of(attacker).len();
        if blockers == 0 {
            return false;
        }
    }
    // Unblockable.
    if kw(a, K::UNBLOCKABLE) {
        return false;
    }
    // Protection (CR 702.16d): can't be blocked by matching creatures.
    if crate::eval::protected_from(state, attacker, blocker) {
        return false;
    }
    true
}

/// Whether a creature deals its combat damage in the given step
/// (CR 510.4): first strikers in the first step, everyone else in the
/// regular one, double strikers in both.
fn strikes_now(state: &GameState, creature: ObjectId, first_strike_step: bool) -> bool {
    let Some(obj) = state.object(creature) else {
        return false;
    };
    let kw = obj.characteristics().keywords;
    if first_strike_step {
        kw.contains(K::FIRST_STRIKE) || kw.contains(K::DOUBLE_STRIKE)
    } else {
        !kw.contains(K::FIRST_STRIKE) || kw.contains(K::DOUBLE_STRIKE)
    }
}

/// How much damage from `source` is lethal to `target` right now
/// (CR 510.1c): toughness minus damage already marked, or 1 if the source
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
pub fn deal_combat_damage(state: &mut GameState, first_strike_step: bool) {
    let attackers = state.combat.attackers.clone();
    for info in &attackers {
        if !strikes_now(state, info.creature, first_strike_step) {
            continue;
        }
        assign_attacker_damage(state, info.creature, info.defending);
    }
    let blockers = state.combat.blockers.clone();
    for info in &blockers {
        if !strikes_now(state, info.blocker, first_strike_step) {
            continue;
        }
        let power = power_of(state, info.blocker);
        if power <= 0 {
            continue;
        }
        deal_damage_to_object(state, info.blocker, info.attacker, power, true);
        if has_keyword(state, info.blocker, K::LIFELINK)
            && let Some(controller) = state.object(info.blocker).map(|o| o.controller)
        {
            gain_life(state, controller, power);
        }
    }
}

/// One attacker's damage assignment (CR 510.1a–c).
fn assign_attacker_damage(state: &mut GameState, attacker: ObjectId, defending: PlayerId) {
    let power = power_of(state, attacker);
    let trample = has_keyword(state, attacker, K::TRAMPLE);
    let lifelink = has_keyword(state, attacker, K::LIFELINK);
    let Some(controller) = state.object(attacker).map(|o| o.controller) else {
        return;
    };
    let mut lifelinked = 0i16;
    let blockers = state.combat.blockers_of(attacker);
    // CR 509.1h: an attacker with no blockers left is still *blocked*, so
    // it deals no damage to the player — unless it has trample, which
    // assigns everything past the (now absent) blockers to the defender.
    let live: Vec<ObjectId> = blockers
        .iter()
        .copied()
        .filter(|b| state.object(*b).is_some())
        .collect();
    if blockers.is_empty() {
        deal_damage_to_player(state, attacker, defending, power, true);
        lifelinked += power;
    } else {
        let mut remaining = power;
        for blocker in &live {
            if remaining <= 0 {
                break;
            }
            // Only trample lets an attacker hold damage back; without it
            // the whole assignment goes to the blocker in front of it.
            let assigned = if trample {
                remaining.min(lethal_damage(state, attacker, *blocker))
            } else {
                remaining
            };
            deal_damage_to_object(state, attacker, *blocker, assigned, true);
            remaining -= assigned;
            lifelinked += assigned;
        }
        if trample && remaining > 0 {
            deal_damage_to_player(state, attacker, defending, remaining, true);
            lifelinked += remaining;
        }
    }
    if lifelink {
        gain_life(state, controller, lifelinked);
    }
}

fn deal_damage_to_player(
    state: &mut GameState,
    source: ObjectId,
    player: PlayerId,
    amount: i16,
    is_combat: bool,
) {
    if amount <= 0 {
        return;
    }
    if prevent_from(state, source) {
        return;
    }
    let p = &mut state.players[player.get() as usize];
    let old = p.life;
    p.life -= i32::from(amount);
    let new = p.life;
    state.journal.record(GameEvent::LifeChanged {
        player,
        old,
        new,
        cause: crate::event::Cause::Spell,
    });
    state.journal.record(GameEvent::DamageDealt {
        source: Some(source),
        target: DamageTarget::Player(player),
        amount: amount as u16,
        is_combat,
    });
}

fn deal_damage_to_object(
    state: &mut GameState,
    source: ObjectId,
    target: ObjectId,
    amount: i16,
    is_combat: bool,
) {
    if amount <= 0 {
        return;
    }
    if prevent_from(state, source)
        || prevent_to(state, target)
        || crate::eval::protected_from(state, target, source)
    {
        return;
    }
    let deathtouch = has_keyword(state, source, K::DEATHTOUCH);
    if let Some(obj) = state.object_mut(target) {
        obj.damage = obj.damage.saturating_add(amount as u16);
        obj.deathtouched |= deathtouch;
    }
    state.journal.record(GameEvent::DamageDealt {
        source: Some(source),
        target: DamageTarget::Object(target),
        amount: amount as u16,
        is_combat,
    });
}

/// True if the source object may not deal damage (`PreventDamageFromIt`).
fn prevent_from(state: &GameState, source: ObjectId) -> bool {
    state.effects.iter().any(|fx| {
        matches!(fx.modifier, baylee_cards_dsl::Modifier::PreventDamageFromIt)
            && matches!(&fx.filter, crate::effects::EffectFilter::ObjectIs(id) if *id == source)
    })
}

/// True if the target object may not be dealt damage (`PreventDamageToIt`).
fn prevent_to(state: &GameState, target: ObjectId) -> bool {
    state.effects.iter().any(|fx| {
        matches!(fx.modifier, baylee_cards_dsl::Modifier::PreventDamageToIt)
            && matches!(&fx.filter, crate::effects::EffectFilter::ObjectIs(id) if *id == target)
    })
}

fn gain_life(state: &mut GameState, player: PlayerId, amount: i16) {
    if amount <= 0 {
        return;
    }
    let p = &mut state.players[player.get() as usize];
    let old = p.life;
    p.life += i32::from(amount);
    let new = p.life;
    state.journal.record(GameEvent::LifeChanged {
        player,
        old,
        new,
        cause: crate::event::Cause::Spell,
    });
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
        let seat = || SeatSpec {
            controller: SeatController::Open,
            deck: vec![],
            sideboard: vec![],
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
                dev_mode: false,
                house_rules: HouseRules::default(),
                modifiers: vec![],
                prints: vec![],
                seats: vec![seat(), seat()],
            },
            &NoCards,
        )
        .expect("an empty two-seat board")
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
        let obj = state.object_mut(id).expect("just created");
        obj.base.types = TypeSet::CREATURE;
        obj.base.power = Some(power);
        obj.base.toughness = Some(toughness);
        obj.base.keywords = keywords;
        id
    }

    fn attack(state: &mut GameState, creature: ObjectId, defending: PlayerId) {
        state
            .combat
            .attackers
            .push(AttackerInfo { creature, defending });
    }

    fn block(state: &mut GameState, blocker: ObjectId, attacker: ObjectId) {
        state.combat.blockers.push(BlockerInfo { blocker, attacker });
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

        crate::sba::run(&mut state);
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
        let wall = creature(
            &mut state,
            P1,
            0,
            4,
            KeywordSet::INDESTRUCTIBLE,
        );
        attack(&mut state, biter, P1);
        block(&mut state, wall, biter);
        deal_combat_damage(&mut state, false);

        crate::sba::run(&mut state);
        assert!(
            on_battlefield(&state, wall),
            "indestructible survives deathtouch (CR 702.12b)"
        );
        assert!(!state.object(wall).expect("alive").deathtouched);

        // Losing indestructibility later must not make it die retroactively.
        state
            .object_mut(wall)
            .expect("alive")
            .base
            .keywords = KeywordSet::EMPTY;
        crate::sba::run(&mut state);
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
        crate::sba::run(&mut state);
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
        assert_eq!(
            damage(&state, small),
            5,
            "both blockers deal damage: 2 + 3"
        );
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
}
