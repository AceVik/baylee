use super::{
    Catalog, Cohort, Collection, ConnectionTrait, Deserialize, Digest, Error, Serialize, Sha256,
    Store, Uuid, column, decode, encode, uuid7,
};

/// Reviewed immutable cohort inputs. No secrets, external decks or caller-defined effects.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Publication {
    /// Native revision, generator and exact retained engine identity.
    pub cohort: Cohort,
    /// Reviewed, fully implemented card pool and frozen prices.
    pub catalog: Catalog,
    /// Three legacy choices, or five to eight independent run choices.
    pub starters: Vec<Collection>,
}

impl Publication {
    /// Structural admission; does not certify engine coverage or game balance.
    /// # Errors
    /// Incompatible cohort or invalid starter/card pool.
    pub fn validate(&self) -> Result<(), Error> {
        self.cohort.validate(&self.catalog)?;
        self.catalog.validate_starters(&self.starters)?;
        Ok(())
    }
}

impl Store {
    /// Publish an immutable cohort under a separately authorized operator role.
    /// Exact repeats return the existing database-allocated identity.
    /// # Errors
    /// Invalid content, unavailable database or a structural bound.
    pub async fn publish(&self, publication: &Publication) -> Result<Uuid, Error> {
        publication.validate()?;
        let mut publication = publication.clone();
        publication.catalog.cards.sort_by_key(|card| card.card);
        for starter in &mut publication.starters {
            starter.owned.sort_by_key(|stack| stack.card);
            starter.deck.sort_by_key(|stack| stack.card);
            if let Some(formation) = &mut starter.formation {
                formation.sideboard.sort_by_key(|stack| stack.card);
            }
        }
        let bytes = encode(&publication)?;
        let contract = Sha256::new()
            .chain_update(b"baylee/tower/cohort/v2\0")
            .chain_update(encode(&publication.cohort)?)
            .finalize()
            .to_vec();
        let digest = Sha256::new()
            .chain_update(b"baylee/tower/publication/v2\0")
            .chain_update(&bytes)
            .finalize()
            .to_vec();
        let row = self
            .db
            .query_one_raw(self.statement(
                "INSERT INTO @tower.cohorts AS saved(digest,publication,contract) VALUES($1,$2,$3)
            ON CONFLICT(contract) DO UPDATE SET contract=EXCLUDED.contract
            WHERE saved.publication=EXCLUDED.publication RETURNING id",
                vec![digest.into(), bytes.into(), contract.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::Conflict)?;
        column(&row, "id")
    }

    pub(super) async fn publication(
        &self,
        db: &impl ConnectionTrait,
        id: Uuid,
        starting: bool,
    ) -> Result<Publication, Error> {
        uuid7(id)?;
        let row = db
            .query_one_raw(self.statement(
                "SELECT publication FROM @tower.cohorts WHERE id=$1 AND (NOT $2 OR accepting)",
                vec![id.into(), starting.into()],
            ))
            .await
            .map_err(|_| Error::Unavailable)?
            .ok_or(Error::NotFound)?;
        let publication: Publication = decode(&column::<Vec<u8>>(&row, "publication")?)?;
        publication.validate()?;
        Ok(publication)
    }
}
