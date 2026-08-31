//! Structured combat state machine.
//!
//! Implemented: attacker/blocker declaration with the keyword restrictions
//! (flying/reach, menace, unblockable, protection), first/double strike as
//! a per-creature property, deathtouch, trample, lifelink, and damage
//! assignment in declaration order.
//!
//! Attacks are aimed at a [`Defender`], so a planeswalker can be attacked
//! and its loyalty comes off (CR 306.8). Battles are the remaining case.
//!
//! Not yet: the attacking player's *choice* of damage assignment order
//! among multiple blockers (CR 509.2) — the declaration order stands in
//! for it.

use crate::event::{DamageTarget, GameEvent};
use crate::object::{GameObject, Status};
use crate::state::GameState;
use baylee_cards_dsl::KeywordSet as K;
use baylee_core::ids::{Defender, ObjectId, PlayerId};
use baylee_core::types::TypeSet;

/// One declared attacker.
#[derive(Clone, Copy, Debug)]
pub struct AttackerInfo {
    /// The attacking creature.
    pub creature: ObjectId,
    /// What it attacks: a player, or one of their planeswalkers.
    pub defending: Defender,
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
/// summoning-sick, no defender).
#[must_use]
pub fn can_attack(state: &GameState, player: PlayerId, creature: ObjectId) -> bool {
    let Some(obj) = state.object(creature) else {
        return false;
    };
    obj.zone == crate::zone::Zone::Battlefield
        && obj.controller == player
        && obj.characteristics().types.contains(TypeSet::CREATURE)
        // Defender (CR 702.3b): can't attack, however untapped it is.
        && !obj.characteristics().keywords.contains(K::DEFENDER)
        && !obj.status.contains(Status::TAPPED)
        && !summoning_sick(state, obj)
}

/// Everything `player` may declare an attack against right now: each
/// surviving opponent, and every planeswalker those opponents control
/// (CR 508.1a).
///
/// One list rather than "pick a player, then pick one of their walkers":
/// the choice is a single one in the rules, and a flat list is also what
/// a client needs to render the choice.
#[must_use]
pub fn defender_options(state: &GameState, player: PlayerId) -> Vec<Defender> {
    let opponents: Vec<PlayerId> = state
        .players
        .iter()
        .filter(|p| p.id != player && !p.has_lost)
        .map(|p| p.id)
        .collect();
    let mut options: Vec<Defender> = opponents.iter().copied().map(Defender::Player).collect();
    options.extend(
        state
            .zones
            .list(crate::zone::ZoneLocation::Battlefield)
            .iter()
            .filter(|id| {
                state.object(**id).is_some_and(|o| {
                    opponents.contains(&o.controller)
                        && o.characteristics().types.contains(TypeSet::PLANESWALKER)
                })
            })
            .map(|id| Defender::Planeswalker(*id)),
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
            .filter(|o| o.zone == crate::zone::Zone::Battlefield)
            .map(|o| o.controller),
    }
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
        let dealt = deal_damage_to_object(state, info.blocker, info.attacker, power, true);
        if has_keyword(state, info.blocker, K::LIFELINK)
            && let Some(controller) = state.object(info.blocker).map(|o| o.controller)
        {
            gain_life(state, controller, dealt);
        }
    }
}

/// One attacker's damage assignment (CR 510.1a–c).
fn assign_attacker_damage(state: &mut GameState, attacker: ObjectId, defending: Defender) {
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
        lifelinked += deal_damage_to_defender(state, attacker, defending, power);
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
            // Assignment and dealing are separate steps (CR 510.1c/510.2):
            // prevented damage is still assigned, so it still uses up the
            // attacker's power — but it was never dealt, so it links no life.
            lifelinked += deal_damage_to_object(state, attacker, *blocker, assigned, true);
            remaining -= assigned;
        }
        if trample && remaining > 0 {
            // CR 702.19b: what tramples through goes to "the player or
            // planeswalker it's attacking", not to the player regardless.
            lifelinked += deal_damage_to_defender(state, attacker, defending, remaining);
        }
    }
    if lifelink {
        gain_life(state, controller, lifelinked);
    }
}

/// Combat damage aimed at whatever the attacker declared against, and how
/// much of it was actually dealt (prevention and a departed planeswalker
/// both make that zero, and neither links any life).
fn deal_damage_to_defender(
    state: &mut GameState,
    source: ObjectId,
    defender: Defender,
    amount: i16,
) -> i16 {
    match defender {
        Defender::Player(player) => deal_damage_to_player(state, source, player, amount, true),
        // CR 506.4c: the attack stands even after the planeswalker has
        // gone, but there is nothing left for the damage to land on.
        Defender::Planeswalker(walker) => {
            if state
                .object(walker)
                .is_none_or(|o| o.zone != crate::zone::Zone::Battlefield)
            {
                return 0;
            }
            deal_damage_to_object(state, source, walker, amount, true)
        }
    }
}

/// Deals damage to a player and returns how much landed.
fn deal_damage_to_player(
    state: &mut GameState,
    source: ObjectId,
    player: PlayerId,
    amount: i16,
    is_combat: bool,
) -> i16 {
    if amount <= 0 {
        return 0;
    }
    if prevent_from(state, source) {
        return 0;
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
    amount
}

/// Deals damage to a permanent and returns how much landed.
fn deal_damage_to_object(
    state: &mut GameState,
    source: ObjectId,
    target: ObjectId,
    amount: i16,
    is_combat: bool,
) -> i16 {
    if amount <= 0 {
        return 0;
    }
    if prevent_from(state, source)
        || prevent_to(state, target)
        || crate::eval::protected_from(state, target, source)
    {
        return 0;
    }
    // Damage to a planeswalker removes loyalty instead of marking damage
    // (CR 306.8), the same way the spell-resolution path does it.
    let is_walker = state
        .object(target)
        .is_some_and(|o| o.characteristics().types.contains(TypeSet::PLANESWALKER));
    if is_walker {
        let old = state.object(target).map_or(0, |o| {
            o.counters.get(baylee_cards_dsl::CounterKind::Loyalty)
        });
        let new = old.saturating_sub(amount as u16);
        if let Some(obj) = state.object_mut(target) {
            obj.counters
                .set(baylee_cards_dsl::CounterKind::Loyalty, new);
        }
        state.journal.record(GameEvent::CounterChanged {
            object: target,
            kind: baylee_cards_dsl::CounterKind::Loyalty,
            old,
            new,
        });
    } else {
        let deathtouch = has_keyword(state, source, K::DEATHTOUCH);
        if let Some(obj) = state.object_mut(target) {
            obj.damage = obj.damage.saturating_add(amount as u16);
            obj.deathtouched |= deathtouch;
        }
    }
    state.journal.record(GameEvent::DamageDealt {
        source: Some(source),
        target: DamageTarget::Object(target),
        amount: amount as u16,
        is_combat,
    });
    amount
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
        let b = state.object_mut(id).expect("just created").base_mut();
        b.types = TypeSet::CREATURE;
        b.power = Some(power);
        b.toughness = Some(toughness);
        b.keywords = keywords;
        id
    }

    fn attack(state: &mut GameState, creature: ObjectId, defending: PlayerId) {
        state.combat.attackers.push(AttackerInfo {
            creature,
            defending: Defender::Player(defending),
        });
    }

    /// Declares `creature` as attacking a planeswalker instead of a seat.
    fn attack_walker(state: &mut GameState, creature: ObjectId, walker: ObjectId) {
        state.combat.attackers.push(AttackerInfo {
            creature,
            defending: Defender::Planeswalker(walker),
        });
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
        state
            .combat
            .blockers
            .push(BlockerInfo { blocker, attacker });
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
        let wall = creature(&mut state, P1, 0, 4, KeywordSet::INDESTRUCTIBLE);
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
        state.object_mut(wall).expect("alive").base_mut().keywords = KeywordSet::EMPTY;
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
        state.turn_start_timestamp = u64::MAX;
        assert!(!can_attack(&state, P0, wall), "a wall attacked");
        assert!(can_attack(&state, P0, bear), "the control could not attack");
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
}
