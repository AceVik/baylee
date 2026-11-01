//! Receiving: deliveries opened, acknowledged or held, and invites.

#[allow(clippy::wildcard_imports)] // the machine's own vocabulary
use super::*;

impl Social {
    // ---- receiving ----

    #[allow(clippy::too_many_arguments)] // matches Down::Global fields + machine clock/rng/out
    pub(super) fn receive_global(
        &mut self,
        id: &str,
        at: u64,
        from: Person,
        raw_body: &str,
        now_ms: u64,
        rng: &mut dyn Entropy,
        out: &mut Vec<Effect>,
    ) {
        if self.state.seen.contains_key(id)
            || now_ms.saturating_sub(at) > MAX_AGE_MS
            || sealing::uuid_ms(id).is_none()
        {
            return;
        }
        let Some(body) = serde_json::from_str::<Body>(raw_body)
            .ok()
            .and_then(Body::checked)
        else {
            return;
        };
        self.state.seen.insert(id.to_string(), now_ms);
        self.dirty = true;
        if self.contacts.is_blocked(&from.tag) {
            return;
        }
        let conv = Conv::Global;
        let mine = from.tag == self.me.tag;
        let from = Person {
            tag: from.tag,
            handle: super::super::text::clean_name(&from.handle),
        };
        let mentions_me = !mine && body.mentions_tag(&self.me.tag);
        let ignored = self.state.ignored.contains(&from.tag);
        let key = LineKey {
            conv: conv.clone(),
            id: id.to_string(),
        };
        let reading = self.panel_open && self.active.as_ref() == Some(&conv);
        let focused = self.focused;
        let entry = self.conv_entry(&conv, now_ms);
        entry.open = true;
        let muted = entry.muted;
        entry.insert(ChatLine {
            key,
            at,
            kind: LineKind::Message {
                from: from.clone(),
                mine,
                body: body.clone(),
            },
            state: LineState::Final,
            hidden: ignored,
            mentions_me,
            failure: None,
        });
        if !(mine || (reading && focused) || ignored) {
            entry.unread += 1;
        }
        let record = Record {
            id: id.to_string(),
            at,
            from: from.clone(),
            mine,
            body,
            evidence: None,
        };
        self.append_history(&conv, &record, rng, out);
        if !mine {
            let ctx = NotifyContext {
                reading,
                dnd: self.status == Status::Dnd,
                focused: self.focused,
                question_up: false,
                mention: mentions_me,
                at_table: self.table.is_some(),
            };
            let notify = notify::on_line(ConvKind::Global, muted, ignored, &self.prefs, ctx);
            if notify.toast {
                let name = self.shown_name(&from);
                out.push(Effect::Toast(ChatToast {
                    phrase: ChatPhrase::ToastMention,
                    args: vec![name],
                    conv: Some(conv),
                    actions: Vec::new(),
                    until_answered: false,
                    about: None,
                }));
            }
            if let Some(cue) = notify.sound {
                out.push(Effect::Sound(cue));
            }
        }
    }

    pub(super) fn receive(
        &mut self,
        msg: Held,
        retried: bool,
        now_ms: u64,
        rng: &mut dyn Entropy,
        out: &mut Vec<Effect>,
    ) {
        let seq = msg.seq;
        if seq <= self.acked && !retried {
            // Already handled and acked: an at-least-once redelivery.
            return;
        }
        let verdict = self.open_delivery(&msg, retried, now_ms, rng, out);
        match verdict {
            Verdict::Hold => {
                if !self.held.iter().any(|h| h.seq == seq) {
                    let bytes = self.held.iter().fold(msg.bytes(), |total, entry| {
                        total.saturating_add(entry.bytes())
                    });
                    if self.held.len() >= MAX_HELD_DELIVERIES || bytes > MAX_HELD_BYTES {
                        self.held_paused = true;
                        self.link = Link::Stopped;
                        out.push(Effect::Hang);
                        return;
                    }
                    self.held.push(msg);
                }
                return;
            }
            Verdict::Done => {}
        }
        self.held.retain(|h| h.seq != seq);
        self.high = self.high.max(seq);
    }

    pub(super) fn ack(&mut self, out: &mut Vec<Effect>) {
        // Hosts must persist history and this device's replay-suppression state
        // before allowing the gateway to discard a delivery. Central hosts also
        // await the encrypted archive's CAS before executing this final effect.
        if self.dirty || self.removed || self.held_paused {
            return;
        }
        let safe = match self.held.iter().map(|h| h.seq).min() {
            Some(lowest) => lowest.saturating_sub(1).min(self.high),
            None => self.high,
        };
        if safe > self.acked {
            self.acked = safe;
            out.push(Effect::Up(Up::Ack { upto: safe }));
        }
    }

    pub(super) fn retry_held(&mut self, now_ms: u64, rng: &mut dyn Entropy, out: &mut Vec<Effect>) {
        // Every held delivery stays in `held` until it is done, so an ack
        // never passes one still waiting (oldest first, as they came).
        self.held.sort_by_key(|h| h.seq);
        let held = self.held.clone();
        for msg in held {
            if !self.fetching.contains(&msg.from.tag) {
                self.receive(msg, true, now_ms, rng, out);
            }
        }
    }

    #[allow(clippy::too_many_lines)]
    fn open_delivery(
        &mut self,
        msg: &Held,
        retried: bool,
        now_ms: u64,
        rng: &mut dyn Entropy,
        out: &mut Vec<Effect>,
    ) -> Verdict {
        let Ok(conv) = msg.conv.parse::<Conv>() else {
            return Verdict::Done;
        };
        // A notice goes only where the sender may write: a forged delivery
        // must not open a tab (or a state entry) for a conversation this
        // account isn't in.
        let member = self.may_hear(&conv, &msg.from.tag, now_ms);
        let decoded = match Decoded::read(&msg.conv, &msg.from, &msg.sealed) {
            Ok(decoded) => decoded,
            Err(fault) => {
                if member {
                    let phrase = if fault == OpenFault::Mismatch {
                        ChatPhrase::MisaddressedDropped
                    } else {
                        ChatPhrase::MessageDropped
                    };
                    self.notice(&conv, phrase, Vec::new(), now_ms);
                }
                return Verdict::Done;
            }
        };
        if !member || self.contacts.is_blocked(&msg.from.tag) {
            return Verdict::Done;
        }
        // Known directory changes cannot authenticate a new incoming identity.
        // Retain the previous pins while confirmation is pending, so legitimate
        // old inbox deliveries still open without accepting injected candidates.
        let contact = self.state.keys.get(&msg.from.tag);
        let device = self
            .state
            .keys
            .get(&msg.from.tag)
            .and_then(|k| k.pinned_device(&msg.from.device))
            .filter(|device| {
                msg.from.tag != self.me.tag
                    || self
                        .trusted_own_devices
                        .as_ref()
                        .is_none_or(|keys| keys.contains(&(device.ik, device.dk)))
            })
            .cloned();
        let Some(device) = device else {
            if contact.is_some_and(|keys| {
                keys.device(&msg.from.device).is_some()
                    || keys.pinned_device(&msg.from.device).is_some()
            }) {
                return Verdict::Hold;
            }
            if !retried && !self.refetched.contains(&msg.from.tag) {
                self.refetched.insert(msg.from.tag.clone());
                self.request_keys(std::slice::from_ref(&msg.from.tag), out);
                return Verdict::Hold;
            }
            if self.fetching.contains(&msg.from.tag) {
                return Verdict::Hold;
            }
            if member {
                let name = self.shown_name(&self.person_of(&msg.from.tag));
                self.notice(&conv, ChatPhrase::UnknownDevice, vec![name], now_ms);
            }
            return Verdict::Done;
        };
        let Ok(opened) = decoded.open(&self.device, &device) else {
            if member {
                self.notice(&conv, ChatPhrase::MessageDropped, Vec::new(), now_ms);
            }
            return Verdict::Done;
        };
        let header = &decoded.header;
        // §4.2 step 4: a repeat, or a message too old.
        if self.state.seen.contains_key(&header.id)
            || now_ms.saturating_sub(header.ts) > MAX_AGE_MS
            || sealing::uuid_ms(&header.id).is_none()
        {
            return Verdict::Done;
        }
        // Step 5: the body.
        let Some(body) = opened.body.clone().checked() else {
            if member {
                self.notice(&conv, ChatPhrase::MessageDropped, Vec::new(), now_ms);
            }
            return Verdict::Done;
        };
        // Step 6: membership — what stops a forwarded message.
        if !member {
            return Verdict::Done;
        }
        self.state.seen.insert(header.id.clone(), now_ms);
        self.dirty = true;
        if self.contacts.is_blocked(&msg.from.tag) {
            return Verdict::Done;
        }
        let mine = msg.from.tag == self.me.tag;
        let from = Person {
            tag: msg.from.tag.clone(),
            handle: super::super::text::clean_name(&msg.from.handle),
        };
        let mentions_me = !mine && body.mentions_tag(&self.me.tag);
        let ignored = self.state.ignored.contains(&from.tag);
        let evidence = (!mine).then(|| Evidence {
            sealed: Delivered {
                hdr: msg.sealed.hdr.clone(),
                n: msg.sealed.n.clone(),
                ct: msg.sealed.ct.clone(),
                epk: msg.sealed.epk.clone(),
                sig: msg.sealed.sig.clone(),
                k: msg.sealed.k.clone(),
            },
            mk: url::encode(&opened.mk[..]),
            body: url::encode(&opened.plain),
        });
        let key = LineKey {
            conv: conv.clone(),
            id: header.id.clone(),
        };
        if let Some(partner) = conv.partner(&self.me.tag)
            && !mine
        {
            self.state.last_whisper = Some(partner.to_string());
            self.state.whisperers.retain(|p| p.tag != from.tag);
            self.state.whisperers.insert(0, from.clone());
            self.state.whisperers.truncate(RECENT_KEPT);
        }
        let kind = ConvKind::of(&conv);
        let reading = self.panel_open && self.active.as_ref() == Some(&conv);
        let focused = self.focused;
        let entry = self.conv_entry(&conv, now_ms);
        entry.open = true;
        let muted = entry.muted;
        entry.insert(ChatLine {
            key: key.clone(),
            at: msg.at,
            kind: LineKind::Message {
                from: from.clone(),
                mine,
                body: body.clone(),
            },
            state: LineState::Final,
            hidden: ignored,
            mentions_me,
            failure: None,
        });
        if !(mine || (reading && focused) || ignored) {
            entry.unread += 1;
        }
        self.evidence.insert(key.clone(), evidence.clone());
        let record = Record {
            id: header.id.clone(),
            at: msg.at,
            from: from.clone(),
            mine,
            body: body.clone(),
            evidence,
        };
        self.append_history(&conv, &record, rng, out);
        if !mine {
            let ctx = NotifyContext {
                reading,
                dnd: self.status == Status::Dnd,
                focused: self.focused,
                question_up: false,
                mention: mentions_me,
                at_table: self.table.is_some(),
            };
            let notify = notify::on_line(kind, muted, ignored, &self.prefs, ctx);
            if notify.toast {
                let name = self.shown_name(&from);
                let (phrase, args) = if mentions_me {
                    (ChatPhrase::ToastMention, vec![name])
                } else if self.prefs.toast_text {
                    (ChatPhrase::ToastWhisper, vec![name, preview(&body.text)])
                } else {
                    (ChatPhrase::ToastWhisperQuiet, vec![name])
                };
                out.push(Effect::Toast(ChatToast {
                    phrase,
                    args,
                    conv: Some(conv.clone()),
                    actions: Vec::new(),
                    until_answered: false,
                    about: None,
                }));
            }
            if let Some(cue) = notify.sound {
                out.push(Effect::Sound(cue));
            }
        }
        Verdict::Done
    }

    /// §4.2 step 6: `conv` is a conversation this account is in now (or a
    /// table that ended within the hour), and `from` is a member of it.
    fn may_hear(&self, conv: &ConvId, from: &str, now_ms: u64) -> bool {
        let me = self.me.tag.as_str();
        match conv {
            Conv::Global => true,
            Conv::Dm { a, b } => (a == me || b == me) && (a == from || b == from),
            Conv::Group { id } => self.contacts.group(id).is_some_and(|g| {
                g.members.iter().any(|m| m.person.tag == from)
                    && g.members.iter().any(|m| m.person.tag == me)
            }),
            Conv::Table { .. } | Conv::Team { .. } => self.conversation(conv).is_some_and(|c| {
                let live = c
                    .ended_at
                    .is_none_or(|t| now_ms.saturating_sub(t) < ENDED_GRACE_MS);
                live && (from == me || c.has_member(from)) && c.has_member(me)
            }),
        }
    }

    pub(super) fn invite(&mut self, invite: TableInvite, now_ms: u64, out: &mut Vec<Effect>) {
        if self.contacts.is_blocked(&invite.from.tag) {
            return;
        }
        let watch = invite.kind == baylee_protocol::social::InviteKind::Watch;
        let name = self.shown_name(&invite.from);
        let table = super::super::text::clean_name(&invite.table.name);
        let id = invite.id.clone();
        self.contacts.invites.retain(|i| i.invite.id != id);
        self.contacts.invites.push(HeldInvite {
            expires_at: now_ms + u64::from(invite.expires_in) * 1000,
            invite,
        });
        out.push(Effect::Toast(ChatToast {
            phrase: if watch {
                ChatPhrase::ToastWatchInvite
            } else {
                ChatPhrase::ToastJoinInvite
            },
            args: vec![name, table],
            conv: None,
            actions: vec![
                (
                    if watch {
                        ToastButton::Watch
                    } else {
                        ToastButton::Join
                    },
                    Intent::AnswerInvite {
                        id: id.clone(),
                        accept: true,
                    },
                ),
                (
                    ToastButton::NotNow,
                    Intent::AnswerInvite {
                        id: id.clone(),
                        accept: false,
                    },
                ),
            ],
            until_answered: true,
            about: Some(ToastAbout::TableInvite(id)),
        }));
        if self.status != Status::Dnd {
            out.push(Effect::Sound(ChatCue::FriendRequest));
        }
    }

    pub(super) fn forget_device(&mut self, out: &mut Vec<Effect>) {
        self.removed = true;
        self.link = Link::Stopped;
        out.push(Effect::Hang);
        out.push(Effect::ForgetSecrets);
        out.push(Effect::DeleteAllLocal);
        out.push(toast(ChatPhrase::ToastDeviceRemoved, vec![]));
        self.convs.clear();
        self.outbox.clear();
    }

    pub(super) fn last_whisperer(&self) -> Option<Person> {
        self.state.last_whisper.as_ref().map(|t| self.person_of(t))
    }
}
