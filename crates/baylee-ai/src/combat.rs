//! What to attack with, and what to block.
//!
//! Both halves of combat are the same question asked twice — *if these two
//! creatures meet, who dies?* — so they share one function for it and one
//! for the keywords that change the answer. Before this existed the agent
//! swung with its whole board every turn and blocked with nothing, ever:
//! `Pending::ChooseBlockers` was answered `blockers: vec![]` for the life
//! of the project, and the acceptance soak could not see it because it
//! counted finished games and not what was played.
//!
//! Everything here reads a [`PlayerView`] and the offered choice, so the
//! agent still cannot see a card it is not entitled to. Which creature may
//! block which attacker is *not* decided here either: evasion is a pairing
//! question (CR 509.1a) and the engine answers it in
//! [`BlockOption::attackers`].

use baylee_cards_dsl::KeywordSet;
use baylee_core::ids::ObjectId;
use baylee_engine::choice::BlockOption;
use baylee_view::{PlayerView, PublicObject};

/// A creature, reduced to what a combat exchange depends on.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Fighter {
    pub power: i32,
    /// Toughness *left*: damage already marked comes off it, because a
    /// 4/4 with 3 damage on it dies to a 1/1.
    pub toughness: i32,
    pub keywords: u128,
    /// Mana value, the only measure of worth available from a view — a
    /// trade is judged by it, and it is a poor judge that is nonetheless
    /// the same one for both sides.
    pub worth: u32,
}

impl Fighter {
    pub(crate) fn of(view: &PlayerView, id: ObjectId) -> Option<Self> {
        Self::from_object(view.object(id)?)
    }

    fn from_object(o: &PublicObject) -> Option<Self> {
        Some(Self {
            power: i32::from(o.power?),
            toughness: i32::from(o.toughness?) - i32::from(o.damage),
            keywords: o.keywords,
            worth: o.mana_value,
        })
    }

    fn has(self, k: KeywordSet) -> bool {
        self.keywords & k.bits() != 0
    }

    /// Whether this creature's damage is lethal to `other` (CR 704.5g).
    fn kills(self, other: Self) -> bool {
        if other.has(KeywordSet::INDESTRUCTIBLE) {
            return false;
        }
        // Deathtouch: any nonzero amount is lethal (CR 702.2b).
        if self.has(KeywordSet::DEATHTOUCH) && self.power > 0 {
            return true;
        }
        self.power >= other.toughness && self.power > 0
    }

    fn strikes_first(self) -> bool {
        self.has(KeywordSet::FIRST_STRIKE) || self.has(KeywordSet::DOUBLE_STRIKE)
    }
}

/// What happens when `attacker` and `blocker` meet alone.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Exchange {
    pub attacker_dies: bool,
    pub blocker_dies: bool,
}

/// One attacker against one blocker, first strike included.
///
/// First strike is the reason this is a function and not two comparisons:
/// when exactly one side has it *and* kills with it, the other never deals
/// its damage at all, which turns what looks like a trade into a free kill
/// in one direction and a wasted creature in the other. Double strike is
/// treated as first strike here — a creature that kills in the first step
/// does not need the second.
pub(crate) fn exchange(attacker: Fighter, blocker: Fighter) -> Exchange {
    let a_kills = attacker.kills(blocker);
    let b_kills = blocker.kills(attacker);
    match (attacker.strikes_first(), blocker.strikes_first()) {
        // Same step: both connect, whatever happens to either.
        (true, true) | (false, false) => Exchange {
            attacker_dies: b_kills,
            blocker_dies: a_kills,
        },
        (true, false) => Exchange {
            attacker_dies: b_kills && !a_kills,
            blocker_dies: a_kills,
        },
        (false, true) => Exchange {
            attacker_dies: b_kills,
            blocker_dies: a_kills && !b_kills,
        },
    }
}

/// How much of an attacker's damage still reaches the player through a
/// block (CR 702.19b): nothing, unless it tramples.
fn spillover(attacker: Fighter, blocker: Fighter) -> i32 {
    if attacker.has(KeywordSet::TRAMPLE) {
        (attacker.power - blocker.toughness.max(0)).max(0)
    } else {
        0
    }
}

/// Which creatures to block with, and what each of them blocks.
///
/// Three rules, in the order a player applies them:
///
/// 1. **Do not die.** If the unblocked attackers add up to this seat's life
///    total, blocks are made until they do not — with whatever is left,
///    including a creature that only chumps. A creature kept back is worth
///    nothing after the game is over.
/// 2. **Take the good exchanges.** A block where the attacker dies and the
///    blocker lives is free; one where both die is worth making when the
///    attacker is worth at least as much.
/// 3. **Otherwise stay home.** Chump-blocking off a race the seat is not
///    losing throws a creature away for a few points of life.
///
/// Each blocker is assigned to at most one attacker and each attacker gets
/// at most one blocker: multi-blocking is a real option and needs damage
/// *assignment* to be worth anything, which the engine asks separately.
#[must_use]
pub fn choose_blocks(
    view: &PlayerView,
    options: &[BlockOption],
    life: i32,
) -> Vec<(ObjectId, ObjectId)> {
    let attacking: Vec<ObjectId> = view
        .combat
        .attackers
        .iter()
        .map(|a| a.creature)
        .filter(|id| view.object(*id).is_some())
        .collect();
    let incoming: i32 = attacking
        .iter()
        .filter_map(|id| Fighter::of(view, *id))
        .map(|f| f.power)
        .sum();
    // The engine offers a pairing per blocker; a creature with nothing it
    // may legally block is not a decision.
    let mut free: Vec<&BlockOption> = options.iter().filter(|o| !o.attackers.is_empty()).collect();
    // Biggest blockers first, so the creature that can actually kill
    // something is not spent chumping.
    free.sort_by_key(|o| {
        let f = Fighter::of(view, o.blocker);
        (
            -f.map_or(0, |f| f.power),
            -f.map_or(0, |f| f.toughness),
            o.blocker.slot(),
        )
    });

    let mut pairs: Vec<(ObjectId, ObjectId)> = Vec::new();
    let mut taken: Vec<ObjectId> = Vec::new();
    let mut still_coming = incoming;

    for option in free {
        let Some(blocker) = Fighter::of(view, option.blocker) else {
            continue;
        };
        // Of the attackers this creature may block, the one where the
        // exchange is best — and among equals, the one that hits hardest.
        let best = option
            .attackers
            .iter()
            .filter(|id| !taken.contains(id))
            .filter_map(|id| Fighter::of(view, *id).map(|f| (*id, f)))
            .max_by_key(|(id, attacker)| {
                let e = exchange(*attacker, blocker);
                // `false < true`, so this reads as a preference order: our
                // creature surviving first, then theirs dying, then the
                // hardest hitter, then the lowest slot to break ties without
                // consulting a clock.
                (
                    !e.blocker_dies,
                    e.attacker_dies,
                    attacker.power,
                    std::cmp::Reverse(id.slot()),
                )
            });
        let Some((attacker_id, attacker)) = best else {
            continue;
        };
        let e = exchange(attacker, blocker);
        // What the block actually saves: the attacker's damage, less
        // whatever tramples through anyway.
        let saved = attacker.power - spillover(attacker, blocker);
        let lethal = still_coming >= life;
        let worth_it = if lethal {
            // Rule 1. Any block that stops damage is worth making, and a
            // creature that dies for it has done its job.
            saved > 0
        } else if !e.blocker_dies {
            // Rule 2a. Free: nothing of ours dies.
            e.attacker_dies || saved > 0
        } else {
            // Rule 2b. A trade, judged by what each side costs.
            e.attacker_dies && attacker.worth >= blocker.worth
        };
        if worth_it {
            pairs.push((option.blocker, attacker_id));
            taken.push(attacker_id);
            still_coming -= saved;
        }
    }
    pairs
}

/// Whether `blocker` could plausibly be paired with `attacker`.
///
/// An *estimate*, and the only one in this file: on the attacking side the
/// engine has not enumerated anything yet, so evasion has to be guessed at
/// from the keywords both sides carry. It covers the case that decides most
/// attacks — a flyer the defender has nothing to catch it with — and errs
/// toward "yes, it can block", which makes the agent attack less rather
/// than throw creatures away. When it is the *defender's* turn to decide,
/// nothing is guessed: [`choose_blocks`] reads the pairings the engine
/// offered.
fn could_block(attacker: Fighter, blocker: Fighter) -> bool {
    if attacker.has(KeywordSet::UNBLOCKABLE) {
        return false;
    }
    if attacker.has(KeywordSet::FLYING)
        && !blocker.has(KeywordSet::FLYING)
        && !blocker.has(KeywordSet::REACH)
    {
        return false;
    }
    true
}

/// Which of the creatures the engine offered should actually attack.
///
/// The squad used to attack in full every turn, which throws a 1/1 into a
/// 4/4 for as long as the game lasts. A creature swings when one of these
/// holds:
///
/// - **The attack wins.** Total power at least the victim's life: nothing
///   held back matters after that.
/// - **Vigilance.** It does not tap, so attacking costs nothing.
/// - **Nothing kills it.** No untapped creature the victim controls both
///   could block it and would survive doing so.
/// - **The trade is good.** The best block the defender has still loses
///   them something worth at least as much.
///
/// The defender is assumed to make their *best* block against each
/// attacker independently, which over-estimates them — one blocker cannot
/// answer two attackers — and that is the safe direction to be wrong in.
#[must_use]
pub fn choose_attackers(
    view: &PlayerView,
    squad: &[ObjectId],
    victim: baylee_core::ids::PlayerId,
) -> Vec<ObjectId> {
    let defenders: Vec<Fighter> = view
        .battlefield_of(victim)
        .filter(|o| {
            o.types.contains(baylee_core::types::TypeSet::CREATURE)
                && !o.status.contains(baylee_view::ObjectStatus::TAPPED)
        })
        .filter_map(Fighter::from_object)
        .collect();
    let total: i32 = squad
        .iter()
        .filter_map(|id| Fighter::of(view, *id))
        .map(|f| f.power)
        .sum();
    let lethal = total >= view.seat(victim).map_or(i32::MAX, |s| s.life);

    squad
        .iter()
        .copied()
        .filter(|id| {
            let Some(attacker) = Fighter::of(view, *id) else {
                return false;
            };
            if lethal || attacker.has(KeywordSet::VIGILANCE) {
                return true;
            }
            // The worst the defender can do to this attacker.
            !defenders
                .iter()
                .filter(|b| could_block(attacker, **b))
                .map(|b| exchange(attacker, *b))
                .any(|e| e.attacker_dies && !e.blocker_dies)
                && !defenders
                    .iter()
                    .filter(|b| could_block(attacker, **b))
                    .any(|b| {
                        let e = exchange(attacker, *b);
                        e.attacker_dies && b.worth < attacker.worth
                    })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(power: i32, toughness: i32, keywords: KeywordSet) -> Fighter {
        Fighter {
            power,
            toughness,
            keywords: keywords.bits(),
            worth: 2,
        }
    }

    /// A 2/2 into a 2/2 kills both ways.
    #[test]
    fn an_even_trade_kills_both() {
        let e = exchange(f(2, 2, KeywordSet::EMPTY), f(2, 2, KeywordSet::EMPTY));
        assert!(e.attacker_dies && e.blocker_dies);
    }

    /// First strike on one side and lethal damage with it: the other never
    /// swings back. This is the case a pair of `>=` comparisons gets wrong.
    #[test]
    fn first_strike_that_kills_takes_no_damage_back() {
        let e = exchange(
            f(2, 2, KeywordSet::FIRST_STRIKE),
            f(2, 2, KeywordSet::EMPTY),
        );
        assert!(e.blocker_dies, "the first-striker did not kill");
        assert!(!e.attacker_dies, "the dead blocker still dealt its damage");
    }

    /// First strike that does *not* kill saves nobody.
    #[test]
    fn first_strike_that_does_not_kill_still_dies() {
        let e = exchange(
            f(2, 2, KeywordSet::FIRST_STRIKE),
            f(3, 3, KeywordSet::EMPTY),
        );
        assert!(e.attacker_dies && !e.blocker_dies);
    }

    /// Deathtouch makes one point lethal (CR 702.2b), and indestructible
    /// answers it (CR 702.12b).
    #[test]
    fn deathtouch_kills_anything_that_is_not_indestructible() {
        let e = exchange(f(1, 1, KeywordSet::DEATHTOUCH), f(6, 6, KeywordSet::EMPTY));
        assert!(e.blocker_dies, "deathtouch did not kill a 6/6");

        let e = exchange(
            f(1, 1, KeywordSet::DEATHTOUCH),
            f(6, 6, KeywordSet::INDESTRUCTIBLE),
        );
        assert!(!e.blocker_dies, "deathtouch killed an indestructible");
    }

    /// Damage already marked counts: a 4/4 with three on it dies to a 1/1.
    #[test]
    fn marked_damage_lowers_the_bar() {
        let hurt = Fighter {
            toughness: 4 - 3,
            ..f(4, 4, KeywordSet::EMPTY)
        };
        let e = exchange(f(1, 1, KeywordSet::EMPTY), hurt);
        assert!(e.blocker_dies, "a 4/4 with three damage survived a 1/1");
    }

    /// A chump block stops nothing a trampler was going to deal anyway.
    #[test]
    fn trample_goes_through_a_chump_block() {
        let attacker = f(5, 5, KeywordSet::TRAMPLE);
        assert_eq!(spillover(attacker, f(1, 1, KeywordSet::EMPTY)), 4);
        assert_eq!(
            spillover(f(5, 5, KeywordSet::EMPTY), f(1, 1, KeywordSet::EMPTY)),
            0
        );
    }
}
