//! Structured combat state machine.
//!
//! Implemented: attacker/blocker declaration with the keyword restrictions
//! (flying/reach, menace, unblockable, can't block, protection), first/double strike as
//! a per-creature property, deathtouch, trample, lifelink, and damage
//! assignment in declaration order.
//!
//! Attacks are aimed at a [`Defender`], so a planeswalker can be attacked
//! and its loyalty comes off (CR 306.8). Battles are the remaining case.
//!
//! Not yet: the attacking player's *choice* of damage assignment order
//! among multiple blockers (CR 510.1c) — the declaration order stands in
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
    /// Whether a blocker was ever declared against it (CR 509.1h).
    ///
    /// It is set once and never cleared, which is the whole point: a
    /// creature that has been blocked *stays* blocked for the rest of
    /// combat, so "is it blocked" and "what is blocking it" stop being the
    /// same question the moment a blocker leaves the battlefield. Reading
    /// the blocker list for both is what let an attacker whose only blocker
    /// was blinked deal its damage to the player.
    pub blocked: bool,
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
        self.attackers.retain(|a| a.creature != id);
        self.blockers
            .retain(|b| b.blocker != id && b.attacker != id);
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
        && !obj.status.contains(Status::PHASED_OUT)
        && !summoning_sick(state, obj)
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
        .filter(|p| state.is_opponent(p.id, player) && !p.has_lost)
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
                        && !o.status.contains(Status::PHASED_OUT)
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
            .filter(|o| {
                o.zone == crate::zone::Zone::Battlefield && !o.status.contains(Status::PHASED_OUT)
            })
            .map(|o| o.controller),
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
    obj.timestamp > began
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
        || b.status.contains(Status::PHASED_OUT)
        || a.zone != crate::zone::Zone::Battlefield
        || a.status.contains(Status::PHASED_OUT)
        || !a.characteristics().types.contains(TypeSet::CREATURE)
    {
        return false;
    }
    let kw =
        |o: &GameObject, k: baylee_cards_dsl::KeywordSet| o.characteristics().keywords.contains(k);
    // Flying can only be blocked by flying/reach (CR 702.9).
    if kw(a, K::FLYING) && !kw(b, K::FLYING) && !kw(b, K::REACH) {
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
/// additional creature still answers it once. `take(2)` walks the
/// battlefield, which holds each permanent once.
///
/// `true` for an attacker without menace, so callers may ask it of every
/// attacker without asking twice.
#[must_use]
pub fn menace_satisfiable(state: &GameState, defending: PlayerId, attacker: ObjectId) -> bool {
    let Some(a) = state.object(attacker) else {
        return false;
    };
    if !a.characteristics().keywords.contains(K::MENACE) {
        return true;
    }
    state
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|blocker| can_block(state, defending, *blocker, attacker))
        .take(2)
        .count()
        == 2
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
        assign_attacker_damage(state, info.creature, info.defending, info.blocked);
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
fn assign_attacker_damage(
    state: &mut GameState,
    attacker: ObjectId,
    defending: Defender,
    blocked: bool,
) {
    let power = power_of(state, attacker);
    let trample = has_keyword(state, attacker, K::TRAMPLE);
    let lifelink = has_keyword(state, attacker, K::LIFELINK);
    let Some(controller) = state.object(attacker).map(|o| o.controller) else {
        return;
    };
    let mut lifelinked = 0i16;
    // CR 509.1h: an attacker whose blockers have all left is still
    // *blocked*, so it deals no damage to the player — unless it has
    // trample, which assigns everything past the (now absent) blockers to
    // the defender. The question is the declaration, not the list: a
    // creature that left the battlefield is out of combat entirely
    // (CR 506.4) and `CombatState::remove_from_combat` has already dropped
    // its entry, so an empty list here means either "never blocked" or
    // "blocked by creatures that are gone", and only `blocked` tells them
    // apart.
    let live: Vec<ObjectId> = state
        .combat
        .blockers_of(attacker)
        .into_iter()
        .filter(|b| state.object(*b).is_some())
        .collect();
    if blocked {
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
    } else {
        lifelinked += deal_damage_to_defender(state, attacker, defending, power);
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
            if defending_player(state, defender).is_none() {
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
    // Commander damage (CR 903.10a). Combat damage only — a commander's
    // *ability* pinging for twenty-one is not this rule — and it counts by
    // the commander, not by whoever is swinging it: a commander stolen with
    // Agent of Treachery still adds to the tally its owner's opponents keep
    // against it, because the rule names the object and not a controller.
    let from_a_commander = state
        .commanders
        .iter()
        .flatten()
        .any(|c| c.object == source);
    if is_combat && from_a_commander {
        let dealt = amount as u16;
        let tally = &mut state.players[player.get() as usize].commander_damage;
        if let Some(entry) = tally.iter_mut().find(|(id, _)| *id == source) {
            entry.1 = entry.1.saturating_add(dealt);
        } else {
            tally.push((source, dealt));
        }
    }
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
///
/// `EffectFilter::names` and not an id compare written out here: a shield
/// registered against the object that *was* at this id is not a shield on
/// the object that is there now (CR 400.7). These two were the last pair of
/// copies of that compare, which is the whole reason the predicate is one
/// function.
fn prevent_from(state: &GameState, source: ObjectId) -> bool {
    state.object(source).is_some_and(|obj| {
        state.effects.iter().any(|fx| {
            matches!(fx.modifier, baylee_cards_dsl::Modifier::PreventDamageFromIt)
                && fx.filter.names(obj)
        })
    })
}

/// True if the target object may not be dealt damage (`PreventDamageToIt`).
fn prevent_to(state: &GameState, target: ObjectId) -> bool {
    state.object(target).is_some_and(|obj| {
        state.effects.iter().any(|fx| {
            matches!(fx.modifier, baylee_cards_dsl::Modifier::PreventDamageToIt)
                && fx.filter.names(obj)
        })
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
            blocked: false,
        });
    }

    /// Declares `creature` as attacking a planeswalker instead of a seat.
    fn attack_walker(state: &mut GameState, creature: ObjectId, walker: ObjectId) {
        state.combat.attackers.push(AttackerInfo {
            creature,
            defending: Defender::Planeswalker(walker),
            blocked: false,
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
            .has_lost = true;
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
            state.combat.attackers.is_empty(),
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
        assert!(state.combat.attackers.is_empty());
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
