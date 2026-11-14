//! Reconstruct bounded session metadata without adding new record lines.
use super::{Refused, Session};
use crate::{
    record::{Mind, MindKind},
    session::{MAX_DECLARATIONS, SessionRegistry},
};
use baylee_engine::engine::LoopDetection;
use baylee_protocol::v1::{SeatMind, seat_mind::Kind};

pub(super) fn declaration<D: LoopDetection, R: SessionRegistry>(
    session: &mut Session<D, R>,
    mind: Mind,
) -> Result<(), Refused> {
    if session.minds[0].as_ref() == Some(&mind) {
        return Err(Refused::Invalid);
    }
    if mind.kind == MindKind::Undeclared {
        // Socket changes invalidate an earlier declaration automatically. They
        // do not consume the live declaration allowance.
        if mind != Mind::undeclared() || session.minds[0].is_none() {
            return Err(Refused::Invalid);
        }
    } else {
        let kind = match mind.kind {
            MindKind::Human => Kind::Human,
            MindKind::House => Kind::House,
            MindKind::Scripted => Kind::Scripted,
            MindKind::LlmApi => Kind::LlmApi,
            MindKind::LlmCli => Kind::LlmCli,
            MindKind::Undeclared => return Err(Refused::Invalid),
        };
        let said = SeatMind {
            kind: kind.into(),
            provider: mind.provider.clone(),
            model: mind.model.clone(),
            effort: mind.effort.clone(),
            level: mind.level.clone(),
        };
        Mind::from_wire(&said).map_err(|_| Refused::Invalid)?;
        if session.declarations[0] >= MAX_DECLARATIONS {
            return Err(Refused::Invalid);
        }
        session.declarations[0] += 1;
    }
    session.minds[0] = Some(mind);
    session.unsaid[0] = false;
    Ok(())
}
