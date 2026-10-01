//! Envelope round trips for the control and engine planes, and the version
//! refusals every boundary shares.

use baylee_protocol::v1::{self, envelope::Msg};
use baylee_protocol::{PROTOCOL_VERSION, TICKET_REFUSED, WS_TICKET_PATH, version_refusal};
use prost::Message;

fn trip(msg: Msg) -> Msg {
    let wire = v1::Envelope { msg: Some(msg) }.encode_to_vec();
    v1::Envelope::decode(&wire[..])
        .expect("decodes")
        .msg
        .expect("a message came back")
}

#[test]
fn an_agent_hello_keeps_its_token_name_capacity_and_version() {
    let hello = v1::AgentHello {
        token: "s3cret".into(),
        name: "box-1".into(),
        capacity: 7,
        protocol_version: PROTOCOL_VERSION,
    };
    assert_eq!(trip(Msg::AgentHello(hello.clone())), Msg::AgentHello(hello));
}

#[test]
fn an_engine_hello_keeps_its_game_token_and_version() {
    let hello = v1::EngineHello {
        game_id: "g1".into(),
        token: "t".into(),
        protocol_version: PROTOCOL_VERSION,
    };
    assert_eq!(
        trip(Msg::EngineHello(hello.clone())),
        Msg::EngineHello(hello)
    );
}

#[test]
fn a_hello_from_before_the_version_field_reads_as_version_zero() {
    // An engine hello with only fields 1 and 2 on the wire (no field 3).
    let old = v1::EngineHello {
        game_id: "g".into(),
        token: "t".into(),
        protocol_version: 0,
    };
    let wire = old.encode_to_vec();
    let back = v1::EngineHello::decode(&wire[..]).unwrap();
    assert_eq!(back.protocol_version, 0);
    assert!(
        version_refusal("This engine", back.protocol_version)
            .unwrap()
            .contains("does not say")
    );
}

#[test]
fn start_and_stop_engine_round_trip() {
    let start = v1::StartEngine {
        game_id: "g".into(),
        engine_token: "tok".into(),
        gateway_url: "ws://127.0.0.1:1/engine/ws".into(),
    };
    assert_eq!(
        trip(Msg::StartEngine(start.clone())),
        Msg::StartEngine(start)
    );
    let stop = v1::StopEngine {
        game_id: "g".into(),
    };
    assert_eq!(trip(Msg::StopEngine(stop.clone())), Msg::StopEngine(stop));
}

#[test]
fn a_game_record_chunk_keeps_binary_data_exactly() {
    let data: Vec<u8> = (0..=255).collect();
    let chunk = v1::GameRecordChunk {
        game_id: "g".into(),
        seq: 2,
        data,
        last: true,
    };
    let Msg::GameRecordChunk(back) = trip(Msg::GameRecordChunk(chunk.clone())) else {
        panic!("another message came back");
    };
    assert_eq!(back, chunk);
    assert_eq!(back.data.len(), 256);
}

#[test]
fn flush_and_flushed_echo_the_nonce_and_piece_count() {
    let ask = v1::FlushRecord {
        game_id: "g".into(),
        nonce: u64::MAX,
    };
    assert_eq!(trip(Msg::FlushRecord(ask.clone())), Msg::FlushRecord(ask));
    let done = v1::RecordFlushed {
        game_id: "g".into(),
        nonce: u64::MAX,
        pieces: 9,
    };
    assert_eq!(
        trip(Msg::RecordFlushed(done.clone())),
        Msg::RecordFlushed(done)
    );
}

#[test]
fn a_game_end_lists_its_winners_in_order() {
    let ended = v1::GameEnded {
        game_id: "g".into(),
        winners: vec![2, 0, 1],
        reason: "conceded".into(),
    };
    let Msg::GameEnded(back) = trip(Msg::GameEnded(ended.clone())) else {
        panic!()
    };
    assert_eq!(back.winners, [2, 0, 1]);
    assert_eq!(back, ended);
}

#[test]
fn clock_probes_and_heartbeats_keep_their_64_bit_times() {
    let probe = v1::ClockProbe {
        client_time_ms: u64::MAX - 1,
        server_time_ms: 0,
    };
    assert_eq!(trip(Msg::ClockProbe(probe)), Msg::ClockProbe(probe));
    let beat = v1::Heartbeat {
        client_time_ms: 1 << 40,
    };
    assert_eq!(trip(Msg::Heartbeat(beat)), Msg::Heartbeat(beat));
}

#[test]
fn the_empty_messages_still_say_which_kind_they_are() {
    assert_eq!(
        trip(Msg::SeatReady(v1::SeatReady {})),
        Msg::SeatReady(v1::SeatReady {})
    );
    assert_eq!(
        trip(Msg::Curtain(v1::Curtain {})),
        Msg::Curtain(v1::Curtain {})
    );
    let wire = v1::Envelope {
        msg: Some(Msg::Curtain(v1::Curtain {})),
    }
    .encode_to_vec();
    assert!(
        !wire.is_empty(),
        "an empty message is still a tag on the wire"
    );
}

#[test]
fn a_seat_attach_keeps_its_resync_flag() {
    for resync in [false, true] {
        let a = v1::SeatAttached { seat: 3, resync };
        assert_eq!(trip(Msg::SeatAttached(a)), Msg::SeatAttached(a));
    }
}

#[test]
fn nested_seat_frames_forward_without_changing_a_byte() {
    // Three layers deep: a gateway forwards a frame it never reads.
    let inner = v1::Envelope {
        msg: Some(Msg::Error(v1::Error {
            code: 1,
            message: "é ✓".into(),
        })),
    }
    .encode_to_vec();
    let mid = v1::Envelope {
        msg: Some(Msg::SeatFrame(v1::SeatFrame {
            seat: 1,
            envelope: inner.clone(),
        })),
    }
    .encode_to_vec();
    let outer = v1::SeatFrame {
        seat: 2,
        envelope: mid.clone(),
    };
    let Msg::SeatFrame(back) = trip(Msg::SeatFrame(outer)) else {
        panic!()
    };
    assert_eq!(back.envelope, mid);
}

#[test]
fn truncated_or_garbage_bytes_are_an_error_never_a_message() {
    let wire = v1::Envelope {
        msg: Some(Msg::GameEnded(v1::GameEnded {
            game_id: "game".into(),
            winners: vec![0],
            reason: "r".into(),
        })),
    }
    .encode_to_vec();
    assert!(v1::Envelope::decode(&wire[..wire.len() - 1]).is_err());
    // A length-delimited field that claims more bytes than there are.
    assert!(v1::Envelope::decode(&[0x1a, 0xff, 0x01][..]).is_err());
}

#[test]
fn two_messages_of_a_oneof_on_one_wire_keep_the_last() {
    // proto3 oneof: when both appear, the later one wins.
    let a = v1::Envelope {
        msg: Some(Msg::Curtain(v1::Curtain {})),
    }
    .encode_to_vec();
    let b = v1::Envelope {
        msg: Some(Msg::SeatReady(v1::SeatReady {})),
    }
    .encode_to_vec();
    let both = [a, b].concat();
    let env = v1::Envelope::decode(&both[..]).unwrap();
    assert_eq!(env.msg, Some(Msg::SeatReady(v1::SeatReady {})));
}

#[test]
fn a_refusal_names_who_and_both_versions_for_every_other_version() {
    for theirs in [1, PROTOCOL_VERSION - 1, PROTOCOL_VERSION + 1, u32::MAX] {
        let said = version_refusal("This client", theirs).expect("refused");
        assert!(said.starts_with("This client"), "{said}");
        assert!(said.contains(&theirs.to_string()), "{said}");
        assert!(said.contains(&PROTOCOL_VERSION.to_string()), "{said}");
        assert!(said.ends_with('.'), "one sentence: {said}");
    }
    assert!(version_refusal("x", PROTOCOL_VERSION).is_none());
}

#[test]
fn the_ticket_constants_are_the_wire_words_clients_match_on() {
    assert_eq!(WS_TICKET_PATH, "/ws-ticket");
    assert_eq!(TICKET_REFUSED, "ticket expired or used");
}
