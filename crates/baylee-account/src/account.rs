use crate::{
    CentralHttp, Endpoint, Error,
    data::{AccountData, LegacyData},
    migration::{Credentials, LegacyHttp, Snapshot},
    wire::{Discovery, TicketPurpose},
};
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
}

impl Account {
    /// Authenticate centrally and resume a previous migration without a second legacy password.
    /// Only explicit live discovery permits the first proof to the compiled official gateway.
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
                if http.endpoint().legacy_origin().is_none() {
                    return Err(Error::Rejected);
                }
                let enrollment = LegacyHttp::live().enroll(&Credentials {
                    username,
                    password: password.to_string(),
                })?;
                let mut gateways = local_gateways;
                if !gateways.iter().any(|g| g == central_api::LEGACY_ORIGIN) {
                    gateways.insert(0, central_api::LEGACY_ORIGIN.into());
                }
                let data = from_snapshot(gateways, enrollment.snapshot);
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
        let mut account = Self {
            http,
            session,
            submitted_preferences: data.preferences.clone(),
            submitted_gateways: data.gateways.clone(),
            data,
        };
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

    fn capability(&self, purpose: TicketPurpose) -> Result<crate::Token, Error> {
        self.session
            .gateway_capability(&self.http, central_api::LEGACY_ORIGIN.into(), purpose)
    }

    fn resume_migration(&mut self) -> Result<(), Error> {
        let Some(legacy) = &self.data.legacy else {
            return Ok(());
        };
        if legacy.complete {
            return Ok(());
        }
        if self.http.endpoint().legacy_origin() != Some(legacy.gateway.as_str()) {
            return Err(Error::Rejected);
        }
        let gateway = LegacyHttp::live();
        // First link is idempotent: a lost prior acknowledgement must not trigger an export
        // that overwrites already-central edits. Only a source conflict requests a new snapshot.
        for _ in 0..3 {
            let digest = self.data.legacy.as_ref().ok_or(Error::Rejected)?.digest;
            match gateway.link(self.capability(TicketPurpose::Link)?, digest) {
                Ok(()) => {
                    if let Some(legacy) = &mut self.data.legacy {
                        legacy.complete = true;
                    }
                    return self.session.save(&self.http, &self.data.write()?);
                }
                Err(Error::Conflict) => {
                    let snapshot = gateway.export(self.capability(TicketPurpose::Export)?)?;
                    if self
                        .data
                        .legacy
                        .as_ref()
                        .is_none_or(|old| old.account_id != snapshot.account_id)
                    {
                        return Err(Error::Rejected);
                    }
                    self.data = from_snapshot(self.data.gateways.clone(), snapshot);
                    self.session.save(&self.http, &self.data.write()?)?;
                    self.session.refresh(&self.http)?;
                    self.data = AccountData::read(self.session.document())?;
                    self.session.confirm_restored(&self.http)?;
                }
                Err(error) => return Err(error),
            }
        }
        Err(Error::Conflict)
    }

    /// Gateway admission never exposes the central password, master key or session token.
    ///
    /// # Errors
    /// Refuses unsupported/unregistered gateways and failed or expired admission.
    pub fn admit(&self, gateway: &str) -> Result<serde_json::Value, Error> {
        let source = if self.http.endpoint().legacy_origin().is_some() {
            if gateway != central_api::LEGACY_ORIGIN {
                return Err(Error::Rejected);
            }
            LegacyHttp::live()
        } else {
            LegacyHttp::development(gateway)?
        };
        source.admit(self.session.gateway_ticket(&self.http, gateway.into())?)
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
            let target = submitted.as_object_mut().ok_or(Error::Rejected)?;
            target.extend(preferences.as_object().ok_or(Error::Rejected)?.clone());
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

    /// Revoke remotely before discarding local keys.
    ///
    /// # Errors
    /// Failure means remote revocation could not be confirmed; secrets still drop locally.
    pub fn logout(self) -> Result<(), Error> {
        self.session.logout(&self.http)
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

fn from_snapshot(gateways: Vec<String>, snapshot: Snapshot) -> AccountData {
    AccountData {
        version: 1,
        gateways,
        preferences: snapshot.preferences,
        legacy: Some(LegacyData {
            digest: snapshot.digest,
            complete: false,
            gateway: central_api::LEGACY_ORIGIN.into(),
            account_id: snapshot.account_id,
            username: snapshot.username,
            decks: snapshot.decks,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
