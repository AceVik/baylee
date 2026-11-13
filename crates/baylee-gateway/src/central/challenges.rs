//! Resolve an immutable executor from Central, never from a player's request.
use super::{Bridge, StatusCode, wire};
use central_api::challenges::{Board, HostRequest, HostResponse, Request, Response};

impl Bridge {
    pub(crate) async fn challenge_executor(&self, id: [u8; 16]) -> Result<String, StatusCode> {
        let HostResponse::Attempt { attempt } =
            self.challenge(HostRequest::Status { attempt: id }).await?
        else {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        };
        if attempt.id != id {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
        // Public metadata uses no bridge bearer. Central deliberately refuses
        // host credentials on player-facing challenge operations.
        let _queued = self
            .pending
            .try_acquire()
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let _turn = self
            .in_flight
            .acquire()
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let wire::Response::Challenges {
            response: Response::Detail { challenge, .. },
        } = self
            .transport
            .exchange(
                &wire::Request::Challenges {
                    request: Request::Detail {
                        challenge: attempt.challenge,
                        board: Board::FirstWin,
                        offset: 0,
                    },
                },
                None,
            )
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
        else {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        };
        if challenge.id != attempt.challenge || challenge.withdrawn {
            return Err(StatusCode::CONFLICT);
        }
        Ok(challenge.description.executor)
    }
}
