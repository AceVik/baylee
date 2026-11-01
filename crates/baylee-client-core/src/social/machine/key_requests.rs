//! Asking for contacts' keys, and what the answer changes (§10.2).

#[allow(clippy::wildcard_imports)] // the machine's own vocabulary
use super::*;

impl Social {
    // ---- keys ----

    pub(super) fn request_keys(&mut self, tags: &[String], out: &mut Vec<Effect>) {
        let wanted: Vec<String> = tags
            .iter()
            .filter(|t| baylee_protocol::social::valid_tag(t) && !self.fetching.contains(*t))
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        for chunk in wanted.chunks(MAX_KEY_TAGS) {
            for tag in chunk {
                self.fetching.insert(tag.clone());
            }
            out.push(http(
                HttpTag::Keys(chunk.to_vec()),
                Method::Get,
                format!("/social/keys?tag={}", chunk.join(",")),
                None,
            ));
        }
    }

    pub(super) fn keys_diff(&mut self, diff: &KeyDiff, now_ms: u64, out: &mut Vec<Effect>) {
        if diff.tag == self.me.tag {
            let ids: Vec<String> = self
                .state
                .keys
                .get(&diff.tag)
                .map(|k| k.devices.iter().map(|d| d.id.clone()).collect())
                .unwrap_or_default();
            let known = std::mem::replace(&mut self.state.own_devices, ids);
            // A device the account didn't have when this one last looked
            // (and that isn't this one): the new-device notice (§4.5).
            if !known.is_empty() {
                for device in &diff.added {
                    if device.id != self.device.device_id() && !known.contains(&device.id) {
                        out.push(Effect::Toast(ChatToast {
                            phrase: ChatPhrase::OwnDeviceAdded,
                            args: vec![device.label.clone(), day_month(device.added_at)],
                            conv: None,
                            actions: Vec::new(),
                            until_answered: true,
                            about: None,
                        }));
                    }
                }
            }
            self.dirty = true;
            return;
        }
        if !diff.changed() {
            return;
        }
        let person = self.person_of(&diff.tag);
        let name = self.shown_name(&person);
        let convs: Vec<ConvId> = self
            .convs
            .iter()
            .filter(|c| {
                c.id.partner(&self.me.tag) == Some(diff.tag.as_str()) || c.has_member(&diff.tag)
            })
            .map(|c| c.id.clone())
            .collect();
        for conv in convs {
            for device in &diff.added {
                self.notice(
                    &conv,
                    ChatPhrase::DeviceAdded,
                    vec![
                        name.clone(),
                        device.label.clone(),
                        day_month(device.added_at),
                    ],
                    now_ms,
                );
            }
            if !diff.removed.is_empty() {
                self.notice(&conv, ChatPhrase::DeviceRemoved, vec![name.clone()], now_ms);
            }
            self.notice(&conv, ChatPhrase::VerifyAgain, vec![name.clone()], now_ms);
        }
    }

    /// Whether `tag`'s keys are cached and not known to be out of date.
    pub(super) fn keys_fresh(&self, tag: &str) -> bool {
        self.state.keys.has(tag) && !self.stale.contains(tag)
    }

    pub(super) fn keys_confirmed(&self, tag: &str) -> bool {
        let Some(contact) = self.state.keys.get(tag) else {
            return false;
        };
        if tag == self.me.tag
            && let Some(trusted) = &self.trusted_own_devices
        {
            let current = (self.device.ik(), self.device.dk());
            return trusted.contains(&current)
                && contact
                    .devices
                    .iter()
                    .any(|device| (device.ik, device.dk) == current)
                && contact
                    .devices
                    .iter()
                    .all(|device| trusted.contains(&(device.ik, device.dk)));
        }
        !contact.devices.is_empty() && !contact.requires_confirmation
    }

    /// Tags the gateway left out of a key answer: this account may not
    /// write to them; sends waiting on them fail.
    pub(super) fn keys_refused(&mut self, tags: &[String], now_ms: u64) {
        if tags.is_empty() {
            return;
        }
        // A refusal must block sending without erasing the previously seen keys:
        // otherwise an untrusted gateway can turn a replacement into first use.
        for tag in tags {
            self.stale.insert(tag.clone());
            self.state.keys.reject(tag);
        }
        self.dirty = true;
        let failing: Vec<usize> = self
            .outbox
            .iter()
            .enumerate()
            .filter(|(_, p)| {
                p.wait == Wait::Keys
                    && self
                        .members(&p.key.conv)
                        .is_some_and(|m| m.iter().any(|t| tags.contains(t) && !self.keys_fresh(t)))
            })
            .map(|(i, _)| i)
            .collect();
        for i in failing.into_iter().rev() {
            let p = self.outbox.remove(i);
            self.fail(&p.key, ChatPhrase::SendNotAllowed, now_ms);
        }
    }
}
