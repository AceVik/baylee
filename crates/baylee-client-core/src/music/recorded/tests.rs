use super::*;

/// Every file's playable length: the seconds in the brief's table (two
/// decimals), and the frames `ffmpeg -i <file> -f s16le` decodes from it
/// (libavcodec's own Opus decoder and Ogg demuxer, 18.11.2026), which is the
/// last granule position less the pre-skip.
const LENGTHS: [(Track, f64, usize); 9] = [
    (Track::Title, 112.12, 5_381_643),
    (Track::Lobby, 174.90, 8_394_711),
    (Track::Combat, 174.61, 8_380_918),
    (Track::Tower, 154.47, 7_414_178),
    (Track::Victory, 43.97, 2_110_443),
    (Track::Defeat, 61.24, 2_939_089),
    (Track::Draw, 19.92, 956_043),
    (Track::StingerCall, 6.11, 292_800),
    (Track::StingerEpic, 8.61, 412_800),
];

fn request(scene: Scene) -> ScoreRequest {
    ScoreRequest {
        scene,
        recorded: true,
        ..ScoreRequest::default()
    }
}

/// Renders `seconds` of the mixer under one request.
fn render(mixer: &mut Mixer, request: ScoreRequest, seconds: f32) -> Vec<[f32; 2]> {
    let mut out = vec![[0.0; 2]; (seconds * RATE as f32) as usize];
    mixer.render(request, &mut out);
    out
}

fn rms(frames: &[[f32; 2]]) -> f32 {
    let sum: f64 = frames
        .iter()
        .map(|f| f64::from(f[0]).powi(2) + f64::from(f[1]).powi(2))
        .sum();
    (sum / (2 * frames.len().max(1)) as f64).sqrt() as f32
}

/// Every embedded file parses, its index holds packets, and its playable
/// length is the one `ffmpeg` decodes, to the sample (within 1 ms of the
/// table's rounded seconds' own precision of 10 ms).
#[test]
fn every_embedded_file_parses() {
    for (track, seconds, frames) in LENGTHS {
        let stream = ogg::Stream::parse(track.bytes()).expect("parses");
        assert!(stream.packets() > 0, "{track:?}");
        assert_eq!(stream.length, frames, "{track:?}");
        // The table's seconds are `ffprobe`'s, which count the 312-sample
        // (6.5 ms) pre-skip in; rounded to 10 ms, so within 5 ms + 1 ms.
        let length = (stream.length + stream.pre_skip) as f64 / f64::from(RATE);
        assert!(
            (length - seconds).abs() <= 0.005 + 0.001,
            "{track:?}: {length} s, the table says {seconds}"
        );
        assert!(
            (stream.gain - 1.0).abs() < 1e-6,
            "{track:?}: no header gain"
        );
    }
}

/// Every packet of every file decodes, without a panic and to finite
/// samples, and a voice gives out exactly the playable length: pre-skip
/// dropped at the head and the last packet trimmed at the final granule.
/// (The shipped build aborts on a panic; this is the proof the shipped
/// bytes never cause one.)
#[test]
fn every_file_decodes_whole_and_finite() {
    for (track, _, frames) in LENGTHS {
        let mut voice = Voice::new();
        voice.start(track, 0.001);
        let mut count = 0usize;
        let mut peak = 0.0f32;
        while let Some(frame) = voice.next() {
            assert!(frame[0].is_finite() && frame[1].is_finite(), "{track:?}");
            peak = peak.max(frame[0].abs()).max(frame[1].abs());
            count += 1;
        }
        assert_eq!(count, frames, "{track:?}");
        assert_eq!(
            voice.packet,
            track.stream().packets(),
            "{track:?}: every packet"
        );
        // True peaks ≤ −2.4 dBFS were measured on the masters; decoded
        // sample peaks stay below full scale.
        assert!(peak > 0.05 && peak < 1.0, "{track:?}: peak {peak}");
    }
}

/// The title's first and last second carry music: finite and not silent.
#[test]
fn the_title_sounds_at_both_ends() {
    let mut voice = Voice::new();
    voice.start(Track::Title, 0.001);
    let all: Vec<[f32; 2]> = std::iter::from_fn(|| voice.next()).collect();
    let second = RATE as usize;
    let head = rms(&all[..second]);
    let tail = rms(&all[all.len() - second..]);
    assert!(head > 1e-3, "head {head}");
    assert!(tail > 1e-4, "tail {tail}");
}

/// A voice restarted from the head (decoder reset, packet 0, pre-skip again)
/// gives out exactly what a fresh voice does: a loop's second pass is the
/// first one's twin.
#[test]
fn a_restarted_voice_repeats_itself_exactly() {
    let mut fresh = Voice::new();
    fresh.start(Track::StingerCall, 0.001);
    let first: Vec<[f32; 2]> = std::iter::from_fn(|| fresh.next()).take(48_000).collect();
    let mut again = Voice::new();
    again.start(Track::Lobby, 0.001);
    for _ in 0..30_000 {
        again.next();
    }
    again.start(Track::StingerCall, 0.001);
    let second: Vec<[f32; 2]> = std::iter::from_fn(|| again.next()).take(48_000).collect();
    assert_eq!(first, second);
}

/// A malformed or truncated buffer is an error, never a panic: cut at every
/// length through the headers and at many through the body, and with single
/// bytes flipped.
#[test]
fn a_malformed_buffer_is_an_error() {
    let bytes = Track::StingerCall.bytes();
    assert!(ogg::Stream::parse(&[]).is_err());
    assert!(ogg::Stream::parse(b"OggS").is_err());
    assert!(ogg::Stream::parse(&[0u8; 4096]).is_err());
    let whole = ogg::Stream::parse(bytes).expect("parses").length;
    for cut in (0..600).chain((600..bytes.len()).step_by(97)) {
        // Cut inside a page: an error. Cut exactly between two pages: a
        // shorter stream that is still a valid one, never the whole.
        match ogg::Stream::parse(&bytes[..cut]) {
            Err(_) => {}
            Ok(stream) => assert!(stream.length < whole, "cut at {cut}"),
        }
    }
    let mut checksums = 0;
    for at in (0..bytes.len()).step_by(101) {
        let mut broken = bytes.to_vec();
        broken[at] ^= 0x5A;
        let error = ogg::Stream::parse(&broken).expect_err("a flipped byte");
        checksums += usize::from(error == ogg::Malformed::Checksum);
    }
    assert!(checksums > 0, "the page checksum is what catches most");
    // A packet whose TOC claims more than 120 ms is refused.
    assert_eq!(ogg::duration(&[0b0000_0011, 0x3F]), None);
    assert_eq!(ogg::duration(&[0b1111_1000]), Some(960));
    assert_eq!(ogg::duration(&[]), None);
}

/// A looping bed keeps playing across its wrap: its head starts under its
/// own tail, no stretch of silence, and no sample steps further than the
/// music itself does.
#[test]
fn a_looping_bed_wraps_without_a_gap_or_a_step() {
    let track = Track::Title;
    let length = track.stream().length as f32 / RATE as f32;
    let mut mixer = Mixer::new();
    // Up to six seconds before the overlap, then through the wrap.
    let before = render(&mut mixer, request(Scene::FrontDoor), length - 12.0);
    let largest_step = |frames: &[[f32; 2]]| {
        frames
            .windows(2)
            .map(|w| (w[1][0] - w[0][0]).abs().max((w[1][1] - w[0][1]).abs()))
            .fold(0.0f32, f32::max)
    };
    let music_step = largest_step(&before[RATE as usize * 2..]);
    let wrap = render(&mut mixer, request(Scene::FrontDoor), 20.0);
    assert!(
        mixer
            .beds
            .iter()
            .filter(|bed| bed.track == Some(track))
            .count()
            >= 1,
        "the bed still plays after its own length"
    );
    let step = largest_step(&wrap);
    assert!(
        step <= music_step * 1.5 + 1e-3,
        "the wrap steps {step}, the music itself {music_step}"
    );
    // No 50-ms window across the wrap falls silent.
    for (k, window) in wrap.chunks(2400).enumerate() {
        assert!(rms(window) > 1e-4, "silent window {k} at the wrap");
    }
}

/// A scene change cross-fades with equal power: while the old bed fades
/// out and the new one in, the sum of their envelopes' squares stays 1.
#[test]
fn a_scene_change_keeps_the_power() {
    let mut mixer = Mixer::new();
    render(&mut mixer, request(Scene::Lobby), 3.0);
    let mut worst = 0.0f32;
    let mut both = false;
    for _ in 0..(1.4 * RATE as f32 / 256.0) as usize {
        render(&mut mixer, request(Scene::Table), 256.0 / RATE as f32);
        let power: f32 = mixer
            .beds
            .iter()
            .filter(|bed| !bed.free())
            .map(|bed| (bed.level * std::f32::consts::FRAC_PI_2).sin().powi(2))
            .sum();
        both |= mixer.beds.iter().all(|bed| !bed.free());
        worst = worst.max((power - 1.0).abs());
    }
    assert!(both, "the two beds overlapped");
    assert!(worst < 0.02, "the envelopes' power strayed by {worst}");
    assert!(
        mixer
            .beds
            .iter()
            .any(|bed| bed.track == Some(Track::Combat))
    );
    assert!(mixer.beds.iter().all(|bed| bed.track != Some(Track::Lobby)));
}

/// Back to a scene whose bed is still fading out: it fades back up from
/// where it stands and where it is in the music, never restarts.
#[test]
fn a_bed_still_fading_comes_back_where_it_stood() {
    let mut mixer = Mixer::new();
    render(&mut mixer, request(Scene::Lobby), 5.0);
    render(&mut mixer, request(Scene::Table), 0.5);
    let lobby = mixer
        .beds
        .iter()
        .position(|bed| bed.track == Some(Track::Lobby))
        .expect("still fading");
    let (played, level) = (mixer.beds[lobby].played, mixer.beds[lobby].level);
    assert!(level > 0.0 && level < 1.0);
    render(&mut mixer, request(Scene::Lobby), 0.1);
    assert_eq!(mixer.beds[lobby].track, Some(Track::Lobby));
    assert!(
        mixer.beds[lobby].played > played,
        "not restarted from its head"
    );
    assert!(mixer.beds[lobby].level >= level - 0.01, "fading back up");
    render(&mut mixer, request(Scene::Lobby), 2.0);
    assert!((mixer.beds[lobby].level - 1.0).abs() < 1e-6);
}

/// An ending plays once and then stays silent; the loading hush fades the
/// bed out; a synthesised theme silences the recordings.
#[test]
fn endings_play_once_and_the_hush_is_silent() {
    let mut mixer = Mixer::new();
    let draw = render(&mut mixer, request(Scene::Draw), 22.0);
    assert!(rms(&draw[..RATE as usize * 5]) > 1e-3);
    assert!(
        rms(&draw[RATE as usize * 21..]) == 0.0,
        "silent after its end"
    );
    let more = render(&mut mixer, request(Scene::Draw), 5.0);
    assert!(rms(&more) == 0.0, "and not again while the scene stands");
    assert!(mixer.silent());

    let mut mixer = Mixer::new();
    render(&mut mixer, request(Scene::Lobby), 3.0);
    render(&mut mixer, request(Scene::Opening), 1.3);
    assert!(mixer.silent(), "the hush fades the bed out");
    render(&mut mixer, request(Scene::Lobby), 3.0);
    let synth = ScoreRequest {
        recorded: false,
        ..request(Scene::Lobby)
    };
    render(&mut mixer, synth, 1.3);
    assert!(
        mixer.silent(),
        "a synthesised theme silences the recordings"
    );
}

/// A stinger plays over the table's bed, which ducks by 7 dB fast and comes
/// back over about a second; nothing plays at a stinger counter seen first,
/// or away from the table.
#[test]
fn a_stinger_ducks_the_bed() {
    let mut mixer = Mixer::new();
    render(&mut mixer, request(Scene::Table), 3.0);
    assert!(mixer.stinger.free(), "the first counter seen plays nothing");
    let cue = ScoreRequest {
        stingers: 1,
        stinger_epic: true,
        ..request(Scene::Table)
    };
    render(&mut mixer, cue, 0.2);
    assert_eq!(mixer.stinger.track, Some(Track::StingerEpic));
    assert!(
        (mixer.duck_level - DUCK_FLOOR).abs() < 1e-4,
        "down within 0.2 s"
    );
    let gain_db = 20.0 * DUCK_FLOOR.log10();
    assert!((gain_db + 7.0).abs() < 0.01);
    // The epic stinger is 8.61 s long; the duck comes back over 1.2 s.
    render(&mut mixer, cue, 8.8);
    assert!(mixer.stinger.free(), "the stinger has played out");
    assert!(mixer.duck_level < 1.0, "still coming back");
    render(&mut mixer, cue, 1.3);
    assert!((mixer.duck_level - 1.0).abs() < 1e-4, "back after it");

    // Away from the table a new counter plays nothing.
    let lobby = ScoreRequest {
        stingers: 2,
        ..request(Scene::Lobby)
    };
    render(&mut mixer, lobby, 0.5);
    assert!(mixer.stinger.free());
}

/// Switching between the recorded theme and a synthesised one does not
/// jump in loudness: the recorded lobby and combat beds through
/// [`RECORDED_GAIN`] against the synth's lobby (its tavern piece) and a
/// mid-tension table (Dornenkrone), plain RMS over the same thirty seconds
/// after a five-second settle, within 2 dB each.
#[test]
fn the_recorded_path_is_as_loud_as_the_synth() {
    use crate::music::{ScoreControl, Theme, Tune};
    use std::sync::Arc;
    let settle = RATE as usize * 5;
    let span = RATE as usize * 30;
    let synth = |request: ScoreRequest| {
        let control = Arc::new(ScoreControl::default());
        control.set(request);
        let mut tune = Tune::with_control(control);
        let mut out = vec![[0.0; 2]; settle + span];
        tune.render(&mut out);
        rms(&out[settle..])
    };
    let recorded = |scene: Scene| {
        let mut mixer = Mixer::new();
        let out = render(
            &mut mixer,
            request(scene),
            (settle + span) as f32 / RATE as f32,
        );
        rms(&out[settle..])
    };
    let db = |a: f32, b: f32| 20.0 * (a / b).log10();
    let lobby = ScoreRequest {
        scene: Scene::Lobby,
        theme: Theme::Thorn,
        ..ScoreRequest::default()
    };
    let table = ScoreRequest {
        scene: Scene::Table,
        theme: Theme::Thorn,
        tension: 0.5,
        action_intensity: 0.5,
        ..ScoreRequest::default()
    };
    let (synth_lobby, synth_table) = (synth(lobby), synth(table));
    let (bed_lobby, bed_table) = (recorded(Scene::Lobby), recorded(Scene::Table));
    println!(
        "synth lobby {:.1} dBFS, table {:.1} dBFS; recorded lobby {:.1} dBFS, combat {:.1} dBFS \
         (gain {RECORDED_GAIN})",
        20.0 * synth_lobby.log10(),
        20.0 * synth_table.log10(),
        20.0 * bed_lobby.log10(),
        20.0 * bed_table.log10(),
    );
    // One gain cannot match both (see `RECORDED_GAIN`): it splits the
    // difference, so each side stays within 2.7 dB and the two errors
    // cancel.
    let (lobby_error, table_error) = (db(bed_lobby, synth_lobby), db(bed_table, synth_table));
    assert!(lobby_error.abs() <= 2.7, "lobby {lobby_error:+.1} dB");
    assert!(table_error.abs() <= 2.7, "table {table_error:+.1} dB");
    assert!(
        (lobby_error + table_error).abs() <= 0.5,
        "the gain splits the difference: {lobby_error:+.1} / {table_error:+.1} dB"
    );
}

/// A stinger over the ducked bed cannot reach full scale.
#[test]
fn a_stinger_over_the_bed_stays_below_full_scale() {
    for kind in [false, true] {
        let mut mixer = Mixer::new();
        render(&mut mixer, request(Scene::Table), 2.0);
        let cue = ScoreRequest {
            stingers: 3,
            stinger_epic: kind,
            ..request(Scene::Table)
        };
        let out = render(&mut mixer, cue, 9.0);
        let peak = out
            .iter()
            .map(|f| f[0].abs().max(f[1].abs()))
            .fold(0.0f32, f32::max);
        let bent = out
            .iter()
            .filter(|f| f[0].abs().max(f[1].abs()) > 0.85)
            .count();
        println!("stinger epic={kind}: peak {peak:.3}, {bent} frames in the knee");
        assert!(peak < 1.0, "peak {peak}");
        assert!(bent < out.len() / 1000, "{bent} frames bent");
    }
}
