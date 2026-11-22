use crate::league::Record;

pub(super) const PLACEMENT: u16 = 20;
pub(super) const WIDE: i32 = 350;
const QUALIFIES: i32 = 250;
const DAY: u32 = 86_400;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Rule {
    Classic,
    FixedCap,
    Evidence,
    UnboundedLedger,
}
impl Rule {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Classic => "classic",
            Self::FixedCap => "fixed-cap",
            Self::Evidence => "evidence",
            Self::UnboundedLedger => "no-ledger",
        }
    }
}
/// Integral differences make the logistic lookup reproducible and inexpensive.
pub(super) struct Odds(Vec<f64>);
impl Odds {
    pub(super) fn new() -> Self {
        Self(
            (-4000..=4000)
                .map(|diff| 1.0 / (1.0 + 10_f64.powf(-f64::from(diff) / 400.0)))
                .collect(),
        )
    }
    pub(super) fn at(&self, diff: i32, luck: f64) -> f64 {
        luck + (1.0 - 2.0 * luck) * self.0[(diff.clamp(-4000, 4000) + 4000) as usize]
    }
}
#[derive(Clone, Copy)]
struct Mark {
    opponent: usize,
    game: u16,
    time: u32,
}
#[derive(Clone, Copy, Default)]
struct Evidence([Option<Mark>; 3]);
impl Evidence {
    fn count(self, id: usize, game: u16, now: u32) -> u8 {
        self.0
            .iter()
            .flatten()
            .filter(|m| m.opponent != id && game - m.game <= 12 && now - m.time <= 14 * DAY)
            .count() as u8
    }
    fn add(&mut self, mark: Mark) {
        if let Some(slot) = self
            .0
            .iter_mut()
            .find(|m| m.is_some_and(|m| m.opponent == mark.opponent))
        {
            *slot = Some(mark);
        } else {
            let slot = self
                .0
                .iter_mut()
                .min_by_key(|m| m.map(|m| m.game))
                .expect("three slots");
            *slot = Some(mark);
        }
    }
}

#[derive(Clone)]
pub(super) struct Player {
    pub record: Record,
    score: f64,
    allowance: f64,
    placement: f64,
    as_low: Evidence,
    as_high: Evidence,
}
impl Player {
    pub(super) fn at(elo: i32, games: u16) -> Self {
        Self {
            record: Record {
                elo,
                games,
                first: if games == 0 { u16::MAX } else { 0 },
                last: 0,
            },
            score: f64::from(elo),
            allowance: 0.0,
            placement: f64::from(elo),
            as_low: Evidence::default(),
            as_high: Evidence::default(),
        }
    }
    pub(super) fn queue_elo(&self, rule: Rule) -> i32 {
        if rule == Rule::Classic {
            return self.record.elo;
        }
        let remaining = PLACEMENT.saturating_sub(self.record.games);
        let offset = (self.placement - self.score).clamp(-240.0, 240.0) * f64::from(remaining)
            / f64::from(PLACEMENT);
        (self.score + offset).round() as i32
    }
    fn apply(&mut self, delta: f64, fast: f64, day: u16) {
        if self.record.games < PLACEMENT {
            self.placement += fast;
        }
        self.score += delta;
        self.record
            .played(day, self.score.round() as i32 - self.record.elo);
    }
}

#[derive(Clone, Copy)]
pub(super) struct Encounter {
    pub ids: [usize; 2],
    pub now: u32,
    pub repeats: usize,
    pub a_won: bool,
}

#[derive(Clone, Copy)]
pub(super) struct Change {
    pub delta: [f64; 2],
    pub p_a: f64,
    pub low: usize,
    pub wide: bool,
    pub upset: bool,
    pub streak: [u8; 2], // prior independent signals for the underdog and favourite
}

// Split only the two quotes' imbalance between the available personal
// allowances. The remaining transfers sum to zero. A new opponent can
// absorb part of the difference even if the first account is saturated.
fn balance(players: &mut [&mut Player; 2], points: &mut [f64; 2], ordinary: bool) {
    let imbalance = points[0] + points[1];
    let sign = imbalance.signum();
    let room = players
        .each_ref()
        .map(|p| (64.0 - sign * p.allowance).max(0.0));
    let total = imbalance.abs().min(room[0] + room[1]);
    let first = (total / 2.0).clamp((total - room[1]).max(0.0).min(room[0]), room[0]);
    players[0].allowance += sign * first;
    players[1].allowance += sign * (total - first);
    let larger = usize::from(points[1].abs() > points[0].abs());
    points[larger] -= imbalance - sign * total;
    for (p, amount) in players.iter_mut().zip(points.iter_mut()) {
        if ordinary {
            let release =
                p.allowance.signum() * p.allowance.abs().min(1.0).min(0.25 * amount.abs());
            p.allowance -= release;
            *amount -= release;
        }
    }
}

pub(super) fn settle(
    mut players: [&mut Player; 2],
    game: Encounter,
    rule: Rule,
    odds: &Odds,
) -> Change {
    let gap = players[0].record.elo - players[1].record.elo;
    let low = usize::from(gap > 0);
    let high = 1 - low;
    let p_low = odds.at(-gap.abs(), if rule == Rule::Classic { 0.0 } else { 0.04 });
    let p_a = if low == 0 { p_low } else { 1.0 - p_low };
    let a_score = f64::from(u8::from(game.a_won));
    let upset = (low == 0) == game.a_won;
    let mut streak = [0, 0];
    let uses_history = matches!(rule, Rule::Evidence | Rule::UnboundedLedger);
    if uses_history {
        streak[0] = players[low]
            .as_low
            .count(game.ids[high], players[low].record.games, game.now);
        streak[1] =
            players[high]
                .as_high
                .count(game.ids[low], players[high].record.games, game.now);
    }
    let mut delta = [0.0; 2];
    if rule == Rule::Classic {
        delta[0] = (24.0 * (a_score - p_a)).round();
        delta[1] = -delta[0];
    } else {
        let blend = (f64::from(gap.abs() - 150) / f64::from(WIDE - 150)).clamp(0.0, 1.0);
        let standard = 24.0 * (1.0 - p_low);
        let gain = 20.0 + 4.0 * f64::from(streak[0]);
        let loss = if players[low].record.games < PLACEMENT {
            10.0
        } else {
            14.0 + 4.0 * f64::from(streak[1])
        };
        let weight = match game.repeats {
            0 | 1 => 1.0,
            2 | 3 => 0.5,
            _ => 0.25,
        };
        let factor = if upset {
            weight
        } else {
            -weight * p_low / (1.0 - p_low)
        };
        delta[low] = (standard + (gain - standard) * blend) * factor;
        delta[high] = -(standard + (loss - standard) * blend) * factor;
        let ordinary = gap.abs() < QUALIFIES || !upset;
        if rule != Rule::UnboundedLedger {
            balance(&mut players, &mut delta, ordinary);
        }
    }
    let fast_p = odds.at(
        players[0].queue_elo(rule) - players[1].queue_elo(rule),
        0.04,
    );
    let fast = 56.0 * (a_score - fast_p);
    if gap.abs() >= QUALIFIES && uses_history {
        let established = players.each_ref().map(|p| p.record.games >= PLACEMENT);
        for (i, is_low) in [(low, true), (high, false)] {
            let other = 1 - i;
            if !established[other] {
                continue;
            }
            let mark = Mark {
                opponent: game.ids[other],
                game: players[i].record.games,
                time: game.now,
            };
            let history = if is_low {
                &mut players[i].as_low
            } else {
                &mut players[i].as_high
            };
            if upset {
                history.add(mark);
            } else {
                *history = Evidence::default();
            }
        }
    }
    for (i, p) in players.into_iter().enumerate() {
        p.apply(
            delta[i],
            if i == 0 { fast } else { -fast },
            (game.now / DAY) as u16,
        );
    }
    Change {
        delta,
        p_a,
        low,
        wide: gap.abs() >= WIDE,
        upset,
        streak,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extreme_allowances_keep_the_score_direction_caps_and_conservation() {
        for a in [-64.0, -63.0, 0.0, 63.0, 64.0] {
            for b in [-64.0, -63.0, 0.0, 63.0, 64.0] {
                for desired in [[32.0, -10.0], [20.0, -26.0], [-0.95, 0.50], [12.0, -12.0]] {
                    for ordinary in [false, true] {
                        let mut p = Player::at(1500, 100);
                        let mut q = p.clone();
                        p.allowance = a;
                        q.allowance = b;
                        let mut actual = desired;
                        balance(&mut [&mut p, &mut q], &mut actual, ordinary);
                        for (x, y) in actual.into_iter().zip(desired) {
                            assert!(x * y > 0.0);
                        }
                        assert!(p.allowance.abs() <= 64.0 && q.allowance.abs() <= 64.0);
                        assert!(
                            (actual.iter().sum::<f64>() - (p.allowance + q.allowance - a - b))
                                .abs()
                                < 1e-10
                        );
                        if !ordinary {
                            assert!(actual[0].abs() <= desired[0].abs() + 1e-10);
                            assert!(actual[1].abs() <= desired[1].abs() + 1e-10);
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn still_needs_recent_matches_against_distinct_accounts() {
        let mut e = Evidence::default();
        e.add(Mark {
            opponent: 7,
            game: 1,
            time: 1,
        });
        e.add(Mark {
            opponent: 7,
            game: 2,
            time: 2,
        });
        assert_eq!(e.count(7, 3, 3), 0);
        assert_eq!(e.count(8, 14, 14 * DAY + 2), 1);
        assert_eq!(e.count(8, 15, 14 * DAY + 2), 0);
        assert_eq!(e.count(8, 14, 14 * DAY + 3), 0);
    }
}
