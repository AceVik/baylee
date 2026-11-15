use baylee_tower::v2::{execution::MAX_RECORD_BYTES, verification::Resources};
use baylee_tower_service::Error;
use central_api::{Endpoint, Token};
use serde::{Deserialize, Deserializer};
use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
};
use uuid::Uuid;
use zeroize::Zeroizing;

pub(super) const MAX_CONFIG: usize = 64 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Config {
    #[serde(deserialize_with = "secret")]
    pub database_url: Zeroizing<String>,
    pub schema: String,
    #[serde(default)]
    pub development: bool,
    pub service: Option<Service>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Service {
    pub listen: SocketAddr,
    pub origin: String,
    /// Numeric loopback only in explicit development; live Central otherwise.
    pub central: Option<String>,
    pub record_bytes: u64,
    pub workers: Vec<Worker>,
    /// Separate local Courier and Central issuer credentials, mandatory in production.
    pub awards: Option<Awards>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Awards {
    #[serde(deserialize_with = "token")]
    pub courier: Token,
    #[serde(deserialize_with = "token")]
    pub issuer: Token,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Worker {
    pub cohort: Uuid,
    pub artifact: PathBuf,
    #[serde(deserialize_with = "token")]
    pub verifier: Token,
    #[serde(deserialize_with = "token")]
    pub supervisor: Token,
    #[serde(default)]
    pub resources: Resources,
}

fn secret<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Zeroizing<String>, D::Error> {
    String::deserialize(deserializer).map(Zeroizing::new)
}

fn token<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Token, D::Error> {
    let text = secret(deserializer)?;
    Token::from_hex(&text).map_err(|_| serde::de::Error::custom("invalid operator credential"))
}

impl Config {
    pub(super) fn parse(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() > MAX_CONFIG {
            return Err(Error::Contract);
        }
        let config: Self = serde_json::from_slice(bytes).map_err(|_| Error::Contract)?;
        if !baylee_tower_store::valid_schema(&config.schema) {
            return Err(Error::Contract);
        }
        config.database()?;
        if let Some(service) = &config.service {
            service.validate(config.development)?;
        }
        Ok(config)
    }

    fn database(&self) -> Result<(), Error> {
        let url = url::Url::parse(&self.database_url).map_err(|_| Error::Contract)?;
        if !matches!(url.scheme(), "postgres" | "postgresql")
            || url.host().is_none()
            || url.username().is_empty()
            || url.path() == "/"
            || url.path().is_empty()
            || url.fragment().is_some()
        {
            return Err(Error::Contract);
        }
        if self.development {
            let local = match url.host() {
                Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
                Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
                // PostgreSQL is a non-special URL scheme: the URL parser retains
                // a numeric IPv4 address as a domain. Require an actual IP,
                // without DNS resolution or permissive numeric-host aliases.
                Some(url::Host::Domain(host)) => host
                    .parse::<std::net::IpAddr>()
                    .is_ok_and(|ip| ip.is_loopback()),
                None => false,
            };
            if !local || url.query().is_some() {
                return Err(Error::Contract);
            }
        } else {
            let options: Vec<_> = url.query_pairs().collect();
            if options.len() != 2
                || !options
                    .iter()
                    .any(|(key, value)| key == "sslmode" && value == "verify-full")
                || !options.iter().any(|(key, value)| {
                    key == "sslrootcert" && Path::new(value.as_ref()).is_absolute()
                })
            {
                return Err(Error::Contract);
            }
        }
        Ok(())
    }
}

impl Service {
    pub(super) fn endpoint(&self, development: bool) -> Result<Endpoint, Error> {
        match (development, self.central.as_deref()) {
            (false, None) => Ok(Endpoint::live()),
            (true, Some(origin)) => Endpoint::development(origin).map_err(|_| Error::Contract),
            _ => Err(Error::Contract),
        }
    }

    fn validate(&self, development: bool) -> Result<(), Error> {
        let endpoint = self.endpoint(development)?;
        central_api::gateway::validate_origin(&self.origin, development)
            .map_err(|_| Error::Contract)?;
        if !self.listen.ip().is_loopback()
            || (!development && self.listen.port() == 0)
            || self.origin == endpoint.origin()
            || self.record_bytes == 0
            || self.record_bytes > MAX_RECORD_BYTES
            || self.workers.len() > 8
            || (!development && self.workers.is_empty())
            || (!development && self.awards.is_none())
        {
            return Err(Error::Contract);
        }
        if let Some(awards) = &self.awards
            && (awards.courier.digest() == awards.issuer.digest()
                || self.workers.iter().any(|worker| {
                    [worker.verifier.digest(), worker.supervisor.digest()]
                        .iter()
                        .any(|digest| {
                            *digest == awards.courier.digest() || *digest == awards.issuer.digest()
                        })
                }))
        {
            return Err(Error::Contract);
        }
        for (index, worker) in self.workers.iter().enumerate() {
            worker.resources.validate().map_err(|_| Error::Contract)?;
            if !worker.artifact.is_absolute()
                || worker.resources.bytes < self.record_bytes
                || worker.cohort.get_version_num() != 7
                || worker.cohort.get_variant() != uuid::Variant::RFC4122
                || self.workers[..index]
                    .iter()
                    .any(|previous| previous.cohort == worker.cohort)
            {
                return Err(Error::Contract);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
