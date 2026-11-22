use super::rating::{Player, Rule};
use std::collections::{BTreeMap, VecDeque};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Matcher {
    Fixed,
    Adaptive,
    Balanced,
    Fifo,
}
impl Matcher {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Fixed => "fixed-200",
            Self::Adaptive => "adaptive",
            Self::Balanced => "balanced",
            Self::Fifo => "first-ready",
        }
    }
}

#[derive(Default)]
pub(super) struct Pairs {
    counts: BTreeMap<(usize, usize), usize>,
    expiry: VecDeque<(u32, (usize, usize))>,
}
impl Pairs {
    fn key(a: usize, b: usize) -> (usize, usize) {
        (a.min(b), a.max(b))
    }
    pub(super) fn expire(&mut self, now: u32) {
        while self.expiry.front().is_some_and(|(t, _)| *t <= now) {
            let (_, key) = self.expiry.pop_front().expect("checked front");
            let count = self.counts.get_mut(&key).expect("live pair");
            *count -= 1;
            if *count == 0 {
                self.counts.remove(&key);
            }
        }
    }
    pub(super) fn prior(&self, a: usize, b: usize) -> usize {
        *self.counts.get(&Self::key(a, b)).unwrap_or(&0)
    }
    pub(super) fn played(&mut self, a: usize, b: usize, now: u32) {
        let key = Self::key(a, b);
        *self.counts.entry(key).or_default() += 1;
        self.expiry.push_back((now + 6 * 3600, key));
    }
}

#[derive(Clone, Copy)]
pub(super) struct Ticket {
    pub id: usize,
    pub at: u32,
    pub deadline: u32,
}
#[derive(Default)]
pub(super) struct Queue {
    waiting: Vec<Ticket>,
    arrivals: VecDeque<u32>,
}

fn radius(age: u32, sparse: bool) -> i32 {
    let gates = if sparse {
        [10, 20, 35, 60]
    } else {
        [20, 45, 75, 120]
    };
    [100, 200, 350, 600, i32::MAX][gates.partition_point(|&t| t <= age)]
}
impl Queue {
    // Caller submits at most one live ticket per account, partitioned by legal match format.
    pub(super) fn admit(&mut self, ticket: Ticket) {
        self.arrivals.push_back(ticket.at);
        self.waiting.push(ticket);
    }
    pub(super) fn empty(&self) -> bool {
        self.waiting.is_empty()
    }
    pub(super) fn tick(&mut self, now: u32, mut timeout: impl FnMut(Ticket)) {
        while self.arrivals.front().is_some_and(|&at| now - at >= 120) {
            self.arrivals.pop_front();
        }
        self.waiting.retain(|&ticket| {
            if now >= ticket.deadline {
                timeout(ticket);
                false
            } else {
                true
            }
        });
    }
    pub(super) fn take(
        &mut self,
        now: u32,
        matcher: Matcher,
        rule: Rule,
        players: &[Player],
        pairs: &Pairs,
    ) -> Option<[Ticket; 2]> {
        let sparse = self.arrivals.len() <= 6;
        for i in 0..self.waiting.len().saturating_sub(1) {
            let first = self.waiting[i];
            let older = now - first.at;
            // Rotate tickets across a wider, still limited set of neighbours.
            // The phase uses server admission time and stable account ID.
            let variety = matcher == Matcher::Balanced
                && !sparse
                && (u64::from(first.at / 5) + first.id as u64).is_multiple_of(3);
            let limit = match matcher {
                Matcher::Adaptive => radius(older, sparse),
                Matcher::Balanced => radius(older, sparse).max(if variety { 300 } else { 100 }),
                Matcher::Fixed => 200,
                Matcher::Fifo => i32::MAX,
            };
            let first_elo = players[first.id].queue_elo(rule);
            let best = self.waiting[i + 1..]
                .iter()
                .enumerate()
                .filter_map(|(j, &second)| {
                    let gap = (first_elo - players[second.id].queue_elo(rule)).abs();
                    if gap > limit || first.id == second.id {
                        return None;
                    }
                    let younger = now - second.at;
                    let cost = match matcher {
                        Matcher::Fifo => 0,
                        Matcher::Fixed => gap,
                        Matcher::Adaptive | Matcher::Balanced => {
                            if gap > 350 && younger < 10 && older < 120 {
                                return None;
                            }
                            let repeats = i32::try_from(pairs.prior(first.id, second.id).min(3))
                                .expect("at most three");
                            let preference = if variety {
                                (gap - 300).max(0)
                            } else if matcher == Matcher::Balanced {
                                (gap - 80).max(0)
                            } else {
                                gap
                            };
                            preference + 140 * repeats - (younger.min(90) / 2).cast_signed()
                        }
                    };
                    Some((cost, j))
                })
                .min();
            if let Some((_, j)) = best {
                let second = self.waiting.remove(i + 1 + j);
                return Some([self.waiting.remove(i), second]);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ticket(id: usize, at: u32) -> Ticket {
        Ticket {
            id,
            at,
            deadline: at + 240,
        }
    }
    #[test]
    fn sparse_fallback_protects_fresh_arrivals_and_does_not_wait_forever_for_a_close_peer() {
        let players = [Player::at(2200, 100), Player::at(1200, 100)];
        let pairs = Pairs::default();
        let mut q = Queue::default();
        q.admit(ticket(0, 0));
        q.admit(ticket(1, 55));
        assert!(
            q.take(55, Matcher::Balanced, Rule::Evidence, &players, &pairs)
                .is_none()
        );
        assert!(
            q.take(60, Matcher::Balanced, Rule::Evidence, &players, &pairs)
                .is_none()
        );
        assert!(
            q.take(65, Matcher::Balanced, Rule::Evidence, &players, &pairs)
                .is_some()
        );
        let mut q = Queue::default();
        q.admit(ticket(0, 0));
        q.admit(ticket(1, 120));
        assert!(
            q.take(120, Matcher::Balanced, Rule::Evidence, &players, &pairs)
                .is_some()
        );
    }
    #[test]
    fn repeat_penalty_prefers_another_nearby_player_and_expires_at_six_hours() {
        let players = [
            Player::at(1500, 100),
            Player::at(1500, 100),
            Player::at(1540, 100),
        ];
        let mut q = Queue::default();
        for i in 0..3 {
            q.admit(ticket(i, 0));
        }
        let mut pairs = Pairs::default();
        pairs.played(0, 1, 0);
        let pair = q
            .take(0, Matcher::Balanced, Rule::Evidence, &players, &pairs)
            .unwrap();
        assert_eq!(pair.map(|t| t.id), [0, 2]);
        pairs.expire(6 * 3600);
        assert_eq!(pairs.prior(1, 0), 0);
    }
    #[test]
    fn a_lone_ticket_expires_once_and_never_gets_a_match() {
        let mut q = Queue::default();
        q.admit(ticket(0, 0));
        let players = [Player::at(1500, 100)];
        let mut count = 0;
        for now in [0, 239, 240, 245] {
            q.tick(now, |_| count += 1);
            assert!(
                q.take(
                    now,
                    Matcher::Balanced,
                    Rule::Evidence,
                    &players,
                    &Pairs::default()
                )
                .is_none()
            );
        }
        assert_eq!(count, 1);
    }
}
