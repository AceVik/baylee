//! A moving profile picture decoded by the real client executable started as
//! its decoder (`portraits::helper`), the way the client does it: the file
//! read and bounded by the parent, the frames out of the child.

use std::process::Command;

use baylee_client::portraits::helper::{self, DEADLINE, FLAG};

/// The fixture the unit tests use: 256 px, six frames of 100, 200, 300,
/// 400, 500 and 100 ms (`libsvtav1` from ffmpeg's `testsrc2`).
const MOVING: &[u8] = include_bytes!("data/picture-moving.avif");

fn decoder() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_baylee-client"));
    command.arg(FLAG);
    command
}

#[test]
fn the_client_decodes_a_moving_picture_in_a_child_of_its_own() {
    let frames = helper::decode(decoder(), MOVING, DEADLINE).expect("the frames");
    assert_eq!(frames.durations, vec![100, 200, 300, 400, 500, 100]);
    assert_eq!(frames.rgba.len(), 6);
    assert!(frames.rgba.iter().all(|frame| frame.len() == 128 * 128 * 4));
    assert_ne!(frames.rgba[0], frames.rgba[3], "the pattern moves");
}

#[test]
fn the_decoder_refuses_a_damaged_sequence_without_harm() {
    // The tables stand, the AV1 data is overwritten: the parent's reading
    // takes it, the child's decoder refuses it (or dies), and the parent
    // answers a refusal either way.
    let mut damaged = MOVING.to_vec();
    let mdat = damaged
        .windows(4)
        .position(|w| w == b"mdat")
        .expect("an mdat box");
    for byte in damaged.iter_mut().skip(mdat + 4) {
        *byte ^= 0x5a;
    }
    let result = helper::decode(decoder(), &damaged, DEADLINE);
    assert!(result.is_err(), "{result:?}");
    // And the next picture decodes as before.
    assert!(helper::decode(decoder(), MOVING, DEADLINE).is_ok());
}

#[test]
fn the_decoder_takes_no_other_arguments() {
    let status = Command::new(env!("CARGO_BIN_EXE_baylee-client"))
        .arg(FLAG)
        .arg("--anything")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("it ran");
    assert!(!status.success());
}
