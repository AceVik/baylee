//! What the local gateway says about itself, read strictly: missing or
//! malformed state is "unknown", never an empty server. A gateway that does
//! not answer counts as busy, so nothing is swapped on a guess.
use crate::sh;
use serde_json::Value;
use std::process::Command;

/// The gateway on this machine.
pub const GATEWAY: &str = "http://127.0.0.1:28766";
const LIMIT: usize = 1 << 20;

/// A route's body (`curl -fsS -m <secs>`), or nothing.
pub fn fetch(route: &str, secs: u32) -> Option<String> {
    sh::out_quiet(Command::new("curl").args([
        "-fsS",
        "-m",
        &secs.to_string(),
        &format!("{GATEWAY}/{route}"),
    ]))
    .filter(|body| body.len() <= LIMIT)
}

fn count(value: &Value) -> Option<u64> {
    value.as_u64()
}

/// `/health`'s running and waiting games, the running count taken from
/// `field` (`running` or `local_running`, which falls back to `running` only
/// when the gateway does not report it).
pub fn open_games(body: &str, field: &str) -> Option<(u64, u64)> {
    let data: Value = serde_json::from_str(body).ok()?;
    let games = data.get("games")?.as_object()?;
    let running = games.get(field).or_else(|| games.get("running"))?;
    Some((count(running)?, count(games.get("waiting")?)?))
}

/// `/health`'s running and waiting games anywhere, and its admission state
/// (`missing` from a gateway too old to say).
pub fn health(body: &str) -> Option<(u64, u64, String)> {
    let data: Value = serde_json::from_str(body).ok()?;
    let running = count(&data["games"]["running"])?;
    let waiting = count(&data["games"]["waiting"])?;
    let admission = match data.get("admission") {
        None => "missing".to_owned(),
        Some(Value::String(s)) if !s.is_empty() && !s.chars().any(char::is_whitespace) => s.clone(),
        _ => return None,
    };
    Some((running, waiting, admission))
}

/// `/info`'s commit and whether it was built clean (`clean`, `dirty` or
/// `unknown`); a commit that is not 40 hex digits is no commit.
pub fn info(body: &str) -> Option<(String, &'static str)> {
    let data: Value = serde_json::from_str(body).ok()?;
    let commit = data["commit"].as_str()?;
    if commit.len() != 40 || !commit.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let built = match data["dirty"].as_bool() {
        Some(true) => "dirty",
        Some(false) => "clean",
        None => "unknown",
    };
    Some((commit.to_owned(), built))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_counts_fall_back_only_when_the_field_is_absent() {
        assert_eq!(
            open_games(r#"{"games":{"running":4,"waiting":2}}"#, "local_running"),
            Some((4, 2))
        );
        assert_eq!(
            open_games(
                r#"{"games":{"running":4,"local_running":0,"waiting":2}}"#,
                "local_running"
            ),
            Some((0, 2))
        );
        for bad in ["null", "-1", "false", "0.5", "\"0\""] {
            let body = format!(r#"{{"games":{{"running":0,"local_running":{bad},"waiting":0}}}}"#);
            assert_eq!(open_games(&body, "local_running"), None, "{bad}");
        }
        assert_eq!(health("{}"), None);
        assert_eq!(open_games("not json", "running"), None);
    }

    #[test]
    fn health_and_info_cannot_be_forged_into_other_fields() {
        assert_eq!(
            health(r#"{"games":{"running":0,"waiting":1}}"#),
            Some((0, 1, "missing".into()))
        );
        assert_eq!(
            health(r#"{"games":{"running":0,"waiting":1},"admission":"ok\n0 0"}"#),
            None
        );
        assert_eq!(info(r#"{"commit":"fake clean","dirty":false}"#), None);
        let body = format!(r#"{{"commit":"{}","dirty":false}}"#, "a".repeat(40));
        assert_eq!(info(&body), Some(("a".repeat(40), "clean")));
    }
}
