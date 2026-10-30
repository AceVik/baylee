//! Versioned encrypted account document. Device geometry and guest bearer tokens never migrate here.
use crate::Error;
use serde::{Deserialize, Serialize};

/// Personal metadata stored only inside Central's client-encrypted document.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccountData {
    /// Exact application format version; unsupported versions must not be overwritten.
    pub version: u32,
    /// Account-owned saved gateway origins, shown only after successful authentication.
    pub gateways: Vec<String>,
    /// Existing account preferences, preserved including fields unknown to this client.
    pub preferences: serde_json::Value,
    /// Provenance and retained content from the explicitly authorized legacy migration.
    pub legacy: Option<LegacyData>,
    /// Account-encrypted logical chat identities and known installations; old documents start empty.
    #[serde(default)]
    pub chat: crate::chat::ChatArchive,
}

/// Source-preserving legacy backup. Password hashes and session credentials are excluded.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LegacyData {
    /// Exact source snapshot successfully encrypted before linking.
    pub digest: [u8; 32],
    /// Cutover has been acknowledged; false is resumable with central capabilities only.
    pub complete: bool,
    /// Fixed migration gateway, or the explicit loopback synthetic fixture.
    pub gateway: String,
    /// Original immutable gateway account identifier.
    pub account_id: String,
    /// Private canonical username returned only after legacy password verification.
    pub username: String,
    /// Full owned-deck export, including its original identifiers and version history.
    pub decks: serde_json::Value,
}

impl AccountData {
    /// Parse and validate a restored document without substituting local defaults on failure.
    ///
    /// # Errors
    /// Rejects unknown schemas, oversized or malformed data and unsafe saved URLs.
    pub fn read(bytes: &[u8]) -> Result<Self, Error> {
        let value: Self = central_api::decode(bytes)?;
        value.validate()?;
        Ok(value)
    }

    /// Serialize a validated document for client-side encryption.
    ///
    /// # Errors
    /// Rejects invalid or oversized application data.
    pub fn write(&self) -> Result<zeroize::Zeroizing<Vec<u8>>, Error> {
        self.validate()?;
        central_api::encode(self).map(zeroize::Zeroizing::new)
    }

    fn validate(&self) -> Result<(), Error> {
        self.chat.validate()?;
        if self.version != 1 || self.gateways.len() > 128 || !self.preferences.is_object() {
            return Err(Error::Rejected);
        }
        let mut seen = std::collections::BTreeSet::new();
        for gateway in &self.gateways {
            if gateway.len() > 2048 || !seen.insert(gateway) {
                return Err(Error::Rejected);
            }
            let url = url::Url::parse(gateway).map_err(|_| Error::Rejected)?;
            if !matches!(url.scheme(), "http" | "https")
                || url.host_str().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
                || url.query().is_some()
                || url.fragment().is_some()
            {
                return Err(Error::Rejected);
            }
        }
        if let Some(legacy) = &self.legacy {
            if legacy.account_id.is_empty()
                || legacy.account_id.len() > 64
                || legacy.username.len() > 96
                || !legacy.decks.is_array()
            {
                return Err(Error::Rejected);
            }
            if legacy.gateway != central_api::LEGACY_ORIGIN
                && crate::Endpoint::development(&legacy.gateway).is_err()
            {
                return Err(Error::Rejected);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restores_gateways_and_unknown_preference_keys_without_using_device_defaults() {
        let original = AccountData {
            version: 1,
            gateways: vec![central_api::LEGACY_ORIGIN.into()],
            preferences: serde_json::json!({"future_setting": {"value": 3}}),
            legacy: None,
            chat: crate::chat::ChatArchive::default(),
        };
        let restored = AccountData::read(&original.write().unwrap()).unwrap();
        assert_eq!(restored.gateways, original.gateways);
        assert_eq!(restored.preferences, original.preferences);
        for invalid in [b"{}".as_slice(), br#"{"version":2,"gateways":[],"preferences":{},"legacy":null}"#,
            br#"{"version":1,"gateways":["https://secret@host.invalid"],"preferences":{},"legacy":null}"#] {
            assert!(AccountData::read(invalid).is_err());
        }
    }

    /// The document the website seals at registration (baylee-central
    /// `docs/web-accounts.md` §"The initial account document"), byte for byte:
    /// a freshly registered account restores it, lists the official gateway
    /// and starts from default preferences.
    #[test]
    fn the_website_s_initial_document_restores() {
        let sealed = br#"{"version":1,"gateways":["https://baylee.acevik.de"],"preferences":{},"legacy":null,"chat":{"devices":[],"installations":[]}}"#;
        let data = AccountData::read(sealed).unwrap();
        assert_eq!(data.gateways, vec![central_api::LEGACY_ORIGIN.to_string()]);
        assert!(data.legacy.is_none());
        assert_eq!(data.preferences, serde_json::json!({}));
        assert_eq!(data.write().unwrap().as_slice(), sealed.as_slice());
    }
}
