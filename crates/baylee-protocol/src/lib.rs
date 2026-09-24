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
pub const PROTOCOL_VERSION: u32 = 1;

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

    /// The version is a refusal, not a label: an engine and a client that
    /// disagree on it do not talk. It is written down here so that raising
    /// it is a deliberate line in a diff rather than a number that drifted.
    #[test]
    fn the_wire_version_is_one() {
        assert_eq!(PROTOCOL_VERSION, 1);
    }
}
