//! Narrow gateway migration protocol. No gateway can select a password destination.
use crate::{Error, Token, wire::Identity};
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

/// Fresh legacy proof, used only at the compiled official live gateway.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Credentials {
    /// Private legacy login name.
    pub username: String,
    /// Last legacy password proof, never retained after this operation.
    pub password: String,
}
impl Drop for Credentials {
    fn drop(&mut self) {
        self.username.zeroize();
        self.password.zeroize();
    }
}

/// One-use central capability; the receiving route fixes its purpose.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capability {
    /// Random short-lived capability, not a central session.
    pub ticket: Token,
}

/// Original personal content retained by the gateway throughout migration.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    /// Digest binding final cutover to this exact source snapshot.
    pub digest: [u8; 32],
    /// Original immutable account identifier.
    pub account_id: String,
    /// Canonical private login name.
    pub username: String,
    /// Full preferences including fields unknown to this client.
    pub preferences: serde_json::Value,
    /// Owned decks and their complete version history.
    pub decks: serde_json::Value,
}

/// Result of the single fresh legacy password verification.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Enrollment {
    /// Database-reserved immutable central identity.
    pub identity: Identity,
    /// Enrollment capability, incapable of replacing an existing credential.
    pub ticket: Token,
    /// Consistent source snapshot.
    pub snapshot: Snapshot,
}

/// Closed empty acknowledgement; errors never carry remote diagnostics.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Done {}

/// Cutover is bound to the exact snapshot the client encrypted and restored.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Link {
    /// One-use capability specifically for linking.
    pub ticket: Token,
    /// Source digest retained inside the authenticated encrypted account document.
    pub digest: [u8; 32],
}

/// Explicit upgrade response; only this closed response permits another fresh legacy proof.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FreshProofRequired {
    /// Must be true; transport errors and ordinary conflicts never imply this response.
    pub fresh_proof_required: bool,
}

/// Fresh source proof bound to a current Central Link capability and its immutable legacy ID.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preparation {
    /// One-use Link capability; never a Central session bearer.
    pub ticket: Token,
    /// Last legacy password proof, wiped when the request is dropped.
    pub credentials: Credentials,
}

/// A successful cutover, or the narrowly scoped upgrade prerequisite.
pub enum LinkState {
    /// Source reconciliation and credential retirement completed.
    Complete,
    /// The gateway has no source proof from before the migration-generation upgrade.
    FreshProofRequired,
}

/// Strict transport to the fixed migration source. Redirects and unverified TLS are forbidden.
pub struct LegacyHttp {
    pub(crate) origin: String,
    pub(crate) agent: ureq::Agent,
}

impl LegacyHttp {
    /// Fixed official live gateway. No caller-provided credential URL exists.
    #[must_use]
    pub fn live() -> Self {
        Self {
            origin: central_api::LEGACY_ORIGIN.into(),
            agent: crate::http::agent(true),
        }
    }

    /// Explicit synthetic fixture only; never selected by live discovery.
    ///
    /// # Errors
    /// Rejects all but numeric HTTP loopback origins with an explicit port.
    pub fn development(origin: &str) -> Result<Self, Error> {
        let endpoint = crate::Endpoint::development(origin)?;
        Ok(Self {
            origin: endpoint.origin().into(),
            agent: crate::http::agent(false),
        })
    }

    /// Perform the single authorized legacy password proof and export.
    ///
    /// # Errors
    /// Rejects failed authentication, source conflicts or transport failure.
    pub fn enroll(&self, credentials: &Credentials) -> Result<Enrollment, Error> {
        self.post("migrate", credentials)
    }

    /// Resume export without sending a password again.
    ///
    /// # Errors
    /// Rejects invalid capabilities, linked sources and unavailable storage.
    pub fn export(&self, ticket: Token) -> Result<Snapshot, Error> {
        self.post("export", &Capability { ticket })
    }

    /// Finalize only after the central copy was decrypted and the source is unchanged.
    ///
    /// # Errors
    /// Rejects a changed source, conflicting mapping or invalid capability.
    pub fn link(&self, ticket: Token, digest: [u8; 32]) -> Result<(), Error> {
        match self.link_state(ticket, digest)? {
            LinkState::Complete => Ok(()),
            LinkState::FreshProofRequired => Err(Error::Rejected),
        }
    }

    /// Inspect the explicit upgrade prerequisite without treating generic failures as permission
    /// to send a password. A caller must separately restrict this to incomplete legacy migration.
    ///
    /// # Errors
    /// Rejects malformed prerequisites, changed sources, invalid capabilities and transport errors.
    pub fn link_state(&self, ticket: Token, digest: [u8; 32]) -> Result<LinkState, Error> {
        let mut response = self.request("link", &Link { ticket, digest })?;
        if response.status().as_u16() == 428 {
            // The body remains subject to the same content-type, size and schema checks.
            // Normalize only this explicitly defined protocol status for the common decoder.
            *response.status_mut() = ureq::http::StatusCode::OK;
            let answer: FreshProofRequired = crate::http::decode_response(response)?;
            return if answer.fresh_proof_required {
                Ok(LinkState::FreshProofRequired)
            } else {
                Err(Error::Rejected)
            };
        }
        crate::http::decode_response::<Done>(response).map(|_| LinkState::Complete)
    }

    /// Reestablish the source-generation proof after an explicit upgrade prerequisite.
    /// This never enrolls or replaces a Central credential.
    ///
    /// # Errors
    /// Rejects mismatched source identity, invalid password/capability and unavailable storage.
    pub fn prepare(&self, ticket: Token, credentials: Credentials) -> Result<Snapshot, Error> {
        self.post(
            "prepare",
            &Preparation {
                ticket,
                credentials,
            },
        )
    }

    fn post<T: serde::de::DeserializeOwned>(
        &self,
        action: &str,
        body: &impl Serialize,
    ) -> Result<T, Error> {
        crate::http::decode_response(self.request(action, body)?)
    }

    fn request(
        &self,
        action: &str,
        body: &impl Serialize,
    ) -> Result<ureq::http::Response<ureq::Body>, Error> {
        let bytes = zeroize::Zeroizing::new(central_api::encode(body)?);
        self.agent
            .post(format!("{}/auth/central/{action}", self.origin))
            .header("Content-Type", "application/json")
            .send(bytes.as_slice())
            .map_err(|_| Error::Unavailable)
    }
}
