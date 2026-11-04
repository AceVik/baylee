//! The local gateway, read with a bearer token. Not the client's harness
//! (that is `devctl`): a signed-in route of a server, reached at a loopback
//! origin, with no redirect, no proxy and no body or token in a diagnostic.
use anyhow::{Context, Result, ensure};
use serde_json::Value;
use std::{net::IpAddr, time::Duration};

pub(super) fn loopback_url(url: &str) -> Result<String> {
    let uri: ureq::http::Uri = url
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid loopback URL"))?;
    let host = uri.host().context("loopback URL needs a host")?;
    ensure!(
        uri.scheme_str() == Some("http")
            && uri.authority().is_some_and(|a| !a.as_str().contains('@'))
            && uri.query().is_none()
            && uri.path() == "/",
        "expected a plain loopback http origin"
    );
    let ip = if host == "localhost" {
        "127.0.0.1".parse::<IpAddr>()?
    } else {
        host.trim_matches(['[', ']'])
            .parse::<IpAddr>()
            .map_err(|_| anyhow::anyhow!("only literal loopback IPs or localhost accepted"))?
    };
    ensure!(ip.is_loopback(), "external network is not allowed");
    let port = uri.port_u16().unwrap_or(80);
    ensure!(port > 0, "port must be nonzero");
    let host = match ip {
        IpAddr::V4(v) => v.to_string(),
        IpAddr::V6(v) => format!("[{v}]"),
    };
    Ok(format!("http://{host}:{port}"))
}

pub(super) struct Gateway {
    origin: String,
    agent: ureq::Agent,
}

impl Gateway {
    pub fn new(origin: &str) -> Result<Self> {
        Ok(Self {
            origin: loopback_url(origin)?,
            agent: ureq::Agent::config_builder()
                .proxy(None)
                .max_redirects(0)
                .http_status_as_error(false)
                .build()
                .into(),
        })
    }

    pub fn get(&self, path: &str, token: &str, timeout: Duration) -> Result<Value> {
        ensure!(
            path.starts_with('/')
                && !path.starts_with("//")
                && !path.contains(['\r', '\n', '?', '#']),
            "invalid local route"
        );
        let response = self
            .agent
            .get(format!("{}{path}", self.origin))
            .config()
            .timeout_global(Some(timeout))
            .build()
            .header("Authorization", format!("Bearer {token}"))
            .call();
        // Error bodies may echo tokens. Neither they nor the underlying
        // transport error's URL and headers are attached to diagnostics.
        let mut response =
            response.map_err(|_| anyhow::anyhow!("local HTTP request failed ({path})"))?;
        ensure!(
            response.status().is_success(),
            "local HTTP {path}: status {}",
            response.status().as_u16()
        );
        let value: Value = response
            .body_mut()
            .read_json()
            .map_err(|_| anyhow::anyhow!("invalid JSON from {path}"))?;
        ensure!(
            value.get("error").is_none() && value.get("ok") != Some(&Value::Bool(false)),
            "local request refused ({path}); response body withheld"
        );
        Ok(value)
    }
}
