//! Compact verified personal bests. Every write follows the owner-before-run lock order.
use super::{ConnectionTrait, Error, Refusal, Snapshot, Store, Uuid, column};
use baylee_tower_api::ranking::{Board, LEADERS, Lane, Listing, Name, Standing};

impl Store {
    /// Explicit publication, using only a freshly introspected Central public identity.
    /// The HTTP adapter must never take `name` from a player request.
    /// # Errors
    /// Unknown owner, stale privacy revision, invalid public name, or storage failure.
    pub async fn listing(
        &self,
        account: Uuid,
        revision: u64,
        listed: bool,
        name: Option<&Name>,
    ) -> Result<Listing, Error> {
        let next = revision
            .checked_add(1)
            .and_then(|r| i64::try_from(r).ok())
            .ok_or(Refusal::Capacity)?;
        if listed && name.is_none_or(|n| !valid_name(n)) {
            return Err(Refusal::Invalid.into());
        }
        let tx = self.transaction().await?;
        self.owner(&tx, account, false).await?;
        let current = self.rank_listing(&tx, account).await?;
        if current.revision != revision {
            // An exact lost-ACK retry is harmless. Any later toggle fences it out.
            if current.revision == next as u64 && current.listed == listed {
                tx.commit().await.map_err(|_| Error::Unavailable)?;
                return Ok(current);
            }
            return Err(Error::Conflict);
        }
        let name = name.filter(|_| listed);
        tx.execute_raw(self.statement(
            "UPDATE @tower.players SET rank_revision=$2,rank_listed=$3,rank_name=$4,rank_tag=$5 WHERE account=$1",
            vec![account.into(), next.into(), listed.into(), name.map(|n| n.display.clone()).into(), name.map(|n| i64::from(n.tag)).into()],
        )).await.map_err(|_| Error::Unavailable)?;
        tx.execute_raw(self.statement(
            "UPDATE @tower.personal_bests SET listed=$2 WHERE account=$1 AND listed<>$2",
            vec![account.into(), listed.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?;
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(Listing {
            revision: next as u64,
            listed,
        })
    }

    /// Refresh a listed public identity after live introspection; removal withdraws it.
    /// Nothing is created for accounts which have never started a run.
    /// # Errors
    /// Invalid trusted identity or database failure.
    pub async fn ranking_identity(&self, account: Uuid, name: Option<&Name>) -> Result<(), Error> {
        if name.is_some_and(|n| !valid_name(n)) {
            return Err(Refusal::Invalid.into());
        }
        let tx = self.transaction().await?;
        let row = tx.query_one_raw(self.statement(
            "SELECT rank_name,rank_tag FROM @tower.players WHERE account=$1 AND rank_listed FOR UPDATE",
            vec![account.into()],
        )).await.map_err(|_| Error::Unavailable)?;
        if let Some(row) = row {
            if let Some(name) = name {
                if column::<String>(&row, "rank_name")? != name.display
                    || column::<i64>(&row, "rank_tag")? != i64::from(name.tag)
                {
                    tx.execute_raw(self.statement(
                        "UPDATE @tower.players SET rank_name=$2,rank_tag=$3 WHERE account=$1",
                        vec![
                            account.into(),
                            name.display.clone().into(),
                            i64::from(name.tag).into(),
                        ],
                    ))
                    .await
                    .map_err(|_| Error::Unavailable)?;
                }
            } else {
                tx.execute_raw(self.statement("UPDATE @tower.players SET rank_listed=false,rank_name=NULL,rank_tag=NULL,rank_revision=rank_revision+1 WHERE account=$1", vec![account.into()]))
                    .await.map_err(|_| Error::Unavailable)?;
                tx.execute_raw(self.statement(
                    "UPDATE @tower.personal_bests SET listed=false WHERE account=$1 AND listed",
                    vec![account.into()],
                ))
                .await
                .map_err(|_| Error::Unavailable)?;
            }
        }
        tx.commit().await.map_err(|_| Error::Unavailable)
    }

    /// Top 25 plus the authenticated owner's best in one exact cohort/lane.
    /// A repeatable snapshot keeps shared ranks and the own-row projection consistent.
    /// # Errors
    /// Unknown cohort or bounded database timeout/failure.
    pub async fn ranking(&self, account: Uuid, cohort: Uuid, lane: Lane) -> Result<Board, Error> {
        super::uuid7(account)?;
        super::uuid7(cohort)?;
        let tx = self.transaction().await?;
        tx.execute_unprepared("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
            .await
            .map_err(|_| Error::Unavailable)?;
        tx.query_one_raw(self.statement(
            "SELECT id FROM @tower.cohorts WHERE id=$1",
            vec![cohort.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .ok_or(Error::NotFound)?;
        let listing = self.rank_listing(&tx, account).await?;
        let house = lane == Lane::House;
        // The partial covering order starts at the best score; no full-table sort.
        let rows = tx.query_all_raw(self.statement(
            "SELECT b.account,b.wins,b.defeats,p.rank_name,p.rank_tag FROM @tower.personal_bests b
             JOIN @tower.players p ON p.account=b.account
             WHERE b.cohort=$1 AND b.house=$2 AND b.listed AND p.rank_listed
             ORDER BY b.wins DESC,b.defeats,b.id LIMIT 25",
            vec![cohort.into(), house.into()],
        )).await.map_err(|_| Error::Unavailable)?;
        let mut leaders: Vec<Standing> = Vec::with_capacity(LEADERS);
        for row in rows {
            let mut standing = standing(&row, account, true)?;
            standing.rank = Some(
                leaders
                    .last()
                    .filter(|p| p.wins == standing.wins && p.defeats == standing.defeats)
                    .and_then(|p| p.rank)
                    .unwrap_or(leaders.len() as u64 + 1),
            );
            leaders.push(standing);
        }
        let own = tx.query_one_raw(self.statement(
            "SELECT b.account,b.wins,b.defeats,p.rank_name,p.rank_tag FROM @tower.personal_bests b
             JOIN @tower.players p ON p.account=b.account WHERE b.account=$1 AND b.cohort=$2 AND b.house=$3",
            vec![account.into(), cohort.into(), house.into()],
        )).await.map_err(|_| Error::Unavailable)?;
        let mut best = own
            .as_ref()
            .map(|row| standing(row, account, listing.listed))
            .transpose()?;
        if let Some(best) = &mut best
            && listing.listed
        {
            let row = tx.query_one_raw(self.statement(
                "SELECT count(*)+1 AS rank FROM @tower.personal_bests WHERE cohort=$1 AND house=$2 AND listed
                 AND (wins>$3 OR (wins=$3 AND defeats<$4))",
                vec![cohort.into(), house.into(), i16::from(best.wins).into(), i16::from(best.defeats).into()],
            )).await.map_err(|_| Error::Unavailable)?.ok_or(Error::Unavailable)?;
            best.rank =
                Some(u64::try_from(column::<i64>(&row, "rank")?).map_err(|_| Error::Unavailable)?);
        }
        let board = Board {
            cohort,
            lane,
            listing,
            best,
            leaders,
        };
        if !board.valid() {
            return Err(Error::Unavailable);
        }
        tx.commit().await.map_err(|_| Error::Unavailable)?;
        Ok(board)
    }

    async fn rank_listing(
        &self,
        db: &impl ConnectionTrait,
        account: Uuid,
    ) -> Result<Listing, Error> {
        db.query_one_raw(self.statement(
            "SELECT rank_revision,rank_listed FROM @tower.players WHERE account=$1",
            vec![account.into()],
        ))
        .await
        .map_err(|_| Error::Unavailable)?
        .map(|row| {
            Ok(Listing {
                revision: u64::try_from(column::<i64>(&row, "rank_revision")?)
                    .map_err(|_| Error::Unavailable)?,
                listed: column(&row, "rank_listed")?,
            })
        })
        .transpose()
        .map(Option::unwrap_or_default)
    }

    /// Only called inside the verified settlement transaction, after its receipt exists.
    pub(super) async fn rank_completed(
        &self,
        db: &impl ConnectionTrait,
        account: Uuid,
        run: &Snapshot,
    ) -> Result<(), Error> {
        let wins = i16::try_from(run.state.wins()).map_err(|_| Error::Unavailable)?;
        let defeats = i16::try_from(run.state.defeats()).map_err(|_| Error::Unavailable)?;
        if wins != 8 && defeats != 5 {
            return Ok(());
        }
        // At most twelve immutable reservations; technical retries cannot switch lanes.
        // The final reward choice has no bearing on the already verified match score.
        db.execute_raw(self.statement(
            "INSERT INTO @tower.personal_bests AS old(account,cohort,house,wins,defeats,listed)
             SELECT $1,$2,EXISTS(SELECT 1 FROM @tower.reservations r JOIN @tower.settled_duels d ON d.reservation=r.id WHERE r.run=$3 AND r.opponent_account IS NULL),$4,$5,p.rank_listed
             FROM @tower.players p WHERE p.account=$1
             ON CONFLICT(account,cohort,house) DO UPDATE SET wins=EXCLUDED.wins,defeats=EXCLUDED.defeats
             WHERE EXCLUDED.wins>old.wins OR (EXCLUDED.wins=old.wins AND EXCLUDED.defeats<old.defeats)",
            vec![account.into(), run.cohort.into(), run.id.into(), wins.into(), defeats.into()],
        )).await.map_err(|_| Error::Unavailable)?;
        Ok(())
    }
}

fn valid_name(name: &Name) -> bool {
    name.valid()
        && central_api::profile::valid_display_name(&name.display)
        && central_api::profile::valid_tag(name.tag)
}

fn standing(row: &sea_orm::QueryResult, account: Uuid, listed: bool) -> Result<Standing, Error> {
    let name = if listed {
        let name = Name {
            display: column(row, "rank_name")?,
            tag: u32::try_from(column::<i64>(row, "rank_tag")?).map_err(|_| Error::Unavailable)?,
        };
        if !valid_name(&name) {
            return Err(Error::Unavailable);
        }
        Some(name)
    } else {
        None
    };
    Ok(Standing {
        rank: None,
        name,
        wins: u8::try_from(column::<i16>(row, "wins")?).map_err(|_| Error::Unavailable)?,
        defeats: u8::try_from(column::<i16>(row, "defeats")?).map_err(|_| Error::Unavailable)?,
        yours: column::<Uuid>(row, "account")? == account,
    })
}
