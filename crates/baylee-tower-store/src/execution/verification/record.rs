use super::{Error, Principal, Store, VerificationLease};
use crate::StoredChunk;

impl Store {
    /// Read one bounded acknowledged record chunk under a live independent job.
    /// The verifier must replay the entire ordered prefix; a digest is not legal proof.
    /// # Errors
    /// Wrong/stale role/job, corrupt bytes or index outside the frozen sealed head.
    pub async fn verification_chunk(
        &self,
        principal: &Principal,
        lease: VerificationLease,
        index: u64,
    ) -> Result<StoredChunk, Error> {
        let tx = self.transaction().await?;
        let current = self.verifier(&tx, principal, lease).await?;
        if index >= lease.binding.prefix.chunks {
            return Err(Error::NotFound);
        }
        let (_, chunk) = self
            .chunk(&tx, lease.reservation, index)
            .await?
            .ok_or(Error::Unavailable)?;
        super::super::record::checked_chunk(&current, &chunk)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(chunk)
    }
}
