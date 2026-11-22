//! A request returns one revision. All path and query components other than
//! the registered origin are fixed strings, UUIDs or unsigned integers.
use super::GatewayHttp;
use crate::{Error, decks::gateway_export::GatewayDeckVersion};
use uuid::Uuid;

impl GatewayHttp {
    pub(crate) fn private_deck_save(
        &self,
        bearer: &str,
        after: Option<Uuid>,
        current: Option<(Uuid, u64)>,
    ) -> Result<Option<GatewayDeckVersion>, Error> {
        let query = match (after, current) {
            (_, Some((deck, version))) => format!("?deck={deck}&version={version}"),
            (Some(after), None) => format!("?after={after}"),
            (None, None) => String::new(),
        };
        let response = self
            .agent
            .get(format!("{}/decks/private-export{query}", self.origin))
            .header("Authorization", &format!("Bearer {bearer}"))
            .call()
            .map_err(|_| Error::Unavailable)?;
        crate::http::decode_response(response)
    }
}
