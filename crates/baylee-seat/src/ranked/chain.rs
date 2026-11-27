//! A language-model account's chain of access paths, as the ranked runner
//! plays it (`docs/llm-seat.md` §"The ranked runner"): one model at one
//! effort on every path ([`same_model`]), a path that could not answer held
//! back from new games ([`holding`]), and the declared house at the chain's
//! end ([`house_at_the_end`]), so a ranked seat never leaves its table
//! because its model is gone.

use crate::bridge::{Limit, Standby, Swap};
use crate::mind::Mind;
use baylee_ai::AIProfile;
use baylee_client_core::say;
use baylee_protocol::v1::{SeatMind, seat_mind};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

/// How long a rate-limited path stays out of new games when its provider
/// did not say when the limit lifts.
pub const LIMIT_HOLD: Duration = Duration::from_mins(30);

/// How long a path that failed (a 5xx, the network, a CLI that died) stays
/// out of new games.
pub const FAILED_HOLD: Duration = Duration::from_mins(10);

/// The paths held back from new games, and until when.
pub type Held = Arc<Mutex<BTreeMap<String, Instant>>>;

/// What an access path claims to play, as the account's name and rating
/// read it: the model's family (`claude`, `gemini`, `gpt`, `deepseek`,
/// where it can be told), the model, and its effort.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Claimed {
    /// The model's family, where it can be told from its id or its tool.
    pub family: Option<String>,
    /// The model, as its path names it, without an effort tier in its id.
    pub model: String,
    /// The effort it plays at: named, written in its id (`agy`), or
    /// [`STANDARD`] for a model this build knows no efforts for.
    pub effort: String,
}

/// The effort of a model this build knows no efforts for: its own default,
/// which an account's label calls "Standard" (`KI-DeepSeek-Flash ·
/// Standard`).
pub const STANDARD: &str = "standard";

/// The effort tiers `agy` writes into a model id
/// (`claude-sonnet-5-5-low`, `gemini-4-hydrogen-xhigh`), the longest first.
const AGY_TIERS: [&str; 4] = ["xhigh", "medium", "high", "low"];

/// What `mind` (as `declare::llm` says it) claims to play, its profile
/// naming the effort `named`, where this build knows the model's efforts
/// (`known`) or not. A model whose efforts the build knows must name one
/// (in `agy`'s id, its tier is one); one whose efforts it does not know
/// must name none, and plays at its [`STANDARD`].
///
/// # Errors
/// No language model, no model named (a CLI's own default can change under
/// it), no effort where one must be named, or one where none may be.
pub fn claimed(mind: &SeatMind, named: Option<&str>, known: bool) -> Result<Claimed, String> {
    let cli = mind.kind == seat_mind::Kind::LlmCli as i32;
    if !cli && mind.kind != seat_mind::Kind::LlmApi as i32 {
        return Err("an access path is a language model behind a CLI or an API".into());
    }
    if mind.model.is_empty() {
        return Err(format!(
            "the {} path names no model: a CLI's own default can change under it; name it as \
             `{}:<model>`",
            mind.provider, mind.provider
        ));
    }
    let mut model = mind.model.clone();
    let mut effort = named.unwrap_or_default().to_owned();
    if cli
        && mind.provider == "agy"
        && let Some((stem, tier)) = AGY_TIERS.iter().find_map(|tier| {
            model
                .strip_suffix(&format!("-{tier}"))
                .map(|stem| (stem.to_owned(), *tier))
        })
    {
        // `agy` plays `<stem>-<effort>` where an effort is named, else the
        // tier its id names (`cli::agy`).
        if effort.is_empty() {
            tier.clone_into(&mut effort);
        }
        model = stem;
    }
    if known {
        if effort.is_empty() {
            return Err(format!(
                "«{model}» names no effort; give the profile its `effort`, one of the model's"
            ));
        }
    } else {
        if named.is_some() {
            return Err(format!(
                "this build knows no efforts for «{model}»; leave `effort` out, and it plays at \
                 its Standard"
            ));
        }
        STANDARD.clone_into(&mut effort);
    }
    let family = family_of(&model, cli.then_some(mind.provider.as_str()));
    Ok(Claimed {
        family,
        model,
        effort,
    })
}

/// The family a model id's first word names, else its CLI tool's own.
fn family_of(model: &str, tool: Option<&str>) -> Option<String> {
    let first = model.split(['-', '.', '_', '/', ':']).next()?;
    let family = match first {
        "claude" => "claude",
        // Claude Code's own aliases, on Claude Code or an API only.
        "opus" | "sonnet" | "haiku" | "fable" if matches!(tool, None | Some("claude")) => "claude",
        "gemini" => "gemini",
        "gpt" => "gpt",
        "deepseek" => "deepseek",
        _ => match tool {
            Some("claude") => "claude",
            Some("codex") => "gpt",
            _ => return None,
        },
    };
    Some(family.to_owned())
}

/// Whether `path` plays what `first` plays ([`claimed`]), in a sentence
/// where it does not. The same model is reached on different paths under
/// different names: an API's id (`claude-sonnet-5-5`), a CLI's own alias
/// for it (`claude:sonnet`), `agy`'s id with its tier (`agy`'s
/// `claude-sonnet-5-5-low`). So the efforts must be equal, the families
/// too where both can be told, and the models equal or one the other's
/// word.
///
/// # Errors
/// Why they differ.
pub fn same_model(first: &Claimed, path: &Claimed) -> Result<(), String> {
    if first.effort != path.effort {
        return Err(format!(
            "effort «{}» is not «{}»",
            path.effort, first.effort
        ));
    }
    if let (Some(a), Some(b)) = (&first.family, &path.family)
        && a != b
    {
        return Err(format!("a {b} model is not a {a} model"));
    }
    let word = |short: &str, id: &str| id.split(['-', '.', '_', '/']).any(|w| w == short);
    if first.model == path.model
        || word(&first.model, &path.model)
        || word(&path.model, &first.model)
    {
        Ok(())
    } else {
        Err(format!("«{}» is not «{}»", path.model, first.model))
    }
}

/// Holds `profile` back from new games for `hold`, saying why.
pub fn hold_back(held: &Held, profile: &str, hold: Duration, why: &str) {
    say!(
        "the access path {profile} is held back from new games for {} min: {why}",
        hold.as_secs().div_ceil(60)
    );
    let until = Instant::now() + hold;
    let mut held = held.lock().unwrap_or_else(PoisonError::into_inner);
    let at = held.entry(profile.to_owned()).or_insert(until);
    *at = (*at).max(until);
}

/// How long `limit` holds back the path that could not answer.
#[must_use]
pub fn held_for(limit: &Limit) -> Duration {
    if limit.failed {
        FAILED_HOLD
    } else {
        limit
            .lifts_in
            .unwrap_or(LIMIT_HOLD)
            .max(Duration::from_mins(1))
    }
}

/// `standby`, which on taking over first holds back the path that could
/// not answer: `limit.by`, or the game's first path, `first`.
#[must_use]
pub fn holding(standby: Standby, first: String, held: Held) -> Standby {
    let arm = standby.arm;
    Standby {
        label: standby.label,
        arm: Box::new(move |limit: &Limit| {
            let who = limit.by.clone().unwrap_or(first);
            hold_back(&held, &who, held_for(limit), &limit.said());
            arm(limit)
        }),
    }
}

/// The house at `level` as the chain's last link: declared as the house
/// from the question it takes over, playing the rest of the game.
#[must_use]
pub fn house_at_the_end(level: String, house: AIProfile) -> Standby {
    Standby {
        label: format!("house:{level}"),
        arm: Box::new(move |limit: &Limit| {
            say!(
                "the house ({level}) answers from turn {}: no access path is left ({})",
                limit.turn,
                limit.said()
            );
            Ok(Swap {
                mind: Arc::new(crate::HouseMind::new(house)) as Arc<dyn Mind>,
                think: None,
                taken: None,
                declared: Some(crate::declare::house(&level)),
            })
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agy(model: &str) -> SeatMind {
        SeatMind {
            kind: seat_mind::Kind::LlmCli as i32,
            provider: "agy".into(),
            model: model.into(),
            effort: String::new(),
            level: String::new(),
        }
    }

    /// agy's orchestrators (1.5.18) are models of their own: an account on
    /// one claims (gemini, the orchestrator's id, Standard), never the
    /// model it plans with, and none is another. They take no effort
    /// (`llmseat::clis` offers none, agy refuses `--effort`), so a profile
    /// that names one is refused in the runner's own sentence.
    #[test]
    fn an_agy_orchestrator_is_its_own_model_at_its_standard() {
        let neon = claimed(&agy("gemini-4-neon-xhigh"), None, true).unwrap();
        assert_eq!(
            (neon.model.as_str(), neon.effort.as_str()),
            ("gemini-4-neon", "xhigh")
        );
        let orchestras: Vec<Claimed> = [
            "gemini-pro-orchestrator-neon",
            "gemini-pro-orchestrator",
            "gemini-pro-orchestrator-argon",
        ]
        .iter()
        .map(|id| claimed(&agy(id), None, false).unwrap())
        .collect();
        for (i, orchestra) in orchestras.iter().enumerate() {
            assert_eq!(orchestra.family.as_deref(), Some("gemini"));
            assert!(orchestra.model.starts_with("gemini-pro-orchestrator"));
            assert_eq!(orchestra.effort, STANDARD);
            assert!(same_model(&neon, orchestra).is_err(), "{orchestra:?}");
            assert!(same_model(orchestra, &neon).is_err(), "{orchestra:?}");
            for (j, other) in orchestras.iter().enumerate() {
                assert_eq!(same_model(orchestra, other).is_ok(), i == j);
            }
        }
        let why = claimed(&agy("gemini-pro-orchestrator-neon"), Some("xhigh"), false).unwrap_err();
        assert!(why.contains("knows no efforts"), "{why}");
    }
}
