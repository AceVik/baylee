//! Sending: who a conversation goes to, sealing, the queue and what the
//! gateway answers.

#[allow(clippy::wildcard_imports)] // the machine's own vocabulary
use super::*;

impl Social {
    // ---- sending ----

    /// The member tags of a conversation this account may write to now,
    /// itself included; `None` when it may not. A table's members are
    /// checked against the roster drawn (§5.4).
    pub(super) fn members(&self, id: &ConvId) -> Option<Vec<String>> {
        let me = self.me.tag.clone();
        let mut tags: Vec<String> = match id {
            Conv::Global => vec![me.clone()],
            Conv::Dm { a, b } => {
                if *a != me && *b != me {
                    return None;
                }
                vec![a.clone(), b.clone()]
            }
            Conv::Group { id } => {
                let group = self.contacts.group(id)?;
                group.members.iter().map(|m| m.person.tag.clone()).collect()
            }
            Conv::Table { game } | Conv::Team { game, .. } => {
                let table = self
                    .table
                    .as_ref()
                    .filter(|t| t.game == *game && !t.watching)?;
                let team = match id {
                    Conv::Team { team, .. } => Some(*team),
                    _ => None,
                };
                if team.is_some() && table.my_team != team {
                    return None;
                }
                let conv = self.conversation(id)?;
                conv.members
                    .iter()
                    .filter(|m| m.tag == me || table.seats(&m.tag, team))
                    .map(|m| m.tag.clone())
                    .collect()
            }
        };
        if !tags.contains(&me) {
            tags.push(me);
        }
        tags.sort();
        tags.dedup();
        Some(tags)
    }

    /// Members the gateway lists that the roster doesn't show.
    fn off_roster(&self, id: &ConvId) -> Vec<Person> {
        let (Some(table), Some(conv)) = (self.table.as_ref(), self.conversation(id)) else {
            return Vec::new();
        };
        let team = match id {
            Conv::Team { team, .. } => Some(*team),
            Conv::Table { .. } => None,
            _ => return Vec::new(),
        };
        conv.members
            .iter()
            .filter(|m| m.tag != self.me.tag && !table.seats(&m.tag, team))
            .cloned()
            .collect()
    }

    pub(super) fn send(
        &mut self,
        conv: &ConvId,
        body: Body,
        now_ms: u64,
        rng: &mut dyn Entropy,
        out: &mut Vec<Effect>,
    ) {
        let Some(_) = self.members(conv) else {
            self.answer(ChatPhrase::SendNotAllowed, Vec::new(), now_ms);
            return;
        };
        let strays = self.off_roster(conv);
        if !strays.is_empty() {
            self.notice(conv, ChatPhrase::RosterMismatch, Vec::new(), now_ms);
        }
        let id = sealing::uuid_v7(now_ms, rng);
        let hdr = sealing::header_bytes(
            &id,
            &conv.to_string(),
            &self.me.tag,
            self.device.device_id(),
            now_ms,
        );
        let key = LineKey {
            conv: conv.clone(),
            id,
        };
        let me = Person {
            tag: self.me.tag.clone(),
            handle: self.me.handle.clone(),
        };
        let entry = self.conv_entry(conv, now_ms);
        entry.open = true;
        entry.insert(ChatLine {
            key: key.clone(),
            at: now_ms,
            kind: LineKind::Message {
                from: me,
                mine: true,
                body: body.clone(),
            },
            state: LineState::Pending,
            hidden: false,
            mentions_me: false,
            failure: None,
        });
        if let Conv::Group { id } = conv {
            self.last_group = Some(id.clone());
        }
        let req = self.next_req;
        self.next_req = self.next_req.wrapping_add(1).max(1);
        self.outbox.push(Pending {
            req,
            key,
            hdr,
            body,
            reseals: 0,
            wait: Wait::Keys,
        });
        self.pump(now_ms, rng, out);
    }

    /// Seals and sends every send that can go now.
    pub(super) fn pump(&mut self, now_ms: u64, rng: &mut dyn Entropy, out: &mut Vec<Effect>) {
        let mut missing: BTreeSet<String> = BTreeSet::new();
        let mut failed: Vec<(usize, ChatPhrase)> = Vec::new();
        for i in 0..self.outbox.len() {
            if self.outbox[i].wait != Wait::Keys && self.outbox[i].wait != Wait::Link {
                continue;
            }
            let conv = self.outbox[i].key.conv.clone();
            if conv == Conv::Global {
                if self.link != Link::Welcomed {
                    self.outbox[i].wait = Wait::Link;
                    continue;
                }
                let p = &self.outbox[i];
                match serde_json::to_string(&p.body) {
                    Ok(body) if body.len() <= baylee_protocol::social::MAX_BODY_BYTES => {
                        out.push(Effect::Up(Up::Global {
                            req: p.req,
                            id: p.key.id.clone(),
                            body,
                        }));
                        self.outbox[i].wait = Wait::Answer;
                    }
                    _ => failed.push((i, ChatPhrase::SendTooLarge)),
                }
                continue;
            }
            let Some(tags) = self.members(&conv) else {
                failed.push((i, ChatPhrase::SendNotAllowed));
                continue;
            };
            // A table whose member list hasn't come yet (just sat down):
            // wait for `Members` rather than seal for nobody.
            let unlisted = matches!(conv, Conv::Table { .. } | Conv::Team { .. })
                && self
                    .conversation(&conv)
                    .is_none_or(|c| c.members.is_empty());
            if unlisted {
                self.outbox[i].wait = Wait::Keys;
                continue;
            }
            let lacking: Vec<&String> = tags.iter().filter(|t| !self.keys_fresh(t)).collect();
            if !lacking.is_empty() {
                missing.extend(lacking.into_iter().cloned());
                self.outbox[i].wait = Wait::Keys;
                continue;
            }
            if tags.iter().any(|tag| !self.keys_confirmed(tag)) {
                self.outbox[i].wait = Wait::Verification;
                continue;
            }
            if self.link != Link::Welcomed {
                self.outbox[i].wait = Wait::Link;
                continue;
            }
            let devices: Vec<&super::super::keys::CachedDevice> = tags
                .iter()
                .filter_map(|t| self.state.keys.get(t))
                .flat_map(|k| k.devices.iter())
                .collect();
            let p = &self.outbox[i];
            match sealing::seal_for(&self.device, &p.hdr, &p.body, &devices, rng) {
                Ok(sealed) => {
                    out.push(Effect::Up(sealed.frame(p.req, &conv.to_string())));
                    self.outbox[i].wait = Wait::Answer;
                }
                Err(SealFault::TooLarge) => failed.push((i, ChatPhrase::SendTooLarge)),
                Err(SealFault::Key) => failed.push((i, ChatPhrase::SendRefused)),
            }
        }
        for (i, phrase) in failed.into_iter().rev() {
            let p = self.outbox.remove(i);
            self.fail(&p.key, phrase, now_ms);
        }
        let missing: Vec<String> = missing.into_iter().collect();
        self.request_keys(&missing, out);
    }

    pub(super) fn fail(&mut self, key: &LineKey, phrase: ChatPhrase, _now_ms: u64) {
        if let Some(conv) = self.convs.iter_mut().find(|c| c.id == key.conv)
            && let Some(line) = conv.line_mut(&key.id)
        {
            line.state = LineState::Failed;
            line.failure = Some(phrase);
        }
        self.bump();
    }

    pub(super) fn sent(
        &mut self,
        req: u32,
        id: &str,
        at: u64,
        rng: &mut dyn Entropy,
        out: &mut Vec<Effect>,
    ) {
        let Some(i) = self.outbox.iter().position(|p| p.req == req) else {
            return;
        };
        let p = self.outbox.remove(i);
        if p.key.id != id {
            return;
        }
        self.state.seen.insert(p.key.id.clone(), at);
        self.dirty = true;
        let Some(conv) = self.convs.iter_mut().find(|c| c.id == p.key.conv) else {
            return;
        };
        let Some(mut line) = conv.line(&p.key.id).cloned() else {
            return;
        };
        line.state = LineState::Final;
        line.at = at;
        conv.insert(line.clone());
        let record = Record {
            id: p.key.id.clone(),
            at,
            from: Person {
                tag: self.me.tag.clone(),
                handle: self.me.handle.clone(),
            },
            mine: true,
            body: p.body,
            evidence: None,
        };
        self.append_history(&p.key.conv, &record, rng, out);
    }

    pub(super) fn refused(
        &mut self,
        req: u32,
        why: Why,
        now_ms: u64,
        rng: &mut dyn Entropy,
        out: &mut Vec<Effect>,
    ) {
        let Some(i) = self.outbox.iter().position(|p| p.req == req) else {
            return;
        };
        let phrase = match why {
            Why::Devices => {
                if self.outbox[i].reseals < MAX_RESEALS {
                    self.outbox[i].reseals += 1;
                    self.outbox[i].wait = Wait::Keys;
                    let conv = self.outbox[i].key.conv.clone();
                    let tags = self.members(&conv).unwrap_or_default();
                    // Fetched again, not forgotten: the cache keeps the old
                    // set, so the answer's diff says what changed (§4.5) and
                    // a verified mark drops only if the devices did.
                    self.stale.extend(tags);
                    self.pump(now_ms, rng, out);
                    return;
                }
                ChatPhrase::SendFailed
            }
            Why::Rate => {
                self.outbox[i].wait = Wait::RetryAt(now_ms + RATE_RETRY_MS);
                return;
            }
            Why::Duplicate => {
                // The gateway already holds this id: an earlier attempt got
                // through before the link dropped.
                let id = self.outbox[i].key.id.clone();
                self.sent(req, &id, now_ms, rng, out);
                return;
            }
            Why::NotMember => ChatPhrase::SendNotAllowed,
            Why::TooLarge | Why::BadSealed => ChatPhrase::SendTooLarge,
            Why::Stale => ChatPhrase::SendStale,
            Why::RecipientFull => ChatPhrase::SendRecipientFull,
            Why::Off => ChatPhrase::ChatOff,
            Why::Protocol | Why::Device => ChatPhrase::SendRefused,
        };
        let p = self.outbox.remove(i);
        self.fail(&p.key, phrase, now_ms);
    }

    pub(super) fn append_history(
        &mut self,
        conv: &ConvId,
        record: &Record,
        rng: &mut dyn Entropy,
        out: &mut Vec<Effect>,
    ) {
        if self.prefs.history_days == 0 || self.removed {
            return;
        }
        let vault_key = *self.device.vault_key();
        if let Some(framed) = history::seal_record(&vault_key, conv, record, rng) {
            out.push(Effect::AppendHistory {
                file: history::file_path(&vault_key, conv),
                record: framed,
            });
        }
    }
}
