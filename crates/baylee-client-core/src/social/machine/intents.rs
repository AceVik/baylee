//! What the player asks for: intents, the composer's submit, and the slash
//! commands it runs (§10.2, §12.2).

use baylee_protocol::social::{ChatReport, Conv, InviteAnswer, InviteDecline, InviteKind, Person};
use baylee_seal::Entropy;

use super::super::commands::{self, Command, CommandFault, GroupRef, Target};
use super::super::convs::{ConvId, ConvKind, LineState};
use super::super::table::tag_of;
use super::super::words::ChatPhrase;
use super::super::{Effect, HttpTag, Intent, Method};
use super::{Social, http, segment};

impl Social {
    #[allow(clippy::too_many_lines)] // one arm per intent, read top to bottom
    pub(super) fn intent(
        &mut self,
        intent: Intent,
        now_ms: u64,
        rng: &mut dyn Entropy,
        out: &mut Vec<Effect>,
    ) {
        let guest_refused = |social: &mut Self| {
            if social.me.guest {
                social.answer(ChatPhrase::GuestsCannot, Vec::new(), now_ms);
                true
            } else {
                false
            }
        };
        match intent {
            Intent::AddFriend { handle } => {
                if guest_refused(self) {
                    return;
                }
                out.push(http(
                    HttpTag::AddFriend(handle.clone()),
                    Method::Post,
                    "/social/friends/requests".into(),
                    Some(serde_json::json!({ "handle": handle })),
                ));
            }
            Intent::Accept { tag } => out.push(friendship(&tag, Method::Post, "/accept", &tag)),
            Intent::Decline { tag } => out.push(friendship(&tag, Method::Post, "/decline", &tag)),
            Intent::Cancel { tag } => out.push(friendship(&tag, Method::Delete, "", &tag)),
            Intent::Unfriend { tag } => out.push(http(
                HttpTag::Friendship(tag.clone()),
                Method::Delete,
                format!("/social/friends/{}", segment(&tag)),
                None,
            )),
            Intent::Block { tag } => out.push(http(
                HttpTag::Friendship(tag.clone()),
                Method::Post,
                "/social/blocks".into(),
                Some(serde_json::json!({ "tag": tag })),
            )),
            Intent::Unblock { tag } => out.push(http(
                HttpTag::Friendship(tag.clone()),
                Method::Delete,
                format!("/social/blocks/{}", segment(&tag)),
                None,
            )),
            Intent::Ignore { tag } => {
                self.state.ignored.insert(tag.clone());
                self.set_hidden(&tag, true);
                self.dirty = true;
            }
            Intent::Unignore { tag } => {
                self.state.ignored.remove(&tag);
                self.set_hidden(&tag, false);
                self.dirty = true;
            }
            Intent::MuteConv(id) => self.mute(&id, true),
            Intent::UnmuteConv(id) => self.mute(&id, false),
            Intent::MarkRead(id) => {
                if let Some(conv) = self.convs.iter_mut().find(|c| c.id == id) {
                    conv.unread = 0;
                }
            }
            Intent::CloseConv(id) => {
                let seated = self.table.as_ref().is_some_and(|t| {
                    t.table_conv().as_ref() == Some(&id) || t.team_conv().as_ref() == Some(&id)
                });
                if !seated && let Some(conv) = self.convs.iter_mut().find(|c| c.id == id) {
                    conv.open = false;
                    conv.unread = 0;
                }
                if self.active.as_ref() == Some(&id) {
                    self.active = None;
                }
            }
            Intent::OpenConv(id) => self.open_conv(&id, now_ms, out),
            Intent::Whisper { tag } => {
                if let Some(id) = Conv::dm(&self.me.tag, &tag) {
                    self.open_conv(&id, now_ms, out);
                }
            }
            Intent::Panel(Some(id)) => {
                self.panel_open = true;
                self.switch_tab(&id);
                if let Some(conv) = self.convs.iter_mut().find(|c| c.id == id) {
                    conv.unread = 0;
                }
            }
            Intent::Panel(None) => {
                self.panel_open = false;
            }
            Intent::NewGroup { name } => {
                if guest_refused(self) {
                    return;
                }
                out.push(http(
                    HttpTag::Group,
                    Method::Post,
                    "/social/groups".into(),
                    Some(serde_json::json!({ "name": name })),
                ));
            }
            Intent::RenameGroup { id, name } => out.push(http(
                HttpTag::Group,
                Method::Patch,
                format!("/social/groups/{}", segment(&id)),
                Some(serde_json::json!({ "name": name })),
            )),
            Intent::InviteToGroup { id, tag } => out.push(http(
                HttpTag::Group,
                Method::Post,
                format!("/social/groups/{}/invites", segment(&id)),
                Some(serde_json::json!({ "tag": tag })),
            )),
            Intent::JoinGroup { id } => out.push(http(
                HttpTag::Group,
                Method::Post,
                format!("/social/groups/{}/join", segment(&id)),
                None,
            )),
            Intent::DeclineGroup { id } => {
                let me = self.me.tag.clone();
                out.push(http(
                    HttpTag::Group,
                    Method::Delete,
                    format!("/social/groups/{}/invites/{}", segment(&id), segment(&me)),
                    None,
                ));
            }
            Intent::LeaveGroup { id } => {
                let me = self.me.tag.clone();
                out.push(http(
                    HttpTag::Group,
                    Method::Delete,
                    format!("/social/groups/{}/members/{}", segment(&id), segment(&me)),
                    None,
                ));
            }
            Intent::Kick { id, tag } => out.push(http(
                HttpTag::Group,
                Method::Delete,
                format!("/social/groups/{}/members/{}", segment(&id), segment(&tag)),
                None,
            )),
            Intent::HandOver { id, tag } => out.push(http(
                HttpTag::Group,
                Method::Post,
                format!("/social/groups/{}/owner", segment(&id)),
                Some(serde_json::json!({ "tag": tag })),
            )),
            Intent::DeleteGroup { id } => out.push(http(
                HttpTag::Group,
                Method::Delete,
                format!("/social/groups/{}", segment(&id)),
                None,
            )),
            Intent::InviteToTable { tag } => {
                if guest_refused(self) {
                    return;
                }
                let Some(table) = self.table.as_ref().filter(|t| !t.watching) else {
                    self.answer(ChatPhrase::NoTable, Vec::new(), now_ms);
                    return;
                };
                out.push(http(
                    HttpTag::TableInvite(tag.clone()),
                    Method::Post,
                    "/social/invites".into(),
                    Some(serde_json::json!({ "tag": tag, "game": table.game })),
                ));
            }
            Intent::AnswerInvite { id, accept } => {
                let held = self
                    .contacts
                    .invites
                    .iter()
                    .find(|i| i.invite.id == id)
                    .cloned();
                self.contacts.invites.retain(|i| i.invite.id != id);
                if accept {
                    if let Some(held) = held {
                        out.push(match held.invite.kind {
                            InviteKind::Join => Effect::JoinTable {
                                game: held.invite.game,
                                invite: Some(id),
                            },
                            InviteKind::Watch => Effect::WatchTable {
                                game: held.invite.game,
                            },
                        });
                    }
                } else {
                    out.push(http(
                        HttpTag::AnswerInvite(id.clone()),
                        Method::Delete,
                        format!("/social/invites/{}", segment(&id)),
                        Some(
                            serde_json::to_value(InviteDecline {
                                answer: InviteAnswer::Declined,
                            })
                            .unwrap_or_default(),
                        ),
                    ));
                }
            }
            Intent::Watch { tag } => {
                let game = self
                    .contacts
                    .friend(&tag)
                    .and_then(|f| match &f.presence.place {
                        Some(baylee_protocol::social::Place::Playing { game, .. }) => game.clone(),
                        _ => None,
                    });
                if let Some(game) = game {
                    out.push(Effect::WatchTable { game });
                } else {
                    self.answer(ChatPhrase::ToastRefused, Vec::new(), now_ms);
                }
            }
            Intent::SetStatus(status) => {
                self.status = status;
                self.auto_away = false;
                self.state.status = Some(status);
                self.dirty = true;
                out.extend(self.presence_effects());
            }
            Intent::SetSettings(settings) => {
                if guest_refused(self) {
                    return;
                }
                self.contacts.settings = settings;
                out.push(http(
                    HttpTag::Settings,
                    Method::Put,
                    "/social/settings".into(),
                    serde_json::to_value(settings).ok(),
                ));
            }
            Intent::SetPrefs(prefs) => {
                let shorter = prefs.history_days < self.prefs.history_days;
                self.prefs = prefs;
                if shorter {
                    // Pruned on the next start, as at every start (§10.2).
                    self.dirty = true;
                }
            }
            Intent::Verify { tag } => {
                self.state.keys.verify(&tag);
                self.dirty = true;
            }
            Intent::Unverify { tag } => {
                self.state.keys.unverify(&tag);
                self.dirty = true;
            }
            Intent::RemoveDevice { id } => out.push(http(
                HttpTag::RemoveDevice(id.clone()),
                Method::Delete,
                format!("/social/devices/{}", segment(&id)),
                None,
            )),
            Intent::Nickname { tag, name } => {
                match name.map(|n| super::super::text::clean_name(n.trim())) {
                    Some(n) if !n.is_empty() => {
                        self.state.nicknames.insert(tag, n);
                    }
                    _ => {
                        self.state.nicknames.remove(&tag);
                    }
                }
                self.dirty = true;
                self.refresh_titles();
            }
            Intent::Report { line, note } => {
                let Some(Some(evidence)) = self.evidence.get(&line) else {
                    return;
                };
                let report = ChatReport {
                    conv: line.conv.to_string(),
                    sealed: evidence.sealed.clone(),
                    mk: evidence.mk.clone(),
                    body: evidence.body.clone(),
                    note: note
                        .chars()
                        .take(baylee_protocol::social::MAX_REPORT_NOTE)
                        .collect(),
                };
                out.push(http(
                    HttpTag::Report(line),
                    Method::Post,
                    "/social/reports".into(),
                    serde_json::to_value(report).ok(),
                ));
            }
            Intent::Submit => self.submit(now_ms, rng, out),
            Intent::Retry { line } => {
                let failed = self
                    .conversation(&line.conv)
                    .and_then(|c| c.line(&line.id))
                    .filter(|l| l.state == LineState::Failed)
                    .and_then(|l| l.body().cloned());
                if let Some(body) = failed {
                    if let Some(conv) = self.convs.iter_mut().find(|c| c.id == line.conv) {
                        conv.lines.retain(|l| l.key != line);
                    }
                    self.send(&line.conv, body, now_ms, rng, out);
                }
            }
            Intent::Forget { line } => {
                if let Some(conv) = self.convs.iter_mut().find(|c| c.id == line.conv) {
                    conv.lines
                        .retain(|l| !(l.key == line && l.state == LineState::Failed));
                }
            }
            Intent::NoticeSeen => self.prefs.notice_seen = true,
        }
    }

    /// Enter in the composer: run the command, or send the message.
    pub(super) fn submit(&mut self, now_ms: u64, rng: &mut dyn Entropy, out: &mut Vec<Effect>) {
        let text = self.composer.text().trim().to_string();
        if text.is_empty() {
            return;
        }
        let command = match commands::parse(&text) {
            Ok(command) => command,
            Err(fault) => {
                let (phrase, args) = fault.said();
                self.answer(phrase, args, now_ms);
                return;
            }
        };
        let leave = matches!(command, Command::Leave);
        let sent = self.run(command, now_ms, rng, out);
        if !leave {
            self.leave_armed = None;
        }
        match sent {
            Ran::Sent => self.composer.sent(),
            Ran::Done => self.composer.clear(),
            Ran::Kept => {}
        }
    }

    #[allow(clippy::too_many_lines)]
    fn run(
        &mut self,
        command: Command,
        now_ms: u64,
        rng: &mut dyn Entropy,
        out: &mut Vec<Effect>,
    ) -> Ran {
        match command {
            Command::Say { to, text, emote } => self.say(&to, &text, emote, now_ms, rng, out),
            Command::Reply(text) => {
                if let Some(tag) = self.state.last_whisper.clone() {
                    let handle = self.person_of(&tag).handle;
                    self.say(&Target::Whisper(handle), &text, false, now_ms, rng, out)
                } else {
                    self.answer(ChatPhrase::NoReply, Vec::new(), now_ms);
                    Ran::Kept
                }
            }
            Command::Whisper { who, text } => {
                let Some(person) = self.resolve_name(&who, true, now_ms) else {
                    return Ran::Kept;
                };
                self.say(
                    &Target::Whisper(person.handle),
                    &text,
                    false,
                    now_ms,
                    rng,
                    out,
                )
            }
            Command::Friend(who) => {
                let handle = if who.contains('#') {
                    Some(who)
                } else {
                    self.resolve_name(&who, false, now_ms).map(|p| p.handle)
                };
                if let Some(handle) = handle {
                    self.intent(Intent::AddFriend { handle }, now_ms, rng, out);
                    Ran::Done
                } else {
                    Ran::Kept
                }
            }
            Command::Unfriend(who) => {
                self.on_name(&who, now_ms, rng, out, |tag| Intent::Unfriend { tag })
            }
            Command::Block(who) => {
                self.on_name(&who, now_ms, rng, out, |tag| Intent::Block { tag })
            }
            Command::Unblock(who) => {
                // A blocked player is found among the blocked, too.
                let blocked = self
                    .contacts
                    .blocked
                    .iter()
                    .find(|p| {
                        p.handle.eq_ignore_ascii_case(&who) || Some(p.tag.as_str()) == tag_of(&who)
                    })
                    .map(|p| p.tag.clone());
                if let Some(tag) = blocked {
                    self.intent(Intent::Unblock { tag }, now_ms, rng, out);
                    Ran::Done
                } else {
                    self.on_name(&who, now_ms, rng, out, |tag| Intent::Unblock { tag })
                }
            }
            Command::Ignore(who) => {
                let Some(person) = self.resolve_name(&who, false, now_ms) else {
                    return Ran::Kept;
                };
                let name = self.shown_name(&person);
                self.intent(Intent::Ignore { tag: person.tag }, now_ms, rng, out);
                self.answer(ChatPhrase::Ignoring, vec![name], now_ms);
                Ran::Done
            }
            Command::Unignore(who) => {
                let Some(person) = self.resolve_name(&who, false, now_ms) else {
                    return Ran::Kept;
                };
                let name = self.shown_name(&person);
                self.intent(Intent::Unignore { tag: person.tag }, now_ms, rng, out);
                self.answer(ChatPhrase::NotIgnoring, vec![name], now_ms);
                Ran::Done
            }
            Command::Invite(who) => {
                self.on_name(&who, now_ms, rng, out, |tag| Intent::InviteToTable { tag })
            }
            Command::Watch(who) => {
                self.on_name(&who, now_ms, rng, out, |tag| Intent::Watch { tag })
            }
            Command::Status(status) => {
                self.intent(Intent::SetStatus(status), now_ms, rng, out);
                Ran::Done
            }
            Command::Clear => {
                if let Some(id) = self.active.clone()
                    && let Some(conv) = self.convs.iter_mut().find(|c| c.id == id)
                {
                    conv.lines.clear();
                    conv.unread = 0;
                    self.notice(&id, ChatPhrase::Cleared, Vec::new(), now_ms);
                }
                Ran::Done
            }
            Command::Mute | Command::Unmute => {
                let muting = matches!(command, Command::Mute);
                if let Some(id) = self.active.clone() {
                    self.mute(&id, muting);
                    let phrase = if muting {
                        ChatPhrase::Muted
                    } else {
                        ChatPhrase::Unmuted
                    };
                    self.notice(&id, phrase, Vec::new(), now_ms);
                }
                Ran::Done
            }
            Command::Who => {
                self.who(now_ms);
                Ran::Done
            }
            Command::Help(topic) => {
                if let Some(phrase) = topic.as_deref().and_then(commands::help_for) {
                    self.answer(phrase, Vec::new(), now_ms);
                } else {
                    self.answer(ChatPhrase::HelpIntro, Vec::new(), now_ms);
                    for info in commands::COMMANDS {
                        self.answer(info.help, Vec::new(), now_ms);
                    }
                }
                Ran::Done
            }
            Command::Leave => {
                let group = match &self.active {
                    Some(Conv::Group { id }) => Some(id.clone()),
                    _ => None,
                };
                let Some(id) = group else {
                    self.answer(ChatPhrase::NoGroup, Vec::new(), now_ms);
                    return Ran::Done;
                };
                if self.leave_armed.as_deref() == Some(id.as_str()) {
                    self.leave_armed = None;
                    self.intent(Intent::LeaveGroup { id }, now_ms, rng, out);
                } else {
                    let name = self
                        .contacts
                        .group(&id)
                        .map(|g| g.name.clone())
                        .unwrap_or_default();
                    self.leave_armed = Some(id);
                    self.answer(ChatPhrase::LeaveConfirm, vec![name], now_ms);
                }
                Ran::Done
            }
            Command::Friends => {
                out.push(Effect::OpenDrawer);
                Ran::Done
            }
            Command::Nick { who, name } => {
                let Some(person) = self.resolve_name(&who, false, now_ms) else {
                    return Ran::Kept;
                };
                let handle = person.handle.clone();
                match &name {
                    Some(nick) => {
                        self.answer(ChatPhrase::NickSet, vec![handle, nick.clone()], now_ms);
                    }
                    None => self.answer(ChatPhrase::NickCleared, vec![handle], now_ms),
                }
                self.intent(
                    Intent::Nickname {
                        tag: person.tag,
                        name,
                    },
                    now_ms,
                    rng,
                    out,
                );
                Ran::Done
            }
        }
    }

    fn on_name(
        &mut self,
        who: &str,
        now_ms: u64,
        rng: &mut dyn Entropy,
        out: &mut Vec<Effect>,
        make: impl FnOnce(String) -> Intent,
    ) -> Ran {
        match self.resolve_name(who, false, now_ms) {
            Some(person) => {
                self.intent(make(person.tag), now_ms, rng, out);
                Ran::Done
            }
            None => Ran::Kept,
        }
    }

    /// Sends `text` to `to`; with an empty text, makes `to` the composer's
    /// target instead.
    fn say(
        &mut self,
        to: &Target,
        text: &str,
        emote: bool,
        now_ms: u64,
        rng: &mut dyn Entropy,
        out: &mut Vec<Effect>,
    ) -> Ran {
        let conv = match self.resolve(to) {
            Ok(conv) => conv,
            Err(fault) => {
                let (phrase, args) = fault.said();
                self.answer(phrase, args, now_ms);
                return Ran::Kept;
            }
        };
        if text.is_empty() {
            if !matches!(to, Target::Sticky) {
                self.composer.set_target(to.clone());
            }
            self.open_conv(&conv, now_ms, out);
            return Ran::Done;
        }
        let body = self.composer.body_of(text, emote);
        if !matches!(to, Target::Sticky) && !matches!(to, Target::Whisper(_)) {
            // `/t`, `/team`, `/g`: WoW keeps the channel for the next line.
            self.composer.set_target(to.clone());
        }
        self.send(&conv, body, now_ms, rng, out);
        Ran::Sent
    }

    /// The conversation a target names here.
    pub(super) fn resolve(&self, target: &Target) -> Result<ConvId, CommandFault> {
        match target {
            Target::Table => self
                .table
                .as_ref()
                .and_then(super::TableContext::table_conv)
                .ok_or(CommandFault::NotHere(Target::Table)),
            Target::Team => self
                .table
                .as_ref()
                .and_then(super::TableContext::team_conv)
                .ok_or(CommandFault::NotHere(Target::Team)),
            Target::Group(GroupRef::Number(n)) => self
                .contacts
                .groups
                .get(usize::from(*n).saturating_sub(1))
                .and_then(|g| format!("grp:{}", g.id).parse().ok())
                .ok_or(CommandFault::NotHere(target.clone())),
            Target::Group(GroupRef::Current) => {
                let id = match &self.active {
                    Some(Conv::Group { id }) => Some(id.clone()),
                    _ => self.last_group.clone(),
                };
                id.filter(|id| self.contacts.group(id).is_some())
                    .and_then(|id| format!("grp:{id}").parse().ok())
                    .ok_or(CommandFault::NotHere(target.clone()))
            }
            Target::Whisper(name) => {
                let tag = tag_of(name)
                    .map(str::to_string)
                    .or_else(|| {
                        self.known_people()
                            .into_iter()
                            .find(|p| p.handle == *name)
                            .map(|p| p.tag)
                    })
                    .ok_or(CommandFault::NotHere(target.clone()))?;
                Conv::dm(&self.me.tag, &tag).ok_or(CommandFault::NotHere(target.clone()))
            }
            Target::Sticky => match self.composer.target() {
                Target::Sticky => self
                    .active
                    .clone()
                    .filter(|a| self.members(a).is_some())
                    .or_else(|| {
                        self.table
                            .as_ref()
                            .and_then(super::TableContext::table_conv)
                    })
                    .ok_or(CommandFault::NotHere(Target::Table)),
                other => self.resolve(&other.clone()),
            },
        }
    }

    /// A name typed in a command (§12.2): a handle, `#tag`, a nickname, or
    /// a display name unique among friends, whisper partners and the
    /// roster. An ambiguous or unknown name says so and resolves to
    /// nothing. With `any_handle`, a full handle names its account even if
    /// it is nobody known (the gateway decides whether it may be written
    /// to).
    pub(super) fn resolve_name(
        &mut self,
        who: &str,
        any_handle: bool,
        now_ms: u64,
    ) -> Option<Person> {
        let who = who.trim();
        let known = self.known_people();
        if let Some(tag) = tag_of(who) {
            if let Some(person) = known.iter().find(|p| p.tag == tag) {
                return Some(person.clone());
            }
            if any_handle || who.starts_with('#') {
                return Some(Person {
                    tag: tag.to_string(),
                    handle: who.to_string(),
                });
            }
        }
        let lower = who.to_lowercase();
        if let Some((tag, _)) = self
            .state
            .nicknames
            .iter()
            .find(|(_, nick)| nick.to_lowercase() == lower)
        {
            let tag = tag.clone();
            return Some(self.person_of(&tag));
        }
        let matching: Vec<&Person> = known
            .iter()
            .filter(|p| {
                let name = p
                    .handle
                    .rsplit_once('#')
                    .map_or(p.handle.as_str(), |(n, _)| n);
                name.to_lowercase() == lower || p.handle.to_lowercase() == lower
            })
            .collect();
        match matching.as_slice() {
            [one] => Some((*one).clone()),
            [] => {
                self.answer(ChatPhrase::NoSuchPlayer, vec![who.to_string()], now_ms);
                None
            }
            many => {
                let list = many
                    .iter()
                    .map(|p| p.handle.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                self.answer(
                    ChatPhrase::AmbiguousName,
                    vec![who.to_string(), list],
                    now_ms,
                );
                None
            }
        }
    }

    fn who(&mut self, now_ms: u64) {
        let Some(id) = self.active.clone() else {
            self.answer(ChatPhrase::WhoNobody, Vec::new(), now_ms);
            return;
        };
        let tags = self.members(&id).unwrap_or_default();
        let others: Vec<String> = tags.into_iter().filter(|t| *t != self.me.tag).collect();
        if others.is_empty() {
            self.answer(ChatPhrase::WhoNobody, Vec::new(), now_ms);
            return;
        }
        for tag in others {
            let person = self.person_of(&tag);
            let devices = self.state.keys.get(&tag).map_or(0, |k| k.devices.len());
            let name = self.shown_name(&person);
            self.answer(ChatPhrase::WhoLine, vec![name, devices.to_string()], now_ms);
        }
    }

    fn open_conv(&mut self, id: &ConvId, now_ms: u64, out: &mut Vec<Effect>) {
        let conv = self.conv_entry(id, now_ms);
        conv.open = true;
        self.switch_tab(id);
        out.push(Effect::OpenConversation(id.clone()));
    }

    /// Shows `id`'s tab: the composer keeps the old tab's draft, takes the
    /// new one's, and writes there.
    fn switch_tab(&mut self, id: &ConvId) {
        if self.active.as_ref() == Some(id) {
            return;
        }
        if let Some(old) = self.active.clone() {
            self.composer.stash_draft(&old);
        }
        self.composer.restore_draft(id);
        self.active = Some(id.clone());
        let target = match id {
            Conv::Table { .. } => Target::Table,
            Conv::Team { .. } => Target::Team,
            Conv::Dm { .. } => id.partner(&self.me.tag).map_or(Target::Sticky, |t| {
                Target::Whisper(self.person_of(t).handle)
            }),
            Conv::Group { id } => self
                .contacts
                .groups
                .iter()
                .position(|g| g.id == *id)
                .filter(|n| *n < 9)
                .map_or(Target::Sticky, |n| {
                    Target::Group(GroupRef::Number(n as u8 + 1))
                }),
        };
        // At a table a whisper arriving doesn't move the chip (§11.6); a
        // tab the player chose does.
        self.composer.set_target(target);
    }

    fn mute(&mut self, id: &ConvId, muted: bool) {
        let text = id.to_string();
        if muted {
            self.state.muted.insert(text);
        } else {
            self.state.muted.remove(&text);
        }
        if let Some(conv) = self.convs.iter_mut().find(|c| c.id == *id) {
            conv.muted = muted;
        }
        self.dirty = true;
    }

    fn set_hidden(&mut self, tag: &str, hidden: bool) {
        for conv in &mut self.convs {
            for line in &mut conv.lines {
                if line.from_tag() == Some(tag) {
                    line.hidden = hidden;
                }
            }
        }
    }

    fn refresh_titles(&mut self) {
        let titles: Vec<(ConvId, String)> = self
            .convs
            .iter()
            .filter(|c| c.kind() == ConvKind::Whisper)
            .map(|c| (c.id.clone(), self.title_of(&c.id)))
            .collect();
        for (id, title) in titles {
            if let Some(conv) = self.convs.iter_mut().find(|c| c.id == id) {
                conv.title = title;
            }
        }
    }
}

/// What a submitted line did.
enum Ran {
    /// A message went (remember it for `↑`).
    Sent,
    /// A command ran (empty the box).
    Done,
    /// Nothing ran (keep what was typed, to fix it).
    Kept,
}

fn friendship(tag: &str, method: Method, suffix: &str, label: &str) -> Effect {
    http(
        HttpTag::Friendship(label.to_string()),
        method,
        format!("/social/friends/requests/{}{suffix}", segment(tag)),
        None,
    )
}
