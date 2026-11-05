//! Read-only compatibility for a gateway group's already delivered ciphertext.
use super::RetainedMessage;
use super::{Effect, Entropy, Held, Social};
use baylee_protocol::social::{Conv, GroupInfo};

impl Social {
    /// Drain a former gateway group into the encrypted archive. Its membership
    /// never becomes a Central group or grants permission to send. Sender pins,
    /// signatures, replay checks, blocked peers and persist-before-ACK still apply.
    pub fn on_legacy_group(
        &mut self,
        message: RetainedMessage,
        group: &GroupInfo,
        now_ms: u64,
        rng: &mut dyn Entropy,
    ) -> Vec<Effect> {
        let mut out = Vec::new();
        let Ok(Conv::Group { id }) = message.conv.parse::<Conv>() else {
            return out;
        };
        if !self.account_transport
            || self.held_paused
            || self.removed
            || id != group.id
            || self.contacts.group(&id).is_some()
            || group.members.len() > 50
            || !group
                .members
                .iter()
                .any(|member| member.person.tag == self.me.tag)
            || !group
                .members
                .iter()
                .any(|member| member.person.tag == message.from.tag)
        {
            return out;
        }
        self.select_key_scope(true);
        self.receive(
            Held {
                legacy_member: true,
                retained: false,
                seq: message.seq,
                at: message.at,
                conv: message.conv,
                from: message.from,
                sealed: message.sealed,
            },
            false,
            now_ms,
            rng,
            &mut out,
        );
        let conv = Conv::Group { id };
        if let Some(entry) = self.convs.iter_mut().find(|entry| entry.id == conv) {
            entry.title = super::super::text::clean_name(&group.name);
            entry.members = group
                .members
                .iter()
                .map(|member| member.person.clone())
                .collect();
            entry.ended_at = Some(now_ms);
        }
        self.flush_into(&mut out, rng);
        self.ack(&mut out);
        self.bump();
        self.select_key_scope(false);
        out
    }
}
