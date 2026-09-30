//! Banding's three questions and the block it spreads (CR 702.22).
//!
//! What a band *is* lives in `combat.rs` (`AttackerInfo::band`, and the
//! damage a division sends where a player chose); this is the asking:
//!
//! - which attackers are in a band, asked as the attack is declared
//!   (CR 508.1e), of each attacker with banding in turn;
//! - a block on one member of a band blocking every member (CR 702.22h);
//! - how a creature's combat damage is divided, where banding hands that
//!   division to a player (CR 702.22j–k, 510.1d), asked as the damage step
//!   begins, before any of it is dealt (CR 510.1, then 510.2).
use super::{CardLookup, Engine, EngineError, GameEvent, ObjectId, Pending, PlanKind, PlayerId};
use crate::choice::{ChoicePrompt, NumberPrompt};
use crate::combat::{Division, OwedDivision};
use crate::turn::Step;
use baylee_cards_dsl::KeywordSet;

impl<L: CardLookup> Engine<L> {
    /// Whether `id` has banding now.
    fn has_banding(&self, id: ObjectId) -> bool {
        self.state
            .object(id)
            .is_some_and(|o| o.characteristics().keywords.contains(KeywordSet::BANDING))
    }

    /// Asks the band of the next attacker with banding after `after` in
    /// declaration order (from the first when `None`) that is in no band
    /// and has a creature to band with. Returns whether it asked.
    ///
    /// Called as the declaration is accepted, so the question stands before
    /// the machine collects a single attack trigger: bands are announced as
    /// part of the declaration (CR 508.1e), and abilities trigger on it
    /// only once it is complete (CR 508.1m).
    pub(super) fn ask_band(&mut self, player: PlayerId, after: Option<ObjectId>) -> bool {
        let attackers = self.state.combat.attackers().to_vec();
        let start = after.map_or(0, |a| {
            attackers
                .iter()
                .position(|i| i.creature == a)
                .map_or(attackers.len(), |at| at + 1)
        });
        for leader in &attackers[start..] {
            if leader.band.is_some() || !self.has_banding(leader.creature) {
                continue;
            }
            // CR 702.22d: every creature in a band attacks the same player
            // or planeswalker, and each is in one band at most (702.22c).
            let options: Vec<ObjectId> = attackers
                .iter()
                .filter(|a| {
                    a.creature != leader.creature
                        && a.band.is_none()
                        && a.defending == leader.defending
                })
                .map(|a| a.creature)
                .collect();
            if options.is_empty() {
                continue;
            }
            let max = u8::try_from(options.len()).unwrap_or(u8::MAX);
            self.pending_plan = Some(PlanKind::Band {
                leader: leader.creature,
            });
            self.pending = Pending::ChooseCards {
                player,
                options,
                min: 0,
                max,
                prompt: ChoicePrompt::Band {
                    with: leader.creature,
                },
            };
            self.awaiting_answer = true;
            return true;
        }
        false
    }

    /// The answer to [`Self::ask_band`]: `leader` and `members` are one band,
    /// unless `members` is empty; then the next attacker with banding is
    /// asked.
    ///
    /// The menu has already been checked against the offer. What it cannot
    /// say is CR 702.22c's "up to one attacking creature without banding",
    /// so that is refused here, with the question left standing.
    pub(super) fn answer_band(
        &mut self,
        player: PlayerId,
        leader: ObjectId,
        members: Vec<ObjectId>,
    ) -> Result<(), EngineError> {
        let without = members.iter().filter(|m| !self.has_banding(**m)).count();
        if without > 1 {
            self.pending_plan = Some(PlanKind::Band { leader });
            return Err(EngineError::IllegalAction(
                "a band holds at most one creature without banding",
            ));
        }
        if !members.is_empty() {
            let mut band = Vec::with_capacity(members.len() + 1);
            band.push(leader);
            band.extend(members.iter().copied());
            self.state.combat.form_band(&band);
            // Announced (CR 508.1e): every player may read the band.
            for object in members {
                self.state.journal.record(GameEvent::Banded {
                    object,
                    with: leader,
                });
            }
        }
        self.ask_band(player, Some(leader));
        Ok(())
    }

    /// Every other member of a blocked attacker's band becomes blocked by
    /// the same creature (CR 702.22h), once the declared blocks are made.
    ///
    /// No legality is asked of these pairs: the rule makes the block, and
    /// its own example is a flier's band mate blocked by what could block
    /// only the flier. Each pair is journalled as the declared ones are,
    /// so "whenever this blocks a creature" and "whenever this becomes
    /// blocked by a creature" trigger once for each (CR 509.3b, 509.3d).
    pub(super) fn spread_blocks_through_bands(&mut self, declared: &[(ObjectId, ObjectId)]) {
        for &(blocker, attacker) in declared {
            for mate in self.state.combat.band_mates(attacker) {
                if self.state.combat.is_blocking(blocker, mate) {
                    continue;
                }
                self.state.combat.declare_block(blocker, mate);
                self.state.journal.record(GameEvent::BecameBlocker {
                    object: blocker,
                    attacker: mate,
                });
            }
        }
    }

    /// Asks the next division of combat damage owed before the step being
    /// entered deals its damage, if one is. Returns whether it asked.
    ///
    /// Asked as the step that ends — declare blockers, or the first-strike
    /// damage step — hands over to the damage step, which is where the
    /// damage is dealt (`advance_step`): nobody holds priority in between
    /// (CR 510.1, 510.2), so no answer can meet a board it was not given.
    pub(super) fn ask_combat_division(&mut self) -> bool {
        let first_strike_step = match self.state.turn.step {
            Step::DeclareBlockers => self.any_first_or_double_striker(),
            Step::CombatDamageFirst => false,
            _ => return false,
        };
        let Some(owed) = crate::combat::divisions_owed(&self.state, first_strike_step)
            .into_iter()
            .next()
        else {
            return false;
        };
        self.ask_share(owed, Vec::new());
        true
    }

    /// Asks the next recipient's share of `owed`.
    fn ask_share(&mut self, owed: OwedDivision, shares: Vec<i16>) {
        let given: i16 = shares.iter().sum();
        let left = u32::try_from(owed.amount - given).unwrap_or(0);
        let index = shares.len();
        self.pending = Pending::ChooseNumber {
            player: owed.chooser,
            min: 0,
            max: left,
            reason: NumberPrompt::CombatDamage {
                source: owed.source,
                recipient: owed.recipients[index],
                index: u8::try_from(index).unwrap_or(u8::MAX),
                of: u8::try_from(owed.recipients.len()).unwrap_or(u8::MAX),
                left,
            },
        };
        self.pending_plan = Some(PlanKind::CombatDamage { owed, shares });
        self.awaiting_answer = true;
    }

    /// One share of a division. The next recipient is asked, or the last
    /// takes what is left; then the next owed division is asked, or the
    /// step the damage belongs to begins and deals it.
    ///
    /// Straight to `advance_step` and not back through the priority round:
    /// that round was complete when the first share was asked, and entered
    /// again it would open a new one in a step every player has passed.
    pub(super) fn answer_share(&mut self, owed: OwedDivision, mut shares: Vec<i16>, n: u32) {
        shares.push(i16::try_from(n).unwrap_or(i16::MAX));
        if shares.len() + 1 < owed.recipients.len() {
            self.ask_share(owed, shares);
            return;
        }
        let given: i16 = shares.iter().sum();
        shares.push(owed.amount - given);
        self.state.combat.record_division(Division {
            source: owed.source,
            shares: owed.recipients.iter().copied().zip(shares).collect(),
        });
        if !self.ask_combat_division() {
            self.advance_step();
        }
    }
}
