//! Choosing the creatures of a fight (CR 701.14a) and of its one-sided
//! sibling, "deals damage equal to its power to".
//!
//! A fight is the one effect whose two targets mean opposite things to the
//! same spell. Bridgeworks Battle pumps its first target and fights its
//! second; read through [`crate::tactics::meaning`] the whole spell is
//! "beneficial", and the general ranking then scores every creature across
//! the table as a creature the pump must not go to — so the agent answered
//! "up to one" with none and cast a fight spell that never fought. The
//! question has to be read per instance of the word "target", which is what
//! [`DecisionContext::second_instance`] says, and answered by what the fight
//! would *do*: which creature dies, and whether mine survives it.

use baylee_cards_dsl::{Effect, KeywordSet, TargetSlot};
use baylee_core::ids::ObjectId;
use baylee_engine::choice::PlayerAction;
use baylee_engine::engine::DecisionContext;
use baylee_view::{PlayerView, PublicObject};

use crate::HeuristicAgent;
use crate::tactics::{amount, material};

/// The fight an effect list prints, reduced to what choosing its creatures
/// needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Brawl {
    /// Who deals damage (and, in a fight, is dealt it back).
    pub fighter: TargetSlot,
    /// Who is dealt it.
    pub foe: TargetSlot,
    /// A fight deals damage both ways; "deals damage equal to its power"
    /// deals it one way.
    pub both_ways: bool,
    /// The +X/+Y the same effect list gives its first target before the
    /// fight — Bridgeworks Battle's +2/+2 — which the fight is dealt at.
    pub pump: (i16, i16),
}

/// The fight in `effects`, if there is one.
pub(crate) fn brawl(effects: &[Effect], x: u32) -> Option<Brawl> {
    let mut pump = (0i16, 0i16);
    for effect in effects {
        match effect {
            Effect::PumpTarget {
                power, toughness, ..
            } => {
                pump.0 = pump.0.saturating_add(to_i16(amount(*power, x)));
                pump.1 = pump.1.saturating_add(to_i16(amount(*toughness, x)));
            }
            Effect::Fight { fighter, foe } => {
                return Some(Brawl {
                    fighter: *fighter,
                    foe: *foe,
                    both_ways: true,
                    pump: if *fighter == TargetSlot::First {
                        pump
                    } else {
                        (0, 0)
                    },
                });
            }
            Effect::DamageEqualToPower { dealer, to } => {
                return Some(Brawl {
                    fighter: *dealer,
                    foe: *to,
                    both_ways: false,
                    pump: if *dealer == TargetSlot::First {
                        pump
                    } else {
                        (0, 0)
                    },
                });
            }
            _ => {}
        }
    }
    None
}

fn to_i16(n: i32) -> i16 {
    i16::try_from(n).unwrap_or(if n < 0 { i16::MIN } else { i16::MAX })
}

fn deathtouch(o: &PublicObject) -> bool {
    o.keywords & KeywordSet::DEATHTOUCH.bits() != 0
}

/// How much is left to take before `o` is gone: a creature's toughness less
/// its damage, a planeswalker's loyalty (CR 306.8).
fn left(o: &PublicObject) -> Option<i16> {
    o.remaining_toughness()
        .or_else(|| o.loyalty.map(|l| i16::try_from(l).unwrap_or(i16::MAX)))
}

impl HeuristicAgent {
    /// What `fighter` fighting `foe` is worth to this seat.
    ///
    /// Positive when the foe dies: most when the fighter lives through it,
    /// less — by what the fighter is worth — when the two trade. A fight that
    /// kills nothing is worth nothing, and one that kills only my own
    /// creature costs it.
    fn bout(
        &self,
        view: &PlayerView,
        fighter: &PublicObject,
        foe: &PublicObject,
        brawl: Brawl,
    ) -> i64 {
        if !self.hostile(foe.controller, view.seat) {
            return -20_000;
        }
        let power = fighter.power.unwrap_or(0).saturating_add(brawl.pump.0);
        let toughness_left = fighter
            .remaining_toughness()
            .unwrap_or(0)
            .saturating_add(brawl.pump.1);
        let kills = power > 0 && (deathtouch(fighter) || left(foe).is_some_and(|l| power >= l));
        let hits_back = foe.power.unwrap_or(0);
        let survives =
            !brawl.both_ways || hits_back <= 0 || (!deathtouch(foe) && hits_back < toughness_left);
        match (kills, survives) {
            (true, true) => 10_000 + material(foe),
            (true, false) => material(foe) - material(fighter),
            (false, true) => 0,
            (false, false) => -material(fighter),
        }
    }

    /// The creatures a fight's target question should name, or `None` when
    /// the spell is not a fight — or the question is not one of its two.
    pub(crate) fn fight_targets(
        &self,
        view: &PlayerView,
        objects: &[ObjectId],
        min: u8,
        max: u8,
        context: &DecisionContext<'_>,
    ) -> Option<PlayerAction> {
        let brawl = brawl(context.effects, context.x)?;
        let asked = if context.second_instance {
            TargetSlot::Second
        } else {
            TargetSlot::First
        };
        let mut ranked: Vec<(i64, ObjectId)> = if asked == brawl.fighter {
            // Choosing the fighter before its foe is known: each of mine is
            // worth the best bout it could have against what is across the
            // table, so the creature picked is one with a fight worth having.
            let foes: Vec<&PublicObject> = view
                .battlefield
                .iter()
                .filter(|o| self.hostile(o.controller, view.seat) && left(o).is_some())
                .collect();
            objects
                .iter()
                .filter_map(|id| view.object(*id))
                .map(|candidate| {
                    if self.hostile(candidate.controller, view.seat) {
                        return (-20_000, candidate.id);
                    }
                    let best = foes
                        .iter()
                        .map(|foe| self.bout(view, candidate, foe, brawl))
                        .max()
                        .unwrap_or(0);
                    (best, candidate.id)
                })
                .collect()
        } else if asked == brawl.foe {
            let fighter = match brawl.fighter {
                TargetSlot::First => context.first_targets.first().copied(),
                TargetSlot::This => context.source,
                TargetSlot::Second => None,
            }
            .and_then(|id| view.object(id))?;
            objects
                .iter()
                .filter_map(|id| view.object(*id))
                .map(|foe| (self.bout(view, fighter, foe, brawl), foe.id))
                .collect()
        } else {
            return None;
        };
        ranked.sort_by_key(|&(score, id)| (std::cmp::Reverse(score), id));
        // The fighter slot names somebody whenever the card requires it —
        // an unrewarding fight is still the spell the agent chose to cast —
        // but the foe slot of an "up to one" is declined rather than spent
        // on a bout that loses a creature for nothing.
        let wanted = if asked == brawl.fighter {
            usize::from(min.max(1))
        } else {
            ranked.iter().take_while(|(score, _)| *score > 0).count()
        };
        let count = wanted.max(usize::from(min)).min(usize::from(max));
        Some(PlayerAction::ChooseTargets {
            objects: ranked.into_iter().take(count).map(|(_, id)| id).collect(),
            players: vec![],
        })
    }
}
