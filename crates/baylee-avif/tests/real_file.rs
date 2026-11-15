//! A sequence ffmpeg made (`tests/fixtures/opaque.avif`: ffmpeg's own
//! `testsrc2` pattern, 256 × 256, six frames at ten a second, SVT-AV1,
//! `-loop 0`), read and decoded the way the client does.

#![cfg(not(target_arch = "wasm32"))]

use std::sync::Arc;

use baylee_avif::{AlignedPlanes, Bounds, Frame, Player, ROW_ALIGNMENT, Sequence};

const OPAQUE: &[u8] = include_bytes!("fixtures/opaque.avif");

#[test]
fn ffmpegs_sequence_is_read_as_six_frames_of_a_tenth_that_repeat() {
    let seq = Sequence::read(OPAQUE, &Bounds::PICTURE).unwrap();
    let ts = seq.colour.timescale;
    assert_eq!((seq.colour.width, seq.colour.height), (256, 256));
    assert_eq!(seq.colour.samples.len(), 6);
    assert!(seq.colour.samples.iter().all(|s| s.duration_ms(ts) == 100));
    assert!(seq.repeats);
    assert!(seq.alpha.is_none());
}

#[test]
fn every_frame_decodes_at_its_tracks_size_and_the_second_pass_matches_the_first() {
    let mut player = Player::open(Arc::from(OPAQUE), &Bounds::PICTURE, 1).unwrap();
    let mut aligned_player = Player::open_static(OPAQUE, &Bounds::PICTURE, 1).unwrap();
    let mut frame = Frame::default();
    let mut planes = AlignedPlanes::default();
    let mut first = Vec::new();
    for i in 0..6 {
        let (index, ms) = player.advance(&mut frame).unwrap();
        let aligned_index = aligned_player.advance_aligned(&mut planes).unwrap();
        assert_eq!((index, ms), (i, 100));
        assert_eq!(aligned_index, i);
        assert_eq!((frame.width, frame.height), (256, 256));
        assert_eq!(frame.y.len(), 256 * 256);
        assert_eq!(frame.u.len(), 128 * 128);
        assert!(frame.alpha.is_none());
        assert_eq!(planes.y_stride as usize, 256);
        assert_eq!(planes.c_stride as usize, ROW_ALIGNMENT);
        assert_eq!(planes.y_plane(), frame.y.as_slice());
        for row in 0..128 {
            assert_eq!(
                &planes.u_plane()[row * ROW_ALIGNMENT..row * ROW_ALIGNMENT + 128],
                &frame.u[row * 128..(row + 1) * 128]
            );
            assert_eq!(
                &planes.v_plane()[row * ROW_ALIGNMENT..row * ROW_ALIGNMENT + 128],
                &frame.v[row * 128..(row + 1) * 128]
            );
        }
        first.push(frame.y.clone());
    }
    assert_eq!(player.next_index(), 0);
    for (i, luma) in first.iter().enumerate() {
        player.advance(&mut frame).unwrap();
        assert_eq!(
            &frame.y, luma,
            "frame {i} decodes the same on the second pass"
        );
    }
    // The pattern moves: not every frame is the first.
    assert!(first.iter().any(|luma| luma != &first[0]));
    let mut rgba = Vec::new();
    frame.to_rgba(&mut rgba);
    assert_eq!(rgba.len(), 256 * 256 * 4);
    assert!(rgba.chunks(4).all(|p| p[3] == 255));
}

#[test]
fn the_picture_bounds_refuse_a_shipped_loops_size() {
    // A 256-pixel file read under a bound of another exact side is refused
    // before any decoding.
    let narrow = Bounds {
        exact_side: Some(128),
        max_side: 128,
        ..Bounds::PICTURE
    };
    assert!(Sequence::read(OPAQUE, &narrow).is_err());
    assert!(Player::open(Arc::from(OPAQUE), &narrow, 1).is_err());
}

/// Regression test for the `rav1d_submit_frame` error-path unwrap found on
/// 12.11.2026 in `rav1d` 1.1.0 (`decode.rs:4997`): `re_rav1d` 0.1.3 guards
/// `f.frame_hdr.as_ref().is_some_and(...)` so damaged streams cleanly return
/// `Err` without panicking across `extern "C"`.
#[test]
fn a_damaged_frame_is_an_error_and_never_a_panic() {
    let seq = Sequence::read(OPAQUE, &Bounds::PICTURE).unwrap();
    let mut seed = 0x2545_F491_4F6C_DD1Du64;
    for _ in 0..60 {
        let mut bytes = OPAQUE.to_vec();
        for sample in &seq.colour.samples {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let at = sample.offset + (seed as usize) % sample.len;
            bytes[at] ^= (seed >> 40) as u8 | 1;
        }
        let Ok(mut player) = Player::open(Arc::from(bytes), &Bounds::PICTURE, 1) else {
            continue;
        };
        let mut frame = Frame::default();
        for _ in 0..6 {
            if player.advance(&mut frame).is_err() {
                break;
            }
            assert_eq!(frame.y.len(), 256 * 256);
        }
    }
}
