//! Frame-loop orchestration for the chat host.

#[allow(clippy::wildcard_imports)] // the host's own vocabulary
use super::*;

/// The session the lobby is signed in to, if any.
fn wanted_session(lobby: &LobbyState) -> Option<Session> {
    if lobby.central_ready() {
        let raw_handle = {
            #[cfg(not(target_arch = "wasm32"))]
            {
                lobby
                    .central
                    .as_ref()
                    .and_then(|c| c.profile_handle.clone())
                    .filter(|h| !h.trim().is_empty())
            }
            #[cfg(target_arch = "wasm32")]
            {
                None
            }
        }
        .or_else(|| {
            lobby
                .lobby
                .me()
                .filter(|m| !m.guest)
                .map(|m| m.handle.clone())
        })
        .or_else(|| lobby.lobby.me().map(|m| m.handle.clone()))
        .unwrap_or_else(|| "Player#0000".to_string());
        let (handle, tag) = if let Some(tag) = social::tag_of(&raw_handle) {
            (raw_handle.clone(), tag.to_string())
        } else {
            (format!("{raw_handle}#0000"), "0000".to_string())
        };
        let (gateway, token) = match lobby.lobby.token() {
            Some(tok)
                if !lobby.lobby.offline()
                    && !lobby.lobby.guest()
                    && !lobby.gateway.trim().is_empty() =>
            {
                (
                    lobby.gateway.trim_end_matches('/').to_string(),
                    tok.to_string(),
                )
            }
            _ => (
                baylee_client_core::lobby::gateway_list::PINNED.to_string(),
                String::new(),
            ),
        };
        return Some(Session {
            gateway,
            token,
            handle,
            tag,
            guest: false,
        });
    }
    if lobby.lobby.offline() {
        return Some(Session {
            gateway: String::new(),
            token: String::new(),
            handle: "Guest#0000".to_string(),
            tag: "0000".to_string(),
            guest: true,
        });
    }
    let me = lobby.lobby.me()?;
    let tag = social::tag_of(&me.handle)?.to_string();
    if let Some(token) = lobby.lobby.token() {
        return Some(Session {
            gateway: lobby.gateway.trim_end_matches('/').to_string(),
            token: token.to_string(),
            handle: me.handle.clone(),
            tag,
            guest: me.guest,
        });
    }
    None
}

/// Drives the machine: the session, the gateway's answers, the socket, the
/// clock, the debounced state file.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)] // a Bevy system: each input in turn
pub(super) fn drive(
    time: Res<Time>,
    mut lobby: ResMut<LobbyState>,
    prefs: Option<Res<crate::prefs::Prefs>>,
    mut host: ResMut<SocialHost>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    windows: Query<&Window>,
) {
    let host = &mut *host;
    let wanted = wanted_session(&lobby);
    if host.session != wanted
        && (!lobby.central_ready() || !host.rebind_central_gateway(wanted.as_ref()))
    {
        if host.session.is_some() {
            host.end();
        }
        host.session = wanted;
    }
    if let Some(prefs) = prefs.as_deref() {
        host.sync_prefs(&prefs.all().chat);
    }
    // Vault credentials are an invisible one-shot handoff. Mutating only that
    // field must not flag visible lobby state and rebuild its UI every frame.
    vault_access::discard_stale_login(lobby.bypass_change_detection());
    let Some(session) = host.session.clone() else {
        return;
    };
    if host.info == InfoState::Unknown {
        if session.token.is_empty() {
            let social = Some((baylee_protocol::social::SOCIAL_VERSION, 30));
            host.info = InfoState::Known(social);
            if host.machine.is_none() {
                let mut chat = prefs
                    .as_deref()
                    .map(|p| p.all().chat.clone())
                    .unwrap_or_default();
                if session.guest {
                    chat.notice_seen = true;
                }
                host.start(30, chat);
            }
        } else {
            host.info = InfoState::Asking;
            transport::ask_info(&host.mailbox, &session.gateway);
        }
    }
    let dt = time.delta_secs();
    let now = now_ms();
    let mut news = Vec::new();
    for arrived in host.mailbox.drain() {
        match arrived {
            Arrived::Info(social) => {
                host.info = InfoState::Known(social);
                if let Some((version, keep)) = social
                    && version == baylee_protocol::social::SOCIAL_VERSION
                    && host.machine.is_none()
                {
                    let chat = prefs
                        .as_deref()
                        .map(|p| p.all().chat.clone())
                        .unwrap_or_default();
                    host.start(keep, chat);
                }
            }
            Arrived::Http { tag, status, body } => {
                #[cfg(not(target_arch = "wasm32"))]
                if central::gateway_http(host, &tag, status, &body) {
                    continue;
                }
                if let Some(machine) = host.machine.as_mut() {
                    let effects = machine.on_http(tag, status, &body, now, &mut OsEntropy);
                    host.carry_out(effects);
                }
            }
            Arrived::Ticket(status, body) => {
                host.socket.ticket(
                    status,
                    &body,
                    &session.gateway,
                    &session.token,
                    &host.mailbox,
                    &mut news,
                );
            }
        }
    }
    vault_access::follow(host, lobby.bypass_change_detection());
    if host.machine.is_none() {
        return;
    }
    if !session.token.is_empty() {
        host.socket.pump(
            dt,
            &session.gateway,
            &session.token,
            &host.mailbox,
            &mut news,
        );
    }
    #[cfg(not(target_arch = "wasm32"))]
    central::tick(host, now);
    for item in news {
        #[cfg(not(target_arch = "wasm32"))]
        if central::gateway_news(host, &item) {
            continue;
        }
        let Some(machine) = host.machine.as_mut() else {
            break;
        };
        let effects = match item {
            LinkNews::Up => machine.on_link(SocialLink::Up, now),
            LinkNews::Down => machine.on_link(SocialLink::Down, now),
            LinkNews::Refused | LinkNews::Unauthorized => machine.on_link(SocialLink::Refused, now),
            LinkNews::Frame(text) => {
                if let Ok(frame) = serde_json::from_str::<baylee_protocol::social::Down>(&text) {
                    machine.on_frame(frame, now, &mut OsEntropy)
                } else {
                    debug!("the social socket sent something unreadable");
                    Vec::new()
                }
            }
        };
        host.carry_out(effects);
    }
    // Input resets the idle clock (auto-away after ten minutes, §6).
    if keys.get_just_pressed().next().is_some()
        || mouse.is_some_and(|m| m.get_just_pressed().next().is_some())
    {
        host.idle = 0.0;
    } else {
        host.idle += dt;
    }
    host.since_tick += dt;
    if host.since_tick >= TICK_SECS {
        host.since_tick = 0.0;
        let idle_ms = std::time::Duration::from_secs_f32(host.idle.max(0.0))
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX);
        let focused = windows.iter().any(|w| w.focused);
        if let Some(machine) = host.machine.as_mut() {
            machine.set_focus(focused);
            let effects = machine.tick(now, idle_ms);
            host.carry_out(effects);
        }
    }
    if let Some(machine) = host.machine.as_mut()
        && machine.wants_entropy()
    {
        let effects = machine.flush(now, &mut OsEntropy);
        host.carry_out(effects);
    }
    if let Some((left, _)) = host.save.as_mut() {
        *left -= dt;
        if *left <= 0.0 {
            host.flush_save();
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(upto) = host.vault.ready_ack()
        && let Ok(text) = serde_json::to_string(&baylee_protocol::social::Up::Ack { upto })
        && host.socket.send(text)
    {
        host.vault.sent_ack(upto);
    }
}
