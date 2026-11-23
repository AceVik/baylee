use super::{Frames, clock};
use baylee_core::{ids::PlayerId, wire};
use baylee_engine::choice::PlayerAction;
use baylee_protocol::v1::{Envelope, envelope::Msg};
use prost::Message as _;

pub(super) const MAX_BYTES: usize = 32 * 1024;

pub(super) fn apply(
    session: &mut crate::variant::HostedSession,
    game_id: &str,
    seat: PlayerId,
    authority: bool,
    bytes: &[u8],
) -> Result<Frames, &'static str> {
    let operation = Envelope::decode(bytes).map_err(|_| "malformed operation")?;
    // Reject concatenated/unknown fields too. The accepted encoding is
    // exactly the bytes that the account authorized and the two hosts read.
    if operation.encode_to_vec() != bytes {
        return Err("noncanonical operation");
    }
    match (authority, operation.msg) {
        (false, Some(Msg::PlayerAction(input)))
            if input.seat_token.is_empty()
                && (input.game_id.is_empty() || input.game_id == game_id) =>
        {
            let action =
                wire::decode::<PlayerAction>(&input.action).map_err(|_| "invalid action")?;
            session
                .act_slice(seat, action, 0)
                .map_err(|_| "refused action")
        }
        (false, Some(Msg::SeatSetting(input))) => {
            let setting = wire::decode(&input.setting).map_err(|_| "invalid setting")?;
            session
                .seat_setting(seat, setting)
                .map_err(|_| "refused setting")
        }
        (true, Some(Msg::RankedAuthorityEvent(event))) => clock::apply(session, seat, event),
        _ => Err("operation outside ranked authority"),
    }
}
