//! A card looked up in the gateway's catalog under the seat's own session
//! (`docs/protocol.md` §"Card text"): `GET /catalog/search` for the card,
//! then `GET /catalog/text` for its faces. Blocking.

use super::{Gateway, fold};
use serde_json::Value;
use std::fmt::Write as _;
use std::time::Duration;

/// The most of an answer read.
const MAX_BODY: u64 = 1 << 20;

/// The text of the card the gateway's catalog knows by `name`: `None` when
/// it knows none.
///
/// # Errors
/// When the gateway cannot be reached or does not answer the catalog's
/// shape; the sentence never carries the session.
pub(super) fn find(
    gateway: &Gateway,
    name: &str,
    timeout: Duration,
) -> Result<Option<String>, String> {
    let agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .max_redirects(0)
        .timeout_global(Some(timeout))
        .build()
        .new_agent();
    let search = format!(
        "{}/catalog/search?q={}&limit=5",
        gateway.base,
        crate::lobby::escape(name)
    );
    let hits = get(&agent, gateway, &search)?;
    let wanted = fold(name);
    let hits = hits.as_array().map(Vec::as_slice).unwrap_or_default();
    let named = |hit: &&Value, field: &str| {
        hit.get(field)
            .and_then(Value::as_str)
            .is_some_and(|n| fold(n) == wanted)
    };
    let Some(id) = hits
        .iter()
        .find(|hit| named(hit, "english_name") || named(hit, "name"))
        .or_else(|| hits.first())
        .and_then(|hit| hit.get("scryfall_id"))
        .and_then(Value::as_str)
    else {
        return Ok(None);
    };
    let text = format!(
        "{}/catalog/text?ids={}",
        gateway.base,
        crate::lobby::escape(id)
    );
    let entries = get(&agent, gateway, &text)?;
    let Some(faces) = entries
        .as_array()
        .and_then(|entries| entries.first())
        .and_then(|entry| entry.get("faces"))
        .and_then(Value::as_array)
    else {
        return Ok(None);
    };
    let field = |face: &Value, key: &str| {
        face.get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string()
    };
    let mut out = String::new();
    for face in faces {
        let name = field(face, "english_name");
        let name = if name.is_empty() {
            field(face, "name")
        } else {
            name
        };
        let _ = write!(out, "{name}");
        let cost = field(face, "mana_cost");
        if !cost.is_empty() {
            let _ = write!(out, " {cost}");
        }
        let _ = writeln!(out, " · {}", field(face, "type_line"));
        for line in field(face, "oracle_text")
            .lines()
            .filter(|line| !line.trim().is_empty())
        {
            let _ = writeln!(out, "  {line}");
        }
    }
    Ok((!out.is_empty()).then_some(out))
}

/// One `GET` under the session, read as JSON.
fn get(agent: &ureq::Agent, gateway: &Gateway, url: &str) -> Result<Value, String> {
    let mut answer = agent
        .get(url)
        .header(
            "authorization",
            format!("Bearer {}", gateway.session.bearer()),
        )
        .header("accept", "application/json")
        .call()
        .map_err(|e| format!("the gateway's catalog could not be reached ({e})"))?;
    let status = answer.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(format!("the gateway's catalog answered HTTP {status}"));
    }
    answer
        .body_mut()
        .with_config()
        .limit(MAX_BODY)
        .read_json()
        .map_err(|e| format!("the gateway's catalog answered no JSON ({e})"))
}
