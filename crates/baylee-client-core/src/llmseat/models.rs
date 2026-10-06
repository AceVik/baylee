//! Which models a profile's provider offers, what each is called, and which
//! efforts it takes (`docs/llm-seat.md` §"Models and efforts").
//!
//! A model is offered under a label a player reads (`Claude Opus 5.5`) and
//! kept by its exact id (`claude-opus-5-5`), which is what the settings file
//! and the bridge's `--mind` carry. Three sources, none of which sends a key
//! anywhere:
//!
//! - **Known**: this build's tables, for hosted providers whose listing
//!   would need the key ([`HOSTED`], by host, whichever protocol reaches
//!   it), and for the agent CLIs this build speaks, whose models are their
//!   own aliases ([`cli_models`]). A snapshot: the provider's
//!   documentation is the authority, and a model missing here is still
//!   played when its id is typed into the settings panel.
//! - **Listed**: an OpenAI-compatible server on this machine (LM Studio, a
//!   llama.cpp server) answers `GET {base}/models` with what it has loaded,
//!   asked without a key ([`listing_url`], [`parse_listing`]). Only a
//!   loopback address is asked: anything else would need the key.
//! - **Typed**: any other id, labelled with itself.
//!
//! Efforts are offered only where this build knows which ones the model
//! takes ([`Resolved::efforts`]); a model it does not know plays at its own
//! default, and the chair says so rather than offering a word the model
//! might refuse. Pure data, for every target.

use super::{CliTool, Provider, is_loopback, model_fault};

/// The efforts Claude's current models take: Fable 5 and 5.1, Opus 5.5, 5,
/// 4.8 and 4.7, Sonnet 5.5 and 5 (Anthropic's model and Claude Code's
/// effort tables, read 2026-10-06).
pub const CLAUDE_EFFORTS: &[&str] = &["low", "medium", "high", "xhigh", "max"];

/// The efforts Opus 4.6 and Sonnet 4.6 take: no `xhigh`, which came with
/// Opus 4.7.
pub const CLAUDE_46_EFFORTS: &[&str] = &["low", "medium", "high", "max"];

/// No effort this build can name: the model's own default plays.
const NONE: &[&str] = &[];

/// A model this build knows, under its provider.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KnownModel {
    /// Its id exactly as a profile's `model` writes it: for a CLI the tool
    /// first (`claude:opus`).
    pub id: &'static str,
    /// What a player reads.
    pub label: &'static str,
    /// The efforts it takes, in rising order; empty where none is known.
    pub efforts: &'static [&'static str],
    /// The effort it plays at when none is named, where that is known.
    pub default_effort: Option<&'static str>,
}

const fn model(
    id: &'static str,
    label: &'static str,
    efforts: &'static [&'static str],
    default_effort: Option<&'static str>,
) -> KnownModel {
    KnownModel {
        id,
        label,
        efforts,
        default_effort,
    }
}

/// Anthropic's models that the bridge's request suits (adaptive thinking
/// and an effort; Haiku 4.5 takes neither, so it is not offered). The
/// bridge names `medium` when a profile names no effort
/// ([`Provider::default_effort`]).
const ANTHROPIC: &[KnownModel] = &[
    model(
        "claude-opus-5-5",
        "Claude Opus 5.5",
        CLAUDE_EFFORTS,
        Some("medium"),
    ),
    model(
        "claude-sonnet-5-5",
        "Claude Sonnet 5.5",
        CLAUDE_EFFORTS,
        Some("medium"),
    ),
    model(
        "claude-fable-5-1",
        "Claude Fable 5.1",
        CLAUDE_EFFORTS,
        Some("medium"),
    ),
    model(
        "claude-opus-5",
        "Claude Opus 5",
        CLAUDE_EFFORTS,
        Some("medium"),
    ),
    model(
        "claude-sonnet-5",
        "Claude Sonnet 5",
        CLAUDE_EFFORTS,
        Some("medium"),
    ),
    model(
        "claude-opus-4-8",
        "Claude Opus 4.8",
        CLAUDE_EFFORTS,
        Some("medium"),
    ),
    model(
        "claude-opus-4-7",
        "Claude Opus 4.7",
        CLAUDE_EFFORTS,
        Some("medium"),
    ),
    model(
        "claude-opus-4-6",
        "Claude Opus 4.6",
        CLAUDE_46_EFFORTS,
        Some("medium"),
    ),
    model(
        "claude-sonnet-4-6",
        "Claude Sonnet 4.6",
        CLAUDE_46_EFFORTS,
        Some("medium"),
    ),
];

/// `DeepSeek`'s models at `api.deepseek.com`, as its `/models` listed them
/// on 2026-09-16. Which `reasoning_effort` its OpenAI-compatible route
/// takes is not known here, so none is offered.
const DEEPSEEK: &[KnownModel] = &[
    model("deepseek-flash", "DeepSeek V4.1 Flash", NONE, None),
    model("deepseek-v4-pro", "DeepSeek V4 Pro", NONE, None),
];

/// The hosted APIs this build knows the models of, by host: whichever wire
/// protocol a profile speaks there (`DeepSeek` answers both, at
/// `/v1` and at `/anthropic`), the host's models are the ones it serves.
pub const HOSTED: &[(&str, &[KnownModel])] = &[
    ("api.anthropic.com", ANTHROPIC),
    ("api.deepseek.com", DEEPSEEK),
];

/// What a player reads for an agent CLI.
#[must_use]
pub const fn tool_label(tool: CliTool) -> &'static str {
    match tool {
        CliTool::Claude => "Claude Code",
        CliTool::Agy => "agy",
        CliTool::Codex => "Codex",
        CliTool::Opencode => "opencode",
        CliTool::Junie => "Junie",
    }
}

/// The models a chair offers for an agent CLI, in order: each one the
/// tool's table names ([`super::clis::choices`], the one place a CLI's
/// models and efforts are read from), then the bare tool, which plays the
/// tool's own default. Each takes the efforts the tool's effort flag takes.
#[must_use]
pub fn cli_models(tool: CliTool) -> Vec<Resolved> {
    let choices = super::clis::choices(tool);
    let label = tool_label(tool);
    let mut out: Vec<Resolved> = choices
        .models
        .iter()
        .map(|model| Resolved {
            id: format!("{}:{model}", tool.name()),
            label: format!("{model} ({label})"),
            efforts: choices.efforts,
            default_effort: None,
            source: Source::Known,
        })
        .collect();
    out.push(Resolved {
        id: tool.name().to_string(),
        label: choices.default_model.map_or_else(
            || format!("{label}'s default model"),
            |model| format!("{model} ({label}'s default)"),
        ),
        efforts: choices.efforts,
        default_effort: None,
        source: Source::Known,
    });
    out
}

/// Whom a profile plays through: what [`known`] and [`resolve`] look up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Endpoint<'a> {
    /// The provider.
    pub provider: Provider,
    /// Its address, for an API: the profile's `base_url`, else the
    /// provider's variable, else its default ([`Provider::default_base`]).
    pub base: Option<&'a str>,
    /// For a CLI, the tool its model names: a profile keeps to its tool,
    /// whose program its `command` may name.
    pub tool: Option<CliTool>,
}

impl<'a> Endpoint<'a> {
    /// The endpoint of a profile of `provider` whose address is `base`, or
    /// the provider's default address where it names none.
    #[must_use]
    pub fn new(provider: Provider, base: Option<&'a str>) -> Self {
        Self {
            provider,
            base: base.or_else(|| provider.default_base()),
            tool: None,
        }
    }

    /// A CLI's endpoint, for the tool `model` names (`claude:opus`).
    #[must_use]
    pub fn cli(model: &str) -> Self {
        Self {
            provider: Provider::Cli,
            base: None,
            tool: model.split(':').next().and_then(CliTool::named),
        }
    }

    /// Whether this is a server on this machine, which may be asked what it
    /// has.
    #[must_use]
    pub fn on_this_machine(&self) -> bool {
        self.provider == Provider::OpenAi && self.base.is_some_and(is_loopback)
    }
}

/// The host of `base` (`https://api.deepseek.com/v1` → `api.deepseek.com`).
fn host(base: &str) -> &str {
    let rest = base.split_once("://").map_or(base, |(_, rest)| rest);
    let host = rest.split(['/', '?']).next().unwrap_or_default();
    host.rsplit_once(':')
        .filter(|(h, port)| !h.is_empty() && port.chars().all(|c| c.is_ascii_digit()))
        .map_or(host, |(h, _)| h)
}

/// The models this build knows of an API at `endpoint`, in the order
/// offered: by its host ([`HOSTED`]), and Anthropic's at any other address
/// that speaks its protocol (a proxy serves the same ids). A CLI's are
/// [`cli_models`].
#[must_use]
pub fn known(endpoint: Endpoint<'_>) -> &'static [KnownModel] {
    if endpoint.provider == Provider::Cli {
        return NONE_MODELS;
    }
    let at = endpoint.base.map(host);
    HOSTED
        .iter()
        .find(|(name, _)| at == Some(*name))
        .map(|(_, models)| *models)
        .or((endpoint.provider == Provider::Anthropic).then_some(ANTHROPIC))
        .unwrap_or(NONE_MODELS)
}

/// No model.
const NONE_MODELS: &[KnownModel] = &[];

/// The known model `id` at `endpoint`; for a CLI, among the models of the
/// tool its id names, and a Claude Code full model id read as Anthropic's
/// (`claude:claude-opus-5-5`).
fn find(endpoint: Endpoint<'_>, id: &str) -> Option<Resolved> {
    if endpoint.provider == Provider::Cli {
        let tool = id.split(':').next().and_then(CliTool::named)?;
        if let Some(found) = cli_models(tool).into_iter().find(|m| m.id == id) {
            return Some(found);
        }
        let own = id.split_once(':').map(|(_, own)| own)?;
        return ANTHROPIC
            .iter()
            .find(|m| m.id == own)
            .filter(|_| tool == CliTool::Claude)
            .map(|m| Resolved::from(*m));
    }
    known(endpoint)
        .iter()
        .find(|m| m.id == id)
        .map(|m| Resolved::from(*m))
}

/// Where a resolved model's label comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// This build's tables.
    Known,
    /// The server on this machine said it has it.
    Listed,
    /// Neither: an id the player wrote.
    Typed,
}

/// A model as a chair offers it: its exact id, what a player reads, and the
/// efforts that can be named for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Resolved {
    /// The id, exactly as the profile and `--mind` carry it.
    pub id: String,
    /// What a player reads: the known name, else the id itself.
    pub label: String,
    /// The efforts it takes, in rising order; empty where this build does
    /// not know any, and then none is offered.
    pub efforts: &'static [&'static str],
    /// The effort it plays at when none is named, where known.
    pub default_effort: Option<&'static str>,
    /// Where the label came from.
    pub source: Source,
}

impl Resolved {
    /// Whether `effort` may be named for it.
    #[must_use]
    pub fn takes(&self, effort: &str) -> bool {
        self.efforts.contains(&effort)
    }

    /// The label and, where they differ, the exact id beside it:
    /// `Claude Opus 5.5 · claude-opus-5-5`.
    #[must_use]
    pub fn caption(&self) -> String {
        if self.label == self.id {
            self.id.clone()
        } else {
            format!("{} · {}", self.label, self.id)
        }
    }
}

impl From<KnownModel> for Resolved {
    fn from(known: KnownModel) -> Self {
        Self {
            id: known.id.to_string(),
            label: known.label.to_string(),
            efforts: known.efforts,
            default_effort: known.default_effort,
            source: Source::Known,
        }
    }
}

/// `model` at `endpoint`, labelled: known, else one the server on this
/// machine `listed`, else as typed.
#[must_use]
pub fn resolve(endpoint: Endpoint<'_>, model: &str, listed: &[String]) -> Resolved {
    if let Some(known) = find(endpoint, model) {
        return known;
    }
    let source = if listed.iter().any(|id| id == model) {
        Source::Listed
    } else {
        Source::Typed
    };
    Resolved {
        id: model.to_string(),
        label: model.to_string(),
        efforts: NONE,
        default_effort: None,
        source,
    }
}

/// Every model a chair offers at `endpoint`: this build's for it, then
/// what the server on this machine `listed` that is not among them.
#[must_use]
pub fn offered(endpoint: Endpoint<'_>, listed: &[String]) -> Vec<Resolved> {
    let mut out: Vec<Resolved> = if endpoint.provider == Provider::Cli {
        endpoint.tool.map(cli_models).unwrap_or_default()
    } else {
        known(endpoint).iter().copied().map(Into::into).collect()
    };
    if endpoint.on_this_machine() {
        for id in listed {
            if !out.iter().any(|m| m.id == *id) {
                out.push(resolve(endpoint, id, listed));
            }
        }
    }
    out
}

/// Where to ask the server at `endpoint` what it has, for a server on this
/// machine only: `{base}/models`. `None` elsewhere, where the listing would
/// need the key.
#[must_use]
pub fn listing_url(endpoint: Endpoint<'_>) -> Option<String> {
    endpoint
        .on_this_machine()
        .then_some(endpoint.base)
        .flatten()
        .map(|base| format!("{}/models", base.trim_end_matches('/')))
}

/// The most models one listing offers.
pub const MOST_LISTED: usize = 64;

/// The model ids in an OpenAI-compatible `/models` answer
/// (`{"data": [{"id": …}, …]}`), sorted, each once, and only those that are
/// model ids ([`model_fault`]), at most [`MOST_LISTED`].
///
/// # Errors
/// A sentence, for an answer that is not such a list.
pub fn parse_listing(body: &str) -> Result<Vec<String>, String> {
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|e| format!("the server's model list is not JSON: {e}"))?;
    let data = value
        .get("data")
        .and_then(serde_json::Value::as_array)
        .ok_or("the server's model list has no data")?;
    let mut ids: Vec<String> = data
        .iter()
        .filter_map(|entry| entry.get("id").and_then(serde_json::Value::as_str))
        .filter(|id| model_fault(id).is_none())
        .map(str::to_string)
        .collect();
    ids.sort();
    ids.dedup();
    ids.truncate(MOST_LISTED);
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anthropic() -> Endpoint<'static> {
        Endpoint::new(Provider::Anthropic, None)
    }

    #[test]
    fn a_known_model_is_labelled_and_keeps_its_exact_id() {
        let opus = resolve(anthropic(), "claude-opus-5-5", &[]);
        assert_eq!(opus.label, "Claude Opus 5.5");
        assert_eq!(opus.id, "claude-opus-5-5");
        assert_eq!(opus.caption(), "Claude Opus 5.5 · claude-opus-5-5");
        assert_eq!(opus.source, Source::Known);
        // Every id is one the file and the bridge take, and once only.
        for table in HOSTED.iter().map(|(_, table)| *table) {
            for (at, m) in table.iter().enumerate() {
                let own = m.id.split_once(':').map_or(m.id, |(_, own)| own);
                assert!(model_fault(own).is_none(), "{}", m.id);
                assert!(!table[..at].iter().any(|o| o.id == m.id), "{} twice", m.id);
                assert!(!m.label.is_empty());
                for effort in m.efforts {
                    assert!(super::super::effort_is_a_word(effort), "{effort}");
                }
                assert!(
                    m.default_effort.is_none_or(|d| m.efforts.contains(&d)),
                    "{}'s default is one of its efforts",
                    m.id
                );
            }
        }
        // A CLI model's tool is one this build speaks, and its own; each
        // id once, each effort the tool's own (`clis::choices`).
        for tool in CliTool::ALL {
            let models = cli_models(tool);
            for (at, m) in models.iter().enumerate() {
                let named = super::super::cli_model(&m.id).map(|(t, _)| t);
                assert_eq!(named, Ok(tool), "{}", m.id);
                assert!(!models[..at].iter().any(|o| o.id == m.id), "{} twice", m.id);
                assert_eq!(m.efforts, super::super::clis::choices(tool).efforts);
                assert_eq!(m.source, Source::Known);
            }
            assert_eq!(
                models.last().map(|m| m.id.as_str()),
                Some(tool.name()),
                "the bare tool, its own default, comes last"
            );
        }
    }

    /// The models are the host's whichever protocol a profile speaks there:
    /// `DeepSeek`'s Anthropic-compatible address offers `DeepSeek`'s, not
    /// Claude; Anthropic's protocol anywhere else (a proxy) offers Claude;
    /// an OpenAI-compatible host this build does not know offers nothing.
    #[test]
    fn a_hosts_models_are_offered_whichever_protocol_reaches_it() {
        for (provider, base) in [
            (Provider::OpenAi, "https://api.deepseek.com/v1"),
            (Provider::Anthropic, "https://api.deepseek.com/anthropic"),
        ] {
            let ids: Vec<String> = offered(Endpoint::new(provider, Some(base)), &[])
                .into_iter()
                .map(|m| m.id)
                .collect();
            assert_eq!(ids, ["deepseek-flash", "deepseek-v4-pro"], "{base}");
        }
        let proxy = Endpoint::new(Provider::Anthropic, Some("https://llm.example.org"));
        assert_eq!(known(proxy), ANTHROPIC);
        let elsewhere = Endpoint::new(Provider::OpenAi, Some("https://llm.example.org/v1"));
        assert!(known(elsewhere).is_empty());
        assert!(known(Endpoint::new(Provider::OpenAi, None)).is_empty());
    }

    /// Only the efforts a model takes are offered: no `xhigh` on the 4.6
    /// generation, none on Haiku, none on a model this build does not know.
    #[test]
    fn only_the_efforts_a_model_takes_are_offered() {
        let sonnet46 = resolve(anthropic(), "claude-sonnet-4-6", &[]);
        assert!(sonnet46.takes("max") && !sonnet46.takes("xhigh"));
        let opus = resolve(anthropic(), "claude-opus-5-5", &[]);
        assert!(opus.takes("xhigh"));
        assert_eq!(opus.efforts, CLAUDE_EFFORTS);
        let cli = Endpoint::cli("claude:opus");
        assert!(resolve(cli, "claude:haiku", &[]).efforts.is_empty());
        assert!(resolve(cli, "claude:opus", &[]).takes("max"));
        // A Claude Code full id reads as Anthropic's model.
        let full = resolve(cli, "claude:claude-sonnet-4-6", &[]);
        assert_eq!(full.label, "Claude Sonnet 4.6");
        assert!(!full.takes("xhigh"));
        // agy's bare tool is its default model; its effort flag's levels
        // stand beside each model.
        let agy = Endpoint::cli("agy");
        assert_eq!(resolve(agy, "agy", &[]).source, Source::Known);
        assert!(resolve(agy, "agy:gemini-3.8-flash-high", &[]).takes("high"));
        // A CLI profile is offered its own tool's models only.
        assert!(offered(cli, &[]).iter().all(|m| m.id.starts_with("claude")));
        assert!(offered(agy, &[]).iter().all(|m| m.id.starts_with("agy")));
        // The newer dialects come from the same table: Codex its models and
        // `ultra`, opencode only its own default (its ids are the player's
        // providers'), each at the tool's efforts.
        let codex = offered(Endpoint::cli("codex"), &[]);
        assert!(codex.iter().any(|m| m.id == "codex:gpt-6-astra"));
        assert!(codex.iter().all(|m| m.takes("ultra")));
        let opencode = offered(Endpoint::cli("opencode"), &[]);
        assert_eq!(
            opencode.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
            ["opencode"]
        );
        assert!(resolve(Endpoint::cli("junie"), "junie:opus", &[]).takes("high"));
        // Unknown: labelled with itself, no effort offered.
        let typed = resolve(anthropic(), "claude-next-9", &[]);
        assert_eq!(typed.label, "claude-next-9");
        assert_eq!(typed.source, Source::Typed);
        assert!(typed.efforts.is_empty());
        // Haiku 4.5 is not offered to the API path at all.
        assert!(
            !offered(anthropic(), &[])
                .iter()
                .any(|m| m.id.contains("haiku"))
        );
    }

    /// A hosted OpenAI-compatible endpoint is never asked (its listing needs
    /// the key); `DeepSeek`'s is known; a server on this machine is listed.
    #[test]
    fn a_listing_is_asked_only_of_a_server_on_this_machine() {
        let deepseek = Endpoint::new(Provider::OpenAi, Some("https://api.deepseek.com/v1"));
        assert_eq!(listing_url(deepseek), None);
        let ids: Vec<String> = offered(deepseek, &[]).into_iter().map(|m| m.id).collect();
        assert_eq!(ids, ["deepseek-flash", "deepseek-v4-pro"]);
        let openai = Endpoint::new(Provider::OpenAi, None);
        assert_eq!(openai.base, Some("https://api.openai.com/v1"));
        assert_eq!(listing_url(openai), None);
        assert!(offered(openai, &[]).is_empty());
        let lm = Endpoint::new(Provider::OpenAi, Some("http://127.0.0.1:1234/v1/"));
        assert_eq!(
            listing_url(lm).as_deref(),
            Some("http://127.0.0.1:1234/v1/models")
        );
        let listed = vec!["qwen3-8b".to_string(), "gemma-4-12b".to_string()];
        let offered_here = offered(lm, &listed);
        assert_eq!(offered_here.len(), 2);
        assert!(offered_here.iter().all(|m| m.source == Source::Listed));
        // A listing named for a hosted endpoint offers none of it.
        assert!(
            offered(deepseek, &listed)
                .iter()
                .all(|m| m.source == Source::Known)
        );
        // Anthropic and a CLI are never listed.
        assert_eq!(listing_url(anthropic()), None);
        assert_eq!(listing_url(Endpoint::cli("claude")), None);
    }

    #[test]
    fn a_listing_reads_the_ids_and_nothing_else() {
        let body = r#"{"object":"list","data":[
            {"id":"qwen3-8b","object":"model"},
            {"id":"gemma-4-12b"},
            {"id":"qwen3-8b"},
            {"id":"two words"},
            {"name":"no id"}
        ]}"#;
        assert_eq!(parse_listing(body).unwrap(), ["gemma-4-12b", "qwen3-8b"]);
        assert!(parse_listing("not json").is_err());
        assert!(parse_listing(r#"{"models":[]}"#).is_err());
        let many: Vec<String> = (0..100)
            .map(|i| format!(r#"{{"id":"m-{i:03}"}}"#))
            .collect();
        let body = format!(r#"{{"data":[{}]}}"#, many.join(","));
        assert_eq!(parse_listing(&body).unwrap().len(), MOST_LISTED);
    }
}
