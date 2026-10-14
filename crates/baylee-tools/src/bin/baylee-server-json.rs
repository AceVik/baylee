//! Strict, native JSON reader for the server's deployment/reboot scripts.
//! Missing or malformed state is an error, never an empty-server guess.
use std::io::{self, Read};

use anyhow::{Context, Result, bail};
use serde_json::Value;

fn count(value: &Value) -> Result<u64> {
    value
        .as_u64()
        .context("health count must be a nonnegative integer")
}

fn read_state(input: &str, args: &[String]) -> Result<String> {
    let data: Value = serde_json::from_str(input).context("invalid gateway JSON")?;
    match args
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["open-games", field @ ("running" | "local_running")] => {
            let games = data
                .get("games")
                .and_then(Value::as_object)
                .context("no games object")?;
            let running = games
                .get(*field)
                .or_else(|| games.get("running"))
                .context("no running count")?;
            let waiting = games.get("waiting").context("no waiting count")?;
            Ok(format!("{} {}", count(running)?, count(waiting)?))
        }
        ["health"] => {
            let running = count(&data["games"]["running"])?;
            let waiting = count(&data["games"]["waiting"])?;
            let admission = match data.get("admission") {
                None => "missing",
                Some(Value::String(s)) if !s.is_empty() && !s.chars().any(char::is_whitespace) => s,
                _ => bail!("invalid admission state"),
            };
            Ok(format!("{running} {waiting} {admission}"))
        }
        ["info"] => {
            let commit = data["commit"].as_str().context("no commit")?;
            if commit.len() != 40 || !commit.bytes().all(|b| b.is_ascii_hexdigit()) {
                bail!("invalid commit");
            }
            let dirty = match data["dirty"].as_bool() {
                Some(true) => "dirty",
                Some(false) => "clean",
                None => "unknown",
            };
            Ok(format!("{commit} {dirty}"))
        }
        _ => bail!("usage: baylee-server-json open-games running|local_running | health | info"),
    }
}

fn main() -> Result<()> {
    let mut input = String::new();
    io::stdin().take((1 << 20) + 1).read_to_string(&mut input)?;
    if input.len() > 1 << 20 {
        bail!("gateway response too large");
    }
    println!(
        "{}",
        read_state(&input, &std::env::args().skip(1).collect::<Vec<_>>())?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args(s: &[&str]) -> Vec<String> {
        s.iter().map(|s| (*s).into()).collect()
    }
    #[test]
    fn local_counts_fall_back_only_when_field_is_absent() {
        let data = r#"{"games":{"running":4,"waiting":2}}"#;
        assert_eq!(
            read_state(data, &args(&["open-games", "local_running"])).unwrap(),
            "4 2"
        );
        let data = r#"{"games":{"running":4,"local_running":0,"waiting":2}}"#;
        assert_eq!(
            read_state(data, &args(&["open-games", "local_running"])).unwrap(),
            "0 2"
        );
        for bad in ["null", "-1", "false", "0.5", "\"0\""] {
            let data = format!(r#"{{"games":{{"running":0,"local_running":{bad},"waiting":0}}}}"#);
            assert!(read_state(&data, &args(&["open-games", "local_running"])).is_err());
        }
        assert!(read_state("{}", &args(&["health"])).is_err());
    }
    #[test]
    fn health_and_info_cannot_forge_shell_fields() {
        assert_eq!(
            read_state(r#"{"games":{"running":0,"waiting":1}}"#, &args(&["health"])).unwrap(),
            "0 1 missing"
        );
        assert!(
            read_state(
                r#"{"games":{"running":0,"waiting":1},"admission":"ok\n0 0"}"#,
                &args(&["health"])
            )
            .is_err()
        );
        assert!(read_state(r#"{"commit":"fake clean","dirty":false}"#, &args(&["info"])).is_err());
        let data = format!(r#"{{"commit":"{}","dirty":false}}"#, "a".repeat(40));
        assert_eq!(
            read_state(&data, &args(&["info"])).unwrap(),
            format!("{} clean", "a".repeat(40))
        );
    }
}
