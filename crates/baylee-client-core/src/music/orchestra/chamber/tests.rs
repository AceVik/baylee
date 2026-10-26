use super::*;
use crate::music::score::memorial;

#[test]
fn complete_performance_has_recordings_and_natural_release_room() {
    for event in memorial::events() {
        let note = event.note();
        assert!((1..=127).contains(&note.velocity));
        for layer in 0..=usize::from(note.instrument == 0) {
            let clip = bank().clip(note.instrument, note.pitch, layer).unwrap();
            assert!(
                clip.frames >= (note.frames + RATE) as usize,
                "instrument {}, pitch {}, layer {}: {} frames for {} held",
                note.instrument,
                note.pitch,
                layer,
                clip.frames,
                note.frames
            );
        }
    }
}

#[test]
fn blocks_match_frames_through_note_ends_and_repeated_release() {
    let mut scalar = Ensemble::default();
    let mut blocked = Ensemble::default();
    for event in memorial::events().step_by(57).take(24) {
        let mut note = event.note();
        note.frames = 900;
        scalar.note(note);
        blocked.note(note);
    }
    let scalar_capacity = scalar.voices.capacity();
    let blocked_capacity = blocked.voices.capacity();
    for step in 0..180 {
        if step == 2 || step == 5 {
            scalar.release();
            blocked.release();
        }
        let mut expected = [[0.0; 2]; 127];
        let mut expected_feed = [0.0; 127];
        for (frame, feed) in expected.iter_mut().zip(expected_feed.iter_mut()) {
            scalar.frame(frame, feed);
        }
        let mut actual = [[0.0; 2]; 127];
        let mut actual_feed = [0.0; 127];
        blocked.render(&mut actual, &mut actual_feed);
        assert_eq!(actual, expected);
        assert_eq!(actual_feed, expected_feed);
    }
    assert!(scalar.voices.is_empty() && blocked.voices.is_empty());
    assert_eq!(scalar.voices.capacity(), scalar_capacity);
    assert_eq!(blocked.voices.capacity(), blocked_capacity);
}

#[test]
fn a_second_scene_change_does_not_rewind_a_releasing_note() {
    let mut voice = Voice::new(memorial::events().next().unwrap().note());
    for _ in 0..4000 {
        voice.next();
    }
    voice.release(true);
    let start = voice.released_at;
    let level = voice.released_level;
    for _ in 0..3000 {
        voice.next();
    }
    voice.release(true);
    assert_eq!(voice.released_at, start);
    assert_eq!(voice.released_level.to_bits(), level.to_bits());
    for _ in 0..RATE / 4 {
        voice.next();
    }
    assert!(voice.next().is_none());
}
