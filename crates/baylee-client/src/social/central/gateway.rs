//! The game gateway serves only game-scoped chat (table, team, invites);
//! personal chats, contacts, and friend presence live exclusively on Baylee Central.
use super::{Down, HttpTag, SocialHost, apply, now_ms, personal, transport, wire};

/// The most table and team conversations a game gateway's `Members` frames
/// may open beside its snapshot, each with this account among its members.
/// A player sits at one table and its team at a time, and a table left or
/// ended leaves the list; a gateway that names more is not believed, so a
/// hostile one cannot grow the list (and the chat's conversations) without
/// bound.
pub(super) const MAX_TABLE_CONVS: usize = 32;

/// Whether `conv` is a table's or a team's conversation: the only kinds a
/// game gateway names in a `Members` frame.
fn game_conv(conv: &str) -> bool {
    matches!(
        conv.parse::<wire::Conv>(),
        Ok(wire::Conv::Table { .. } | wire::Conv::Team { .. })
    )
}

pub(super) enum Registration {
    Waiting(u64),
    Pending,
    Ready,
}

pub(in crate::social) fn gateway_http(
    host: &mut SocialHost,
    tag: &HttpTag,
    status: u16,
    body: &[u8],
) -> bool {
    if host.central.is_none() {
        return false;
    }
    if matches!(tag, HttpTag::AddFriend(_) | HttpTag::Friendship(_)) {
        return true;
    }
    if matches!(tag, HttpTag::RegisterDevice) {
        if let Some(bridge) = host.central.as_mut() {
            let registered = (200..300).contains(&status)
                && serde_json::from_slice::<wire::Created>(body)
                    .is_ok_and(|created| created.id == bridge.device);
            bridge.game_state = if registered {
                Registration::Ready
            } else {
                Registration::Waiting(now_ms() + 30_000)
            };
            if registered && bridge.wanted {
                host.socket.want();
            }
        }
        return true;
    }
    false
}
pub(in crate::social) fn gateway_news(host: &mut SocialHost, news: &transport::LinkNews) -> bool {
    let Some(bridge) = host.central.as_mut() else {
        return false;
    };
    match news {
        transport::LinkNews::Up => {
            if let Some(hello) = &bridge.hello
                && let Ok(text) = serde_json::to_string(hello)
            {
                host.socket.send(text);
            }
        }
        transport::LinkNews::Frame(text) => {
            let Ok(frame) = serde_json::from_str::<Down>(text) else {
                return true;
            };
            receive_frame(host, frame);
        }
        transport::LinkNews::Down
        | transport::LinkNews::Refused
        | transport::LinkNews::Unauthorized => {
            bridge.game_welcomed = false;
            let pending = std::mem::take(&mut bridge.game_requests);
            for req in pending {
                super::refused(host, req, wire::Why::Off);
            }
        }
    }
    true
}

fn receive_frame(host: &mut SocialHost, frame: Down) {
    let Some(bridge) = host.central.as_mut() else {
        return;
    };
    let forwarded = match frame {
        Down::Welcome { .. }
        | Down::Friend { .. }
        | Down::Group { .. }
        | Down::Refused { req: None, .. } => None,
        Down::Snapshot(snapshot) => {
            bridge.game = snapshot;
            bridge.game_welcomed = true;
            Some(Down::Snapshot(bridge.snapshot()))
        }
        Down::Sent { req, .. } | Down::Refused { req: Some(req), .. }
            if !bridge.game_requests.remove(&req) =>
        {
            None
        }
        Down::Msg { ref conv, .. }
        | Down::Members { ref conv, .. }
        | Down::Notice { ref conv, .. }
            if personal(conv) =>
        {
            None
        }
        Down::Members {
            ref conv,
            ref members,
        } => {
            let known = bridge
                .game
                .tables
                .iter()
                .position(|table| table.conv == *conv);
            // A table this account has left goes as one that ended does, so
            // a long session's tables never fill the list below.
            let mine = members.iter().any(|m| m.tag == bridge.session.tag);
            match known {
                Some(at) if !mine => {
                    bridge.game.tables.remove(at);
                    Some(frame)
                }
                Some(at) => {
                    bridge.game.tables[at].members.clone_from(members);
                    Some(frame)
                }
                // Only a table's or a team's conversation of this account's
                // is the game gateway's to name, and only so many: anything
                // else, or one past the cap, reaches neither this list nor
                // the chat.
                None if mine && game_conv(conv) && bridge.game.tables.len() < MAX_TABLE_CONVS => {
                    bridge.game.tables.push(wire::TableConv {
                        conv: conv.clone(),
                        name: String::new(),
                        members: members.clone(),
                    });
                    Some(frame)
                }
                None => None,
            }
        }
        Down::Presence {
            ref tag,
            ref presence,
        } => {
            if let Some(friend) = bridge
                .game
                .friends
                .iter_mut()
                .find(|friend| friend.person.tag == *tag)
            {
                friend.presence.clone_from(presence);
            }
            Some(Down::Snapshot(bridge.snapshot()))
        }
        Down::Invite(ref invite) => {
            bridge.game.invites.retain(|old| old.id != invite.id);
            bridge.game.invites.push(invite.clone());
            Some(frame)
        }
        Down::InviteGone { ref id, .. } => {
            bridge.game.invites.retain(|old| old.id != *id);
            Some(frame)
        }
        _ => Some(frame),
    };
    if let Some(frame) = forwarded {
        apply(host, frame);
    }
}

pub(super) fn register(host: &mut SocialHost, now: u64) {
    let Some(bridge) = host.central.as_mut() else {
        return;
    };
    if bridge.session.token.is_empty() || bridge.session.gateway.is_empty() {
        return;
    }
    if !bridge.wanted || !matches!(bridge.game_state, Registration::Waiting(at) if now >= at) {
        return;
    }
    let Some(registration) = bridge.game_registration.as_ref() else {
        return;
    };
    bridge.game_state = Registration::Pending;
    transport::request(
        &host.mailbox,
        &bridge.session.gateway,
        &bridge.session.token,
        HttpTag::RegisterDevice,
        super::Method::Post,
        "/social/devices",
        Some(registration),
    );
}
