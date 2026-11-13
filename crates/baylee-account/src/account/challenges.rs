//! Global participation follows the Central account, independently of its current gate.
use super::{Account, Error};
use central_api::challenges::{Request, Response};

impl Account {
    /// Obtain the host from Central afresh and open its official human seat.
    /// No caller-supplied origin receives an account bearer.
    /// # Errors
    /// Finished/foreign attempts, unavailable host, refusal or inconsistent game binding.
    pub fn play_challenge(
        &self,
        id: [u8; 16],
    ) -> Result<
        (
            central_api::challenges::Attempt,
            crate::Admission,
            crate::ChallengeSeat,
        ),
        Error,
    > {
        let Response::Attempt { attempt } = self.challenges(Request::Attempt { attempt: id })?
        else {
            return Err(Error::Rejected);
        };
        if attempt.id != id || !attempt.state.active() {
            return Err(Error::Conflict);
        }
        let (source, ticket) = self.gateway_token(&attempt.host)?;
        let admission = source.admit(&ticket).map_err(|turned| turned.error)?;
        let seat = source.challenge_play(&admission.token, id)?;
        if attempt.game.is_some_and(|game| {
            uuid::Uuid::parse_str(&seat.game_id).ok() != Some(uuid::Uuid::from_bytes(game))
        }) {
            return Err(Error::Rejected);
        }
        Ok((attempt, admission, seat))
    }
    /// Read or update this account's Daily/Weekly participation on its worker.
    /// Retain a Begin request's UUID until its outcome is known.
    /// # Errors
    /// Expired sessions, unavailable service, invalid operations or unexpected responses.
    pub fn challenges(&self, request: Request) -> Result<Response, Error> {
        self.session.challenges(&self.http, request)
    }
}
