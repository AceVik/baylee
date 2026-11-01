//! Reading provider answers against the current question.

use super::{AnswerMode, Decision, Menu, Read, Reply, Stop};

/// A reply's answer before it is read against its question: the decision,
/// the call it came from, and the other calls, which are ignored.
pub(super) struct Taken {
    pub(super) decision: Decision,
    call: Option<String>,
    others: Vec<String>,
}

/// Takes a reply's first `decide` or `concede` call as a decision.
pub(super) fn take(reply: &Reply, mode: AnswerMode) -> Result<Taken, String> {
    let mut chosen = None;
    let mut others = Vec::new();
    for call in &reply.calls {
        let known = call.name == "decide" || call.name == "concede";
        if known && chosen.is_none() {
            chosen = Some(call);
        } else if !call.id.is_empty() {
            others.push(call.id.clone());
        }
    }
    let Some(call) = chosen else {
        return Err(match (mode, &reply.stop) {
            (_, Stop::MaxTokens) => "the reply ran out of tokens before it answered".into(),
            (AnswerMode::Tools, _) => "answer with one call of the decide tool".into(),
            (AnswerMode::Json | AnswerMode::JsonSchema, _) => {
                "answer with one JSON object and nothing else".into()
            }
        });
    };
    let decision = if mode.is_json() && call.id.is_empty() {
        Decision::from_json_answer(&call.input)?
    } else if call.name == "concede" {
        Decision::from_concede(&call.input)
    } else {
        Decision::from_decide(&call.input)?
    };
    Ok(Taken {
        decision,
        call: (!call.id.is_empty()).then(|| call.id.clone()),
        others,
    })
}

/// Reads a taken answer against `menu`.
pub(super) fn read(taken: Taken, menu: &Menu) -> Result<Read, String> {
    let resolved = menu.resolve(&taken.decision)?;
    Ok(Read {
        resolved,
        say: taken.decision.say.filter(|s| !s.is_empty()),
        call: taken.call,
        others: taken.others,
    })
}
