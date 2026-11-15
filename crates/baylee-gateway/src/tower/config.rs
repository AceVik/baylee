use super::{Arc, BTreeMap, Mutex, Tower, TowerHost};
use baylee_account::Token;
use zeroize::Zeroizing;

impl Tower {
    pub(crate) fn from_env() -> Result<Option<Arc<Self>>, ()> {
        fn value(key: &str) -> Result<Option<String>, ()> {
            match std::env::var(key) {
                Ok(value) => Ok(Some(value)),
                Err(std::env::VarError::NotPresent) => Ok(None),
                Err(std::env::VarError::NotUnicode(_)) => Err(()),
            }
        }
        let origin = value("BAYLEE_TOWER_ORIGIN")?;
        let credential = value("BAYLEE_TOWER_HOST_TOKEN")?.map(Zeroizing::new);
        let development = value("BAYLEE_TOWER_DEVELOPMENT")?;
        Self::configured(
            origin.as_deref(),
            credential.as_deref().map(String::as_str),
            development.as_deref(),
        )
    }

    fn configured(
        origin: Option<&str>,
        credential: Option<&str>,
        development: Option<&str>,
    ) -> Result<Option<Arc<Self>>, ()> {
        if origin.is_none() && credential.is_none() && development.is_none() {
            return Ok(None);
        }
        let origin = origin.ok_or(())?;
        let credential = Token::from_hex(credential.ok_or(())?).map_err(|_| ())?;
        let development = match development {
            None | Some("0") => false,
            Some("1") => true,
            _ => return Err(()),
        };
        Ok(Some(Arc::new(Self {
            client: TowerHost::new(origin, credential, development).map_err(|_| ())?,
            origin: origin.into(),
            starts: tokio::sync::Mutex::new(()),
            pending: Mutex::new(super::pending::Claims::default()),
            games: Mutex::new(BTreeMap::new()),
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tower_host_configuration_has_no_partial_or_cross_environment_fallback() {
        assert!(serde_json::from_str::<crate::tower::routes::Play>("{}").is_ok());
        assert!(
            serde_json::from_str::<crate::tower::routes::Play>(r#"{"account":"chosen-by-caller"}"#)
                .is_err()
        );
        let token = "01".repeat(32);
        assert!(Tower::configured(None, None, None).unwrap().is_none());
        assert!(
            Tower::configured(Some("https://tower.example.invalid"), Some(&token), None)
                .unwrap()
                .is_some()
        );
        assert!(
            Tower::configured(Some("http://127.0.0.1:29871"), Some(&token), Some("1"))
                .unwrap()
                .is_some()
        );
        for (origin, credential, development) in [
            (Some("https://tower.example.invalid"), None, None),
            (None, Some(token.as_str()), None),
            (Some("http://127.0.0.1:29871"), Some(token.as_str()), None),
            (
                Some("https://tower.example.invalid"),
                Some(token.as_str()),
                Some("1"),
            ),
            (
                Some("http://localhost:29871"),
                Some(token.as_str()),
                Some("1"),
            ),
            (
                Some("https://tower.example.invalid/redirect"),
                Some(token.as_str()),
                None,
            ),
            (
                Some("https://tower.example.invalid"),
                Some("bad-secret"),
                None,
            ),
        ] {
            assert!(Tower::configured(origin, credential, development).is_err());
        }
    }
}
