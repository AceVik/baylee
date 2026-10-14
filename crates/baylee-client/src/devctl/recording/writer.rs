//! Background-only PNG and ledger writes; a failed append poisons the worker.

use super::{Event, Sample, Work};
use bevy::prelude::Image;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Receiver;

/// Persist frames and the ledger on the bounded background worker.
pub(super) fn write_frames(
    directory: &std::path::Path,
    initial: &str,
    jobs: &Receiver<Work>,
    events: &std::sync::mpsc::Sender<Event>,
    cancelled: &AtomicBool,
) -> Result<(), String> {
    let mut builder = std::fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder
        .recursive(false)
        .create(directory)
        .map_err(|e| e.to_string())?;
    std::fs::write(directory.join("recording.json"), initial).map_err(|e| e.to_string())?;
    let mut ledger =
        std::fs::File::create(directory.join("frames.jsonl")).map_err(|e| e.to_string())?;
    events.send(Event::Ready).map_err(|e| e.to_string())?;
    let mut write_error = None;
    for work in jobs {
        match work {
            Work::Frame(metadata, image) => {
                let slot = metadata.slot;
                let result = write_or_cancel(&mut write_error, cancelled, || {
                    save_frame(directory, &mut ledger, &metadata, *image)
                });
                events
                    .send(Event::Written(slot, result))
                    .map_err(|e| e.to_string())?;
            }
            Work::Finish(json) => {
                let result = std::fs::write(directory.join("recording.json.tmp"), json)
                    .and_then(|()| {
                        std::fs::rename(
                            directory.join("recording.json.tmp"),
                            directory.join("recording.json"),
                        )
                    })
                    .map_err(|e| e.to_string());
                let _ = events.send(Event::Finished(result));
                return Ok(());
            }
        }
    }
    Ok(())
}

// A partial ledger append poisons this worker: no later frame may append
// behind a truncated row, even if it was already queued before the error.
pub(super) fn write_or_cancel(
    write_error: &mut Option<String>,
    cancelled: &AtomicBool,
    save: impl FnOnce() -> Result<(), String>,
) -> Result<bool, String> {
    if let Some(error) = write_error {
        return Err(format!("writer stopped after: {error}"));
    }
    if cancelled.load(Ordering::Relaxed) {
        return Ok(false);
    }
    match save() {
        Ok(()) => Ok(true),
        Err(error) => {
            *write_error = Some(error.clone());
            Err(error)
        }
    }
}

fn save_frame(
    directory: &std::path::Path,
    ledger: &mut std::fs::File,
    metadata: &Sample,
    image: Image,
) -> Result<(), String> {
    let dynamic = image.try_into_dynamic().map_err(|e| e.to_string())?;
    let temp = directory.join(format!("{}.tmp", metadata.file));
    dynamic
        .save_with_format(&temp, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    std::fs::rename(temp, directory.join(&metadata.file)).map_err(|e| e.to_string())?;
    // Serialize before touching the ledger, then commit the row with its
    // newline. Recovery accepts only the valid newline-terminated prefix.
    let mut line = serde_json::to_vec(metadata).map_err(|e| e.to_string())?;
    line.push(b'\n');
    ledger.write_all(&line).map_err(|e| e.to_string())
}
