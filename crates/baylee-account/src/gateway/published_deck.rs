//! Public gateway starters can enter the private Central collection as copies.
use super::{Error, GatewayHttp};
use crate::decks::{DeckDraft, DeckSource};
use uuid::Uuid;

impl GatewayHttp {
    /// Read only the selected version of a shared starter from this gateway.
    /// The supplied bearer is the existing home gateway session.
    ///
    /// # Errors
    /// Refuses a changed revision, private deck record or malformed response.
    pub fn published_deck(&self, bearer: &str, id: Uuid, version: u64) -> Result<DeckDraft, Error> {
        #[derive(serde::Deserialize)]
        struct Published {
            id: Uuid,
            kind: String,
            version: u64,
            name: String,
            format: String,
            description: Option<String>,
            cards: Vec<String>,
            sideboard: Vec<String>,
            commanders: Vec<String>,
        }
        let response = self
            .agent
            .get(format!("{}/decks/{id}", self.origin))
            .header("Authorization", &format!("Bearer {bearer}"))
            .call()
            .map_err(|_| Error::Unavailable)?;
        let shared: Published = crate::http::decode_response(response)?;
        if shared.id != id
            || shared.version != version
            || !matches!(shared.kind.as_str(), "house" | "preconstructed")
        {
            return Err(Error::Conflict);
        }
        Ok(DeckDraft {
            name: shared.name,
            format: shared.format,
            description: shared.description.unwrap_or_default(),
            cards: shared.cards,
            sideboard: shared.sideboard,
            commanders: shared.commanders,
            copied_from: Some(DeckSource {
                gateway: Some(self.origin.clone()),
                account: None,
                deck: id.to_string(),
                revision: version,
            }),
            ..DeckDraft::default()
        })
    }
}
