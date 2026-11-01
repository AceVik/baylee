use crate::{
    CentralHttp, Endpoint, Error, GatewayHttp,
    data::{AccountData, LegacyData},
    wire::Discovery,
};
use central_api::legacy::LegacySnapshot;
use central_client::Session;
use zeroize::Zeroizing;

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
    /// the password only ever unwraps the master key.
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
        let mut session = central_client::login(&http, identity, password.to_string())?;
        drop(password);
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

    fn resume_migration(&mut self) -> Result<(), Error> {
        let Some(legacy) = &self.data.legacy else {
            return Ok(());
        };
        if legacy.complete {
            if self.session.migration_closed(&self.http)? {
                return Ok(());
            }
            // Old clients marked cutover complete before Central had a separate close gate;
            // Central closes it now, without another export or account-document rewrite.
            return self.session.legacy_link(&self.http, legacy.digest);
        }
        // The close is idempotent: a lost answer is retried with the same digest. Only a
        // source that is not the stored one asks for the stored snapshot again.
        for _ in 0..2 {
            let digest = self.data.legacy.as_ref().ok_or(Error::Rejected)?.digest;
            match self.session.legacy_link(&self.http, digest) {
                Ok(()) => {
                    if let Some(legacy) = &mut self.data.legacy {
                        legacy.complete = true;
                    }
                    return self.session.save(&self.http, &self.data.write()?);
                }
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
        let previous = self.data.legacy.as_ref().ok_or(Error::Rejected)?;
        if previous.account_id != snapshot.legacy_id {
            return Err(Error::Rejected);
        }
        let mut next = from_source(self.data.gateways.clone(), &previous.gateway, snapshot)?;
        next.chat = std::mem::take(&mut self.data.chat);
        self.data = next;
        self.session.save(&self.http, &self.data.write()?)?;
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
        let source = if self.http.endpoint().legacy_origin().is_some() {
            GatewayHttp::live(gateway)?
        } else {
            GatewayHttp::development(gateway)?
        };
        source.admit(self.session.gateway_ticket(&self.http, gateway.into())?)
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

    /// Refresh and restore all key records for one exact gateway/account scope.
    /// No absence is returned for network, authentication or format failures.
    ///
    /// # Errors
    /// Refuses unavailable or damaged documents; callers must never generate replacements then.
    pub fn chat_devices(
        &mut self,
        gateway: &str,
        tag: &str,
    ) -> Result<Vec<crate::chat::ChatDevice>, Error> {
        self.session.refresh(&self.http)?;
        let current = AccountData::read(self.session.document())?;
        let devices = current
            .chat
            .devices
            .iter()
            .filter(|entry| entry.gateway == gateway && entry.tag == tag)
            .cloned()
            .collect();
        self.data = current;
        Ok(devices)
    }

    /// Durably add an installation's keys before registering its public device with a gateway.
    /// A retry of the exact record is idempotent; replacement is always a conflict.
    ///
    /// # Errors
    /// Rejects conflicts, invalid records and unavailable storage, preserving every existing key.
    pub fn archive_chat_device(&mut self, device: &crate::chat::ChatDevice) -> Result<(), Error> {
        device.validate()?;
        for _ in 0..3 {
            self.session.refresh(&self.http)?;
            let mut current = AccountData::read(self.session.document())?;
            current.chat.insert(device.clone())?;
            let encoded = current.write()?;
            if encoded.as_slice() != self.session.document() {
                match self.session.save(&self.http, &encoded) {
                    Err(Error::Conflict) => continue,
                    Err(error) => return Err(error),
                    Ok(()) => {}
                }
            }
            self.data = current;
            return Ok(());
        }
        Err(Error::Conflict)
    }

    /// Retain decryptability while marking an installation's identity permanently retired.
    ///
    /// # Errors
    /// Unknown or changed records, conflicts and unavailable storage are refused.
    pub fn retire_chat_device(&mut self, device: &crate::chat::ChatDevice) -> Result<(), Error> {
        self.session.refresh(&self.http)?;
        let mut current = AccountData::read(self.session.document())?;
        let existing = current
            .chat
            .devices
            .iter_mut()
            .find(|entry| entry.same_slot(device))
            .ok_or(Error::Rejected)?;
        if existing.key != device.key || existing.device != device.device {
            return Err(Error::Conflict);
        }
        existing.retired = true;
        let encoded = current.write()?;
        if encoded.as_slice() != self.session.document() {
            self.session.save(&self.http, &encoded)?;
        }
        self.data = current;
        Ok(())
    }

    /// Remember an installation without creating a second active protocol identity.
    ///
    /// # Errors
    /// Invalid identifiers, unknown keys, revoked identities and CAS failures are rejected.
    pub fn remember_chat_installation(
        &mut self,
        device: &crate::chat::ChatDevice,
        installation: String,
    ) -> Result<(), Error> {
        self.session.refresh(&self.http)?;
        let mut current = AccountData::read(self.session.document())?;
        let known = current
            .chat
            .devices
            .iter()
            .find(|entry| entry.same_slot(device))
            .ok_or(Error::Rejected)?;
        if known.key != device.key || known.device != device.device || known.retired {
            return Err(Error::Rejected);
        }
        current.chat.remember(device, installation)?;
        let encoded = current.write()?;
        if encoded.as_slice() != self.session.document() {
            self.session.save(&self.http, &encoded)?;
        }
        self.data = current;
        Ok(())
    }

    /// Publish a complete uploaded snapshot without overwriting another manifest generation.
    ///
    /// # Errors
    /// Any key mismatch, retirement, stale generation or unavailable storage fails closed.
    pub fn save_chat_history(
        &mut self,
        device: &crate::chat::ChatDevice,
        expected: u64,
        history: crate::chat::HistorySnapshot,
    ) -> Result<(), Error> {
        history.validate()?;
        if history.generation != expected.checked_add(1).ok_or(Error::Rejected)? {
            return Err(Error::Rejected);
        }
        self.session.refresh(&self.http)?;
        let mut current = AccountData::read(self.session.document())?;
        let existing = current
            .chat
            .devices
            .iter_mut()
            .find(|entry| entry.same_slot(device))
            .ok_or(Error::Rejected)?;
        if existing.key != device.key || existing.device != device.device || existing.retired {
            return Err(Error::Conflict);
        }
        if existing.history.as_ref() == Some(&history) {
            self.data = current;
            return Ok(());
        }
        if existing
            .history
            .as_ref()
            .map_or(0, |value| value.generation)
            != expected
        {
            return Err(Error::Conflict);
        }
        existing.history = Some(history);
        self.session.save(&self.http, &current.write()?)?;
        self.data = current;
        Ok(())
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

    /// Upload a new immutable history chunk, encrypted inside the SDK.
    ///
    /// # Errors
    /// Allocation/write failure is not published to a manifest; ambiguous objects remain unreferenced.
    pub fn create_chat_chunk(&self, bytes: &[u8]) -> Result<crate::chat::HistoryChunk, Error> {
        use sha2::{Digest, Sha256};
        if bytes.is_empty() || bytes.len() > 256 * 1024 {
            return Err(Error::Rejected);
        }
        let object = self.session.create_object(&self.http)?;
        let revision = self
            .session
            .write_object(&self.http, object.id, object.revision, bytes)?;
        Ok(crate::chat::HistoryChunk {
            id: object.id,
            revision,
            bytes: u32::try_from(bytes.len()).map_err(|_| Error::Rejected)?,
            digest: Sha256::digest(bytes).into(),
        })
    }

    /// Read one exact revision/length from an authenticated history manifest.
    ///
    /// # Errors
    /// Changed, truncated, unavailable or unowned chunks are never treated as absent history.
    pub fn read_chat_chunk(
        &self,
        chunk: &crate::chat::HistoryChunk,
    ) -> Result<Zeroizing<Vec<u8>>, Error> {
        let object = self
            .session
            .read_object(&self.http, chunk.id, chunk.revision)?;
        if object.revision != chunk.revision || !chunk.matches(&object.plaintext) {
            return Err(Error::Rejected);
        }
        Ok(object.plaintext)
    }

    /// Remove an unreferenced old chunk after a new manifest was acknowledged.
    ///
    /// # Errors
    /// A changed revision is not deleted; remote failure leaves harmless encrypted garbage.
    pub fn delete_chat_chunk(&self, chunk: &crate::chat::HistoryChunk) -> Result<(), Error> {
        self.session
            .delete_object(&self.http, chunk.id, chunk.revision)
            .map(|_| ())
    }

    /// Restore a complete ciphertext-file snapshot through authenticated, bounded object reads.
    ///
    /// # Errors
    /// A missing or damaged chunk aborts the whole snapshot without a partial-success result.
    pub fn read_chat_history(
        &self,
        device: &crate::chat::ChatDevice,
    ) -> Result<std::collections::BTreeMap<String, Vec<u8>>, Error> {
        device.validate()?;
        let mut files = std::collections::BTreeMap::new();
        if let Some(history) = &device.history {
            for file in &history.files {
                let mut bytes = Vec::new();
                for chunk in &file.chunks {
                    bytes.extend_from_slice(&self.read_chat_chunk(chunk)?);
                }
                files.insert(file.name.clone(), bytes);
            }
        }
        Ok(files)
    }

    /// Upload a complete per-device snapshot, then atomically publish its encrypted manifest.
    /// Old chunks are removed only after publication is acknowledged.
    ///
    /// # Errors
    /// Oversize input, source/key changes or any failed upload leave the previous manifest intact.
    pub fn upload_chat_history(
        &mut self,
        device: &crate::chat::ChatDevice,
        files: &std::collections::BTreeMap<String, Vec<u8>>,
    ) -> Result<(), Error> {
        use crate::chat::{HistoryFile, HistorySnapshot};
        // Validate all paths and sizes before allocating any server objects.
        let skeleton = HistorySnapshot {
            generation: 1,
            files: files
                .keys()
                .map(|name| HistoryFile {
                    name: name.clone(),
                    chunks: Vec::new(),
                })
                .collect(),
        };
        skeleton.validate()?;
        if files.values().any(|bytes| bytes.len() > 16 * 1024 * 1024)
            || files.values().map(Vec::len).sum::<usize>() > 256 * 1024 * 1024
        {
            return Err(Error::Rejected);
        }
        let records = self.chat_devices(&device.gateway, &device.tag)?;
        let current = records
            .iter()
            .find(|entry| entry.same_slot(device))
            .ok_or(Error::Rejected)?;
        if current.key != device.key || current.device != device.device || current.retired {
            return Err(Error::Conflict);
        }
        let previous = current.history.clone();
        let expected = previous.as_ref().map_or(0, |history| history.generation);
        let mut history = HistorySnapshot {
            generation: expected.checked_add(1).ok_or(Error::Rejected)?,
            files: Vec::new(),
        };
        for (name, bytes) in files {
            let mut chunks = Vec::new();
            let prior = previous
                .as_ref()
                .and_then(|history| history.files.iter().find(|file| file.name == *name));
            for (at, chunk) in bytes.chunks(256 * 1024).enumerate() {
                if let Some(kept) = prior
                    .and_then(|file| file.chunks.get(at))
                    .filter(|kept| kept.matches(chunk))
                {
                    chunks.push(kept.clone());
                } else {
                    chunks.push(self.create_chat_chunk(chunk)?);
                }
            }
            history.files.push(HistoryFile {
                name: name.clone(),
                chunks,
            });
        }
        if previous
            .as_ref()
            .is_some_and(|prior| prior.files == history.files)
        {
            return Ok(());
        }
        let referenced: std::collections::BTreeSet<_> = history
            .files
            .iter()
            .flat_map(|file| file.chunks.iter().map(|chunk| chunk.id))
            .collect();
        self.save_chat_history(device, expected, history)?;
        if let Some(previous) = previous {
            for file in previous.files {
                for chunk in file.chunks {
                    if !referenced.contains(&chunk.id) {
                        let _ = self.delete_chat_chunk(&chunk);
                    }
                }
            }
        }
        Ok(())
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
    let source: Source = serde_json::from_slice(&snapshot.document).map_err(|_| Error::Rejected)?;
    if !source.preferences.is_object() || !source.decks.is_array() {
        return Err(Error::Rejected);
    }
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
        }),
        chat: crate::chat::ChatArchive::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn typed_settings_preserve_unknown_nested_fields_and_explicit_known_resets() {
        let mut prior = json!({"chat":{"enabled":true,"future":{"key":3}},"new":4});
        overlay(&mut prior, json!({"chat":{"enabled":false}}));
        assert_eq!(
            prior,
            json!({"chat":{"enabled":false,"future":{"key":3}},"new":4})
        );
        overlay(&mut prior, json!({"chat":{"enabled":null}}));
        assert_eq!(prior["chat"]["enabled"], serde_json::Value::Null);
        assert_eq!(prior["chat"]["future"]["key"], 3);
    }

    #[test]
    fn concurrent_preferences_preserve_independent_changes_but_reject_same_value_conflicts() {
        let base = json!({"keymap":{"confirm":"A", "cancel":"B"}, "future":1});
        let local = json!({"keymap":{"confirm":"C", "cancel":"B"}, "future":1});
        let remote = json!({"keymap":{"confirm":"A", "cancel":"D"}, "future":2});
        assert_eq!(
            merge(&base, &local, &remote).unwrap(),
            json!({"keymap":{"confirm":"C","cancel":"D"},"future":2})
        );
        assert_eq!(
            merge(&base, &local, &local).unwrap(),
            local,
            "a lost acknowledgement is idempotent"
        );
        assert!(
            merge(
                &base,
                &local,
                &json!({"keymap":{"confirm":"E","cancel":"B"},"future":1})
            )
            .is_err()
        );
        assert!(merge(&json!({}), &json!({"new":1}), &json!({"new":2})).is_err());
    }
}
