//! Explicit, reversible controls for the attack declaration being drafted.
use super::{Defender, Interaction, Mode, ObjectId};

/// A local edit. None of these sends the declaration to the host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttackOption {
    /// Aim subsequent declarations at this defender.
    Aim(Defender),
    /// Send only creatures that have not already been assigned.
    AddRemaining,
    /// Withdraw every attacker without changing the aim.
    WithdrawAll,
    /// Withdraw an assigned creature, or send an unassigned one at the aim.
    Toggle(ObjectId),
}

impl Interaction {
    /// Stable option order while a declaration is edited: defenders, bulk
    /// controls, then the engine's candidates. Invalid or foreign prompts
    /// offer nothing.
    #[must_use]
    pub fn attack_options(&self) -> Vec<AttackOption> {
        if !self.is_mine() {
            return Vec::new();
        }
        let Mode::Attackers {
            candidates,
            defenders,
            ..
        } = &self.mode
        else {
            return Vec::new();
        };
        defenders
            .iter()
            .copied()
            .map(AttackOption::Aim)
            .chain([AttackOption::AddRemaining, AttackOption::WithdrawAll])
            .chain(candidates.iter().copied().map(AttackOption::Toggle))
            .collect()
    }

    /// Apply an option after revalidating it against the current prompt.
    /// Bulk addition preserves attacks already assigned to other defenders.
    pub fn edit_attack(&mut self, option: AttackOption) -> bool {
        if !self.attack_options().contains(&option) {
            return false;
        }
        let Mode::Attackers {
            candidates,
            defenders,
            pairs,
            focus,
        } = &mut self.mode
        else {
            return false;
        };
        match option {
            AttackOption::Aim(defender) => {
                let Some(at) = defenders.iter().position(|d| *d == defender) else {
                    return false;
                };
                *focus = at;
            }
            AttackOption::WithdrawAll => pairs.clear(),
            AttackOption::AddRemaining => {
                let Some(defender) = defenders.get(*focus).copied() else {
                    return false;
                };
                for id in candidates {
                    if !pairs.iter().any(|(a, _)| a == id) {
                        pairs.push((*id, defender));
                    }
                }
            }
            AttackOption::Toggle(id) => {
                if let Some(at) = pairs.iter().position(|(a, _)| *a == id) {
                    pairs.remove(at);
                } else if let Some(defender) = defenders.get(*focus) {
                    pairs.push((id, *defender));
                } else {
                    return false;
                }
            }
        }
        true
    }
}
