//! What a profile's state is, from what happened to it: a pure function
//! of the facts and the clock the caller reads, so every transition can be
//! held to a table (`docs/llm-seat.md` §"A hosted seat").

use baylee_client_core::llmseat::Caps;
use baylee_client_core::llmseat::ledger::Spent;
use baylee_protocol::seathost::State;

/// How long a limit that named no end is waited out before the next check,
/// in minutes, one step further each time it is met again; the last step
/// holds. A game that ends well starts it over.
pub const BACKOFF_MINS: [i64; 5] = [5, 10, 20, 40, 60];

/// How often a profile that failed its check, or whose CLI is signed out,
/// is checked again.
pub const RECHECK_SECS: i64 = 3600;

/// How a check went.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Probe {
    /// It can play.
    Ok,
    /// A CLI is signed out: only its operator can fix that.
    SignedOut(String),
    /// The provider says it is limited, for `lifts_secs` when it said.
    Limited {
        /// Why, one sentence.
        why: String,
        /// When it lifts, if said.
        lifts_secs: Option<i64>,
    },
    /// Anything else: no key, an endpoint refusing, no program.
    Failing(String),
}

/// What happened to one profile, kept by the seat agent.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Runtime {
    /// Live bridges.
    pub games: u32,
    /// A check is running.
    pub probing: bool,
    /// Limited until then (Unix seconds).
    pub limited_until: Option<i64>,
    /// The next backoff step.
    pub backoff_step: usize,
    /// A CLI signed out.
    pub needs_login: bool,
    /// Its check failed, and why.
    pub failing: Option<String>,
    /// The last failure of any kind.
    pub last_error: Option<String>,
    /// The last success.
    pub last_ok: Option<i64>,
    /// When it is checked next; `None` = no check is due on its own.
    pub next_probe: Option<i64>,
}

impl Runtime {
    /// A runtime that has never been checked: checked at once.
    #[must_use]
    pub fn fresh(now: i64) -> Self {
        Self {
            next_probe: Some(now),
            ..Self::default()
        }
    }

    /// Whether a check is due at `now`.
    #[must_use]
    pub fn probe_due(&self, now: i64) -> bool {
        !self.probing && self.next_probe.is_some_and(|at| at <= now)
    }

    /// What a check found.
    pub fn probed(&mut self, probe: Probe, now: i64) {
        self.probing = false;
        match probe {
            Probe::Ok => {
                self.needs_login = false;
                self.failing = None;
                self.limited_until = None;
                self.last_ok = Some(now);
                self.next_probe = None;
            }
            Probe::SignedOut(why) => {
                self.needs_login = true;
                self.failing = None;
                self.last_error = Some(why);
                self.next_probe = Some(now + RECHECK_SECS);
            }
            Probe::Limited { why, lifts_secs } => {
                self.limit(lifts_secs, now);
                self.last_error = Some(why);
            }
            Probe::Failing(why) => {
                self.needs_login = false;
                self.failing = Some(why.clone());
                self.last_error = Some(why);
                self.next_probe = Some(now + RECHECK_SECS);
            }
        }
    }

    /// The provider said it is limited: until it said, or for the next
    /// backoff step; checked again then.
    pub fn limit(&mut self, lifts_secs: Option<i64>, now: i64) {
        let until = if let Some(secs) = lifts_secs {
            now + secs.max(1)
        } else {
            let step = self.backoff_step.min(BACKOFF_MINS.len() - 1);
            self.backoff_step = (self.backoff_step + 1).min(BACKOFF_MINS.len() - 1);
            now + BACKOFF_MINS[step] * 60
        };
        self.limited_until = Some(until);
        self.next_probe = Some(until);
    }

    /// A game ended: well, or with what its bridge said last.
    pub fn ended(&mut self, outcome: Result<(), Probe>, now: i64) {
        self.games = self.games.saturating_sub(1);
        match outcome {
            Ok(()) => {
                self.backoff_step = 0;
                self.last_ok = Some(now);
            }
            // What a game's end says of the profile is what a check would
            // have said, and a check settles it either way.
            Err(probe) => {
                if let Probe::Failing(why) = &probe {
                    // One game failing (a chair refused, a room gone) says
                    // nothing of the profile: noted, and checked now.
                    self.last_error = Some(why.clone());
                    self.next_probe = Some(now);
                } else {
                    self.probed(probe, now);
                }
            }
        }
    }
}

/// What the state also depends on besides what happened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Facts {
    /// Switched on, and the seat agent not draining.
    pub enabled: bool,
    /// Its bound on games.
    pub max_games: Option<u32>,
    /// The seat agent runs as many bridges as it may.
    pub at_capacity: bool,
    /// When its spend book's caps let it play again, if one is reached now.
    pub capped_until: Option<i64>,
}

/// The state, and when it is expected to end where that is known.
#[must_use]
pub fn state(rt: &Runtime, facts: Facts, now: i64) -> (State, Option<i64>) {
    if !facts.enabled {
        return (State::Disabled, None);
    }
    if rt.probing {
        return (State::Probing, None);
    }
    if rt.needs_login {
        return (State::NeedsLogin, None);
    }
    if let Some(until) = facts.capped_until {
        return (State::Exhausted, Some(until));
    }
    if let Some(until) = rt.limited_until.filter(|until| *until > now) {
        return (State::Exhausted, Some(until));
    }
    if rt.failing.is_some() {
        return (State::Failing, None);
    }
    // Limited, and the time is up, but no check has said yes yet.
    if rt.limited_until.is_some() {
        return (State::Probing, None);
    }
    if facts.at_capacity || facts.max_games.is_some_and(|max| rt.games >= max) {
        return (State::Busy, None);
    }
    (State::Available, None)
}

/// When `caps` let a profile play again, if `day` or `month` (its spend
/// book's sums for the day and the month of `now`, UTC) has reached one.
#[must_use]
pub fn capped_until(caps: &Caps, day: &Spent, month: &Spent, now: i64) -> Option<i64> {
    let reached = |usd: Option<f64>, tokens: Option<u64>, spent: &Spent| {
        usd.is_some_and(|cap| spent.usd >= cap) || tokens.is_some_and(|cap| spent.tokens >= cap)
    };
    let by_month = reached(caps.month_usd, caps.month_tokens, month).then(|| next_utc_month(now));
    let by_day = reached(caps.day_usd, caps.day_tokens, day).then(|| next_utc_day(now));
    by_month.or(by_day)
}

/// The next UTC midnight after `now`.
#[must_use]
pub const fn next_utc_day(now: i64) -> i64 {
    (now.div_euclid(86_400) + 1) * 86_400
}

/// The first second of the next UTC month after `now`.
#[must_use]
pub fn next_utc_month(now: i64) -> i64 {
    let (year, month, _) = civil(now.div_euclid(86_400));
    let (year, month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    days(year, month, 1) * 86_400
}

/// Days since 1970-01-01 as a date (Howard Hinnant's algorithm).
const fn civil(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let (m, d) = (m as u32, d as u32);
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// A date as days since 1970-01-01.
const fn days(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = m as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_791_800_000; // 2026-10-12T…Z

    fn on() -> Facts {
        Facts {
            enabled: true,
            max_games: Some(2),
            at_capacity: false,
            capped_until: None,
        }
    }

    /// Every state, from the facts that make it, in the order they win.
    #[test]
    fn the_state_table() {
        let fresh = Runtime::default();
        assert_eq!(state(&fresh, on(), NOW), (State::Available, None));
        let off = Facts {
            enabled: false,
            ..on()
        };
        let probing = Runtime {
            probing: true,
            ..fresh.clone()
        };
        assert_eq!(state(&probing, off, NOW).0, State::Disabled, "off wins");
        assert_eq!(state(&probing, on(), NOW).0, State::Probing);
        let busy = Runtime {
            games: 2,
            ..fresh.clone()
        };
        assert_eq!(state(&busy, on(), NOW).0, State::Busy);
        let unbounded = Facts {
            max_games: None,
            ..on()
        };
        assert_eq!(state(&busy, unbounded, NOW).0, State::Available);
        let full = Facts {
            at_capacity: true,
            ..unbounded
        };
        assert_eq!(state(&fresh, full, NOW).0, State::Busy);
        let capped = Facts {
            capped_until: Some(NOW + 60),
            ..on()
        };
        assert_eq!(
            state(&fresh, capped, NOW),
            (State::Exhausted, Some(NOW + 60))
        );
        let mut limited = fresh.clone();
        limited.limit(Some(600), NOW);
        assert_eq!(
            state(&limited, on(), NOW),
            (State::Exhausted, Some(NOW + 600))
        );
        assert_eq!(
            state(&limited, on(), NOW + 601).0,
            State::Probing,
            "time is up, not yet checked"
        );
        assert!(limited.probe_due(NOW + 600));
        let mut signed_out = fresh.clone();
        signed_out.probed(Probe::SignedOut("signed out".into()), NOW);
        assert_eq!(state(&signed_out, on(), NOW).0, State::NeedsLogin);
        let mut failing = fresh.clone();
        failing.probed(Probe::Failing("no key".into()), NOW);
        assert_eq!(state(&failing, on(), NOW).0, State::Failing);
        assert_eq!(failing.last_error.as_deref(), Some("no key"));
        assert_eq!(failing.next_probe, Some(NOW + RECHECK_SECS));
        failing.probed(Probe::Ok, NOW + 5);
        assert_eq!(state(&failing, on(), NOW + 5).0, State::Available);
        assert_eq!(failing.last_ok, Some(NOW + 5));
    }

    /// A limit that names no end is waited out 5, 10, 20, 40, then 60
    /// minutes at a time; a game that ends well starts it over.
    #[test]
    fn a_limit_without_an_end_backs_off_and_a_good_game_resets_it() {
        let mut rt = Runtime::default();
        let mut waits = Vec::new();
        for _ in 0..6 {
            rt.limit(None, NOW);
            waits.push((rt.limited_until.unwrap() - NOW) / 60);
        }
        assert_eq!(waits, [5, 10, 20, 40, 60, 60]);
        rt.games = 1;
        rt.ended(Ok(()), NOW);
        assert_eq!((rt.games, rt.backoff_step), (0, 0));
        rt.games = 1;
        rt.ended(
            Err(Probe::Limited {
                why: "quota".into(),
                lifts_secs: None,
            }),
            NOW,
        );
        assert_eq!(rt.limited_until, Some(NOW + 300));
        rt.games = 1;
        rt.ended(Err(Probe::Failing("the room is gone".into())), NOW);
        assert_eq!(rt.failing, None, "one game's failure is not the profile's");
        assert!(rt.probe_due(NOW));
    }

    #[test]
    fn caps_end_at_the_next_utc_day_or_month() {
        let caps = Caps {
            day_usd: Some(10.0),
            month_usd: Some(100.0),
            ..Caps::default()
        };
        let spent = |usd| Spent {
            usd,
            ..Spent::default()
        };
        assert_eq!(capped_until(&caps, &spent(9.0), &spent(50.0), NOW), None);
        assert_eq!(
            capped_until(&caps, &spent(10.0), &spent(50.0), NOW),
            Some(next_utc_day(NOW))
        );
        assert_eq!(
            capped_until(&caps, &spent(10.0), &spent(100.0), NOW),
            Some(next_utc_month(NOW)),
            "the month's cap lasts longer"
        );
        assert_eq!(
            capped_until(&Caps::default(), &spent(1e9), &spent(1e9), NOW),
            None,
            "no caps, no limit"
        );
        // 2026-10-12 → 2026-10-13T00:00Z and 2026-11-01T00:00Z.
        assert_eq!(next_utc_day(NOW), 1_791_849_600);
        assert_eq!(next_utc_month(NOW), 1_793_491_200);
        // December rolls into January.
        // 2026-12-15 → 2027-01-01T00:00Z.
        assert_eq!(next_utc_month(1_797_292_800), 1_798_761_600);
    }
}
