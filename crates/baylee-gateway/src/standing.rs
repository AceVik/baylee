//! Whether what a socket stands on still stands, and what happens when
//! nobody can say (`docs/central-login.md` §"When Central cannot answer").
//!
//! A socket asks every few seconds, and a seat's before each frame. Three
//! answers come back, and only one of them is a reason to close: a session
//! that is gone is gone at once, but Central being busy or away, or the
//! database failing, says nothing about the session at all. An unknown answer
//! is waited out, for [`UNKNOWN_FOR`], and a good one ends the wait.
//!
//! The price is that a revocation Central cannot deliver takes that long to
//! land, at most, while it cannot be delivered. The owner chose it, until the
//! sessions of `docs/proposals/central-session-tokens.md` replace all of this.

use crate::{ErrorBody, Json, Shared, StatusCode, TICKET_REFUSED, err};
use std::time::{Duration, Instant};

/// How long unknown answers may last in a row before a socket closes all
/// the same.
pub(crate) const UNKNOWN_FOR: Duration = Duration::from_secs(60);

/// The longest one question may hold up its socket, which waits for it in
/// its own loop. Out of time is an unknown answer.
const ASKED_WITHIN: Duration = Duration::from_secs(2);

/// What is known about a session or a chair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Standing {
    /// It stands.
    Good,
    /// It is gone: signed out, lapsed, banned, or Central refused it.
    Revoked,
    /// Nobody could say.
    Unknown,
}

impl Standing {
    /// What a session's check answered, for `account`.
    ///
    /// `401` is Central refusing, and the only status that says anything
    /// about the session (`central/transport.rs`).
    pub(crate) fn of_answer(
        answer: Result<Option<crate::store::Session>, StatusCode>,
        account: &str,
    ) -> Self {
        match answer {
            Ok(Some(session))
                if session.account_id == account && crate::moderation::admit(&session).is_ok() =>
            {
                Self::Good
            }
            Ok(_) | Err(StatusCode::UNAUTHORIZED) => Self::Revoked,
            Err(_) => Self::Unknown,
        }
    }

    /// As an answer at a door, where nothing is open yet to be kept open:
    /// `401` for what is gone, and `503` for what is not known, which a client
    /// must not read as its sign-in having ended.
    pub(crate) fn at_the_door(self) -> Result<(), (StatusCode, Json<ErrorBody>)> {
        match self {
            Self::Good => Ok(()),
            Self::Revoked => Err(err(StatusCode::UNAUTHORIZED, TICKET_REFUSED)),
            Self::Unknown => Err(err(
                StatusCode::SERVICE_UNAVAILABLE,
                "account authorization unavailable",
            )),
        }
    }
}

/// What the session behind `digest` stands at, for `account`.
pub(crate) async fn of_session(state: &Shared, digest: &[u8], account: &str) -> Standing {
    asked(async {
        Standing::of_answer(
            crate::central::resolve_digest(state, digest.to_vec()).await,
            account,
        )
    })
    .await
}

/// `question`'s answer, or [`Standing::Unknown`] once it took too long.
pub(crate) async fn asked(question: impl Future<Output = Standing>) -> Standing {
    tokio::time::timeout(ASKED_WITHIN, question)
        .await
        .unwrap_or(Standing::Unknown)
}

/// One socket's patience with unknown answers.
#[derive(Debug, Default)]
pub(crate) struct Grace {
    /// When the current run of unknown answers began.
    unknown_since: Option<Instant>,
}

impl Grace {
    /// Whether the socket stays open after `standing`, heard at `now`.
    pub(crate) fn keeps_open(&mut self, standing: Standing, now: Instant) -> bool {
        match standing {
            Standing::Good => {
                self.unknown_since = None;
                true
            }
            Standing::Revoked => false,
            Standing::Unknown => {
                let since = *self.unknown_since.get_or_insert(now);
                now.saturating_duration_since(since) < UNKNOWN_FOR
            }
        }
    }

    /// Whether the last answer was an unknown one. A question per frame is
    /// not asked again until a periodic one has been heard: a move need not
    /// wait on a Central that is not answering.
    pub(crate) const fn in_doubt(&self) -> bool {
        self.unknown_since.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A revocation is not waited out, whatever came before it.
    #[test]
    fn a_revocation_closes_at_once() {
        let now = Instant::now();
        let mut grace = Grace::default();
        assert!(!grace.keeps_open(Standing::Revoked, now));
        let mut doubtful = Grace::default();
        assert!(doubtful.keeps_open(Standing::Unknown, now));
        assert!(!doubtful.keeps_open(Standing::Revoked, now));
    }

    /// Unknown answers close a socket only once they have lasted the whole
    /// window in a row.
    #[test]
    fn unknown_answers_close_only_after_they_last_sixty_seconds() {
        let now = Instant::now();
        let mut grace = Grace::default();
        assert!(grace.keeps_open(Standing::Unknown, now));
        assert!(grace.keeps_open(Standing::Unknown, now + UNKNOWN_FOR / 2));
        assert!(grace.keeps_open(
            Standing::Unknown,
            now + UNKNOWN_FOR - Duration::from_millis(1)
        ));
        assert!(!grace.keeps_open(Standing::Unknown, now + UNKNOWN_FOR));
    }

    /// One good answer starts the count again, so a Central that stutters
    /// never closes anybody.
    #[test]
    fn a_good_answer_ends_the_doubt() {
        let now = Instant::now();
        let mut grace = Grace::default();
        assert!(grace.keeps_open(Standing::Unknown, now));
        assert!(grace.in_doubt());
        assert!(grace.keeps_open(Standing::Good, now + UNKNOWN_FOR / 2));
        assert!(!grace.in_doubt());
        let later = now + UNKNOWN_FOR * 2;
        assert!(grace.keeps_open(Standing::Unknown, later));
        assert!(grace.keeps_open(
            Standing::Unknown,
            later + UNKNOWN_FOR - Duration::from_secs(1)
        ));
    }

    /// Only Central's own refusal reads as a revocation: every other failure
    /// says nothing about the session.
    #[test]
    fn only_a_refusal_reads_as_a_revocation() {
        let of = |status| Standing::of_answer(Err(status), "account");
        assert_eq!(of(StatusCode::UNAUTHORIZED), Standing::Revoked);
        assert_eq!(of(StatusCode::TOO_MANY_REQUESTS), Standing::Unknown);
        assert_eq!(of(StatusCode::SERVICE_UNAVAILABLE), Standing::Unknown);
        assert_eq!(of(StatusCode::FORBIDDEN), Standing::Unknown);
        assert_eq!(Standing::of_answer(Ok(None), "account"), Standing::Revoked);
    }

    /// A question that does not come back in time is an unknown answer.
    #[tokio::test(start_paused = true)]
    async fn a_question_that_takes_too_long_is_an_unknown_answer() {
        let never = std::future::pending::<Standing>();
        assert_eq!(asked(never).await, Standing::Unknown);
        assert_eq!(asked(async { Standing::Good }).await, Standing::Good);
    }
}
