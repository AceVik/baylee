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

/// Whether `path` plays the model `first` plays at its effort, in a
/// sentence where it does not. The same model is reached on different
/// paths under different names: an API's model id (`claude-haiku-4-5`),
/// a CLI's own name for it (`claude:haiku`). So the efforts must be equal
/// and named, every path must name its model (a CLI's own default may
/// change under it), two paths of one kind must name the same model, and
/// a CLI's name must be the API's id or one of its words.
///
/// # Errors
/// Why they differ.
pub fn same_model(first: &SeatMind, path: &SeatMind) -> Result<(), String> {
    let cli = seat_mind::Kind::LlmCli as i32;
    let api = seat_mind::Kind::LlmApi as i32;
    for mind in [first, path] {
        if mind.kind != cli && mind.kind != api {
            return Err("an access path is a language model behind a CLI or an API".into());
        }
        if mind.model.is_empty() {
            return Err(format!(
                "the {} path names no model: a CLI's own default can change under it",
                mind.provider
            ));
        }
        if mind.effort.is_empty() {
            return Err(format!(
                "«{}» names no effort: the chain plays one effort, named on every path",
                mind.model
            ));
        }
    }
    if first.effort != path.effort {
        return Err(format!(
            "effort «{}» is not «{}»",
            path.effort, first.effort
        ));
    }
    let alias = |cli: &str, id: &str| cli == id || id.split(['-', '.', '_', '/']).any(|w| w == cli);
    let same = match (first.kind == cli, path.kind == cli) {
        (true, true) => first.provider == path.provider && first.model == path.model,
        (false, false) => first.model == path.model,
        (true, false) => alias(&first.model, &path.model),
        (false, true) => alias(&path.model, &first.model),
    };
    if !same {
        return Err(format!("«{}» is not «{}»", path.model, first.model));
    }
    Ok(())
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
