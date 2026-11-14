use super::{
    ConnectionTrait, Digest, Error, MAX_REQUESTS, Refusal, Serialize, Sha256, Snapshot, Store,
    Uuid, column, decode, encode, uuid7,
};

pub(super) fn digest(kind: &[u8], request: &impl Serialize) -> Result<Vec<u8>, Error> {
    Ok(Sha256::new()
        .chain_update(b"baylee/tower/request/v2\0")
        .chain_update(kind)
        .chain_update([0])
        .chain_update(encode(request)?)
        .finalize()
        .to_vec())
}

impl Store {
    pub(super) async fn owner(
        &self,
        db: &impl ConnectionTrait,
        account: Uuid,
        create: bool,
    ) -> Result<(), Error> {
        uuid7(account)?;
        if create {
            db.execute_raw(self.statement("INSERT INTO @tower.players(account,echo_consent) VALUES($1,true) ON CONFLICT(account) DO NOTHING", vec![account.into()]))
                .await.map_err(|_| Error::Unavailable)?;
        }
        db.query_one_raw(self.statement(
            "SELECT account FROM @tower.players WHERE account=$1 FOR UPDATE",
            vec![account.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::NotFound)?;
        Ok(())
    }

    pub(super) async fn repeated(
        &self,
        db: &impl ConnectionTrait,
        account: Uuid,
        request: Uuid,
        digest: &[u8],
    ) -> Result<Option<Snapshot>, Error> {
        uuid7(request)?;
        let row = db
            .query_one_raw(self.statement(
                "SELECT digest,response FROM @tower.requests WHERE account=$1 AND request=$2",
                vec![account.into(), request.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?;
        row.map(|row| {
            if column::<Vec<u8>>(&row, "digest")? != digest {
                return Err(Error::Conflict);
            }
            decode(&column::<Vec<u8>>(&row, "response")?)
        })
        .transpose()
    }

    pub(super) async fn receipt(
        &self,
        db: &impl ConnectionTrait,
        account: Uuid,
        request: Uuid,
        digest: Vec<u8>,
        response: &Snapshot,
    ) -> Result<(), Error> {
        let row = db
            .query_one_raw(self.statement(
                "SELECT count(*)::bigint AS count FROM @tower.requests WHERE run=$1",
                vec![response.id.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Unavailable)?;
        if column::<i64>(&row, "count")? >= MAX_REQUESTS {
            return Err(Refusal::Capacity.into());
        }
        db.execute_raw(self.statement("INSERT INTO @tower.requests(account,request,run,digest,response) VALUES($1,$2,$3,$4,$5)",
            vec![account.into(), request.into(), response.id.into(), digest.into(), encode(response)?.into()]))
            .await.map_err(|_| Error::Unavailable)?;
        Ok(())
    }
}
