/// Feasibility of partial attacker subsets, shared by sampling and PPO masks.
struct AttackSubset<'a> {
    required: &'a [ObjectId],
    alone: &'a [ObjectId],
    minimum: usize,
    remaining: BTreeMap<ObjectId, usize>,
    chosen: Vec<ObjectId>,
}

impl<'a> AttackSubset<'a> {
    fn new(
        ids: impl Iterator<Item = ObjectId>,
        required: &'a [ObjectId],
        alone: &'a [ObjectId],
        minimum: Option<u32>,
    ) -> Self {
        let mut remaining = BTreeMap::new();
        for id in ids {
            *remaining.entry(id).or_insert(0) += 1;
        }
        Self {
            required,
            alone,
            minimum: minimum.map_or(required.len(), |n| n as usize),
            remaining,
            chosen: Vec::new(),
        }
    }

    /// Remove this defender candidate, then test taking and skipping it.
    fn options(&mut self, id: ObjectId) -> (bool, bool) {
        if let Some(left) = self.remaining.get_mut(&id) {
            *left = left.saturating_sub(1);
        }
        let take = !self.chosen.contains(&id) && self.feasible(Some(id));
        (take, self.feasible(None))
    }

    fn take(&mut self, id: ObjectId) {
        self.chosen.push(id);
    }

    fn feasible(&self, extra: Option<ObjectId>) -> bool {
        let selected: Vec<_> = self.chosen.iter().copied().chain(extra).collect();
        let solo = selected.iter().any(|id| self.alone.contains(id));
        if solo && selected.len() != 1 {
            return false;
        }
        let obeyed = selected
            .iter()
            .filter(|id| self.required.contains(id))
            .count();
        if obeyed >= self.minimum {
            return true;
        }
        if solo {
            return false;
        }
        let remaining = |id: &&ObjectId| {
            !selected.contains(id) && self.remaining.get(*id).is_some_and(|&n| n > 0)
        };
        let together = self
            .required
            .iter()
            .filter(remaining)
            .filter(|id| !self.alone.contains(id))
            .count();
        obeyed + together >= self.minimum
            || (selected.is_empty()
                && self.minimum <= 1
                && self
                    .required
                    .iter()
                    .filter(remaining)
                    .any(|id| self.alone.contains(id)))
    }
}

#[cfg(test)]
mod attack_subset_tests {
    use super::*;

    #[test]
    fn alternative_required_solo_attackers_are_not_forced_together() {
        let ids = [ObjectId::new(1, 0), ObjectId::new(2, 0)];
        let mut choice = AttackSubset::new(ids.into_iter(), &ids, &ids, Some(1));
        assert_eq!(choice.options(ids[0]), (true, true));
        choice.take(ids[0]);
        assert_eq!(choice.options(ids[1]), (false, true));
        let mut choice = AttackSubset::new(ids.into_iter(), &ids, &ids, Some(1));
        assert_eq!(choice.options(ids[0]), (true, true));
        assert_eq!(choice.options(ids[1]), (true, false));
    }

    #[test]
    fn optional_solo_attack_cannot_displace_two_required_attackers() {
        let ids = [
            ObjectId::new(1, 0),
            ObjectId::new(2, 0),
            ObjectId::new(3, 0),
        ];
        let mut choice = AttackSubset::new(ids.into_iter(), &ids[1..], &ids[..1], Some(2));
        assert_eq!(choice.options(ids[0]), (false, true));
        assert_eq!(choice.options(ids[1]), (true, false));
        choice.take(ids[1]);
        assert_eq!(choice.options(ids[2]), (true, false));
    }
}
