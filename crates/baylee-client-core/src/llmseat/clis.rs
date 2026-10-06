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

/// What a profile may name for one tool.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CliChoices {
    /// The tool.
    pub tool: CliTool,
    /// Its own model ids (`claude:<id>`), the tool's default first where
    /// it has a fixed one. Empty where the ids are the player's own
    /// providers' (opencode's `provider/model`).
    pub models: &'static [&'static str],
    /// How a model id is spelled, for a tool whose list is empty or long.
    pub model_hint: &'static str,
    /// The model played when the profile names none: `None` for the tool's
    /// own default.
    pub default_model: Option<&'static str>,
    /// The effort levels its effort flag takes, least first; empty for a
    /// tool with none.
    pub efforts: &'static [&'static str],
    /// Whether the conversation goes on across turns (one process holds
    /// it, or the tool resumes it by its id from the seat's own store);
    /// otherwise each question is a conversation of its own.
    pub keeps_conversation: bool,
}

/// What a profile may name for `tool`.
#[must_use]
pub const fn choices(tool: CliTool) -> CliChoices {
    match tool {
        // `claude --help` (2.1.290): "an alias for the latest model (e.g.
        // 'fable', 'opus', or 'sonnet') or a model's full name"; `--effort`
        // "(low, medium, high, xhigh, max)".
        CliTool::Claude => CliChoices {
            tool,
            models: &["opus", "sonnet", "fable"],
            model_hint: "an alias (opus, sonnet, fable) or a model's full name",
            default_model: None,
            efforts: &["low", "medium", "high", "xhigh", "max"],
            keeps_conversation: true,
        },
        // `agy --help` (1.2.17): `--effort` "(low|medium|high|xhigh|max)";
        // its models are listed by `agy models`.
        CliTool::Agy => CliChoices {
            tool,
            models: &[DEFAULT_AGY_MODEL, "gemini-3.8-flash-medium"],
            model_hint: "a model as `agy models` lists it",
            default_model: Some(DEFAULT_AGY_MODEL),
            efforts: &["low", "medium", "high", "xhigh", "max"],
            keeps_conversation: true,
        },
        // Codex's models page: Astra, Sol, Luna; `model_reasoning_effort`
        // up to `ultra` (Luna up to `max`).
        CliTool::Codex => CliChoices {
            tool,
            models: &["gpt-6-astra", "gpt-6.1-sol", "gpt-6-luna"],
            model_hint: "a model id from Codex's models page",
            default_model: None,
            efforts: &["low", "medium", "high", "xhigh", "max", "ultra"],
            keeps_conversation: false,
        },
        // opencode's models page: ids are `provider/model`; `--variant` is
        // provider-specific (Anthropic high, max; OpenAI none to xhigh;
        // Google low, high).
        CliTool::Opencode => CliChoices {
            tool,
            models: &[],
            model_hint: "provider/model, as `opencode models` lists them",
            default_model: None,
            efforts: &["none", "minimal", "low", "medium", "high", "xhigh", "max"],
            // `run --session <id>`, from a database in the seat's store.
            keeps_conversation: true,
        },
        // Junie's model-selection page: its aliases; `junie --help`
        // (26.9.22): `--effort` "low, medium, high".
        CliTool::Junie => CliChoices {
            tool,
            models: &[
                "sonnet",
                "opus",
                "gpt",
                "gpt-codex",
                "gemini-pro",
                "gemini-flash",
                "grok",
            ],
            model_hint: "an alias (sonnet, opus, gpt, …) or a model id",
            default_model: None,
            efforts: &["low", "medium", "high"],
            keeps_conversation: false,
        },
    }
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
            for model in choices.models {
                assert_eq!(model_fault(model), None, "{model}");
                let named = format!("{}:{model}", tool.name());
                assert_eq!(cli_model(&named), Ok((tool, Some(*model))));
            }
            if let Some(default) = choices.default_model {
                assert_eq!(choices.models.first(), Some(&default), "{tool:?}");
            }
            for effort in choices.efforts {
                assert!(effort_is_a_word(effort), "{effort}");
            }
        }
        assert!(choices(CliTool::Claude).keeps_conversation);
        assert!(!choices(CliTool::Codex).keeps_conversation);
        assert!(choices(CliTool::Opencode).keeps_conversation);
    }
}
