//! Replay a time ruling minted once by Central and verified by both engines.
use crate::variant::HostedSession;
use baylee_core::ids::PlayerId;
use baylee_protocol::v1;

pub(super) fn apply(
    session: &mut HostedSession,
    seat: PlayerId,
    event: v1::RankedAuthorityEvent,
) -> Result<Vec<(PlayerId, v1::Envelope)>, &'static str> {
    if event.seat != u32::from(seat.get()) || event.seat > 1 || !matches!(event.reason, 1 | 2) {
        return Err("invalid authority ruling");
    }
    let decision = event.decision.ok_or("missing authority decision")?;
    if session.asked_at(seat) != Some(decision) {
        return Err("obsolete authority decision");
    }
    session
        .concede_for_absent(seat)
        .ok_or("seat cannot concede")
}

#[cfg(test)]
mod tests {
    use super::*;
    use prost::Message as _;

    #[test]
    fn system_command_matches_central_wire_exactly() {
        let rules = baylee_core::preset::HouseRules::ranked();
        assert_eq!(
            (rules.decision_timeout_secs, rules.reconnect_window_secs),
            (180, 60)
        );
        let event = v1::Envelope {
            msg: Some(v1::envelope::Msg::RankedAuthorityEvent(
                v1::RankedAuthorityEvent {
                    seat: 1,
                    decision: Some(128),
                    reason: 2,
                },
            )),
        };
        assert_eq!(event.encode_to_vec(), [0x92, 3, 7, 8, 1, 16, 128, 1, 24, 2]);
    }
}
