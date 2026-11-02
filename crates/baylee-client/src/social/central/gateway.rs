//! The game gateway cannot replace Central contacts or acknowledge personal sends.
use super::{Down, HttpTag, OsEntropy, SocialHost, apply, now_ms, personal, transport, wire};
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
            Some(Down::Snapshot(bridge.snapshot()))
        }
        Down::Sent { req, .. } | Down::Refused { req: Some(req), .. }
            if !bridge.game_requests.remove(&req) =>
        {
            None
        }
        Down::Msg {
            seq,
            at,
            conv,
            from,
            sealed,
        } if conv.starts_with("grp:") => {
            let group = bridge
                .game
                .groups
                .iter()
                .find(|group| conv == format!("grp:{}", group.id))
                .cloned();
            if let Some(group) = group
                && let Some(machine) = host.machine.as_mut()
            {
                let effects = machine.on_legacy_group(
                    baylee_client_core::social::RetainedMessage {
                        seq,
                        at,
                        conv,
                        from,
                        sealed,
                    },
                    &group,
                    now_ms(),
                    &mut OsEntropy,
                );
                host.carry_out(effects);
            }
            None
        }
        Down::Members { ref conv, .. } | Down::Notice { ref conv, .. } if personal(conv) => None,
        Down::Members {
            ref conv,
            ref members,
        } => {
            if let Some(table) = bridge
                .game
                .tables
                .iter_mut()
                .find(|table| table.conv == *conv)
            {
                table.members.clone_from(members);
            }
            Some(frame)
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
            Some(frame)
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
