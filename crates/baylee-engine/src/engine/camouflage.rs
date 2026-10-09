//! Camouflage: blocks made by piles assigned at random, instead of declared.
//!
//! "This turn, instead of declaring blockers, each defending player chooses
//! any number of creatures they control and divides them into a number of
//! piles equal to the number of attacking creatures for whom that player is
//! defending. Then each defending player assigns each of their piles to a
//! different one of those attacking creatures at random. Each creature in a
//! pile that can block the creature that pile is assigned to does so.
//! (Piles can be empty.)"
//!
//! A replacement of the declare-blockers turn-based action (CR 509.1, CR
//! 614.1), on for the rest of the turn once the spell resolves
//! (`PerTurn::camouflage`). Each defending player, in the order they would
//! declare (CR 802.4), names their piles one question at a time
//! (`ChoicePrompt::CamouflagePile`), each creature into one pile — or as
//! many as the attackers it could block, "creatures … that can block
//! additional creatures may likewise be put into additional piles" — and
//! what is left unnamed is in none. The piles go to the attackers at random,
//! drawn from the game's seeded generator (the owner's decision of
//! 09.10.2026), so a replay deals them the same. A creature then blocks the
//! attacker its pile went to if it can (`combat::can_block`: evasion,
//! protection, Raging River's labels); an attacker that can only be blocked
//! by two or more (menace) is blocked by its pile only if two or more of it
//! can. Requirements to block (CR 509.1c) bind a declaration, and there is
//! none. The blocks are made as declared ones are (`make_blocks`), banding
//! spreading them (CR 702.22h), and the next defending player is asked.
use super::{CardLookup, Engine, ObjectId, Pending, PlanKind, PlayerId};
use crate::choice::ChoicePrompt;
use crate::combat;
use baylee_core::types::TypeSet;

impl<L: CardLookup> Engine<L> {
    /// The attackers `defending` is defending against, in declaration order.
    fn attackers_of(&self, defending: PlayerId) -> Vec<ObjectId> {
        self.state
            .combat
            .attackers()
            .iter()
            .filter(|a| combat::blocking_player(&self.state, a.defending) == Some(defending))
            .map(|a| a.creature)
            .collect()
    }

    /// The creatures `defending` controls that may go into the next pile:
    /// those in no pile yet, and "creatures those players control that can
    /// block additional creatures may likewise be put into additional
    /// piles", as many as the attackers they could block (CR 509.1a).
    fn pile_candidates(&self, defending: PlayerId, taken: &[Vec<ObjectId>]) -> Vec<ObjectId> {
        let rules = combat::BlockRules::new(&self.state);
        let room = |id: &ObjectId| {
            let used = taken.iter().filter(|pile| pile.contains(id)).count();
            rules.capacity(*id).is_none_or(|most| used < most)
        };
        self.state
            .battlefield_view()
            .iter()
            .copied()
            .filter(|&id| {
                self.state.object(id).is_some_and(|o| {
                    o.controller == defending
                        && !o.status.contains(crate::object::Status::PHASED_OUT)
                        && o.characteristics().types.contains(TypeSet::CREATURE)
                })
            })
            .filter(room)
            .collect()
    }

    /// Instead of asking `defending` to declare blockers: the first pile,
    /// or, with no attacker or no creature to divide, no blocks at all.
    pub(super) fn ask_camouflage(&mut self, defending: PlayerId) {
        let of = u8::try_from(self.attackers_of(defending).len()).unwrap_or(u8::MAX);
        self.next_pile(defending, Vec::new(), of);
    }

    fn next_pile(&mut self, defending: PlayerId, mut piles: Vec<Vec<ObjectId>>, of: u8) {
        let options = self.pile_candidates(defending, &piles);
        if piles.len() >= usize::from(of) || options.is_empty() {
            piles.resize(usize::from(of), Vec::new());
            self.assign_piles(defending, &piles);
            return;
        }
        let pile = u8::try_from(piles.len() + 1).unwrap_or(u8::MAX);
        let max = u8::try_from(options.len()).unwrap_or(u8::MAX);
        self.pending_plan = Some(PlanKind::CamouflagePiles {
            defending,
            piles,
            of,
        });
        self.pending = Pending::ChooseCards {
            player: defending,
            options,
            min: 0,
            max,
            prompt: ChoicePrompt::CamouflagePile { pile, of },
            total: None,
        };
        self.awaiting_answer = true;
    }

    /// One pile named; the next is asked, or the piles are assigned.
    pub(super) fn answer_camouflage(
        &mut self,
        defending: PlayerId,
        mut piles: Vec<Vec<ObjectId>>,
        of: u8,
        chosen: Vec<ObjectId>,
    ) {
        piles.push(chosen);
        self.next_pile(defending, piles, of);
    }

    /// Assigns each pile to a different attacker at random and makes the
    /// blocks its creatures can make.
    fn assign_piles(&mut self, defending: PlayerId, piles: &[Vec<ObjectId>]) {
        let mut attackers = self.attackers_of(defending);
        if !piles.iter().all(Vec::is_empty) {
            self.state.rng.shuffle(&mut attackers);
        }
        let mut blocks: Vec<(ObjectId, ObjectId)> = Vec::new();
        for (pile, &attacker) in piles.iter().zip(&attackers) {
            let able: Vec<ObjectId> = pile
                .iter()
                .copied()
                .filter(|&creature| combat::can_block(&self.state, defending, creature, attacker))
                .collect();
            let enough = combat::block_bound(&self.state, attacker).is_none_or(|bound| {
                u32::try_from(able.len()).unwrap_or(u32::MAX) >= bound.min_blockers
            });
            if enough {
                blocks.extend(able.into_iter().map(|creature| (creature, attacker)));
            }
        }
        self.make_blocks(defending, &blocks);
    }
}
