//! Durable Central inboxes have a different cursor and no destructive ACK.
use super::{Delivered, Effect, Entropy, Held, Sender, Social};

/// One already account-authorized personal delivery from Central. Public sender
/// metadata remains untrusted until the usual pinned-key signature checks pass.
#[derive(Clone, Debug)]
pub struct RetainedMessage {
    /// Central-only ordering cursor, never forwarded to a gateway.
    pub seq: u64,
    /// Server acceptance time in Unix milliseconds.
    pub at: u64,
    /// A direct or group conversation; game conversations are refused here.
    pub conv: String,
    /// Public sender and signing-device identifier.
    pub from: Sender,
    /// Original ciphertext with only this receiving device's wrap.
    pub sealed: Delivered,
}

impl Social {
    /// Enable account-owned personal keys independently of gateway game keys.
    /// An existing directory seeds both namespaces so neither silently loses pins
    /// or a pending verification barrier during the transport migration.
    pub fn use_account_transport(&mut self) {
        if self.state.game_keys.is_none() {
            self.state.game_keys = Some(self.state.keys.clone());
            self.state
                .game_own_devices
                .clone_from(&self.state.own_devices);
        }
        self.account_transport = true;
        self.dirty = true;
    }

    /// Receive durable personal ciphertext without changing gateway acknowledgement
    /// state. The host pauses polling on `Hang` and replays Central from zero on
    /// `Dial`; seen IDs and held-key barriers retain their ordinary meaning.
    pub fn on_retained(
        &mut self,
        message: RetainedMessage,
        now_ms: u64,
        rng: &mut dyn Entropy,
    ) -> Vec<Effect> {
        let mut out = Vec::new();
        if self.held_paused
            || self.removed
            || message.seq == 0
            || !matches!(
                message.conv.parse::<super::Conv>(),
                Ok(super::Conv::Dm { .. } | super::Conv::Group { .. })
            )
        {
            return out;
        }
        self.last_now = self.last_now.max(now_ms);
        self.receive(
            Held {
                legacy_member: false,
                retained: true,
                seq: message.seq,
                at: message.at,
                conv: message.conv,
                from: Sender {
                    handle: super::super::text::clean_name(&message.from.handle),
                    ..message.from
                },
                sealed: message.sealed,
            },
            false,
            now_ms,
            rng,
            &mut out,
        );
        self.flush_into(&mut out, rng);
        self.ack(&mut out);
        self.bump();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::social::{ChatPrefs, Me};
    use baylee_protocol::social::{Down, Up};
    use baylee_seal::testkit::Seeded;

    fn message(seq: u64) -> RetainedMessage {
        RetainedMessage {
            seq,
            at: 100,
            conv: "dm:0007:0009".into(),
            from: Sender {
                tag: "0009".into(),
                handle: "Bob#0009".into(),
                device: "a".repeat(16),
            },
            sealed: Delivered {
                hdr: "invalid".into(),
                n: String::new(),
                ct: String::new(),
                epk: String::new(),
                sig: String::new(),
                k: String::new(),
            },
        }
    }
    fn machine() -> (Social, Seeded) {
        let mut rng = Seeded::new(54);
        let (machine, _) = Social::start(
            Me {
                tag: "0007".into(),
                handle: "Alice#0007".into(),
                guest: false,
                gateway: "https://gateway.example".into(),
            },
            None,
            None,
            ChatPrefs::default(),
            100,
            &mut rng,
        );
        (machine, rng)
    }
    #[test]
    fn central_cursors_cannot_acknowledge_a_gateways_inbox() {
        let (mut machine, mut rng) = machine();
        let effects = machine.on_retained(message(50_000), 100, &mut rng);
        assert!(
            !effects
                .iter()
                .any(|effect| matches!(effect, Effect::Up(Up::Ack { .. })))
        );
        assert_eq!((machine.high, machine.acked), (0, 0));
        let m = message(7);
        let effects = machine.on_frame(
            Down::Msg {
                seq: m.seq,
                at: m.at,
                conv: m.conv,
                from: m.from,
                sealed: m.sealed,
            },
            100,
            &mut rng,
        );
        assert!(
            effects
                .iter()
                .any(|effect| matches!(effect, Effect::Up(Up::Ack { upto: 7 })))
        );
        assert_eq!((machine.high, machine.acked), (7, 7));
        machine.on_retained(message(2), 100, &mut rng);
        assert_eq!((machine.high, machine.acked), (7, 7));
    }
    #[test]
    fn central_does_not_inject_table_or_public_messages() {
        let (mut machine, mut rng) = machine();
        for conv in [
            "global",
            "tbl:019a3d70-0000-7000-8000-000000000001",
            "grp:not-a-uuid",
        ] {
            let mut msg = message(1);
            msg.conv = conv.into();
            assert!(machine.on_retained(msg, 100, &mut rng).is_empty());
        }
        assert_eq!((machine.high, machine.acked), (0, 0));
    }
}
