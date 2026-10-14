//! In-process HTTPS transport; keys never enter process arguments or diagnostics.
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use std::fmt::Write as _;
use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};

use super::policy;

const URL: &str = "https://api.deepseek.com/anthropic/v1/messages?beta=true";
const MODEL: &str = "deepseek-flash[1m]";

pub(super) struct Provider {
    key: String,
    pub max_tokens: u64,
    input_limit: u64,
    timeout: Duration,
    remaining_tokens: AtomicU64,
}

fn usable_key(key: &str) -> Result<()> {
    ensure!(
        !key.is_empty() && key.len() <= 4096 && key.bytes().all(|b| b.is_ascii_graphic()),
        "no usable DeepSeek API key"
    );
    Ok(())
}

pub(super) fn key_from_auth(text: &str) -> Result<String> {
    let value: Value =
        serde_json::from_str(text).map_err(|_| anyhow::anyhow!("auth file is not valid JSON"))?;
    let key = value
        .pointer("/deepseek/key")
        .and_then(Value::as_str)
        .context("auth file has no usable deepseek.key")?;
    usable_key(key)?;
    Ok(key.to_owned())
}

impl Provider {
    pub fn load() -> Result<Self> {
        let key = match std::env::var("DEEPSEEK_API_KEY") {
            Ok(key) if !key.is_empty() => key,
            Ok(_) | Err(std::env::VarError::NotPresent) => {
                let home = std::env::var_os("HOME").context("HOME is unavailable")?;
                let text = policy::read(
                    &PathBuf::from(home).join(".local/share/opencode/auth.json"),
                    1024 * 1024,
                )
                .map_err(|_| {
                    anyhow::anyhow!(
                        "no DeepSeek key: set DEEPSEEK_API_KEY or sign in with opencode"
                    )
                })?;
                key_from_auth(&text)?
            }
            Err(_) => bail!("DeepSeek key is not UTF-8"),
        };
        usable_key(&key)?;
        Ok(Self {
            key,
            max_tokens: policy::bound("DS_MAX_TOKENS", 64_000, 131_072)?,
            input_limit: policy::bound("BAYLEE_LLM_MAX_INPUT_TOKENS", 1_000_000, 1_000_000_000)?,
            timeout: Duration::from_secs(policy::bound("BAYLEE_LLM_REQUEST_SECONDS", 900, 3600)?),
            remaining_tokens: AtomicU64::new(policy::bound(
                "BAYLEE_LLM_BATCH_MAX_TOKENS",
                32_000_000,
                1_000_000_000,
            )?),
        })
    }

    pub fn ask(
        &self,
        prefix: &str,
        rules: &str,
        source: &str,
        deadline: Instant,
    ) -> Result<String> {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .context("batch deadline exceeded")?;
        ensure!(
            prefix.len() + rules.len() + source.len() <= 4 * 1024 * 1024,
            "packed request exceeds byte limit"
        );
        // One byte per token is a conservative admission bound for UTF-8 inputs.
        ensure!(
            (prefix.len() + rules.len() + source.len()) as u64 <= self.input_limit,
            "request exceeds configured input token admission limit"
        );
        ensure!(
            !prefix.contains(&self.key)
                && !rules.contains(&self.key)
                && !source.contains(&self.key),
            "refusing to send credentials in prompt text"
        );
        let reserved = (prefix.len() + rules.len() + source.len()) as u64 + self.max_tokens;
        reserve_tokens(&self.remaining_tokens, reserved)?;
        let request = json!({
            "model": MODEL, "max_tokens": self.max_tokens,
            "thinking": {"type":"adaptive", "display":"omitted"},
            "system":[{"type":"text", "text":prefix,"cache_control":{"type":"ephemeral"}}],
            "messages":[{"role":"user","content":format!("{rules}\n\n```rust\n{source}\n```")}],
        });
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(self.timeout.min(remaining)))
            .max_redirects(0)
            .http_status_as_error(false)
            .build()
            .into();
        let mut response = agent
            .post(URL)
            .header("x-api-key", &self.key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .send_json(request)
            .map_err(|_| anyhow::anyhow!("DeepSeek transport failed; details suppressed"))?;
        ensure!(
            response.status().is_success(),
            "DeepSeek HTTP {}; response body suppressed",
            response.status().as_u16()
        );
        let bytes = response
            .body_mut()
            .with_config()
            .limit(policy::OUTPUT_LIMIT)
            .read_to_vec()
            .map_err(|_| {
                anyhow::anyhow!("DeepSeek response exceeded size, encoding or deadline limit")
            })?;
        let answer = parse_response(&bytes, self.max_tokens, self.input_limit)?;
        ensure!(
            !answer.contains(&self.key),
            "credential appeared in model output; output suppressed"
        );
        Ok(answer)
    }
}

fn reserve_tokens(remaining: &AtomicU64, tokens: u64) -> Result<()> {
    // compare_exchange_weak keeps the workspace's 1.97.1 MSRV: try_update
    // was introduced after that floor, and fetch_update is now deprecated.
    let mut current = remaining.load(Ordering::Relaxed);
    loop {
        let next = current
            .checked_sub(tokens)
            .context("batch token admission budget exhausted")?;
        match remaining.compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return Ok(()),
            Err(observed) => current = observed,
        }
    }
}

pub(super) fn parse_response(bytes: &[u8], max_tokens: u64, input_limit: u64) -> Result<String> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|_| anyhow::anyhow!("DeepSeek returned invalid JSON"))?;
    ensure!(
        value.get("error").is_none(),
        "DeepSeek returned an API error; body suppressed"
    );
    let stop = value
        .get("stop_reason")
        .and_then(Value::as_str)
        .context("response has no stop reason")?;
    ensure!(
        stop == "end_turn" || stop == "stop_sequence",
        "DeepSeek answer is incomplete or requires tools"
    );
    let output = value
        .pointer("/usage/output_tokens")
        .and_then(Value::as_u64)
        .context("response has no output token usage")?;
    let input = value
        .pointer("/usage/input_tokens")
        .and_then(Value::as_u64)
        .context("response has no input token usage")?;
    ensure!(
        output <= max_tokens && input <= input_limit,
        "DeepSeek token usage exceeds configured limit"
    );
    let content = value
        .get("content")
        .and_then(Value::as_array)
        .context("response has no content blocks")?;
    let mut text = String::new();
    for block in content {
        if block.get("type").and_then(Value::as_str) == Some("text") {
            text.push_str(
                block
                    .get("text")
                    .and_then(Value::as_str)
                    .context("invalid response text block")?,
            );
        }
    }
    ensure!(!text.trim().is_empty(), "DeepSeek returned no text");
    Ok(text)
}

pub(super) fn pack(root: &Path, tests: bool) -> Result<String> {
    const CARDS: &[&str] = &[
        "docs/card-dsl.md",
        "crates/baylee-cards-dsl/src/effect/mod.rs",
        "crates/baylee-cards-dsl/src/cost.rs",
        "crates/baylee-cards-dsl/src/ability.rs",
        "crates/baylee-cards-dsl/src/filter.rs",
        "crates/baylee-cards-dsl/src/static_ability.rs",
        "crates/baylee-cards-dsl/src/counters.rs",
        "crates/baylee-cards-dsl/src/build.rs",
        "crates/baylee-cards/src/filters.rs",
        "crates/baylee-cards/src/tokens.rs",
    ];
    const TESTS: &[&str] = &[
        "crates/baylee-engine/src/engine/testkit.rs",
        "crates/baylee-engine/src/engine/card_tests/mod.rs",
        "crates/baylee-engine/src/choice.rs",
        "crates/baylee-engine/src/engine/card_tests/artifacts.rs",
    ];
    const EXAMPLES: &[&str] = &[
        "crates/baylee-cards/src/cards/lands/restricted/bleachbone_verge.rs",
        "crates/baylee-cards/src/cards/lands/pathway/hengegate_pathway.rs",
        "crates/baylee-cards/src/cards/artifacts/equipment/mv_1/skullclamp.rs",
        "crates/baylee-cards/src/cards/instants/mv_1/swords_to_plowshares.rs",
        "crates/baylee-cards/src/cards/creatures/mv_1/llanowar_elves.rs",
    ];
    let mut prefix = if tests {
        "# Testkit, Helfer, Taxonomie und eine fertige Testdatei\n"
    } else {
        "# Der Autorenvertrag und der ganze DSL-Wortschatz\n"
    }
    .to_owned();
    for rel in if tests { TESTS } else { CARDS } {
        append(&mut prefix, root, rel)?;
    }
    if !tests {
        prefix.push_str("\n# Fertige Dateien als Vorbild\n");
        for rel in EXAMPLES {
            if root.join(rel).exists() {
                append(&mut prefix, root, rel)?;
            }
        }
    }
    Ok(prefix)
}

fn append(prefix: &mut String, root: &Path, rel: &str) -> Result<()> {
    let path = policy::relative(root, rel)?;
    let text = policy::read(&path, policy::FILE_LIMIT)?;
    write!(prefix, "\n## `{rel}`\n\n```rust\n{text}\n```\n")?;
    ensure!(
        prefix.len() <= 4 * 1024 * 1024,
        "packed context exceeds limit"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn response_errors_and_truncation_never_echo_provider_data() {
        let bytes = br#"{"stop_reason":"end_turn","usage":{"input_tokens":10,"output_tokens":20},"content":[{"type":"thinking","thinking":"private"},{"type":"text","text":"```rust\ncode\n```"}]}"#;
        assert_eq!(
            parse_response(bytes, 64, 100).unwrap(),
            "```rust\ncode\n```"
        );
        assert!(parse_response(bytes, 19, 100).is_err());
        assert!(parse_response(bytes, 64, 9).is_err());
        let truncated = String::from_utf8(bytes.to_vec())
            .unwrap()
            .replace("end_turn", "max_tokens");
        assert!(parse_response(truncated.as_bytes(), 64, 100).is_err());
        let error = parse_response(br#"{"error":{"message":"SECRET"}}"#, 64, 100).unwrap_err();
        assert!(!error.to_string().contains("SECRET"));
        let error = parse_response(b"SECRET", 64, 100).unwrap_err();
        assert!(!error.to_string().contains("SECRET"));
    }
    #[test]
    fn auth_errors_never_echo_credentials() {
        assert_eq!(
            key_from_auth(r#"{"deepseek":{"key":"example-key"}}"#).unwrap(),
            "example-key"
        );
        assert!(key_from_auth(r#"{"deepseek":{"key":"bad\nkey"}}"#).is_err());
        assert!(
            !key_from_auth("SECRET")
                .unwrap_err()
                .to_string()
                .contains("SECRET")
        );
    }
    #[test]
    fn batch_token_reservations_never_overdraw() {
        let remaining = AtomicU64::new(100);
        reserve_tokens(&remaining, 60).unwrap();
        assert!(reserve_tokens(&remaining, 41).is_err());
        assert_eq!(remaining.load(Ordering::Relaxed), 40);
        reserve_tokens(&remaining, 40).unwrap();
        assert!(reserve_tokens(&remaining, 1).is_err());
    }
}
