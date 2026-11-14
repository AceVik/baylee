use super::{Command, Error, Snapshot, Store, Uuid, requests, uuid7};

impl Store {
    /// Lock ownership, check/deduplicate the exact request, and commit mutation plus
    /// original response together. Same request with different contents is a conflict.
    /// # Errors
    /// Wrong owner, stale command, receipt conflict, capacity or storage failure.
    pub async fn command(
        &self,
        account: Uuid,
        request: Uuid,
        id: Uuid,
        revision: u64,
        command: &Command,
    ) -> Result<Snapshot, Error> {
        uuid7(id)?;
        let digest = requests::digest(b"command", &(id, revision, command))?;
        let tx = self.transaction().await?;
        self.owner(&tx, account, false).await?;
        if let Some(previous) = self.repeated(&tx, account, request, &digest).await? {
            tx.commit().await.map_err(|_| Error::Unavailable)?;
            return Ok(previous);
        }
        let mut saved = self.locked_run(&tx, account, id).await?;
        saved.snapshot.state = saved.snapshot.state.command(
            revision,
            command,
            &saved.publication.catalog,
            &saved.seed,
        )?;
        let response = saved.snapshot;
        self.save_run(&tx, account, &response).await?;
        self.receipt(&tx, account, request, digest, &response)
            .await?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(response)
    }
}
