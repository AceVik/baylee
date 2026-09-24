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
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::types::TypeSet;
use baylee_engine::choice::BlockOption;
use baylee_view::{ObjectStatus, PlayerView, PublicObject};

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

/// Every id in `ids`, once each, in the order they first appear.
///
/// A `Vec` scan rather than a set: these lists are a combat's worth of
/// creatures, and an ordering that depends on a hash is an ordering the
/// agent would replay differently.
pub(crate) fn deduped(ids: impl Iterator<Item = ObjectId>) -> Vec<ObjectId> {
    ids.fold(Vec::new(), |mut acc: Vec<ObjectId>, id| {
        if !acc.contains(&id) {
            acc.push(id);
        }
        acc
    })
}

/// Every creature attacking that this seat may have to survive.
///
/// Two sources, because they fail in different ways. `view.combat.attackers`
/// is the whole attack, including what this seat may not block; the engine's
/// pairings are the authority on what is attacking *this* seat and are the
/// one source that survives a view the attack cannot be read out of. In a
/// healthy game the second is a subset of the first and the union is the
/// first, so nothing moves.
fn attacking(view: &PlayerView, options: &[BlockOption]) -> Vec<ObjectId> {
    deduped(
        view.combat
            .attackers
            .iter()
            .map(|a| a.creature)
            .chain(options.iter().flat_map(|o| o.attackers.iter().copied())),
    )
}

/// Which creatures to block with, and what each of them blocks.
///
/// Three rules, in the order a player applies them:
///
/// 1. **Do not die.** If the unblocked attackers add up to this seat's life
///    total, blocks are made until they do not — with whatever is left,
///    including a creature that only chumps. A creature kept back is worth
///    nothing after the game is over. An attacker the view cannot describe
///    counts here as well: the engine offered a pairing against it, so it
///    exists, and what it will deal is unknown rather than nought.
/// 2. **Take the good exchanges.** A block where the attacker dies and the
///    blocker lives is free; one where both die is worth making when the
///    attacker is worth at least as much.
/// 3. **Otherwise stay home.** Chump-blocking off a race the seat is not
///    losing throws a creature away for a few points of life.
///
/// Each blocker is assigned to at most one attacker and each attacker gets
/// at most one blocker: multi-blocking is a real option and needs damage
/// *assignment* to be worth anything, which the engine asks separately. The
/// one exception is not a judgement but a legality — menace takes two
/// blockers or none, and [`enforce_menace`] settles that against the finished
/// declaration rather than while it is being built.
#[must_use]
pub fn choose_blocks(
    view: &PlayerView,
    options: &[BlockOption],
    life: i32,
) -> Vec<(ObjectId, ObjectId)> {
    let attacking = attacking(view, options);
    let incoming: i32 = attacking
        .iter()
        .filter_map(|id| Fighter::of(view, *id))
        .map(|f| f.power)
        .sum();
    // Every attacker the engine offered a pairing against that the view
    // cannot describe. `Fighter::of` is three `?` in a row — the object, its
    // power, its toughness — and each of them reads to a caller as "no such
    // attacker". Here that is wrong twice over: it is the engine that named
    // this creature, so it is *there*, and its damage is **unknown** rather
    // than nought. Counted as nought it left `incoming` short and left every
    // blocker with nothing to pair with, which is how this decision answered
    // a lethal attack with no blocks at all.
    let mut unread: Vec<ObjectId> = attacking
        .iter()
        .copied()
        // Only what the engine offered a pairing against. An id the view
        // still names but nothing can be blocked against is an attacker
        // that has left the battlefield, and that one really is absent.
        .filter(|id| options.iter().any(|o| o.attackers.contains(id)))
        .filter(|id| Fighter::of(view, *id).is_none())
        .collect();
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
        // Read before the pairing, because an attacker nobody can describe
        // is what makes it true in the case this exists for.
        let lethal = !unread.is_empty() || still_coming >= life;
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
            // Nothing readable left to pair with. An unreadable attacker is
            // still an attacker, and a seat that cannot prove it survives
            // chumps: the creature costs one card, and the alternative is
            // the game. It is not offered a *choice* between unreadables —
            // there is nothing to choose on — so it takes the first.
            if lethal
                && let Some(i) = unread
                    .iter()
                    .position(|id| option.attackers.contains(id) && !taken.contains(id))
            {
                let attacker_id = unread.remove(i);
                pairs.push((option.blocker, attacker_id));
                taken.push(attacker_id);
            }
            continue;
        };
        let e = exchange(attacker, blocker);
        // What the block actually saves: the attacker's damage, less
        // whatever tramples through anyway.
        let saved = attacker.power - spillover(attacker, blocker);
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
    enforce_menace(view, options, &mut pairs);
    pairs
}

/// Menace (CR 702.111b): two blockers or none, never one.
///
/// The loop above pairs one blocker with one attacker by construction, so
/// left alone it answers a menace attacker with exactly the declaration the
/// rules forbid — and `Engine::declare_blockers` refuses the **whole**
/// answer, not the offending pair. One illegal block therefore costs every
/// other block in the same declaration, which is why this is a pass over the
/// finished list rather than a judgement made per attacker: it can see the
/// answer that is actually going to be sent.
///
/// Where a second blocker exists it is added, because the alternative is
/// dropping a block the loop already decided was worth making, and in the
/// lethal case that is the game. The one added is the *cheapest* that may
/// legally be paired, since it is being spent to satisfy a rule rather than
/// to win an exchange — the deeper profiles reach this through `search`,
/// which evaluates the two-blocker leaf on its own merits instead.
///
/// It reads menace off the view's projected keywords, so a granted or
/// removed one counts (CR 613.1). An attacker the view cannot describe at
/// all is the one case it cannot answer: `Fighter::of` is `None` there, so
/// nothing can be read about it, menace included — see the chump-block
/// branch above, which is the same gap seen from the other side.
fn enforce_menace(
    view: &PlayerView,
    options: &[BlockOption],
    pairs: &mut Vec<(ObjectId, ObjectId)>,
) {
    let menacing = deduped(pairs.iter().map(|(_, attacker)| *attacker));
    for attacker in menacing {
        if !Fighter::of(view, attacker).is_some_and(|f| f.has(KeywordSet::MENACE)) {
            continue;
        }
        if pairs.iter().filter(|(_, a)| *a == attacker).count() != 1 {
            continue;
        }
        let mut spare: Vec<&BlockOption> = options
            .iter()
            .filter(|o| o.attackers.contains(&attacker))
            .filter(|o| !pairs.iter().any(|(blocker, _)| *blocker == o.blocker))
            .collect();
        spare.sort_by_key(|o| {
            let f = Fighter::of(view, o.blocker);
            (
                f.map_or(0, |f| f.power),
                f.map_or(0, |f| f.toughness),
                o.blocker.slot(),
            )
        });
        if let Some(second) = spare.first() {
            pairs.push((second.blocker, attacker));
        } else {
            pairs.retain(|(_, a)| *a != attacker);
        }
    }
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
pub(crate) fn could_block(attacker: Fighter, blocker: Fighter) -> bool {
    if attacker.has(KeywordSet::UNBLOCKABLE) || blocker.has(KeywordSet::CANT_BLOCK) {
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
///
/// None of these rules looks at the turn after. What this returns, and what
/// the search returns, passes `hold_back_for_the_crack_back` before it is
/// sent.
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

/// Never leave the table unable to survive the swing back.
///
/// The owner's #123 game: a 75/75 first striker attacked, and on the house
/// AI's turn it was still tapped, so nothing on the other side could block
/// and every rule above said "swing". Eight creatures went, dealt sixteen
/// into twenty, and stayed tapped through the next turn (CR 502.3 untaps
/// only the active player's permanents), when the 75/75 untapped and walked
/// into a table the engine could offer no block on. Only EXPERT prices that
/// retaliation (`search`, `lookahead >= 2`); the other four profiles did
/// exactly this, the default among them.
///
/// So like `enforce_menace` this is a pass over the **finished** answer and
/// runs for every profile: "do not die" is block rule 1 seen from the other
/// side, not a skill level. If the attack does not end the game, and what
/// some hostile seat could swing back with would get through what stays
/// home, creatures are held back one at a time — the one that stops the most,
/// the cheapest on a tie — until it would not. A table that dies next turn
/// whatever it keeps home attacks as it meant to, since holding back buys it
/// nothing.
///
/// What comes back is every creature a hostile seat controls, tapped or
/// phased out or not: it phases in and untaps first (CR 502.1, 502.3), and
/// none of it is summoning sick by then (CR 302.6).
/// Defender stays home. What blocks is what is untapped now and not
/// attacking, plus a vigilant attacker (CR 702.20b), paired through the same
/// [`Fighter`] model the rest of this file uses — flying needs flying or
/// reach, menace needs two, trample pushes the excess through a chump.
///
/// Each hostile seat is asked on its own, because each attacks in its own
/// turn and a creature that blocks is not tapped by it; damage that several
/// opponents add up to over one round is not modelled, and neither is
/// commander damage or poison. Double strike is twice the power, and with
/// trample its second step meets no blocker at all (CR 702.19d). Deathtouch
/// makes one point lethal damage (CR 702.2c), which is not read into a
/// trampler's excess here, so a deathtouch trampler is undercounted.
pub(crate) fn hold_back_for_the_crack_back(
    view: &PlayerView,
    going: Vec<ObjectId>,
    victim: PlayerId,
    attack_is_lethal: bool,
    hostile: impl Fn(PlayerId) -> bool,
) -> Vec<ObjectId> {
    let Some(life) = view.seat(view.seat).map(|s| s.life) else {
        return going;
    };
    let attackers: Vec<Fighter> = going
        .iter()
        .filter_map(|&id| Fighter::of(view, id))
        .collect();
    let lethal = attack_is_lethal
        || view
            .seat(victim)
            .is_some_and(|s| through(&attackers, &creatures(view, victim, ready)) >= s.life);
    // What each seat still in the game could swing back with. The victim's
    // is nothing when this attack ends them.
    let threats: Vec<Vec<Fighter>> = view
        .seats
        .iter()
        .filter(|s| !s.has_lost && hostile(s.player))
        .filter(|s| !(lethal && s.player == victim))
        .map(|s| {
            creatures(view, s.player, |_| true)
                .into_iter()
                .filter(|f| !f.has(KeywordSet::DEFENDER) && f.power > 0)
                .collect()
        })
        .collect();
    let dies = |going: &[ObjectId]| -> i32 {
        let home = creatures(view, view.seat, |o| {
            ready(o) && (!going.contains(&o.id) || o.keywords & KeywordSet::VIGILANCE.bits() != 0)
        });
        threats.iter().map(|t| through(t, &home)).max().unwrap_or(0)
    };
    if dies(&going) < life {
        return going;
    }
    let mut kept = going.clone();
    loop {
        // A vigilant attacker defends from where it is; keeping it home
        // would cost its damage and stop nothing more.
        let pick = kept
            .iter()
            .enumerate()
            .filter(|(_, id)| {
                Fighter::of(view, **id).is_some_and(|f| !f.has(KeywordSet::VIGILANCE))
            })
            .min_by_key(|(i, id)| {
                let mut rest = kept.clone();
                rest.remove(*i);
                let f = Fighter::of(view, **id);
                (
                    dies(&rest),
                    f.map_or(0, |f| f.worth),
                    f.map_or(0, |f| f.power),
                    id.slot(),
                )
            })
            .map(|(i, _)| i);
        let Some(i) = pick else {
            return going;
        };
        kept.remove(i);
        if dies(&kept) < life {
            return kept;
        }
    }
}

/// `seat`'s creatures on the battlefield that pass `keep`, phased out or
/// not, as they will be at the next combat: damage marked now is gone by
/// then (CR 514.2).
fn creatures(
    view: &PlayerView,
    seat: PlayerId,
    keep: impl Fn(&PublicObject) -> bool,
) -> Vec<Fighter> {
    view.battlefield_of(seat)
        .filter(|o| o.types.contains(TypeSet::CREATURE) && keep(o))
        .filter_map(|o| {
            Fighter::from_object(o).map(|mut f| {
                f.toughness += i32::from(o.damage);
                f
            })
        })
        .collect()
}

/// Able to block now and through the next turn: untapped, and phased in —
/// ours phase back in only on our own untap step (CR 502.1).
fn ready(o: &PublicObject) -> bool {
    !o.status.contains(ObjectStatus::TAPPED) && !o.status.contains(ObjectStatus::PHASED_OUT)
}

/// How much of `attackers` gets past `blockers`, each blocker used once.
///
/// Greedy, and on purpose: the attacker with the fewest possible blockers is
/// answered first, so a lone reach creature is not spent on a ground beater
/// while a flyer walks in, and the biggest goes first among equals. Each
/// takes the blocker that lets the least through, the cheapest on a tie; a
/// menace attacker takes two or nothing. Whatever is left over then stands
/// in front of a trampler as well, since every extra blocker's toughness is
/// damage kept off the player (CR 702.19b).
fn through(attackers: &[Fighter], blockers: &[Fighter]) -> i32 {
    let mut free: Vec<Fighter> = blockers.to_vec();
    // Double strike is twice the damage, and with trample the second step
    // meets no blocker (CR 702.19d), so a wall of toughness T lets 2P - T
    // through: the arithmetic of one attacker with twice the power.
    let mut order: Vec<Fighter> = attackers
        .iter()
        .map(|a| {
            if a.has(KeywordSet::DOUBLE_STRIKE) {
                Fighter {
                    power: a.power * 2,
                    ..*a
                }
            } else {
                *a
            }
        })
        .collect();
    order.sort_by_key(|a| {
        (
            free.iter().filter(|b| could_block(*a, **b)).count(),
            std::cmp::Reverse(a.power),
        )
    });
    let mut total = 0;
    // Tramplers that were blocked and still push damage through, with the
    // damage left, for the leftover blockers below.
    let mut spilling: Vec<(Fighter, i32)> = Vec::new();
    for a in order {
        let need = if a.has(KeywordSet::MENACE) { 2 } else { 1 };
        let mut able: Vec<usize> = (0..free.len())
            .filter(|&i| could_block(a, free[i]))
            .collect();
        if able.len() < need {
            total += a.power.max(0);
            continue;
        }
        able.sort_by_key(|&i| (spillover(a, free[i]), free[i].worth));
        let mut chosen: Vec<usize> = able.into_iter().take(need).collect();
        let wall = Fighter {
            toughness: chosen.iter().map(|&i| free[i].toughness.max(0)).sum(),
            ..free[chosen[0]]
        };
        let spill = spillover(a, wall);
        total += spill;
        if spill > 0 {
            spilling.push((a, spill));
        }
        chosen.sort_unstable_by(|x, y| y.cmp(x));
        for i in chosen {
            free.remove(i);
        }
    }
    free.sort_by_key(|b| b.worth);
    for b in free {
        let biggest = spilling
            .iter_mut()
            .filter(|(a, left)| *left > 0 && could_block(*a, b))
            .max_by_key(|(_, left)| *left);
        if let Some((_, left)) = biggest {
            let soaked = (*left).min(b.toughness.max(0));
            *left -= soaked;
            total -= soaked;
        }
    }
    total
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
