use super::{
    Acknowledged, ConnectionTrait, Current, Error, Lease, Phase, Prefix, Principal, Refusal, Store,
    StoredChunk, WorkerCommand, column, decode, encode, live, signed,
};
use crate::{MAX_ACCOUNT_RECORD_BYTES, MAX_ACCOUNT_RECORD_CHUNKS};
use baylee_tower::v2::execution::MAX_CHUNK_BYTES;

impl Store {
    /// Atomically retain raw record bytes, quota and head before returning their ACK.
    /// Exact live-worker retries return the original head without charging twice.
    /// # Errors
    /// Wrong/stale worker, reordered/changed chunk, capacity or failed durable commit.
    pub async fn append(
        &self,
        principal: &Principal,
        lease: Lease,
        expected: Prefix,
        chunk: &[u8],
    ) -> Result<Acknowledged, Error> {
        if chunk.is_empty() || chunk.len() > MAX_CHUNK_BYTES {
            return Err(Refusal::Capacity.into());
        }
        let tx = self.transaction().await?;
        let mut current = self.worker(&tx, principal, lease).await?;
        let now = self.now(&tx).await?;
        live(&current, lease, now)?;
        let old = self.chunk(&tx, lease.reservation, expected.chunks).await?;
        let prefix = if let Some((fence, old)) = old {
            if fence != lease.fence || old.previous != expected || old.data != chunk {
                return Err(Error::Conflict);
            }
            checked_chunk(&current, &old)?;
            old.head
        } else {
            self.append_new(&tx, &mut current, now, expected, chunk)
                .await?
        };
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(Acknowledged {
            fence: lease.fence,
            prefix,
        })
    }

    async fn append_new(
        &self,
        db: &impl ConnectionTrait,
        current: &mut Current,
        now: u64,
        expected: Prefix,
        chunk: &[u8],
    ) -> Result<Prefix, Error> {
        let state = &current.saved.snapshot.state;
        let fence = current.execution()?.fence;
        current.saved.snapshot.state = state.worker(
            state.revision,
            now,
            WorkerCommand::Append {
                fence,
                expected,
                chunk,
            },
            &current.saved.publication.catalog,
        )?;
        let prefix = current.execution()?.prefix;
        let charged = db.execute_raw(self.statement("UPDATE @tower.players SET record_bytes=record_bytes+$2,record_chunks=record_chunks+1 WHERE account=$1 AND record_bytes<=$3-$2 AND record_chunks<$4",
            vec![current.saved.account.into(), signed(chunk.len() as u64)?.into(), MAX_ACCOUNT_RECORD_BYTES.into(), MAX_ACCOUNT_RECORD_CHUNKS.into()]))
            .await.map_err(|_| Error::Unavailable)?;
        if charged.rows_affected() != 1 {
            return Err(Refusal::Capacity.into());
        }
        db.execute_raw(self.statement("INSERT INTO @tower.record_chunks(reservation,seq,fence,previous,head,data) VALUES($1,$2,$3,$4,$5,$6)",
            vec![current.reservation.into(), signed(expected.chunks)?.into(), signed(fence)?.into(), encode(&expected)?.into(), encode(&prefix)?.into(), chunk.to_vec().into()]))
            .await.map_err(|_| Error::Unavailable)?;
        self.save_current(db, current).await?;
        Ok(prefix)
    }

    async fn chunk(
        &self,
        db: &impl ConnectionTrait,
        reservation: uuid::Uuid,
        index: u64,
    ) -> Result<Option<(u64, StoredChunk)>, Error> {
        let row = db.query_one_raw(self.statement("SELECT fence,previous,head,data FROM @tower.record_chunks WHERE reservation=$1 AND seq=$2",
            vec![reservation.into(), signed(index)?.into()])).await.map_err(|_| Error::Unavailable)?;
        row.map(|row| {
            Ok((
                u64::try_from(column::<i64>(&row, "fence")?).map_err(|_| Error::Unavailable)?,
                StoredChunk {
                    index,
                    previous: decode(&column::<Vec<u8>>(&row, "previous")?)?,
                    head: decode(&column::<Vec<u8>>(&row, "head")?)?,
                    data: column(&row, "data")?,
                },
            ))
        })
        .transpose()
    }

    /// Read one retained row for exact replay by the currently leased host.
    /// Previous workers and unassigned hosts cannot read hidden records.
    /// # Errors
    /// Wrong/stale operator/lease or a position outside the acknowledged prefix.
    pub async fn read_chunk(
        &self,
        principal: &Principal,
        lease: Lease,
        index: u64,
    ) -> Result<StoredChunk, Error> {
        let tx = self.transaction().await?;
        let current = self.worker(&tx, principal, lease).await?;
        live(&current, lease, self.now(&tx).await?)?;
        if index >= current.execution()?.prefix.chunks {
            return Err(Error::NotFound);
        }
        let (_, chunk) = self
            .chunk(&tx, lease.reservation, index)
            .await?
            .ok_or(Error::Unavailable)?;
        checked_chunk(&current, &chunk)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(chunk)
    }

    /// Freeze a terminal candidate. This never trusts a host-provided win or score.
    /// Same-generation sealing retries are idempotent, including after lease closure.
    /// # Errors
    /// Wrong/stale operator, changed head, missing record or failed commit.
    pub async fn seal(
        &self,
        principal: &Principal,
        lease: Lease,
        expected: Prefix,
    ) -> Result<Acknowledged, Error> {
        let tx = self.transaction().await?;
        let mut current = self.worker(&tx, principal, lease).await?;
        let execution = current.execution()?;
        if current.saved.snapshot.state.phase == Phase::Verifying && execution.sealed {
            if execution.prefix != expected {
                return Err(Refusal::Stale.into());
            }
        } else {
            let state = &current.saved.snapshot.state;
            current.saved.snapshot.state = state.worker(
                state.revision,
                self.now(&tx).await?,
                WorkerCommand::Seal {
                    fence: lease.fence,
                    expected,
                },
                &current.saved.publication.catalog,
            )?;
            self.save_current(&tx, &current).await?;
        }
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(Acknowledged {
            fence: lease.fence,
            prefix: expected,
        })
    }
}

fn checked_chunk(current: &Current, chunk: &StoredChunk) -> Result<(), Error> {
    let stored = current.execution()?;
    // Recompute only a content commitment; this does not verify legal gameplay.
    let mut reference =
        baylee_tower::v2::execution::Execution::new(stored.package, stored.byte_limit)?
            .claimed(0, 1)?;
    reference.prefix = chunk.previous;
    let next = reference.appended(1, 0, chunk.previous, &chunk.data)?;
    if chunk.previous.chunks != chunk.index
        || next.prefix != chunk.head
        || chunk.head.chunks > stored.prefix.chunks
        || chunk.head.bytes > stored.prefix.bytes
        || (chunk.head.chunks == stored.prefix.chunks && chunk.head != stored.prefix)
    {
        return Err(Error::Unavailable);
    }
    Ok(())
}
