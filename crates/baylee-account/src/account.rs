use crate::{
    CentralHttp, Endpoint, Error, GatewayHttp,
    data::{AccountData, LegacyData},
    wire::{Discovery, GatewayPurpose},
};
use central_api::legacy::LegacySnapshot;
use central_client::Session;
use zeroize::Zeroizing;

mod advent;
mod archive;
pub use advent::check_advent_image;
mod challenges;
mod decks;
mod funding;
mod gateway_import;
mod legacy_decks;
mod paged_decks;
mod pairing;
mod ranked;
mod social;
mod tower;

pub use funding::funding_read;
pub use gateway_import::GatewayDeckImport;
pub use pairing::{PairedSession, PairingClaim};
pub use ranked::{
    RankedConsent, RankedHost, RankedHostDocument, RankedHostDocuments, RankedPlay, ranked_read,
};
pub use tower::TowerPlay;

/// Restored native account. All network operations are blocking and belong on a worker.
pub struct Account {
    http: CentralHttp,
    session: Session,
    data: AccountData,
    // Last values submitted by this caller. A background merge must not make a
    // still-open settings form's unchanged values look like new edits.
    submitted_preferences: serde_json::Value,
    submitted_gateways: Vec<String>,
    /// A legacy move waits for Baylee Central's terms: its close is refused
    /// until they are accepted. Nothing is held for it; the password was
    /// dropped with the sign-in (`docs/central-login.md` §"Legacy migration").
    held: bool,
    /// Baylee Central's terms in force were unaccepted when the sign-in read
    /// them, and have not been accepted through this account since.
    terms_open: bool,
}

impl Account {
    /// Authenticate centrally and resume a previous migration. A legacy
    /// account (discovery says `Legacy`) signs in once at this Central with
    /// its old password, which Central checks against the official gateway's
    /// hash; the account is then enrolled and signed in through OPAQUE, and
    /// the password only ever unwraps the master key. Right after, the
    /// account's key check is stored where Central has none, so the website
    /// can pair a client with it later ([`Account::claim_pairing`]).
    ///
    /// While the account has not accepted Baylee Central's terms in force
    /// ([`Account::central_terms_open`]), the sign-in succeeds and the
    /// migration waits: show the terms, accept them ([`Account::accept_terms`])
    /// and call [`Account::finish_migration`] ([`Account::migration_held`]
    /// says whether one waits).
    ///
    /// # Errors
    /// Refuses corrupt/unsupported account data, conflicts and failed or ambiguous network calls.
    pub fn login(
        endpoint: Endpoint,
        username: String,
        password: String,
        local_gateways: Vec<String>,
    ) -> Result<Self, Error> {
        let password = Zeroizing::new(password);
        let http = CentralHttp::new(endpoint);
        let identity = match central_client::discover(&http, username.clone())? {
            Discovery::Central { identity } => identity,
            Discovery::Legacy => {
                // The one time the password leaves the device, to this Central.
                let enrollment = central_client::legacy_login(&http, username, &password)?;
                let mut gateways = local_gateways;
                if !gateways.iter().any(|g| g == &enrollment.gateway) {
                    gateways.insert(0, enrollment.gateway.clone());
                }
                let data = from_source(gateways, &enrollment.gateway, enrollment.snapshot)?;
                central_client::enroll(
                    &http,
                    enrollment.identity,
                    enrollment.ticket,
                    password.to_string(),
                    &data.write()?,
                )?;
                enrollment.identity
            }
        };
        let session = central_client::login(&http, identity, password.to_string())?;
        drop(password);
        // The key check, which an account needs before the website can pair
        // a client with it (`docs/central-login.md` §"Signing in from the
        // website"): Central takes it only within two minutes of a password
        // proof, so right here, and once. A Central that cannot store it now
        // costs this sign-in nothing; the next one tries again.
        let _ = session.ensure_key_check(&http);
        Self::restore(http, session)
    }

    /// An AI runner's session of the AI account `account`: the runner token
    /// `token` traded at Central for an ordinary native session of an
    /// account it lists (baylee-central `docs/ai-players.md` §2). No
    /// password opened it and it holds no master key, so it restores
    /// nothing: its document, library, chat keys and vault refuse, and
    /// [`Account::data`] is empty. Renewal, the terms, ranked admission and
    /// the relay work as for any native session.
    ///
    /// # Errors
    /// `Rejected` for every refusal of the token (unknown, revoked,
    /// expired, not listing the account, no AI); `Busy` past Central's
    /// budgets; transport failure. The session ends on a failure after it.
    pub fn ai_runner(
        endpoint: Endpoint,
        token: &crate::Token,
        account: [u8; 16],
    ) -> Result<Self, Error> {
        let http = CentralHttp::new(endpoint);
        let session = central_client::ai_runner_session(&http, token, account)?;
        let terms_open = match session.terms_status(&http) {
            Ok(status) => !status.accepted,
            Err(error) => {
                let _ = session.logout(&http);
                return Err(error);
            }
        };
        let data = AccountData::unrestored();
        Ok(Self {
            http,
            session,
            submitted_preferences: data.preferences.clone(),
            submitted_gateways: Vec::new(),
            data,
            held: false,
            terms_open,
        })
    }

    /// Whether this is an AI runner's session ([`Account::ai_runner`]).
    #[must_use]
    pub const fn is_ai_runner(&self) -> bool {
        self.session.is_runner()
    }

    /// The signed-in account from a fresh session, a password's or a
    /// pairing's: its document read and checked, the restore confirmed,
    /// Central's terms asked about, and a move from a gateway account
    /// resumed. The session ends on any failure.
    fn restore(http: CentralHttp, mut session: Session) -> Result<Self, Error> {
        let data = match AccountData::read(session.document()) {
            Ok(data) => data,
            Err(error) => {
                let _ = session.logout(&http);
                return Err(error);
            }
        };
        if let Err(error) = session.confirm_restored(&http) {
            let _ = session.logout(&http);
            return Err(error);
        }
        let terms_open = match session.terms_status(&http) {
            Ok(status) => !status.accepted,
            Err(error) => {
                let _ = session.logout(&http);
                return Err(error);
            }
        };
        let mut account = Self {
            http,
            session,
            submitted_preferences: data.preferences.clone(),
            submitted_gateways: data.gateways.clone(),
            data,
            held: false,
            terms_open,
        };
        if terms_open && account.data.legacy.is_some() {
            account.held = true;
            return Ok(account);
        }
        if let Err(error) = account.resume_migration() {
            let _ = account.session.logout(&account.http);
            return Err(error);
        }
        account
            .submitted_preferences
            .clone_from(&account.data.preferences);
        account
            .submitted_gateways
            .clone_from(&account.data.gateways);
        Ok(account)
    }

    /// Validated account-owned metadata, available only after successful restoration.
    #[must_use]
    pub const fn data(&self) -> &AccountData {
        &self.data
    }

    /// Trusted Central origin (`http://127.0.0.1:...` or `https://central.baylee.app`).
    #[must_use]
    pub fn endpoint_origin(&self) -> &str {
        self.http.endpoint().origin()
    }

    /// Central's stable public profile identifier for the authenticated account.
    #[must_use]
    pub fn public_id(&self) -> crate::ranked::Id {
        self.session.identity().account
    }

    /// Active `Authorization: Bearer ...` header for opening the Central `/v1/ws` push stream.
    #[must_use]
    pub fn bearer_header(&self) -> Option<Zeroizing<String>> {
        self.http.last_bearer()
    }

    /// Public `Name#Tag` handle (or username) for the native lobby header,
    /// read after sign-in without requiring accepted terms.
    #[must_use]
    pub fn profile_handle(&self) -> Option<String> {
        let export = self.session.export(&self.http).ok()?;
        let name = export
            .display_name
            .filter(|name| !name.trim().is_empty())
            .unwrap_or(export.username);
        Some(format_profile_handle(name, export.tag))
    }

    fn resume_migration(&mut self) -> Result<(), Error> {
        if self.data.legacy.is_none() {
            return Ok(());
        }
        for _ in 0..3 {
            let legacy = self.data.legacy.as_ref().ok_or(Error::Rejected)?;
            let digest = legacy.digest;
            let result = if legacy.paged.is_some() {
                self.finish_paged_move()
            } else if legacy.complete && self.session.migration_closed(&self.http)? {
                Ok(())
            } else {
                self.session.legacy_link(&self.http, digest)
            };
            match result {
                Ok(()) => return self.confirm_legacy_closed(digest),
                Err(Error::Conflict) => {
                    let snapshot = self.session.legacy_export(&self.http)?;
                    self.restore_source(snapshot)?;
                }
                Err(error) => return Err(error),
            }
        }
        Err(Error::Conflict)
    }

    fn restore_source(&mut self, snapshot: LegacySnapshot) -> Result<(), Error> {
        self.session.refresh(&self.http)?;
        let mut current = AccountData::read(self.session.document())?;
        let previous = current.legacy.as_ref().ok_or(Error::Rejected)?;
        if previous.account_id != snapshot.legacy_id {
            return Err(Error::Rejected);
        }
        if previous.digest == snapshot.digest {
            self.data = current;
            return self.session.confirm_restored(&self.http);
        }
        let next = from_source(current.gateways.clone(), &previous.gateway, snapshot)?;
        if next
            .legacy
            .as_ref()
            .is_some_and(|source| source.paged.is_none())
        {
            current.preferences = next.preferences;
        }
        current.legacy = next.legacy;
        self.session.save(&self.http, &current.write()?)?;
        self.session.refresh(&self.http)?;
        self.data = AccountData::read(self.session.document())?;
        self.session.confirm_restored(&self.http)
    }

    /// Whether a migration waits for Baylee Central's terms to be accepted.
    #[must_use]
    pub const fn migration_held(&self) -> bool {
        self.held
    }

    /// Whether Baylee Central's terms in force waited for this account when it
    /// signed in, and have not been accepted through it since: they come
    /// before any gateway, since Central gives no ticket until then.
    #[must_use]
    pub const fn central_terms_open(&self) -> bool {
        self.terms_open
    }

    /// Resume the migration [`Account::login`] held back for Baylee Central's
    /// terms, once they are accepted. Nothing when none waits.
    ///
    /// # Errors
    /// As the sign-in's migration: conflicts, refusals and failed network
    /// calls. The next sign-in resumes it.
    pub fn finish_migration(&mut self) -> Result<(), Error> {
        if !std::mem::take(&mut self.held) {
            return Ok(());
        }
        self.resume_migration()?;
        self.submitted_preferences
            .clone_from(&self.data.preferences);
        self.submitted_gateways.clone_from(&self.data.gateways);
        Ok(())
    }

    /// Gateway admission never exposes the central password, master key or session token.
    ///
    /// # Errors
    /// Refuses unsupported/unregistered gateways and failed or expired admission.
    pub fn admit(&self, gateway: &str) -> Result<crate::Admission, Error> {
        self.admit_saying(gateway).map_err(|turned| turned.error)
    }

    /// [`Account::admit`], with what the gateway said when it turned the account
    /// away.
    ///
    /// # Errors
    /// As [`Account::admit`].
    pub fn admit_saying(&self, gateway: &str) -> Result<crate::Admission, crate::Turned> {
        let (source, token) = self.gateway_token(gateway)?;
        source.admit(&token)
    }

    /// Carries a gateway's session on with a later token: it lasts as long as
    /// that token, and the terms the account accepted since reach the gateway.
    ///
    /// # Errors
    /// As [`Account::admit`]; [`Error::Rejected`] also when the gateway does
    /// not hold the session any more, and admitting again is the way back.
    pub fn refresh_gateway(&self, gateway: &str) -> Result<crate::Admission, Error> {
        let (source, token) = self.gateway_token(gateway)?;
        // Only Central, asked above, can say its terms are due. A `403` from
        // the gateway says nothing about them, and a client must not offer
        // Central's sheet, whose *Later* signs out of Central, on its word.
        source.refresh(&token).map_err(|error| match error {
            Error::TermsRequired => Error::Rejected,
            other => other,
        })
    }

    /// A token for `gateway`, with the transport to present it there.
    fn gateway_token(&self, gateway: &str) -> Result<(GatewayHttp, crate::SignedToken), Error> {
        self.gateway_token_for(gateway, GatewayPurpose::Admit)
    }

    fn gateway_token_for(
        &self,
        gateway: &str,
        purpose: GatewayPurpose,
    ) -> Result<(GatewayHttp, crate::SignedToken), Error> {
        let source = if self.http.endpoint().legacy_origin().is_some() {
            GatewayHttp::live(gateway)?
        } else {
            GatewayHttp::development(gateway)?
        };
        let (token, _) = self
            .session
            .gateway_token(&self.http, gateway.into(), purpose)?;
        Ok((source, token))
    }

    /// Baylee Central's terms in force and whether this account accepted them
    /// (`docs/protocol.md` §"Terms of use (WG-1)").
    ///
    /// # Errors
    /// Refuses a revoked session and failed network calls.
    pub fn central_terms_status(&self) -> Result<central_client::TermsStatus, Error> {
        self.session.terms_status(&self.http)
    }

    /// Baylee Central's terms in force, checked against their digest.
    ///
    /// # Errors
    /// Refuses a document that does not match its digest and failed network calls.
    pub fn central_terms(&self) -> Result<Option<crate::terms::TermsDocument>, Error> {
        central_client::central_terms(&self.http)
    }

    /// Accept exactly the version and digest of the terms this client showed:
    /// Baylee Central's, or a gateway's by its origin. Accepting it again is
    /// no error.
    ///
    /// # Errors
    /// `Conflict` when the terms changed since they were read (read and show
    /// them again); refuses a revoked session and failed network calls.
    pub fn accept_terms(
        &mut self,
        subject: crate::terms::TermsSubject,
        version: &crate::terms::TermsVersion,
    ) -> Result<(), Error> {
        let central = matches!(subject, crate::terms::TermsSubject::Central);
        self.session.accept_terms(&self.http, subject, version)?;
        if central {
            self.terms_open = false;
        }
        Ok(())
    }

    /// Save changed preferences and/or saved gateways with optimistic concurrency.
    /// Unknown preference fields are retained. A conflict requires explicit reconciliation.
    ///
    /// # Errors
    /// No local success is reported for rejected, stale or ambiguous writes.
    pub fn save(
        &mut self,
        preferences: Option<serde_json::Value>,
        gateways: Option<Vec<String>>,
    ) -> Result<(), Error> {
        let mut submitted = self.submitted_preferences.clone();
        if let Some(preferences) = preferences {
            if !preferences.is_object() {
                return Err(Error::Rejected);
            }
            overlay(&mut submitted, preferences);
        }
        let submitted_gateways = gateways.unwrap_or_else(|| self.submitted_gateways.clone());
        // Read before a write to reconcile independent device edits and ambiguous prior replies.
        // Conflicting edits to the same value are refused, never last-writer-wins.
        self.session.refresh(&self.http)?;
        let current = AccountData::read(self.session.document())?;
        let preferences = merge(
            &self.submitted_preferences,
            &submitted,
            &current.preferences,
        )?;
        let gateways = if submitted_gateways == self.submitted_gateways {
            current.gateways.clone()
        } else if current.gateways == self.submitted_gateways
            || current.gateways == submitted_gateways
        {
            submitted_gateways.clone()
        } else {
            return Err(Error::Conflict);
        };
        let next = AccountData {
            gateways,
            preferences,
            ..current
        };
        let encoded = next.write()?;
        if encoded.as_slice() != self.session.document() {
            self.session.save(&self.http, &encoded)?;
        }
        self.data = next;
        self.submitted_preferences = submitted;
        self.submitted_gateways = submitted_gateways;
        Ok(())
    }

    /// The key of this device's local chat vault whose 32-byte scope is
    /// `scope` (`docs/password-vault.md` §"The account key"), derived from the
    /// master key, which never leaves the SDK. Opens that vault without the
    /// password.
    ///
    /// # Errors
    /// Only for an identity the endpoint cannot bind, never for a live account.
    pub fn local_vault_key(&self, scope: &[u8; 32]) -> Result<Zeroizing<[u8; 32]>, Error> {
        self.session.local_vault_key(scope)
    }

    /// Check whether this account session is still the sole active login.
    ///
    /// # Errors
    /// Rejection means revoked/expired; unavailability is never proof of a valid session.
    pub fn check_active(&self) -> Result<(), Error> {
        self.session.check_active(&self.http)
    }

    /// Renew the Central session: it expires 30 minutes from now, at most 12
    /// hours after this sign-in, and the gateway admissions made from it last
    /// as long (baylee-central `docs/operations.md` §"Sessions"). Returns the
    /// new absolute Unix-seconds expiry.
    ///
    /// # Errors
    /// Refuses an expired, replaced or revoked session, and a Central that does
    /// not renew yet; a failed renewal ends nothing ([`Account::check_active`]
    /// says whether the session lives).
    pub fn renew(&self) -> Result<i64, Error> {
        self.session.renew(&self.http)
    }

    /// Revoke remotely before discarding local keys.
    ///
    /// # Errors
    /// Failure means remote revocation could not be confirmed; secrets still drop locally.
    pub fn logout(self) -> Result<(), Error> {
        self.session.logout(&self.http)
    }
}

// A typed settings form cannot represent fields added by a newer client.
// Overlay recursively so submitting known fields never deletes unknown siblings.
fn overlay(target: &mut serde_json::Value, submitted: serde_json::Value) {
    if let (Some(target), Some(fields)) = (target.as_object_mut(), submitted.as_object()) {
        for (key, value) in fields {
            if let Some(previous) = target.get_mut(key) {
                overlay(previous, value.clone());
            } else {
                target.insert(key.clone(), value.clone());
            }
        }
    } else {
        *target = submitted;
    }
}

fn merge(
    base: &serde_json::Value,
    local: &serde_json::Value,
    remote: &serde_json::Value,
) -> Result<serde_json::Value, Error> {
    if local == base {
        return Ok(remote.clone());
    }
    if remote == base || local == remote {
        return Ok(local.clone());
    }
    let (Some(base), Some(local), Some(remote)) =
        (base.as_object(), local.as_object(), remote.as_object())
    else {
        return Err(Error::Conflict);
    };
    let mut merged = remote.clone();
    for key in base
        .keys()
        .chain(local.keys())
        .collect::<std::collections::BTreeSet<_>>()
    {
        let before = base.get(key);
        let desired = local.get(key);
        let current = remote.get(key);
        if desired == before {
            continue;
        }
        if current == before || current == desired {
            if let Some(desired) = desired {
                merged.insert(key.clone(), desired.clone());
            } else {
                merged.remove(key);
            }
        } else if let (Some(before), Some(desired), Some(current)) = (before, desired, current) {
            merged.insert(key.clone(), merge(before, desired, current)?);
        } else {
            return Err(Error::Conflict);
        }
    }
    Ok(serde_json::Value::Object(merged))
}

/// The official gateway's export of a legacy account, exactly as it wrote it:
/// its whole preferences document and its decks with their history.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    preferences: serde_json::Value,
    decks: serde_json::Value,
}

/// A first account document from a legacy account's source, checked
/// against its digest by the SDK already.
fn from_source(
    gateways: Vec<String>,
    origin: &str,
    snapshot: LegacySnapshot,
) -> Result<AccountData, Error> {
    let (source, paged) = match serde_json::from_slice::<Source>(&snapshot.document) {
        Ok(s) if s.preferences.is_object() && s.decks.is_array() => (s, None),
        Ok(_) => return Err(Error::Rejected),
        Err(_) => {
            let compact: central_api::legacy::ArchiveSource =
                central_api::decode(&snapshot.document)?;
            if !compact.paged_deck_source.valid() {
                return Err(Error::Rejected);
            }
            (
                Source {
                    preferences: serde_json::json!({}),
                    decks: serde_json::json!([]),
                },
                Some(compact.paged_deck_source),
            )
        }
    };
    Ok(AccountData {
        version: 1,
        gateways,
        preferences: source.preferences,
        legacy: Some(LegacyData {
            digest: snapshot.digest,
            complete: false,
            gateway: origin.into(),
            account_id: snapshot.legacy_id,
            username: snapshot.username,
            decks: source.decks,
            paged,
            restored_digest: None,
            restored_root: None,
        }),
        chat: crate::chat::ChatArchive::default(),
        deck_library: None,
    })
}

fn format_profile_handle(name: String, tag: Option<u32>) -> String {
    match tag {
        Some(tag) => format!("{name}#{tag:04x}"),
        None => name,
    }
}

#[cfg(test)]
mod tests;
