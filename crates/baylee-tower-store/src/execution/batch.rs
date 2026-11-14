use super::{
    Acknowledged, ConnectionTrait, Current, Error, Lease, Prefix, Principal, Refusal, Store,
    column, decode, encode, live, signed,
};
use baylee_tower::v2::execution::{MAX_BATCH_BYTES, MAX_BATCH_CHUNKS, MAX_CHUNK_BYTES};
use std::fmt::Write as _;

impl Store {
    /// Commit every chunk of one complete operation, counters and its exact receipt
    /// in one transaction. Nothing is acknowledged between chunks: a process death
    /// cannot leave an acknowledged prefix ending inside a JSON line. A retry must
    /// preserve the entire original batch, including its chunk boundaries.
    /// # Errors
    /// Wrong/stale host, partial/changed batch, exhausted bound or failed commit.
    pub async fn append_batch(
        &self,
        principal: &Principal,
        lease: Lease,
        expected: Prefix,
        chunks: &[Vec<u8>],
    ) -> Result<Acknowledged, Error> {
        validate(chunks)?;
        let tx = self.transaction().await?;
        let mut current = self.worker(&tx, principal, lease).await?;
        let now = self.now(&tx).await?;
        live(&current, lease, now)?;
        let prefix = if let Some(prefix) = self
            .batch_retry(&tx, &current, lease, expected, chunks)
            .await?
        {
            prefix
        } else {
            if current.execution()?.prefix != expected {
                return Err(Refusal::Stale.into());
            }
            let head = self
                .append_many(&tx, &mut current, now, expected, chunks)
                .await?;
            tx.execute_raw(self.statement("INSERT INTO @tower.record_batches(reservation,seq,fence,chunks,previous,head) VALUES($1,$2,$3,$4,$5,$6)",
                vec![lease.reservation.into(), signed(expected.chunks)?.into(), signed(lease.fence)?.into(), i16::try_from(chunks.len()).map_err(|_| Refusal::Capacity)?.into(), encode(&expected)?.into(), encode(&head)?.into()]))
                .await.map_err(|_| Error::Unavailable)?;
            head
        };
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(Acknowledged {
            fence: lease.fence,
            prefix,
        })
    }

    async fn append_many(
        &self,
        db: &impl ConnectionTrait,
        current: &mut Current,
        now: u64,
        expected: Prefix,
        chunks: &[Vec<u8>],
    ) -> Result<Prefix, Error> {
        let mut sql = String::from(
            "INSERT INTO @tower.record_chunks(reservation,seq,fence,previous,head,data) VALUES ",
        );
        let mut values = Vec::with_capacity(chunks.len() * 6);
        let mut head = expected;
        let fence = current.execution()?.fence;
        for (index, chunk) in chunks.iter().enumerate() {
            let state = &current.saved.snapshot.state;
            current.saved.snapshot.state = state.worker(
                state.revision,
                now,
                super::WorkerCommand::Append {
                    fence,
                    expected: head,
                    chunk,
                },
                &current.saved.publication.catalog,
            )?;
            let next = current.execution()?.prefix;
            let first = index * 6 + 1;
            if index != 0 {
                sql.push(',');
            }
            write!(
                &mut sql,
                "(${},${},${},${},${},${})",
                first,
                first + 1,
                first + 2,
                first + 3,
                first + 4,
                first + 5
            )
            .map_err(|_| Error::Unavailable)?;
            values.extend([
                current.reservation.into(),
                signed(head.chunks)?.into(),
                signed(fence)?.into(),
                encode(&head)?.into(),
                encode(&next)?.into(),
                chunk.clone().into(),
            ]);
            head = next;
        }
        let bytes = head
            .bytes
            .checked_sub(expected.bytes)
            .ok_or(Error::Unavailable)?;
        let count = signed(chunks.len() as u64)?;
        let charged = db.execute_raw(self.statement("UPDATE @tower.players SET record_bytes=record_bytes+$2,record_chunks=record_chunks+$3 WHERE account=$1 AND record_bytes<=$4-$2 AND record_chunks<=$5-$3",
            vec![current.saved.account.into(), signed(bytes)?.into(), count.into(), crate::MAX_ACCOUNT_RECORD_BYTES.into(), crate::MAX_ACCOUNT_RECORD_CHUNKS.into()]))
            .await.map_err(|_| Error::Unavailable)?;
        if charged.rows_affected() != 1 {
            return Err(Refusal::Capacity.into());
        }
        db.execute_raw(self.statement(&sql, values))
            .await
            .map_err(|_| Error::Unavailable)?;
        self.save_current(db, current).await?;
        Ok(head)
    }

    async fn batch_retry(
        &self,
        db: &impl ConnectionTrait,
        current: &Current,
        lease: Lease,
        expected: Prefix,
        chunks: &[Vec<u8>],
    ) -> Result<Option<Prefix>, Error> {
        let Some(row) = db.query_one_raw(self.statement("SELECT fence,chunks,previous,head FROM @tower.record_batches WHERE reservation=$1 AND seq=$2", vec![lease.reservation.into(), signed(expected.chunks)?.into()]))
            .await.map_err(|_| Error::Unavailable)? else { return Ok(None); };
        let saved: Prefix = decode(&column::<Vec<u8>>(&row, "head")?)?;
        if column::<i64>(&row, "fence")? != signed(lease.fence)?
            || usize::try_from(column::<i16>(&row, "chunks")?).ok() != Some(chunks.len())
            || decode::<Prefix>(&column::<Vec<u8>>(&row, "previous")?)? != expected
        {
            return Err(Error::Conflict);
        }
        let mut head = expected;
        for data in chunks {
            let (fence, old) = self
                .chunk(db, lease.reservation, head.chunks)
                .await?
                .ok_or(Error::Unavailable)?;
            if fence != lease.fence || old.previous != head || old.data != *data {
                return Err(Error::Conflict);
            }
            super::record::checked_chunk(current, &old)?;
            head = old.head;
        }
        if head != saved {
            return Err(Error::Unavailable);
        }
        Ok(Some(head))
    }
}

fn validate(chunks: &[Vec<u8>]) -> Result<(), Error> {
    if chunks.is_empty()
        || chunks.len() > MAX_BATCH_CHUNKS
        || chunks.last().and_then(|chunk| chunk.last()) != Some(&b'\n')
    {
        return Err(Refusal::Capacity.into());
    }
    let mut size = 0_usize;
    for chunk in chunks {
        if chunk.is_empty() || chunk.len() > MAX_CHUNK_BYTES {
            return Err(Refusal::Capacity.into());
        }
        size = size
            .checked_add(chunk.len())
            .filter(|size| *size <= MAX_BATCH_BYTES)
            .ok_or(Refusal::Capacity)?;
    }
    Ok(())
}
