//! Deterministic replay, with diagnostics drawn only from each acting seat's view.
use anyhow::{Context as _, ensure};
use baylee_core::ids::PlayerId;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_gamehost::{Session, record::Line};
use baylee_train::features4::{PendingKind4, SeatKnowledge4, encode_executed_action4};
use baylee_train::features7::{encode_candidates7_grouped, encode_state7};
use baylee_train::features8::encode_state8;
use serde_json::{Value, json};

pub(super) fn replay(lines: &[Line]) -> anyhow::Result<Value> {
    let Some(Line::Header {
        preset,
        hash,
        record,
        ..
    }) = lines.first()
    else {
        anyhow::bail!("record has no header");
    };
    ensure!(
        *record == baylee_gamehost::record::RECORD_VERSION,
        "unsupported record version"
    );
    let mut session = Session::new(preset).context("session could not be built")?;
    check_hash(&session, hash)?;
    for seat in 0..preset.seats.len() {
        session.take_over(PlayerId::new(u8::try_from(seat)?));
    }
    let teams: Vec<_> = preset.seats.iter().map(|seat| seat.team).collect();
    let mut knowledge7: Vec<_> = (0..preset.seats.len())
        .map(|seat| {
            SeatKnowledge4::from_preset(preset, PlayerId::new(u8::try_from(seat).expect("seat")))
        })
        .collect();
    let mut knowledge8 = knowledge7.clone();
    let mut frames = Vec::new();
    let mut after_cancel = [false; 8];
    let mut replayed = 0_u64;
    let mut ended = false;
    for line in lines.iter().skip(1) {
        ensure!(!ended, "record continued after End");
        ensure!(!matches!(line, Line::Header { .. }), "second record header");
        let Line::Input {
            n,
            at,
            seat,
            action,
            hash,
            ..
        } = line
        else {
            if matches!(line, Line::End { .. }) {
                ensure!(
                    matches!(session.pending(), Pending::GameOver(_)),
                    "End before GameOver"
                );
                ended = true;
            }
            continue;
        };
        ensure!(usize::from(*seat) < after_cancel.len(), "invalid seat");
        // Observe every seat chronologically, as live/recurrent conversion does.
        for index in 0..preset.seats.len() {
            let player = PlayerId::new(u8::try_from(index)?);
            knowledge7[index].observe(&session.observe_view_with_context(player));
            knowledge8[index]
                .observe_with_payment_context(&session.observe_view_with_payment_context(player));
        }
        let player = PlayerId::new(*seat);
        if !action.is_automation_setting()
            && let Some(frame) = diagnostic_frame(
                &session,
                player,
                action,
                *n,
                after_cancel[usize::from(*seat)],
                (
                    &knowledge7[usize::from(*seat)],
                    &knowledge8[usize::from(*seat)],
                ),
                &teams,
            )?
        {
            frames.push(frame);
        }
        session.tell_time(*at);
        session
            .act_slice(player, action.clone(), 0)
            .map_err(|error| anyhow::anyhow!("input {n} refused: {error}"))?;
        check_hash(&session, hash).with_context(|| format!("after input {n}"))?;
        replayed += 1;
        after_cancel[usize::from(*seat)] = matches!(action, PlayerAction::CancelCast);
    }
    Ok(
        json!({"inputs_replayed":replayed,"ended":ended,"all_snapshot_hashes_match":true,
        "scope":"actual authorized views only; numeric bounds are not a payability proof; question-local faults exclude contextual CancelCast permission",
        "frames":frames}),
    )
}

fn diagnostic_frame(
    session: &Session,
    player: PlayerId,
    action: &PlayerAction,
    n: u64,
    after_cancel: bool,
    knowledge: (&SeatKnowledge4, &SeatKnowledge4),
    teams: &[Option<u8>],
) -> anyhow::Result<Option<Value>> {
    let seat = player.get();
    let (pending, view7) = session
        .view_for_with_context(player)
        .context("missing acting seat question")?;
    let (_, view8) = session
        .view_for_with_payment_context(player)
        .context("missing payment seat question")?;
    let selected = view8.announced_cast.is_some()
        || view8.casting.is_some()
        || after_cancel
        || matches!(
            action,
            PlayerAction::CastSpell { .. }
                | PlayerAction::CancelCast
                | PlayerAction::ChooseNumber { .. }
        );
    if !selected {
        return Ok(None);
    }
    let state7 = encode_state7(&view7, Some(&pending), Some(knowledge.0), teams);
    let state8 = encode_state8(&view8, Some(&pending), Some(knowledge.1), teams);
    let menu7 = encode_candidates7_grouped(&state7, &view7, &pending);
    let menu8 = encode_candidates7_grouped(&state8, &view8, &pending);
    let source_row = view8
        .announced_cast
        .or(view8.casting)
        .and_then(|source| state8.find_entity(source));
    let source_name = view8.announced_cast.or(view8.casting).and_then(|source| {
        view8
            .hand
            .iter()
            .find(|object| object.id == source)
            .map(|object| object.name.as_str())
    });
    let public_source_name = view8.announced_cast.or(view8.casting).and_then(|source| {
        view8
            .battlefield
            .iter()
            .chain(&view8.stack)
            .chain(view8.graveyards.iter().flatten())
            .chain(view8.exile.iter().flatten())
            .chain(view8.command.iter().flatten())
            .chain(&view8.looking_at)
            .chain(&view8.library_tops)
            .find(|object| object.id == source)
            .map(|object| object.name.as_str())
    });
    let scalar_bounds = match &pending {
        Pending::ChooseNumber {
            min, max, reason, ..
        } => Some(json!({"min":min,"max":max,"reason":format!("{reason:?}")})),
        _ => None,
    };
    Ok(Some(json!({"input_n":n,"seat":seat,"turn":view8.turn,
                    "action":action,"pending_kind":format!("{:?}", PendingKind4::of(&pending)),
                    "question_local_fault":pending.answer_fault(action).map(|fault|format!("{fault:?}")),
                    "contextual_cancel_permission":view8.casting.is_some(),
                    "announced_cast":view8.announced_cast,"source_entity_row":source_row,
                    "source_hand_name":source_name,"source_visible_name":source_name.or(public_source_name),
                    "scalar_bounds":scalar_bounds,
                    "remaining_payment":view8.owed,
                    "visible_mana_pool":view8.seats.iter().find(|row|row.player == player).map(|row|&row.mana_pool),
                    "e7_payment_globals":&state7.global_feat[29..40],
                    "e8_payment_globals":&state8.global_feat[29..40],
                    "e8_source_marker":source_row.map(|row|state8.entity_feat[row][33]),
                    "candidate_count7":menu7.candidates.len(),"candidate_count8":menu8.candidates.len(),
                    "executed_action_encoded7":encode_executed_action4(&menu7,&pending,action).is_some(),
                    "executed_action_encoded8":encode_executed_action4(&menu8,&pending,action).is_some(),
                    "after_same_seat_cancel":after_cancel})))
}

fn check_hash(session: &Session, expected: &str) -> anyhow::Result<()> {
    let actual = format!("{:016x}", session.snapshot_hash());
    ensure!(
        expected == actual,
        "snapshot mismatch: recorded {expected}, replayed {actual}"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_core::preset::AIProfile;

    #[test]
    fn recorded_input_replays_but_corrupt_snapshot_never_certifies() {
        let deck = baylee_train::housedeck::HouseDeck::named("victory").unwrap();
        let preset =
            baylee_train::selfplay::table_for(91, &[&deck, &deck], &[AIProfile::STEADY; 2], &[]);
        let mut session = Session::new_recorded(&preset, "payment-test").unwrap();
        for seat in [0, 1] {
            session.take_over(PlayerId::new(seat));
        }
        let player = PlayerId::new(0);
        let action = session.house_action(player).unwrap();
        session.act_slice(player, action, 0).unwrap();
        let bytes = session.take_record();
        let mut lines: Vec<Line> = bytes
            .split(|&byte| byte == b'\n')
            .filter(|line| !line.is_empty())
            .map(|line| serde_json::from_slice(line).unwrap())
            .collect();
        let report = replay(&lines).unwrap();
        assert_eq!(report["inputs_replayed"], 1);
        assert_eq!(report["all_snapshot_hashes_match"], true);
        for line in &mut lines {
            if let Line::Input { hash, .. } = line {
                *hash = "incorrect".into();
            }
        }
        assert!(replay(&lines).is_err());
        assert!(replay(&[]).is_err());
    }
}
