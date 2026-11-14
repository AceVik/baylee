use super::{Account, Error};
use crate::TowerHttp;

impl Account {
    /// Open the distinct Tower origin delegated by Central. No gateway chooses it
    /// or receives this credential. Refresh this handle after its five-minute expiry.
    /// Like all account methods, run on the client's worker, never the render thread.
    /// # Errors
    /// Missing restoration/terms, unavailable service or refused identity delegation.
    pub fn tower(&self) -> Result<TowerHttp, Error> {
        let grant = self.session.tower_token(&self.http)?;
        TowerHttp::new(self.http.endpoint(), grant)
    }
}
