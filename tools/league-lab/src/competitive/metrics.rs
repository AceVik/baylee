use super::queue::Ticket;
use super::rating::{Change, Player};
use crate::league::{CHAMPION, DIAMOND, GOLD, GRANDMASTER, MASTER, REVISED, Record, standings};

#[derive(Default)]
pub(super) struct Metrics {
    pub attempts: u64,
    pub played: u64,
    pub expired: u64,
    pub elite_attempts: u64,
    pub elite_expired: u64,
    wait: Vec<u64>,
    expired_wait: Vec<u64>,
    pub wide: u64,
    pub lopsided: u64,
    pub rematches: u64,
    pub brier: f64,
    prediction_error: f64,
    movement: (u64, u64, u64),
    pub wrong_wins: u64,
    pub gain: f64,
    pub loss: f64,
    pub max_gain: f64,
    pub max_loss: f64,
    pub min_master: Option<u16>,
    pub ever_master: Vec<bool>,
    pub tiers: [usize; 11],
}
fn pct(part: u64, total: u64) -> f64 {
    100.0 * part as f64 / total.max(1) as f64
}
impl Metrics {
    pub(super) fn new(n: usize) -> Self {
        Self {
            wait: vec![0; 301],
            expired_wait: vec![0; 301],
            ever_master: vec![false; n],
            ..Self::default()
        }
    }
    pub(super) fn timeout(&mut self, t: Ticket, elite: bool) {
        self.expired += 1;
        self.elite_expired += u64::from(elite);
        self.expired_wait[(t.deadline - t.at) as usize] += 1;
    }
    pub(super) fn game(
        &mut self,
        tickets: [Ticket; 2],
        now: u32,
        result: Change,
        true_p: f64,
        won: bool,
        repeat: bool,
    ) {
        self.played += 1;
        for t in tickets {
            self.wait[(now - t.at) as usize] += 1;
        }
        self.wide += u64::from(result.wide);
        self.lopsided += u64::from(true_p.min(1.0 - true_p) < 0.25);
        self.rematches += u64::from(repeat);
        self.brier += (result.p_a - f64::from(u8::from(won))).powi(2);
        self.prediction_error += (result.p_a - true_p).powi(2);
        if result.wide && result.upset {
            self.wrong_wins += 1;
            self.gain += result.delta[result.low];
            self.loss -= result.delta[1 - result.low];
            self.max_gain = self.max_gain.max(result.delta[result.low]);
            self.max_loss = self.max_loss.max(-result.delta[1 - result.low]);
        }
    }
    pub(super) fn publish(
        &mut self,
        players: &[Player],
        day: u16,
        divisions: &mut crate::divisions::Comparison,
    ) {
        let records: Vec<Record> = players.iter().map(|p| p.record).collect();
        let table = standings(&records, REVISED, day);
        divisions.update(&table.tiers, &records, day);
        self.movement = divisions.movement();
        self.tiers.fill(0);
        for (id, tier) in table.tiers.into_iter().enumerate() {
            self.tiers[tier] += 1;
            if tier >= MASTER {
                let age = day - records[id].first;
                assert!(
                    age >= crate::league::MASTER_AGE && records[id].games >= REVISED.gate(MASTER).1
                );
                self.min_master = Some(self.min_master.map_or(age, |a| a.min(age)));
                self.ever_master[id] = true;
            }
        }
        assert!(self.tiers[CHAMPION] <= crate::league::CHAMPION_LIMIT);
    }
    fn quantile(&self, p: u64, all: bool) -> String {
        let total = if all { self.attempts } else { self.played * 2 };
        if total == 0 {
            return String::new();
        }
        let threshold = (total * p).div_ceil(100);
        let mut n = 0;
        self.wait
            .iter()
            .zip(&self.expired_wait)
            .position(|(&count, &expired)| {
                n += count + if all { expired } else { 0 };
                n >= threshold
            })
            .expect("every ticket has a measured exit")
            .to_string()
    }
    pub(super) fn report(&self, players: &[Player], skills: &[i32]) -> String {
        assert_eq!(self.attempts, 2 * self.played + self.expired);
        let n = players.len() as f64;
        let rmse = (players
            .iter()
            .zip(skills)
            .map(|(p, s)| f64::from(p.record.elo - s).powi(2))
            .sum::<f64>()
            / n)
            .sqrt();
        let drift = players
            .iter()
            .map(|p| f64::from(p.record.elo - 1500))
            .sum::<f64>()
            / n;
        let upsets = self.wrong_wins.max(1) as f64;
        let mut values = vec![
            self.attempts.to_string(),
            self.played.to_string(),
            self.expired.to_string(),
            self.elite_attempts.to_string(),
            self.elite_expired.to_string(),
            self.quantile(50, false),
            self.quantile(95, false),
            self.quantile(95, true),
        ];
        for v in [
            pct(2 * self.played, self.attempts),
            pct(self.elite_expired, self.elite_attempts),
            pct(self.wide, self.played),
            pct(self.lopsided, self.played),
            pct(self.rematches, self.played),
            self.brier / self.played.max(1) as f64,
            100.0 * (self.prediction_error / self.played.max(1) as f64).sqrt(),
            rmse,
            drift,
            self.gain / upsets,
            self.loss / upsets,
            self.max_gain,
            self.max_loss,
        ] {
            values.push(format!("{v:.5}"));
        }
        for tier in [GOLD, DIAMOND, MASTER, GRANDMASTER, CHAMPION] {
            values.push(self.tiers[tier..].iter().sum::<usize>().to_string());
        }
        values.push(self.ever_master.iter().filter(|&&v| v).count().to_string());
        values.push(self.min_master.map_or(String::new(), |v| v.to_string()));
        let (days, changes, back) = self.movement;
        values.extend([days, changes, back].map(|n| n.to_string()));
        values.join(",")
    }
}
pub(super) const HEADER: &str = "scenario,seed,accounts,matcher,rule,attempts,matches,expired,elite_attempts,elite_expired,wait_p50_s,wait_p95_s,all_exit_p95_s,served_pct,elite_timeout_pct,wide_pct,true_lopsided_pct,repeat_6h_pct,brier,forecast_rmse_pp,skill_rmse,mean_drift,wide_upset_gain,wide_upset_loss,max_wide_gain,max_wide_loss,gold_plus,diamond_plus,master_plus,gm_plus,champion,ever_master,first_master_elapsed,division_days,division_changes,division_reversals";
