//! Global participation follows the Central account, independently of its current gate.
use super::{Account, Error};
use central_api::challenges::{Request, Response};

impl Account {
    /// Read or update this account's Daily/Weekly participation on its worker.
    /// Retain a Begin request's UUID until its outcome is known.
    /// # Errors
    /// Expired sessions, unavailable service, invalid operations or unexpected responses.
    pub fn challenges(&self, request: Request) -> Result<Response, Error> {
        self.session.challenges(&self.http, request)
    }
}
