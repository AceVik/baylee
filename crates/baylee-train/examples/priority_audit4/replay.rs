use anyhow::{Context, Result};
use baylee_ai::HeuristicAgent;
use baylee_core::ids::PlayerId;
use baylee_core::preset::AIProfile;
use baylee_gamehost::Session;
use baylee_gamehost::record::{Line, RECORD_VERSION, seen};
use serde_json::{Value, json};

use crate::views;

fn verify(session: &Session, hash: &str, visible: Option<&str>) -> Result<bool> {
    if format!("{:016x}", session.snapshot_hash()) == hash {
        return Ok(false);
    }
    // This is the only full-state access: cross-build integrity, never policy.
    anyhow::ensure!(
        visible.is_some_and(|value| value == format!("{:016x}", seen(session.state()))),
        "replay diverged: both exact hash and public seen integrity failed"
    );
    Ok(true)
}

fn completed_lines(bytes: &[u8]) -> Result<Vec<Line>> {
    let lines: Vec<Line> = bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(serde_json::from_slice)
        .collect::<std::result::Result<_, _>>()?;
    anyhow::ensure!(
        matches!(lines.last(), Some(Line::End { .. })),
        "only completed records ending in End may be audited"
    );
    Ok(lines)
}

pub fn audit(bytes: &[u8], seat: u8) -> Result<Value> {
    let lines = completed_lines(bytes)?;
    let Some(Line::Header {
        record,
        preset,
        hash,
        seen,
        ..
    }) = lines.first()
    else {
        anyhow::bail!("record must begin with Header");
    };
    anyhow::ensure!(*record == RECORD_VERSION, "unsupported record version");
    anyhow::ensure!(usize::from(seat) < preset.seats.len(), "seat outside table");
    let teams: Vec<_> = preset.seats.iter().map(|seat| seat.team).collect();
    let sharp = HeuristicAgent::new(AIProfile::SHARP).with_teams(teams.clone());
    let mut session = Session::new(preset).context("invalid record preset")?;
    for index in 0..preset.seats.len() {
        let player = PlayerId::new(u8::try_from(index)?);
        // Human seats already accept external inputs; AI seats must be driven
        // so act() cannot silently pump unrecorded house answers.
        session.take_over(player);
    }
    let mut fallback_checks = usize::from(verify(&session, hash, seen.as_deref())?);
    let mut fallback_inputs = Vec::new();
    let player = PlayerId::new(seat);
    let mut events = Vec::new();
    let mut inputs = 0;
    for line in &lines[1..lines.len() - 1] {
        match line {
            Line::Input {
                n,
                seat: actor,
                by,
                action,
                hash,
                seen,
                ..
            } => {
                let before = session.observe_view(player);
                let own = *actor == seat;
                let mut event = json!({"n":n,"actor":actor,"by":by,
                    "kind":if own {"own_action"} else {"other_pool_drop"},
                    "before":views::snapshot(&before)});
                if own {
                    event["actual"] = views::action(&before, action);
                    if let Some((pending, view)) = session.view_for(player) {
                        event["pending"] = json!(
                            format!("{pending:?}")
                                .split([' ', '{', '('])
                                .next()
                                .unwrap_or("Unknown")
                        );
                        event["priority"] = views::priority(&view, &pending, &teams, &sharp)?;
                    }
                }
                session
                    .act(PlayerId::new(*actor), action.clone())
                    .map_err(|error| {
                        crate::diagnostics::refused(
                            &session,
                            player,
                            *actor,
                            *n,
                            action,
                            (inputs, fallback_checks, &fallback_inputs),
                            &error,
                        )
                    })?;
                let fallback = verify(&session, hash, seen.as_deref())
                    .with_context(|| format!("after input {n}"))?;
                if fallback {
                    fallback_inputs.push(*n);
                }
                fallback_checks += usize::from(fallback);
                inputs += 1;
                let after = session.observe_view(player);
                if own || views::pool(&after) < views::pool(&before) {
                    event["after"] = views::snapshot(&after);
                    events.push(event);
                }
            }
            Line::Header { .. } | Line::End { .. } => anyhow::bail!("unexpected nested Header/End"),
            Line::Chair { .. } | Line::DeclaredMind { .. } => {}
        }
    }
    anyhow::ensure!(
        matches!(
            session.pending(),
            baylee_engine::choice::Pending::GameOver(_)
        ),
        "End record did not replay to GameOver"
    );
    Ok(
        json!({"seat":seat,"inputs_verified":inputs,"seen_fallback_checks":fallback_checks,
        "seen_fallback_input_n":fallback_inputs,
        "scope":"completed record; policy diagnostics use only the selected seat PlayerView",
        "end":lines.last(),"events":events}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_engine::choice::PlayerAction;
    use baylee_train::housedeck::HouseDeck;

    fn completed() -> Vec<Line> {
        let deck = HouseDeck::named("victory").unwrap();
        let preset =
            baylee_train::selfplay::table_for(19, &[&deck, &deck], &[AIProfile::STEADY; 2], &[]);
        let mut session = Session::new_recorded(&preset, "priority-audit-test").unwrap();
        session.take_over(PlayerId::new(0));
        session.take_over(PlayerId::new(1));
        session
            .act(PlayerId::new(0), PlayerAction::Concede)
            .unwrap();
        session
            .take_record()
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
            .map(|line| serde_json::from_slice(line).unwrap())
            .collect()
    }

    fn bytes(lines: &[Line]) -> Vec<u8> {
        lines
            .iter()
            .flat_map(|line| {
                let mut bytes = serde_json::to_vec(line).unwrap();
                bytes.push(b'\n');
                bytes
            })
            .collect()
    }

    #[test]
    fn completed_replay_records_own_action_and_rejects_truncation() {
        let mut lines = completed();
        let report = audit(&bytes(&lines), 0).unwrap();
        assert_eq!(report["inputs_verified"], 1);
        assert_eq!(report["events"][0]["kind"], "own_action");
        lines.pop();
        assert!(audit(&bytes(&lines), 0).is_err());
    }

    #[test]
    fn hash_mismatch_requires_matching_seen_at_every_input() {
        let mut lines = completed();
        let input = lines
            .iter_mut()
            .find(|line| matches!(line, Line::Input { .. }))
            .unwrap();
        if let Line::Input { hash, .. } = input {
            *hash = "different-build".into();
        }
        assert_eq!(audit(&bytes(&lines), 0).unwrap()["seen_fallback_checks"], 1);
        let input = lines
            .iter_mut()
            .find(|line| matches!(line, Line::Input { .. }))
            .unwrap();
        if let Line::Input { seen, .. } = input {
            *seen = Some("wrong".into());
        }
        assert!(audit(&bytes(&lines), 0).is_err());
    }
}
