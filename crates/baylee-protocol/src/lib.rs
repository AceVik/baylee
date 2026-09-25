//! baylee-protocol — binary websocket protocol (protobuf), wasm-safe.
//!
//! v0: transport handshake + preset transfer (see `docs/protocol.md`).
//! Choice/action/view messages land with the engine API (M1–M3).
//!
//! Also the rules both ends of the gateway's HTTP API check alike
//! ([`names`]), so that the client can refuse what the gateway would.

#![warn(missing_docs)]

pub mod names;

/// Wire protocol version; incompatible versions refuse the session.
pub const PROTOCOL_VERSION: u32 = 3;

/// Why a peer that says it speaks protocol `theirs` is refused, or `None`
/// when it speaks this build's (#271).
///
/// One sentence for every boundary, so a refused agent, engine or client
/// is told the same thing in the same words: both numbers, because a
/// mismatch is only ever fixed by knowing which side is behind. `who` is the
/// peer as the sentence addresses it ("This agent"). A `theirs` of 0 is a
/// peer from before the version was sent at all, which proto3 cannot tell
/// from one that sent 0, and which says so rather than claim a version.
#[must_use]
pub fn version_refusal(who: &str, theirs: u32) -> Option<String> {
    (theirs != PROTOCOL_VERSION).then(|| {
        if theirs == 0 {
            format!(
                "{who} does not say which protocol it speaks; this gateway speaks {PROTOCOL_VERSION}."
            )
        } else {
            format!("{who} speaks protocol {theirs}; this gateway speaks {PROTOCOL_VERSION}.")
        }
    })
}

/// The path and query a seat socket is opened on, relative to the gateway:
/// `/games/{game_id}/ws?token=…&protocol=…` (#271).
///
/// Every dialer builds it here, so none can leave out the protocol it
/// speaks: the gateway refuses a seat socket that does not say, before a
/// frame of the game is sent (`docs/protocol.md` §"Which side checks the
/// protocol (#271)").
#[must_use]
pub fn seat_socket_path(game_id: &str, seat_token: &str) -> String {
    format!("/games/{game_id}/ws?token={seat_token}&protocol={PROTOCOL_VERSION}")
}

/// Generated protobuf types (`baylee.v1`).
#[allow(missing_docs, clippy::all, clippy::pedantic)]
pub mod v1 {
    include!(concat!(env!("OUT_DIR"), "/baylee.v1.rs"));
}

#[cfg(test)]
mod tests {
    use super::{PROTOCOL_VERSION, v1};
    use prost::Message;

    /// The nesting contract, which is what lets the gateway run no rules.
    ///
    /// A `SeatFrame` carries an **encoded** player-facing `Envelope` as
    /// bytes rather than as a field on `Envelope` itself, so the gateway
    /// forwards exactly what it was handed: it never decodes a player
    /// message, never re-encodes one, and the player-facing protocol keeps
    /// the shape it had before the engine moved into its own process. The
    /// bytes have to come back *identical*, not merely equivalent — a
    /// gateway that round-tripped them through its own prost would be a
    /// second encoder on a wire that is supposed to have one.
    #[test]
    fn a_seat_frame_carries_bytes_the_gateway_never_decodes() {
        let inner = v1::Envelope {
            msg: Some(v1::envelope::Msg::Error(v1::Error {
                code: 7,
                message: "no such seat".into(),
            })),
        };
        let bytes = inner.encode_to_vec();

        let outer = v1::Envelope {
            msg: Some(v1::envelope::Msg::SeatFrame(v1::SeatFrame {
                seat: 3,
                envelope: bytes.clone(),
            })),
        };
        let wire = outer.encode_to_vec();

        let back = v1::Envelope::decode(&wire[..]).expect("the frame decodes");
        let Some(v1::envelope::Msg::SeatFrame(frame)) = back.msg else {
            panic!("a seat frame came back as something else");
        };
        assert_eq!(frame.seat, 3);
        assert_eq!(
            frame.envelope, bytes,
            "the payload is the bytes it was handed, byte for byte"
        );
        assert_eq!(
            v1::Envelope::decode(&frame.envelope[..]).expect("the payload decodes"),
            inner,
            "and the seat reads the message the engine wrote"
        );
    }

    /// A message kind this build has never heard of decodes as *no message*
    /// rather than as a failure. That is what lets one end of the wire learn
    /// a new envelope without the other end refusing every frame that
    /// follows it: prost skips a field number it does not know, so the
    /// `oneof` is simply empty and the reader answers the unknown the way it
    /// answers anything it cannot act on.
    #[test]
    fn an_envelope_from_a_newer_peer_is_read_as_nothing_rather_than_refused() {
        // Field 999, wire type 2 (length-delimited), four bytes of payload —
        // the shape any future member of the `oneof` would have.
        let mut wire = Vec::new();
        prost::encoding::encode_key(999, prost::encoding::WireType::LengthDelimited, &mut wire);
        prost::encoding::encode_varint(4, &mut wire);
        wire.extend_from_slice(b"soon");

        let envelope = v1::Envelope::decode(&wire[..]).expect("an unknown field is skipped");
        assert!(
            envelope.msg.is_none(),
            "an unknown message must arrive as no message, not as a wrong one"
        );
    }

    /// An empty envelope is legal on the wire and means nothing was said.
    /// Every reader matches on `Some(msg)`, so this is the case they all
    /// have to fall through — a decoder that refused it would turn a
    /// harmless frame into a dropped connection.
    #[test]
    fn an_envelope_with_no_message_survives_the_trip() {
        let empty = v1::Envelope { msg: None };
        let wire = empty.encode_to_vec();
        assert!(wire.is_empty(), "nothing said is nothing sent");
        assert_eq!(
            v1::Envelope::decode(&wire[..]).expect("an empty envelope decodes"),
            empty
        );
    }

    /// A peer is refused by the one sentence every boundary shares, and it
    /// names both numbers; this build's own version is not refused.
    #[test]
    fn a_peer_of_another_protocol_is_told_both_numbers() {
        let newer = PROTOCOL_VERSION + 1;
        let said = super::version_refusal("This agent", newer).expect("refused");
        assert!(said.starts_with("This agent"), "{said}");
        assert!(said.contains(&newer.to_string()), "{said}");
        assert!(said.contains(&PROTOCOL_VERSION.to_string()), "{said}");
        let silent = super::version_refusal("This engine", 0).expect("refused");
        assert!(silent.contains("does not say"), "{silent}");
        assert!(silent.contains(&PROTOCOL_VERSION.to_string()), "{silent}");
        assert_eq!(
            super::version_refusal("This client", PROTOCOL_VERSION),
            None
        );
    }

    /// A seat socket says which protocol it speaks in the one path every
    /// dialer opens it on.
    #[test]
    fn a_seat_socket_says_its_protocol() {
        assert_eq!(
            super::seat_socket_path("g", "t"),
            format!("/games/g/ws?token=t&protocol={PROTOCOL_VERSION}")
        );
    }

    /// The version is a refusal, not a label: an engine and a client that
    /// disagree on it do not talk. It is written down here so that raising
    /// it is a deliberate line in a diff rather than a number that drifted.
    #[test]
    fn the_wire_version_is_three() {
        // 2: `SeatReady` and `Curtain` (#256).
        // 3: `SeatSettingMsg` (#265). An engine built before it drops the
        // frame without a word, so a client that sent one would wait for a
        // view that never comes.
        assert_eq!(PROTOCOL_VERSION, 3);
    }
}
