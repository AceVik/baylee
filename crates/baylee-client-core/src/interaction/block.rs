//! A named list for the block declaration being drafted, beside the table.
//!
//! Report 31 (12.10.2026): *"with many creatures it's hard to click the
//! right ones to block"*. On the table a block is two clicks on two small
//! cards; this is the same declaration as rows of names, the way the attack
//! declaration has been since its own list ([`super::AttackOption`]). Every
//! row is a click the table already takes ([`Interaction::toggle`]): an
//! attacker aims the next block, a creature blocks the attacker in aim or is
//! taken back. Which creature may block which attacker is the engine's
//! `BlockOption` list and nothing else.

use super::{Interaction, Mode, ObjectId, SelectionOutcome};

/// One row of the block list. None of these sends the declaration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockRow {
    /// Aim the next block at this attacker.
    Aim(ObjectId),
    /// Block the attacker in aim with this creature, or, where it already
    /// blocks, take that block back.
    Toggle(ObjectId),
}

impl Interaction {
    /// The rows while this seat declares blockers: every attacker a block may
    /// be put in front of, then the creatures that may block the one in aim
    /// and the ones already blocking (so each can be taken back), in the
    /// engine's order. Empty for any other question, and for another seat's.
    #[must_use]
    pub fn block_rows(&self) -> Vec<BlockRow> {
        if !self.is_mine() {
            return Vec::new();
        }
        let Mode::Blockers {
            options,
            pairs,
            attackers,
            focus,
            ..
        } = &self.mode
        else {
            return Vec::new();
        };
        let aim = attackers.get(*focus);
        attackers
            .iter()
            .copied()
            .map(BlockRow::Aim)
            .chain(
                options
                    .iter()
                    .filter(|o| {
                        aim.is_some_and(|at| o.attackers.contains(at))
                            || pairs.iter().any(|(b, _)| *b == o.blocker)
                    })
                    .map(|o| BlockRow::Toggle(o.blocker)),
            )
            .collect()
    }

    /// Takes one row, after checking it is still offered. Whether it changed
    /// anything is the answer.
    pub fn edit_block(&mut self, row: BlockRow) -> bool {
        if !self.block_rows().contains(&row) {
            return false;
        }
        let (BlockRow::Aim(id) | BlockRow::Toggle(id)) = row;
        !matches!(
            self.toggle(id),
            SelectionOutcome::Rejected | SelectionOutcome::Full
        )
    }

    /// The attacker `blocker` is blocking in the draft, if it is.
    #[must_use]
    pub fn blocking(&self, blocker: ObjectId) -> Option<ObjectId> {
        let Mode::Blockers { pairs, .. } = &self.mode else {
            return None;
        };
        pairs
            .iter()
            .find_map(|(b, attacker)| (*b == blocker).then_some(*attacker))
    }

    /// The attacker in aim, while blockers are declared.
    #[must_use]
    pub fn block_aim(&self) -> Option<ObjectId> {
        let Mode::Blockers {
            attackers, focus, ..
        } = &self.mode
        else {
            return None;
        };
        attackers.get(*focus).copied()
    }
}
