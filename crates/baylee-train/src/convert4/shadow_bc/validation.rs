use super::{
    Context, EncodedCandidates4, EncodedState4, Pending, PlayerAction, PlayerView, ShadowBc4,
    ShadowLabelProvenance4, StepProvenance4, TeacherTarget4, ensure, sha,
};
use crate::features4::{
    HeadKind4, SourceKind4, decode_action4, encode_teacher_action4, view_only_state_hash,
};
use baylee_ai::HeuristicAgent;
use baylee_core::preset::AIProfile;

#[allow(
    clippy::too_many_arguments,
    reason = "Explicit independently reconstructed replay bindings"
)]
pub(super) fn target(
    shadow: &mut ShadowBc4,
    input_n: u64,
    decision_seq: u64,
    seat: u8,
    action: &PlayerAction,
    provenance: &StepProvenance4,
    view: &PlayerView,
    pending: &Pending,
    state: &EncodedState4,
    menu: &EncodedCandidates4,
    teams: &[Option<u8>],
) -> anyhow::Result<Option<(TeacherTarget4, ShadowLabelProvenance4)>> {
    let Some(label) = shadow.labels.remove(&input_n) else {
        return Ok(None);
    };
    ensure!(
        provenance.source == SourceKind4::Net
            && *provenance == label.executed_provenance
            && *action == label.executed_action
            && seat == label.seat
            && decision_seq == label.decision_seq,
        "Shadow behavior/decision provenance mismatch"
    );
    ensure!(
        label.view_hash_fnv1a64
            == format!(
                "{:016x}",
                if shadow.encoder_version == 8 {
                    crate::features8::view_only_state_hash8(view, pending)
                } else {
                    view_only_state_hash(view, pending)
                }
            )
            && label.pending_sha256 == sha(&serde_json::to_vec(pending)?)
            && label.encoded_state_sha256 == sha(&serde_json::to_vec(state)?)
            && label.candidates_sha256 == sha(&serde_json::to_vec(menu)?),
        "Shadow view/state/menu mismatch"
    );
    let profile = shadow.header["contract"]["teacher"]
        .as_str()
        .context("Teacher")?;
    let teacher = HeuristicAgent::new(if profile == "sharp" {
        AIProfile::SHARP
    } else {
        AIProfile::EXPERT
    })
    .with_teams(teams.to_vec())
    .with_seed(
        shadow.header["teacher_seed"]
            .as_u64()
            .context("Teacher seed")?
            ^ decision_seq.rotate_left(17)
            ^ u64::from(seat),
    );
    ensure!(
        teacher.act(view, pending) == label.teacher_action && teacher.fallbacks() == 0,
        "Shadow teacher answer mismatch/refit"
    );
    ensure!(
        pending.answer_fault(&label.teacher_action).is_none(),
        "Illegal shadow teacher answer"
    );
    let target = encode_teacher_action4(menu, pending, &label.teacher_action)
        .context("Unmappable teacher action")?;
    let decoded =
        decode_action4(menu, view, pending, &target).context("Undecodable teacher label")?;
    ensure!(
        pending.answer_fault(&decoded).is_none()
            && (menu.head == HeadKind4::SingleChoice || decoded == label.teacher_action)
            && encode_teacher_action4(menu, pending, &decoded).as_ref() == Some(&target)
            && target == label.teacher_target,
        "Shadow label is repaired or differs from reconstructed teacher target"
    );
    let provenance = ShadowLabelProvenance4 {
        run_sha256: shadow.header["run_sha256"]
            .as_str()
            .context("Run SHA")?
            .into(),
        source_game: shadow.header["game"].as_str().context("Game key")?.into(),
        record_sha256: shadow.record_sha256.clone(),
        record_input_n: input_n,
        teacher: profile.into(),
        teacher_action: label.teacher_action,
        executed_action: label.executed_action,
        executed_provenance: label.executed_provenance,
    };
    Ok(Some((target, provenance)))
}
