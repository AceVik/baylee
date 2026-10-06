//! What the seat tells the table answers it (`v1::SeatMind`), for the
//! game's record (#315): the kind of mind, and for a language model its
//! provider, exact model and effort. Never how the model is reached: no
//! address, no key, no profile name, no prompt (`docs/protocol.md` §"Who
//! answers a seat, as it says"). The engine refuses a declaration by
//! `baylee_protocol::mind::fault`, so each one built here is held to it.

use crate::llm::Settings;
use baylee_client_core::llmseat::{Provider, cli_model};
use baylee_protocol::v1::{SeatMind, seat_mind::Kind};

/// The house heuristic at `level` (`--level`).
#[must_use]
pub fn house(level: &str) -> SeatMind {
    SeatMind {
        kind: Kind::House as i32,
        level: level.to_owned(),
        ..SeatMind::default()
    }
}

/// The script that gives the least answer.
#[must_use]
pub fn scripted() -> SeatMind {
    SeatMind {
        kind: Kind::Scripted as i32,
        ..SeatMind::default()
    }
}

/// A language model as `settings` plays it: an API's provider (`anthropic`,
/// `openai`) and model id, or a CLI's tool and the model it names (empty
/// for the tool's own default), with the effort asked for.
#[must_use]
pub fn llm(settings: &Settings) -> SeatMind {
    let effort = settings.effort.clone().unwrap_or_default();
    match settings.provider {
        Provider::Cli => {
            let (tool, model) = cli_model(&settings.model)
                .map(|(tool, own)| (tool.name().to_owned(), own.unwrap_or_default().to_owned()))
                .unwrap_or_default();
            SeatMind {
                kind: Kind::LlmCli as i32,
                provider: tool,
                model,
                effort,
                level: String::new(),
            }
        }
        Provider::Anthropic | Provider::OpenAi => SeatMind {
            kind: Kind::LlmApi as i32,
            provider: match settings.provider {
                Provider::Anthropic => "anthropic",
                _ => "openai",
            }
            .to_owned(),
            model: settings.model.clone(),
            effort,
            level: String::new(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{Kind, house, llm, scripted};
    use crate::llm::{Settings, Spec};
    use baylee_protocol::mind::fault;

    fn settings(mind: &str, effort: Option<&str>) -> Settings {
        let spec = Spec::parse(mind).expect("a model").expect("a valid one");
        let mut settings = Settings::new(&spec);
        settings.effort = effort.map(str::to_owned);
        settings
    }

    /// Each mind the bridge plays declares itself in a shape the engine
    /// takes, naming the exact model and its effort.
    #[test]
    fn every_mind_declares_what_it_is_in_a_shape_the_engine_takes() {
        let api = llm(&settings("anthropic:claude-opus-5-5", Some("high")));
        assert_eq!(api.kind, Kind::LlmApi as i32);
        assert_eq!(
            (
                api.provider.as_str(),
                api.model.as_str(),
                api.effort.as_str()
            ),
            ("anthropic", "claude-opus-5-5", "high")
        );
        let open = llm(&settings("openai:deepseek-chat", None));
        assert_eq!(
            (open.provider.as_str(), open.model.as_str()),
            ("openai", "deepseek-chat")
        );
        let cli = llm(&settings("cli:claude:opus", Some("low")));
        assert_eq!(cli.kind, Kind::LlmCli as i32);
        assert_eq!(
            (
                cli.provider.as_str(),
                cli.model.as_str(),
                cli.effort.as_str()
            ),
            ("claude", "opus", "low")
        );
        let bare = llm(&settings("cli:codex", None));
        assert_eq!((bare.provider.as_str(), bare.model.as_str()), ("codex", ""));
        for declared in [api, open, cli, bare, house("steady"), scripted()] {
            assert_eq!(fault(&declared), None, "{declared:?}");
        }
    }
}
