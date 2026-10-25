//! The screenshot a report may carry: taken as the form opens, kept on this
//! machine unless its box is ticked.
//!
//! PNG only, shrunk until it fits a budget well inside the 2 MB the gateway
//! takes for the whole `client` object; a picture that will not fit even
//! small is dropped rather than sent cut. Native only: a browser build has
//! no encoder linked, and its form says there is no picture to send.

use bevy::prelude::*;

/// Whether this build takes a picture of its window at all: the form says
/// so at the screenshot box where it does not.
pub(super) const TAKES_PICTURES: bool = cfg!(not(target_arch = "wasm32"));

/// The most the encoded picture may weigh, before base64.
#[cfg(not(target_arch = "wasm32"))]
const BUDGET: usize = 900_000;

/// The widths tried, widest first.
#[cfg(not(target_arch = "wasm32"))]
const WIDTHS: [u32; 3] = [1280, 960, 640];

/// The renderer's request and a bounded wait for its readback.
pub(super) struct Capture {
    entity: Entity,
    frames: u8,
}

/// Cancels an old or timed-out readback. Its observer also checks identity,
/// so a late answer cannot capture the dialog or overwrite a newer report.
pub(super) fn cancel(commands: &mut Commands, desk: &mut super::ReportDesk) {
    if let Some(capture) = desk.capture.take() {
        commands.entity(capture.entity).try_despawn();
        desk.shooting = false;
    }
}

/// Keep the form out of the frame until readback, but never strand the player
/// when the renderer cannot produce a screenshot (including headless tests).
pub(super) fn awaiting_frame(commands: &mut Commands, desk: &mut super::ReportDesk) -> bool {
    if !desk.open {
        cancel(commands, desk);
        return false;
    }
    let Some(capture) = desk.capture.as_mut() else {
        return false;
    };
    capture.frames += 1;
    if capture.frames <= 30 {
        return true;
    }
    cancel(commands, desk);
    false
}

/// Asks for the current frame. Encoding can continue while the form is up;
/// only the GPU readback must precede drawing it.
#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::unnecessary_wraps)] // the browser implementation returns None
pub(super) fn take(commands: &mut Commands, opening: u64) -> Option<Capture> {
    use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
    let entity = commands
        .spawn(Screenshot::primary_window())
        .observe(
            move |captured: On<ScreenshotCaptured>, mut desk: ResMut<super::ReportDesk>| {
                if !desk.open
                    || desk.opening != opening
                    || desk
                        .capture
                        .as_ref()
                        .is_none_or(|capture| capture.entity != captured.entity)
                {
                    return;
                }
                desk.capture = None;
                let image = captured.image.clone();
                desk.shot_encoding = Some(
                    bevy::tasks::AsyncComputeTaskPool::get()
                        .spawn(async move { (opening, encode(image)) }),
                );
            },
        )
        .id();
    Some(Capture { entity, frames: 0 })
}

/// A browser build takes no picture.
#[cfg(target_arch = "wasm32")]
pub(super) fn take(_commands: &mut Commands, _opening: u64) -> Option<Capture> {
    None
}

/// The frame as a PNG under [`BUDGET`], or `None`.
#[cfg(not(target_arch = "wasm32"))]
fn encode(image: Image) -> Option<baylee_client_core::bugreport::Screenshot> {
    let frame = image.try_into_dynamic().ok()?.to_rgb8();
    for width in WIDTHS {
        let small = if frame.width() > width {
            let height = frame.height() * width / frame.width().max(1);
            image::imageops::thumbnail(&frame, width, height.max(1))
        } else {
            frame.clone()
        };
        let mut bytes = std::io::Cursor::new(Vec::new());
        if small.write_to(&mut bytes, image::ImageFormat::Png).is_err() {
            return None;
        }
        let bytes = bytes.into_inner();
        if bytes.len() <= BUDGET {
            return Some(baylee_client_core::bugreport::Screenshot {
                width: small.width(),
                height: small.height(),
                png_base64: baylee_client_core::bugreport::base64_encode(&bytes),
            });
        }
    }
    None
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use bevy::render::view::screenshot::ScreenshotCaptured;

    #[test]
    fn readback_releases_the_form_before_encoding_finishes() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<super::super::ReportDesk>();
        let capture = take(&mut app.world_mut().commands(), 1).unwrap();
        let entity = capture.entity;
        {
            let mut desk = app.world_mut().resource_mut::<super::super::ReportDesk>();
            desk.open = true;
            desk.opening = 1;
            desk.shooting = true;
            desk.capture = Some(capture);
        }
        app.world_mut().flush();
        app.world_mut().trigger(ScreenshotCaptured {
            entity,
            image: Image::default(),
        });
        let desk = app.world().resource::<super::super::ReportDesk>();
        assert!(desk.capture.is_none());
        assert!(desk.shot_encoding.is_some());
        assert!(desk.shooting);
    }
}
