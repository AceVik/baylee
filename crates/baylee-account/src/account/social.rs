//! Account-owned social transport. Credentials remain inside the shared SDK.
use super::{Account, Error};
use central_api::social::{SocialRequest, SocialResponse};

impl Account {
    /// Perform a Central contact, group or encrypted-chat operation on the account worker.
    /// The native bearer token never crosses this boundary or reaches a gateway.
    ///
    /// # Errors
    /// Rejects expired/replaced sessions, unauthorized transitions and malformed replies.
    pub fn social(&self, request: SocialRequest) -> Result<SocialResponse, Error> {
        self.session.social(&self.http, request)
    }

    /// Account-derived chat signing/agreement seeds and an independent local vault key.
    /// This is the same logical identity the Central website derives; it contains
    /// neither the account master key nor the login token. Dropping it wipes its bytes.
    ///
    /// # Errors
    /// Rejects an invalid authenticated account context or failed derivation.
    pub fn chat_identity(&self) -> Result<crate::chat::DeviceKey, Error> {
        self.session
            .chat_identity()
            .map(|keys| crate::chat::DeviceKey::new(keys.to_bytes()))
    }
}
