//! What a `cli` profile may name for each agent CLI this build speaks: its
//! models and its effort levels, for the settings panel to offer
//! (`docs/llm-seat.md` §"A CLI as the model"). Pure: nothing here starts
//! a program or asks one what it knows.
//!
//! The lists are what each tool's own help or documentation named when
//! they were written (2026-10), not what a player's installation offers:
//! a tool adds models, and a subscription may not reach every one. A model
//! or an effort not listed is still played (the tool refuses one it does
//! not know); the list is a suggestion, never a check.

use super::{CliTool, DEFAULT_AGY_MODEL};

/// The efforts Claude's current models take (`low`, `medium`, `high`, `xhigh`, `max`).
pub const CLAUDE_EFFORTS: &[&str] = &["low", "medium", "high", "xhigh", "max"];

/// The efforts `agy` accepts on `--effort`.
pub const AGY_EFFORTS: &[&str] = &["low", "medium", "high", "xhigh", "max"];

/// The efforts Codex models up to `ultra` accept (`gpt-6-astra`, `gpt-6.1-sol`, `gpt-6.1-terra`, `gpt-6-sol`).
pub const CODEX_ULTRA_EFFORTS: &[&str] = &["low", "medium", "high", "xhigh", "max", "ultra"];

/// The efforts Codex models up to `max` accept (`gpt-6-luna`, `gpt-reserve`, `gpt-5.6-luna`).
pub const CODEX_MAX_EFFORTS: &[&str] = &["low", "medium", "high", "xhigh", "max"];

/// The efforts `opencode` accepts across providers.
pub const OPENCODE_EFFORTS: &[&str] = &["none", "minimal", "low", "medium", "high", "xhigh", "max"];

/// The efforts `junie` accepts on `--effort`.
pub const JUNIE_EFFORTS: &[&str] = &["low", "medium", "high"];

/// One model or alias a CLI tool offers, with its human-readable name and
/// the reasoning/thinking effort levels it accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CliModelSpec {
    /// Its own model id after `<tool>:` (`opus`, `gpt-6-astra`).
    pub id: &'static str,
    /// Human-readable display name (`Claude Opus 5.5`, `GPT-6 Astra`).
    pub label: &'static str,
    /// Supported reasoning/thinking efforts in rising order.
    pub efforts: &'static [&'static str],
    /// Default reasoning/thinking effort when none is named, if known.
    pub default_effort: Option<&'static str>,
}

const fn spec(
    id: &'static str,
    label: &'static str,
    efforts: &'static [&'static str],
    default_effort: Option<&'static str>,
) -> CliModelSpec {
    CliModelSpec {
        id,
        label,
        efforts,
        default_effort,
    }
}

const CLAUDE_MODELS: &[&str] = &[
    "opus",
    "sonnet",
    "fable",
    "claude-opus-5-5",
    "claude-sonnet-5-5",
    "claude-fable-5-1",
    "claude-haiku-5-5",
    "claude-opus-5",
    "claude-sonnet-5",
    "claude-fable-5",
    "claude-opus-4-8",
];

const CLAUDE_SPECS: &[CliModelSpec] = &[
    spec(
        "opus",
        "Claude Opus 5.5 (Alias)",
        CLAUDE_EFFORTS,
        Some("medium"),
    ),
    spec(
        "sonnet",
        "Claude Sonnet 5.5 (Alias)",
        CLAUDE_EFFORTS,
        Some("medium"),
    ),
    spec(
        "fable",
        "Claude Fable 5.1 (Alias)",
        CLAUDE_EFFORTS,
        Some("high"),
    ),
    spec(
        "claude-opus-5-5",
        "Claude Opus 5.5",
        CLAUDE_EFFORTS,
        Some("medium"),
    ),
    spec(
        "claude-sonnet-5-5",
        "Claude Sonnet 5.5",
        CLAUDE_EFFORTS,
        Some("medium"),
    ),
    spec(
        "claude-fable-5-1",
        "Claude Fable 5.1",
        CLAUDE_EFFORTS,
        Some("high"),
    ),
    spec(
        "claude-haiku-5-5",
        "Claude Haiku 5.5",
        CLAUDE_EFFORTS,
        Some("medium"),
    ),
    spec(
        "claude-opus-5",
        "Claude Opus 5",
        CLAUDE_EFFORTS,
        Some("high"),
    ),
    spec(
        "claude-sonnet-5",
        "Claude Sonnet 5",
        CLAUDE_EFFORTS,
        Some("high"),
    ),
    spec(
        "claude-fable-5",
        "Claude Fable 5",
        CLAUDE_EFFORTS,
        Some("high"),
    ),
    spec(
        "claude-opus-4-8",
        "Claude Opus 4.8",
        CLAUDE_EFFORTS,
        Some("high"),
    ),
];

const AGY_MODELS: &[&str] = &[
    DEFAULT_AGY_MODEL,
    "gemini-3.8-flash-medium",
    "gemini-3.8-flash-low",
    "gemini-4-argon-high",
    "gemini-4-argon-medium",
    "gemini-4-argon-low",
    "claude-opus-5-5-high",
    "claude-opus-5-5-medium",
    "claude-opus-5-5-low",
    "claude-sonnet-5-5-high",
    "claude-sonnet-5-5-medium",
    "claude-sonnet-5-5-low",
    "gpt-oss-120b-medium",
];

const AGY_SPECS: &[CliModelSpec] = &[
    spec(
        DEFAULT_AGY_MODEL,
        "Gemini 3.8 Flash (High)",
        AGY_EFFORTS,
        Some("high"),
    ),
    spec(
        "gemini-3.8-flash-medium",
        "Gemini 3.8 Flash (Medium)",
        AGY_EFFORTS,
        Some("medium"),
    ),
    spec(
        "gemini-3.8-flash-low",
        "Gemini 3.8 Flash (Low)",
        AGY_EFFORTS,
        Some("low"),
    ),
    spec(
        "gemini-4-argon-high",
        "Gemini 4 Argon (High)",
        AGY_EFFORTS,
        Some("high"),
    ),
    spec(
        "gemini-4-argon-medium",
        "Gemini 4 Argon (Medium)",
        AGY_EFFORTS,
        Some("medium"),
    ),
    spec(
        "gemini-4-argon-low",
        "Gemini 4 Argon (Low)",
        AGY_EFFORTS,
        Some("low"),
    ),
    spec(
        "claude-opus-5-5-high",
        "Claude Opus 5.5 (High)",
        AGY_EFFORTS,
        Some("high"),
    ),
    spec(
        "claude-opus-5-5-medium",
        "Claude Opus 5.5 (Medium)",
        AGY_EFFORTS,
        Some("medium"),
    ),
    spec(
        "claude-opus-5-5-low",
        "Claude Opus 5.5 (Low)",
        AGY_EFFORTS,
        Some("low"),
    ),
    spec(
        "claude-sonnet-5-5-high",
        "Claude Sonnet 5.5 (High)",
        AGY_EFFORTS,
        Some("high"),
    ),
    spec(
        "claude-sonnet-5-5-medium",
        "Claude Sonnet 5.5 (Medium)",
        AGY_EFFORTS,
        Some("medium"),
    ),
    spec(
        "claude-sonnet-5-5-low",
        "Claude Sonnet 5.5 (Low)",
        AGY_EFFORTS,
        Some("low"),
    ),
    spec(
        "gpt-oss-120b-medium",
        "GPT-OSS 120B (Medium)",
        AGY_EFFORTS,
        Some("medium"),
    ),
];

const CODEX_MODELS: &[&str] = &[
    "gpt-6-astra",
    "gpt-6.1-sol",
    "gpt-6.1-terra",
    "gpt-6-sol",
    "gpt-6-luna",
    "gpt-reserve",
    "gpt-5.6-sol",
    "gpt-5.6-terra",
    "gpt-5.6-luna",
];

const CODEX_SPECS: &[CliModelSpec] = &[
    spec(
        "gpt-6-astra",
        "GPT-6 Astra",
        CODEX_ULTRA_EFFORTS,
        Some("medium"),
    ),
    spec(
        "gpt-6.1-sol",
        "GPT-6.1 Sol",
        CODEX_ULTRA_EFFORTS,
        Some("low"),
    ),
    spec(
        "gpt-6.1-terra",
        "GPT-6.1 Terra",
        CODEX_ULTRA_EFFORTS,
        Some("low"),
    ),
    spec(
        "gpt-6-sol",
        "GPT-6 Sol",
        CODEX_ULTRA_EFFORTS,
        Some("medium"),
    ),
    spec(
        "gpt-6-luna",
        "GPT-6 Luna",
        CODEX_MAX_EFFORTS,
        Some("medium"),
    ),
    spec(
        "gpt-reserve",
        "GPT Reserve",
        CODEX_MAX_EFFORTS,
        Some("medium"),
    ),
    spec(
        "gpt-5.6-sol",
        "GPT-5.6 Sol",
        CODEX_ULTRA_EFFORTS,
        Some("low"),
    ),
    spec(
        "gpt-5.6-terra",
        "GPT-5.6 Terra",
        CODEX_ULTRA_EFFORTS,
        Some("medium"),
    ),
    spec(
        "gpt-5.6-luna",
        "GPT-5.6 Luna",
        CODEX_MAX_EFFORTS,
        Some("medium"),
    ),
];

const JUNIE_MODELS: &[&str] = &[
    "sonnet",
    "opus",
    "gpt",
    "gpt-codex",
    "gemini-pro",
    "gemini-flash",
    "grok",
    "claude-opus-5",
    "gpt-6-astra",
    "gpt-6.1-sol",
    "gemini-3.8-flash",
    "gemini-3.7-flash",
    "junie-lite",
];

const JUNIE_SPECS: &[CliModelSpec] = &[
    spec(
        "sonnet",
        "Claude Sonnet (Alias)",
        JUNIE_EFFORTS,
        Some("medium"),
    ),
    spec("opus", "Claude Opus (Alias)", JUNIE_EFFORTS, Some("high")),
    spec("gpt", "GPT (Alias)", JUNIE_EFFORTS, Some("medium")),
    spec(
        "gpt-codex",
        "GPT Codex (Alias)",
        JUNIE_EFFORTS,
        Some("medium"),
    ),
    spec(
        "gemini-pro",
        "Gemini Pro (Alias)",
        JUNIE_EFFORTS,
        Some("medium"),
    ),
    spec(
        "gemini-flash",
        "Gemini Flash (Alias)",
        JUNIE_EFFORTS,
        Some("medium"),
    ),
    spec("grok", "Grok (Alias)", JUNIE_EFFORTS, Some("medium")),
    spec(
        "claude-opus-5",
        "Claude Opus 5",
        JUNIE_EFFORTS,
        Some("high"),
    ),
    spec("gpt-6-astra", "GPT-6 Astra", JUNIE_EFFORTS, Some("high")),
    spec("gpt-6.1-sol", "GPT-6.1 Sol", JUNIE_EFFORTS, Some("high")),
    spec(
        "gemini-3.8-flash",
        "Gemini 3.8 Flash",
        JUNIE_EFFORTS,
        Some("medium"),
    ),
    spec(
        "gemini-3.7-flash",
        "Gemini 3.7 Flash",
        JUNIE_EFFORTS,
        Some("medium"),
    ),
    spec("junie-lite", "Junie Lite", JUNIE_EFFORTS, Some("low")),
];

/// What a profile may name for one tool.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CliChoices {
    /// The tool.
    pub tool: CliTool,
    /// Its own model ids (`claude:<id>`), the tool's default first where
    /// it has a fixed one. Empty where the ids are the player's own
    /// providers' (opencode's `provider/model`).
    pub models: &'static [&'static str],
    /// Detailed model specifications with human-readable labels and
    /// per-model supported reasoning/thinking efforts.
    pub model_specs: &'static [CliModelSpec],
    /// How a model id is spelled, for a tool whose list is empty or long.
    pub model_hint: &'static str,
    /// The model played when the profile names none: `None` for the tool's
    /// own default.
    pub default_model: Option<&'static str>,
    /// The effort levels its effort flag takes, least first; empty for a
    /// tool with none.
    pub efforts: &'static [&'static str],
    /// Whether the conversation goes on across turns (one process holds
    /// it, or the tool resumes it by its id); otherwise each question is a
    /// conversation of its own.
    pub keeps_conversation: bool,
}

/// What a profile may name for `tool`.
#[must_use]
pub const fn choices(tool: CliTool) -> CliChoices {
    match tool {
        // `claude --help` (2.1.299): "an alias for the latest model (e.g.
        // 'fable', 'opus', or 'sonnet') or a model's full name"; `--effort`
        // "(low, medium, high, xhigh, max)".
        CliTool::Claude => CliChoices {
            tool,
            models: CLAUDE_MODELS,
            model_specs: CLAUDE_SPECS,
            model_hint: "an alias (opus, sonnet, fable) or a model's full name",
            default_model: None,
            efforts: CLAUDE_EFFORTS,
            keeps_conversation: true,
        },
        // `agy --help` (1.4.0): `--effort` "(low|medium|high|xhigh|max)";
        // its models are listed by `agy models`.
        CliTool::Agy => CliChoices {
            tool,
            models: AGY_MODELS,
            model_specs: AGY_SPECS,
            model_hint: "a model as `agy models` lists it",
            default_model: Some(DEFAULT_AGY_MODEL),
            efforts: AGY_EFFORTS,
            keeps_conversation: true,
        },
        // Codex's models cache (0.163.0): Astra, Sol, Terra, Luna, Reserve;
        // `model_reasoning_effort` up to `ultra` (Luna and Reserve up to `max`).
        CliTool::Codex => CliChoices {
            tool,
            models: CODEX_MODELS,
            model_specs: CODEX_SPECS,
            model_hint: "a model id from Codex's models list",
            default_model: None,
            efforts: CODEX_ULTRA_EFFORTS,
            // `exec resume <id>`, its sessions under `CODEX_HOME`.
            keeps_conversation: true,
        },
        // opencode's models (1.19.0): ids are `provider/model`; `--variant` is
        // model-specific (`opencode models --verbose`).
        CliTool::Opencode => CliChoices {
            tool,
            models: &[],
            model_specs: &[],
            model_hint: "provider/model, as `opencode models` lists them",
            default_model: None,
            efforts: OPENCODE_EFFORTS,
            // `run --session <id>`, from a database in the seat's store.
            keeps_conversation: true,
        },
        // Junie's model-selection page & `~/.junie/settings.json` (26.9.22):
        // `--effort` "low, medium, high".
        CliTool::Junie => CliChoices {
            tool,
            models: JUNIE_MODELS,
            model_specs: JUNIE_SPECS,
            model_hint: "an alias (sonnet, opus, gpt, …) or a model id",
            default_model: None,
            efforts: JUNIE_EFFORTS,
            // One process holds its ACP session across turns.
            keeps_conversation: true,
        },
    }
}

/// Canonical rising order of all known reasoning/thinking effort names.
const CANONICAL_EFFORTS: &[&str] = &[
    "none", "minimal", "low", "medium", "high", "xhigh", "max", "ultra",
];

/// Maps a single effort name to its static slice if it is one of the known
/// effort levels.
#[must_use]
pub fn intern_effort(effort: &str) -> Option<&'static str> {
    CANONICAL_EFFORTS.iter().copied().find(|e| *e == effort)
}

/// Maps a list of effort names into a `'static` slice in canonical rising
/// order. Common combinations resolve without allocation; custom combinations
/// from `opencode models --verbose` are deduplicated in a bounded static table.
#[must_use]
pub fn intern_efforts(efforts: &[&str]) -> &'static [&'static str] {
    use std::sync::{Mutex, OnceLock};
    static DYNAMIC: OnceLock<Mutex<Vec<&'static [&'static str]>>> = OnceLock::new();
    const COMMON: &[&[&str]] = &[
        &[],
        CLAUDE_EFFORTS,
        CODEX_ULTRA_EFFORTS,
        JUNIE_EFFORTS,
        OPENCODE_EFFORTS,
        &["low", "medium", "high", "max"],
        &["minimal", "low", "medium", "high"],
        &["none", "low", "medium", "high"],
        &["none", "low", "medium", "high", "xhigh"],
        &["low", "high", "max"],
        &["low", "high"],
        &["medium", "high"],
        &["high", "max"],
    ];
    let mut normalized: Vec<&'static str> = CANONICAL_EFFORTS
        .iter()
        .copied()
        .filter(|c| efforts.contains(c))
        .collect();
    for raw in efforts {
        if !normalized.contains(raw)
            && super::effort_is_a_word(raw)
            && let Some(k) = intern_effort(raw)
        {
            normalized.push(k);
        }
    }
    for &known in COMMON {
        if known == normalized.as_slice() {
            return known;
        }
    }
    let mutex = DYNAMIC.get_or_init(|| Mutex::new(Vec::new()));
    let mut guard = mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(&existing) = guard.iter().find(|&&s| s == normalized.as_slice()) {
        return existing;
    }
    if guard.len() >= 64 {
        return CLAUDE_EFFORTS;
    }
    let leaked: &'static [&'static str] = Box::leak(normalized.into_boxed_slice());
    guard.push(leaked);
    leaked
}

/// Encodes a live-discovered CLI model into a single `listed` entry string
/// (`<tool>:<id>\t<label>\t<efforts_csv>\t<default_effort>`) that
/// [`super::models::offered`] and [`super::models::resolve`] decode.
#[must_use]
pub fn encode_cli_listed(
    tool: CliTool,
    own_id: &str,
    label: &str,
    efforts: &[&str],
    default_effort: Option<&str>,
) -> Option<String> {
    let own_id = own_id.trim();
    if super::model_fault(own_id).is_some() {
        return None;
    }
    let clean_label = label.replace(['\t', '\n', '\r'], " ");
    let clean_label = clean_label.trim();
    let efforts_csv = efforts.join(",");
    let def = default_effort.unwrap_or_default().trim();
    Some(format!(
        "{}:{own_id}\t{clean_label}\t{efforts_csv}\t{def}",
        tool.name()
    ))
}

/// Parses `agy models` stdout into encoded `listed` entries for [`CliTool::Agy`].
#[must_use]
pub fn parse_agy_models(stdout: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.ends_with(':') {
            continue;
        }
        let mut parts = trimmed.splitn(2, char::is_whitespace);
        let Some(id) = parts.next().map(str::trim).filter(|s| !s.is_empty()) else {
            continue;
        };
        let raw_label = parts.next().map_or(id, str::trim);
        // Fix upstream `agy models` typo where `claude-sonnet-5-5-low` is
        // printed as `Claude Opus 5.5 (Low)`.
        let label = if id.starts_with("claude-sonnet-") && raw_label.contains("Opus") {
            raw_label.replace("Opus", "Sonnet")
        } else {
            raw_label.to_string()
        };
        let default_effort = if id.ends_with("-high") {
            Some("high")
        } else if id.ends_with("-medium") {
            Some("medium")
        } else if id.ends_with("-low") {
            Some("low")
        } else {
            None
        };
        if let Some(encoded) =
            encode_cli_listed(CliTool::Agy, id, &label, AGY_EFFORTS, default_effort)
            && !out.contains(&encoded)
        {
            out.push(encoded);
        }
    }
    out
}

/// Parses `~/.codex/models_cache.json` into encoded `listed` entries for [`CliTool::Codex`].
#[must_use]
pub fn parse_codex_cache(json: &str) -> Vec<String> {
    let Ok(val) = serde_json::from_str::<serde_json::Value>(json) else {
        return Vec::new();
    };
    let Some(models) = val.get("models").and_then(serde_json::Value::as_array) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in models {
        let Some(slug) = item.get("slug").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let raw_name = item
            .get("display_name")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(slug);
        let label = prettify_codex_name(slug, raw_name);
        let default_effort = item
            .get("default_reasoning_level")
            .and_then(serde_json::Value::as_str);
        let efforts: Vec<&str> = item
            .get("supported_reasoning_levels")
            .and_then(serde_json::Value::as_array)
            .map_or_else(
                || CODEX_ULTRA_EFFORTS.to_vec(),
                |arr| {
                    arr.iter()
                        .filter_map(|e| e.get("effort").and_then(serde_json::Value::as_str))
                        .filter(|e| super::effort_is_a_word(e))
                        .collect()
                },
            );
        if let Some(encoded) =
            encode_cli_listed(CliTool::Codex, slug, &label, &efforts, default_effort)
            && !out.contains(&encoded)
        {
            out.push(encoded);
        }
    }
    out
}

fn prettify_codex_name(slug: &str, display_name: &str) -> String {
    if let Some(found) = CODEX_SPECS.iter().find(|s| s.id == slug) {
        return found.label.to_string();
    }
    // Turn `GPT-6-Astra` or `gpt-6-astra` into `GPT-6 Astra`.
    let base = if display_name.eq_ignore_ascii_case(slug) {
        slug
    } else {
        display_name
    };
    if let Some(rest) = base
        .strip_prefix("gpt-")
        .or_else(|| base.strip_prefix("GPT-"))
    {
        let parts: Vec<String> = rest
            .split('-')
            .map(|p| {
                let mut chars = p.chars();
                match chars.next() {
                    Some(first) if first.is_ascii_alphabetic() => {
                        format!("{}{}", first.to_ascii_uppercase(), chars.as_str())
                    }
                    _ => p.to_string(),
                }
            })
            .collect();
        return format!("GPT-{}", parts.join(" "));
    }
    display_name.to_string()
}

/// Parses `opencode models --verbose` stdout into encoded `listed` entries for
/// [`CliTool::Opencode`]. Filters out non-text models (image, audio, TTS,
/// embedding) and preserves each model's exact `variants` effort list.
#[must_use]
pub fn parse_opencode_verbose(stdout: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut lines = stdout.lines().peekable();
    while let Some(line) = lines.next() {
        let header = line.trim();
        if header.is_empty() || !header.contains('/') || header.starts_with('{') {
            continue;
        }
        if super::model_fault(header).is_some() {
            continue;
        }
        // Collect the JSON object immediately following the `provider/model` line.
        let mut json_buf = String::new();
        let mut depth = 0i32;
        let mut started = false;
        while let Some(&next_line) = lines.peek() {
            let t = next_line.trim();
            if !started && !t.starts_with('{') {
                break;
            }
            let consumed = lines.next().unwrap_or_default();
            started = true;
            for ch in consumed.chars() {
                if ch == '{' {
                    depth += 1;
                } else if ch == '}' {
                    depth -= 1;
                }
            }
            json_buf.push_str(consumed);
            json_buf.push('\n');
            if started && depth <= 0 {
                break;
            }
        }
        if !started {
            // Plain `opencode models` output without `--verbose`.
            if !is_non_text_model_id(header)
                && let Some(encoded) =
                    encode_cli_listed(CliTool::Opencode, header, header, OPENCODE_EFFORTS, None)
            {
                out.push(encoded);
            }
            continue;
        }
        let Ok(val) = serde_json::from_str::<serde_json::Value>(&json_buf) else {
            continue;
        };
        if val
            .pointer("/capabilities/output/text")
            .and_then(serde_json::Value::as_bool)
            == Some(false)
        {
            continue;
        }
        if is_non_text_model_id(header) {
            continue;
        }
        let name = val
            .get("name")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(header);
        let provider = val
            .get("providerID")
            .and_then(serde_json::Value::as_str)
            .or_else(|| header.split_once('/').map(|(p, _)| p))
            .unwrap_or_default();
        let label = if provider.is_empty() || name.contains(provider) {
            name.to_string()
        } else {
            format!("{name} ({provider})")
        };
        let variant_keys: Vec<&str> = val
            .get("variants")
            .and_then(serde_json::Value::as_object)
            .map(|obj| {
                CANONICAL_EFFORTS
                    .iter()
                    .copied()
                    .filter(|k| obj.contains_key(*k))
                    .collect()
            })
            .unwrap_or_default();
        if let Some(encoded) =
            encode_cli_listed(CliTool::Opencode, header, &label, &variant_keys, None)
            && !out.contains(&encoded)
        {
            out.push(encoded);
        }
    }
    out
}

fn is_non_text_model_id(id: &str) -> bool {
    let lower = id.to_ascii_lowercase();
    lower.contains("embedding")
        || lower.contains("tts")
        || lower.contains("whisper")
        || lower.contains("gpt-image")
        || lower.contains("dall-e")
        || lower.contains("sora")
        || lower.contains("moderation")
        || lower.contains("realtime")
        || lower.contains("transcribe")
        || lower.contains("audio")
}

/// Parses `~/.junie/settings.json` into encoded `listed` entries for [`CliTool::Junie`].
#[must_use]
pub fn parse_junie_settings(json: &str) -> Vec<String> {
    let Ok(val) = serde_json::from_str::<serde_json::Value>(json) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if let Some(map) = val
        .get("effortPerModel")
        .and_then(serde_json::Value::as_object)
    {
        for (model_id, effort_val) in map {
            let default_effort = effort_val
                .as_str()
                .map(str::to_ascii_lowercase)
                .filter(|e| JUNIE_EFFORTS.contains(&e.as_str()));
            let label = prettify_junie_name(model_id);
            if let Some(encoded) = encode_cli_listed(
                CliTool::Junie,
                model_id,
                &label,
                JUNIE_EFFORTS,
                default_effort.as_deref(),
            ) && !out.contains(&encoded)
            {
                out.push(encoded);
            }
        }
    }
    if let Some(launch) = val
        .get("modelForLaunch")
        .and_then(serde_json::Value::as_str)
    {
        let label = prettify_junie_name(launch);
        if let Some(encoded) =
            encode_cli_listed(CliTool::Junie, launch, &label, JUNIE_EFFORTS, None)
            && !out
                .iter()
                .any(|e| e.starts_with(&format!("junie:{launch}\t")))
        {
            out.push(encoded);
        }
    }
    out
}

fn prettify_junie_name(model_id: &str) -> String {
    if let Some(found) = JUNIE_SPECS.iter().find(|s| s.id == model_id) {
        return found.label.to_string();
    }
    if let Some(rest) = model_id.strip_prefix("local:lm-studio:") {
        return format!("{rest} (LM Studio)");
    }
    model_id.to_string()
}

/// Parses `~/.claude/settings.json` into encoded `listed` entries for [`CliTool::Claude`].
#[must_use]
pub fn parse_claude_settings(json: &str) -> Vec<String> {
    let Ok(val) = serde_json::from_str::<serde_json::Value>(json) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    if let Some(model_id) = val.get("model").and_then(serde_json::Value::as_str) {
        let label = CLAUDE_SPECS
            .iter()
            .find(|s| s.id == model_id)
            .map_or(model_id, |s| s.label);
        if let Some(encoded) =
            encode_cli_listed(CliTool::Claude, model_id, label, CLAUDE_EFFORTS, None)
        {
            out.push(encoded);
        }
    }
    if let Some(map) = val
        .get("modelSettings")
        .and_then(serde_json::Value::as_object)
    {
        for model_id in map.keys() {
            let label = CLAUDE_SPECS
                .iter()
                .find(|s| s.id == model_id)
                .map_or(model_id.as_str(), |s| s.label);
            if let Some(encoded) =
                encode_cli_listed(CliTool::Claude, model_id, label, CLAUDE_EFFORTS, None)
                && !out.contains(&encoded)
            {
                out.push(encoded);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llmseat::{cli_model, effort_is_a_word, model_fault};

    /// Every tool has its choices, and each one listed is one a profile
    /// may name: a model the file takes beside its tool, an effort the
    /// file takes as a word.
    #[test]
    fn every_listed_choice_is_one_the_file_takes() {
        for tool in CliTool::ALL {
            let choices = choices(tool);
            assert_eq!(choices.tool, tool);
            assert!(!choices.model_hint.is_empty());
            assert_eq!(choices.models.len(), choices.model_specs.len(), "{tool:?}");
            for (model, spec) in choices.models.iter().zip(choices.model_specs.iter()) {
                assert_eq!(*model, spec.id);
                assert!(!spec.label.is_empty());
                assert_eq!(model_fault(model), None, "{model}");
                let named = format!("{}:{model}", tool.name());
                assert_eq!(cli_model(&named), Ok((tool, Some(*model))));
                for effort in spec.efforts {
                    assert!(choices.efforts.contains(effort), "{model}: {effort}");
                }
                if let Some(def) = spec.default_effort {
                    assert!(spec.efforts.contains(&def), "{model}: {def}");
                }
            }
            if let Some(default) = choices.default_model {
                assert_eq!(choices.models.first(), Some(&default), "{tool:?}");
            }
            for effort in choices.efforts {
                assert!(effort_is_a_word(effort), "{effort}");
            }
        }
        assert!(choices(CliTool::Claude).keeps_conversation);
        for tool in CliTool::ALL {
            assert!(choices(tool).keeps_conversation, "{tool:?}");
        }
    }

    #[test]
    fn cli_discovery_parsers_extract_models_labels_and_efforts() {
        let agy_out = "\
Available models:
  gemini-4-argon-high           Gemini 4 Argon (High)
  claude-sonnet-5-5-low         Claude Opus 5.5 (Low)
";
        let agy = parse_agy_models(agy_out);
        assert_eq!(agy.len(), 2);
        assert!(agy[0].starts_with("agy:gemini-4-argon-high\tGemini 4 Argon (High)\t"));
        assert!(
            agy[1].starts_with("agy:claude-sonnet-5-5-low\tClaude Sonnet 5.5 (Low)\t"),
            "typo in agy output is normalized: {}",
            agy[1]
        );

        let codex_json = r#"{
          "models": [
            {
              "slug": "gpt-6-astra",
              "display_name": "GPT-6-Astra",
              "default_reasoning_level": "medium",
              "supported_reasoning_levels": [
                {"effort": "low"},
                {"effort": "medium"},
                {"effort": "high"},
                {"effort": "xhigh"},
                {"effort": "max"},
                {"effort": "ultra"}
              ]
            },
            {
              "slug": "gpt-6-luna",
              "display_name": "gpt-6-luna",
              "default_reasoning_level": "medium",
              "supported_reasoning_levels": [
                {"effort": "low"},
                {"effort": "medium"},
                {"effort": "high"},
                {"effort": "xhigh"},
                {"effort": "max"}
              ]
            }
          ]
        }"#;
        let codex = parse_codex_cache(codex_json);
        assert_eq!(
            codex,
            [
                "codex:gpt-6-astra\tGPT-6 Astra\tlow,medium,high,xhigh,max,ultra\tmedium",
                "codex:gpt-6-luna\tGPT-6 Luna\tlow,medium,high,xhigh,max\tmedium",
            ]
        );

        let opencode_out = "\
openai/gpt-6-astra
{
  \"id\": \"gpt-6-astra\",
  \"providerID\": \"openai\",
  \"name\": \"GPT-6 Astra\",
  \"capabilities\": { \"output\": { \"text\": true } },
  \"variants\": { \"low\": {}, \"medium\": {}, \"high\": {}, \"xhigh\": {}, \"max\": {} }
}
openai/gpt-image-1.5
{
  \"id\": \"gpt-image-1.5\",
  \"providerID\": \"openai\",
  \"name\": \"GPT Image 1.5\",
  \"capabilities\": { \"output\": { \"text\": false } },
  \"variants\": {}
}
";
        let opencode = parse_opencode_verbose(opencode_out);
        assert_eq!(
            opencode,
            ["opencode:openai/gpt-6-astra\tGPT-6 Astra (openai)\tlow,medium,high,xhigh,max\t"]
        );

        let junie_json = r#"{
          "modelForLaunch": "claude-opus-5",
          "effortPerModel": {
            "claude-opus-5": "HIGH",
            "local:lm-studio:google/gemma-4-26b-a4b": "MEDIUM"
          }
        }"#;
        let junie = parse_junie_settings(junie_json);
        assert_eq!(junie.len(), 2);
        assert!(
            junie.contains(&"junie:claude-opus-5\tClaude Opus 5\tlow,medium,high\thigh".into())
        );
        assert!(junie.contains(
            &"junie:local:lm-studio:google/gemma-4-26b-a4b\tgoogle/gemma-4-26b-a4b (LM Studio)\tlow,medium,high\tmedium"
                .into()
        ));
    }
}
