//! "This before that", kept only as far as it fits: pairs arrive heaviest
//! first, and one that would close a circle is refused, so the order always
//! exists and a contradiction costs the lighter pair and nothing else.

/// What became of a pair offered to a [`Plan`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Added {
    /// It is in, and says something new.
    New,
    /// What it says already follows from the plan.
    Already,
    /// The plan says the opposite.
    Refused,
}

/// A growing partial order over `0..n`.
pub(super) struct Plan {
    /// Who comes directly after whom.
    after: Vec<Vec<usize>>,
    /// Row `a`, bit `b`: `a` comes before `b`, directly or not.
    before: Vec<Vec<u64>>,
}

impl Plan {
    pub(super) fn new(n: usize) -> Self {
        Self {
            after: vec![Vec::new(); n],
            before: vec![vec![0; n.div_ceil(64)]; n],
        }
    }

    fn comes_before(&self, first: usize, then: usize) -> bool {
        self.before[first][then / 64] >> (then % 64) & 1 == 1
    }

    /// Offers "`first` before `then`".
    pub(super) fn add(&mut self, first: usize, then: usize) -> Added {
        if first == then || self.comes_before(then, first) {
            return Added::Refused;
        }
        if self.comes_before(first, then) {
            return Added::Already;
        }
        self.after[first].push(then);
        let mut reached = self.before[then].clone();
        reached[then / 64] |= 1 << (then % 64);
        for row in 0..self.before.len() {
            if row == first || self.comes_before(row, first) {
                for (word, more) in self.before[row].iter_mut().zip(&reached) {
                    *word |= more;
                }
            }
        }
        Added::New
    }

    /// The whole order: whatever the plan leaves open stays in the order it
    /// came in, so a suggestion moves nothing it has no reason to move.
    pub(super) fn order(&self) -> Vec<usize> {
        let mut waiting_on = vec![0_usize; self.after.len()];
        for then in self.after.iter().flatten() {
            waiting_on[*then] += 1;
        }
        let mut order = Vec::with_capacity(self.after.len());
        let mut placed = vec![false; self.after.len()];
        while order.len() < self.after.len() {
            let Some(next) = (0..self.after.len()).find(|&i| !placed[i] && waiting_on[i] == 0)
            else {
                // Unreachable: `add` refuses every circle.
                break;
            };
            placed[next] = true;
            order.push(next);
            for then in &self.after[next] {
                waiting_on[*then] -= 1;
            }
        }
        order
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pair_that_closes_a_circle_is_refused_and_the_order_stands() {
        let mut plan = Plan::new(3);
        assert_eq!(plan.add(0, 1), Added::New);
        assert_eq!(plan.add(1, 2), Added::New);
        assert_eq!(plan.add(2, 0), Added::Refused);
        assert_eq!(plan.add(0, 2), Added::Already);
        assert_eq!(plan.order(), vec![0, 1, 2]);
    }

    #[test]
    fn what_the_plan_leaves_open_stays_as_it_came() {
        let mut plan = Plan::new(4);
        plan.add(3, 1);
        assert_eq!(plan.order(), vec![0, 2, 3, 1]);
    }

    #[test]
    fn the_plan_holds_past_one_word() {
        let mut plan = Plan::new(130);
        for i in 0..129 {
            assert_eq!(plan.add(i + 1, i), Added::New);
        }
        assert_eq!(plan.add(0, 129), Added::Refused);
        assert_eq!(plan.order()[0], 129);
    }
}
