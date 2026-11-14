//! Resolve an immutable executor from Central, never from a player's request.
use super::{Bridge, StatusCode};
use central_api::challenges::{HostRequest, HostResponse};

impl Bridge {
    pub(crate) async fn challenge_executor(&self, id: [u8; 16]) -> Result<String, StatusCode> {
        let HostResponse::Executor { executor } = self
            .challenge(HostRequest::Executor { attempt: id })
            .await?
        else {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        };
        if executor.is_empty() || executor.len() > 160 {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
        Ok(executor)
    }
}
