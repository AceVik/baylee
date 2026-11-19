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

impl Account {
    /// Choose how the account is shown to its friends: online, away or
    /// offline. Stored at Central, so it holds on every device; the answer
    /// is the stored choice, and each push socket of the account says it too.
    ///
    /// # Errors
    /// `TermsRequired` until Central's terms in force are accepted, `Busy`
    /// past ten choices a minute, a rejected session and transport failures.
    pub fn set_presence(
        &self,
        choice: central_api::presence::Choice,
    ) -> Result<central_api::presence::Choice, Error> {
        self.session.set_presence(&self.http, choice)
    }

    /// The account's stored choice.
    ///
    /// # Errors
    /// `Busy` past ten choices and readings a minute, a rejected session and
    /// transport failures.
    pub fn presence_choice(&self) -> Result<central_api::presence::Choice, Error> {
        self.session.presence_choice(&self.http)
    }

    /// Say what the client is doing: playing, at a table or neither, whether
    /// it has been idle for ten minutes, and the table it sits at if the
    /// player shares it. Central hears it only while this session's push
    /// socket is open, so it is said again after each dial. Give a table only
    /// after the socket's `ready` announced that Central takes one: an older
    /// Central refuses the whole report for it.
    ///
    /// # Errors
    /// A rejected session and transport failures.
    pub fn report_presence(
        &self,
        activity: Option<central_api::presence::Activity>,
        idle: bool,
        table: Option<central_api::presence::Table>,
    ) -> Result<(), Error> {
        self.session
            .report_presence(&self.http, activity, idle, table)
    }
}
