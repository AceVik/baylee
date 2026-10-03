//! Stable identities and provenance for resolved prevention effects.

use super::Shield;
use baylee_core::ids::{AbilityRef, ObjectId};

/// Public origin of a resolved shield, retained after its source leaves.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ShieldOrigin {
    /// Source object of the spell or ability.
    pub source: ObjectId,
    /// Printed ability, if one generated the effect.
    pub ability: Option<AbilityRef>,
}

/// Insertion-ordered shields with identities which are never reused.
/// Removing an earlier shield cannot change a suspended choice's meaning.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ShieldStore {
    next_id: u64,
    shields: Vec<Shield>,
    identities: Vec<(u64, Option<ShieldOrigin>)>,
}

impl ShieldStore {
    /// Register a shield with no printed origin (principally rules fixtures).
    pub fn push(&mut self, shield: Shield) {
        self.push_from(shield, None);
    }

    /// Register a shield and retain its origin for later decisions.
    ///
    /// # Panics
    /// Panics after exhausting all `u64` identities in one game.
    pub fn push_from(&mut self, shield: Shield, origin: Option<ShieldOrigin>) -> u64 {
        let id = self.next_id;
        self.next_id = self
            .next_id
            .checked_add(1)
            .expect("shield identity exhausted");
        self.shields.push(shield);
        self.identities.push((id, origin));
        id
    }

    /// Remove one shield, returning its rules data.
    pub fn remove(&mut self, index: usize) -> Shield {
        self.identities.remove(index);
        self.shields.remove(index)
    }

    /// End shields matching no longer applicable durations or owners.
    pub fn retain(&mut self, mut keep: impl FnMut(&Shield) -> bool) {
        for i in (0..self.shields.len()).rev() {
            if !keep(&self.shields[i]) {
                self.remove(i);
            }
        }
    }

    /// End every shield without resetting the identity source.
    pub fn clear(&mut self) {
        self.shields.clear();
        self.identities.clear();
    }

    /// Shields with their stable identity and retained origin.
    pub(crate) fn identified(&self) -> impl Iterator<Item = (u64, &Shield, Option<ShieldOrigin>)> {
        self.shields
            .iter()
            .zip(&self.identities)
            .map(|(shield, &(id, origin))| (id, shield, origin))
    }

    /// Locate a stable identity after other shields were removed.
    pub(crate) fn position(&self, id: u64) -> Option<usize> {
        self.identities
            .iter()
            .position(|&(candidate, _)| candidate == id)
    }
}

impl std::ops::Deref for ShieldStore {
    type Target = [Shield];
    fn deref(&self) -> &Self::Target {
        &self.shields
    }
}

impl std::ops::Index<usize> for ShieldStore {
    type Output = Shield;
    fn index(&self, index: usize) -> &Self::Output {
        &self.shields[index]
    }
}

impl std::ops::IndexMut<usize> for ShieldStore {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.shields[index]
    }
}

impl<'a> IntoIterator for &'a ShieldStore {
    type Item = &'a Shield;
    type IntoIter = std::slice::Iter<'a, Shield>;
    fn into_iter(self) -> Self::IntoIter {
        self.shields.iter()
    }
}

impl From<Vec<Shield>> for ShieldStore {
    fn from(shields: Vec<Shield>) -> Self {
        let mut store = Self::default();
        for shield in shields {
            store.push(shield);
        }
        store
    }
}
