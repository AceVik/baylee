//! A moving picture's frames, decoded in a child process.
//!
//! The only AV1 decoder is rav1d (`baylee_avif::Player`), which runs through
//! a C interface and assembly. A picture is another player's upload served by
//! Central, so a damaged or hostile bitstream must not be able to take the
//! client down (`baylee-avif`'s security note; `docs/legal.md` §"The vista
//! loops as AVIF, and their decoder"). So it is decoded outside the client's
//! process, by this executable started again as `--picture-frames`, and the
//! client reads back nothing but pixels:
//!
//! - the parent reads the file first with the safe-Rust [`Sequence::read`]
//!   within [`Bounds::PICTURE`] (256 px square, at most 100 frames, 512 KiB,
//!   10 s) and keeps the frames' durations from its own reading;
//! - the child gets the file as one length-prefixed frame on stdin, decodes
//!   it within the same bounds, and writes each frame as exactly
//!   256 × 256 × 4 bytes of RGBA, nothing else, then ends;
//! - the parent reads exactly as many frames as its own reading counted,
//!   then wants the end of the stream and a clean exit, all within
//!   [`DEADLINE`]; any other byte, a crash, a hang, a short frame or a
//!   failed exit refuses the picture, and the child is killed and reaped
//!   whatever happened. The still stands instead.
//!
//! The child starts with an empty environment, its stderr discarded, and
//! takes no argument but its flag; nothing from the network names it.

use std::io::{self, Read, Write};
use std::process::{Command, Stdio};
use std::sync::{Arc, mpsc};
use std::time::{Duration, Instant};

use baylee_avif::{Bounds, Player, Sequence};
use baylee_client_core::portrait::{KEPT_SIDE, SERVED_SIDE, halve};

/// The flag that starts this executable as the decoder.
pub const FLAG: &str = "--picture-frames";

/// How long the child may take from its start to its exit: decoding 100
/// frames of 256 px takes well under a second.
pub const DEADLINE: Duration = Duration::from_secs(10);

/// One decoded frame as the child writes it.
pub const FRAME_BYTES: usize = (SERVED_SIDE as usize) * (SERVED_SIDE as usize) * 4;

/// A moving picture's frames at [`KEPT_SIDE`], with the durations the
/// parent's own reading gave them.
#[derive(Debug, PartialEq, Eq)]
pub struct Frames {
    /// Each frame, `KEPT_SIDE² × 4` bytes of straight RGBA.
    pub rgba: Vec<Vec<u8>>,
    /// How long each is shown, in milliseconds.
    pub durations: Vec<u32>,
}

/// Why no frames came.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Refused {
    /// The file itself, by the parent's reading.
    File,
    /// The child: it could not start, crashed, hung, or wrote anything but
    /// the frames.
    Child,
}

/// Runs the decoder when this process was started as one, and says so; the
/// caller then returns before opening anything. An invocation with anything
/// beside the flag never falls through into the client.
#[must_use]
pub fn helper_if_asked() -> bool {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new(FLAG)) {
        return false;
    }
    let ok = args.next().is_none()
        && serve(
            &mut io::stdin().lock(),
            &mut io::BufWriter::new(io::stdout().lock()),
        )
        .is_ok();
    std::process::exit(i32::from(!ok));
}

/// The child's work: one sequence in, its frames out (module docs).
///
/// # Errors
/// [`Refused::File`]: a file outside the bounds, a frame the decoder
/// refuses or that is not 256 px square, or a pipe that fails.
pub fn serve(input: &mut impl Read, output: &mut impl Write) -> Result<(), Refused> {
    let bytes =
        baylee_worker::frame::read(input, Bounds::PICTURE.max_bytes).map_err(|_| Refused::File)?;
    let mut player =
        Player::open(Arc::from(bytes), &Bounds::PICTURE, 1).map_err(|_| Refused::File)?;
    let mut frame = baylee_avif::Frame::default();
    let mut rgba = Vec::with_capacity(FRAME_BYTES);
    for _ in 0..player.frames() {
        player.advance(&mut frame).map_err(|_| Refused::File)?;
        if frame.width != SERVED_SIDE || frame.height != SERVED_SIDE {
            return Err(Refused::File);
        }
        frame.to_rgba(&mut rgba);
        if rgba.len() != FRAME_BYTES {
            return Err(Refused::File);
        }
        output.write_all(&rgba).map_err(|_| Refused::File)?;
    }
    output.flush().map_err(|_| Refused::File)
}

/// The command that starts this executable as the decoder.
///
/// # Errors
/// The executable cannot be found.
pub fn command() -> io::Result<Command> {
    let mut command = Command::new(std::env::current_exe()?);
    command.arg(FLAG);
    Ok(command)
}

/// The durations of `bytes`' frames by the parent's own safe reading, or
/// `None` for a file outside [`Bounds::PICTURE`].
#[must_use]
pub fn durations(bytes: &[u8]) -> Option<Vec<u32>> {
    let sequence = Sequence::read(bytes, &Bounds::PICTURE).ok()?;
    let track = &sequence.colour;
    if track.width != SERVED_SIDE || track.height != SERVED_SIDE || track.samples.is_empty() {
        return None;
    }
    track
        .samples
        .iter()
        .map(|sample| u32::try_from(sample.duration_ms(track.timescale)).ok())
        .collect()
}

/// Decodes `bytes`, a moving picture's sequence, with `command` (this
/// executable as [`command`] makes it, or a stand-in in a test) within
/// `deadline`.
///
/// # Errors
/// [`Refused::File`] when the parent's own reading refuses the file,
/// [`Refused::Child`] for anything the child did wrong.
pub fn decode(mut command: Command, bytes: &[u8], deadline: Duration) -> Result<Frames, Refused> {
    let durations = durations(bytes).ok_or(Refused::File)?;
    let count = durations.len();
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .env_clear();
    #[cfg(windows)]
    if let Some(root) = std::env::var_os("SystemRoot") {
        command.env("SystemRoot", root);
    }
    let mut child = command.spawn().map_err(|_| Refused::Child)?;
    let started = Instant::now();
    let (Some(mut stdin), Some(mut stdout)) = (child.stdin.take(), child.stdout.take()) else {
        let _ = child.kill();
        let _ = child.wait();
        return Err(Refused::Child);
    };
    // The watchdog owns the child: it kills it at the deadline, or at once
    // when the reader gives up, and always reaps it.
    let (done, finished) = mpsc::channel::<bool>();
    let watchdog = std::thread::Builder::new()
        .name("baylee-picture-watch".into())
        .spawn(move || {
            let read_well = finished
                .recv_timeout(deadline.saturating_sub(started.elapsed()))
                .unwrap_or(false);
            if read_well {
                // The frames are in: the child has what is left of its time
                // to end on its own.
                loop {
                    match child.try_wait() {
                        Ok(Some(status)) => return status.success(),
                        Ok(None) if started.elapsed() < deadline => {
                            std::thread::sleep(Duration::from_millis(5));
                        }
                        _ => break,
                    }
                }
            }
            let _ = child.kill();
            let _ = child.wait();
            false
        });
    let Ok(watchdog) = watchdog else {
        // `child` went into the closure that never ran; dropping it neither
        // kills nor waits, so close its pipes and let it end on its own.
        return Err(Refused::Child);
    };
    let file = bytes.to_vec();
    let writer = std::thread::Builder::new()
        .name("baylee-picture-feed".into())
        .spawn(move || {
            let _ = baylee_worker::frame::write(&mut stdin, &file, Bounds::PICTURE.max_bytes);
            // Dropping stdin closes the pipe: the child sees the end.
        });
    let frames = read_frames(&mut stdout, count);
    let _ = done.send(frames.is_ok());
    drop(stdout);
    let exited_well = watchdog.join().unwrap_or(false);
    if let Ok(writer) = writer {
        let _ = writer.join();
    }
    match frames {
        Ok(rgba) if exited_well => Ok(Frames { rgba, durations }),
        _ => Err(Refused::Child),
    }
}

/// Reads exactly `count` frames of [`FRAME_BYTES`] and then the end of the
/// stream, halving each frame to [`KEPT_SIDE`] as it comes.
///
/// # Errors
/// [`Refused::Child`]: a short frame, a byte after the last, or a failing
/// pipe.
pub fn read_frames(stdout: &mut impl Read, count: usize) -> Result<Vec<Vec<u8>>, Refused> {
    let mut frames = Vec::with_capacity(count);
    let mut frame = vec![0u8; FRAME_BYTES];
    for _ in 0..count {
        stdout.read_exact(&mut frame).map_err(|_| Refused::Child)?;
        frames.push(halve(&frame, SERVED_SIDE).ok_or(Refused::Child)?);
    }
    let mut more = [0u8; 1];
    loop {
        match stdout.read(&mut more) {
            Ok(0) => break,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            _ => return Err(Refused::Child),
        }
    }
    debug_assert!(
        frames
            .iter()
            .all(|f| f.len() == (KEPT_SIDE * KEPT_SIDE * 4) as usize)
    );
    Ok(frames)
}
