//! The screenshot a report may carry: taken as the form opens, kept on this
//! machine unless its box is ticked.
//!
//! PNG only, shrunk until it fits a budget well inside the 2 MB the gateway
//! takes for the whole `client` object; a picture that will not fit even
//! small is dropped rather than sent cut. Native only: a browser build has
//! no encoder linked, and its form says there is no picture to send.

use bevy::prelude::*;

/// The most the encoded picture may weigh, before base64.
#[cfg(not(target_arch = "wasm32"))]
const BUDGET: usize = 900_000;

/// The widths tried, widest first.
#[cfg(not(target_arch = "wasm32"))]
const WIDTHS: [u32; 3] = [1280, 960, 640];

/// Asks the renderer for the window as it is now. `true` when a picture is
/// on its way.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn take(commands: &mut Commands) -> bool {
    use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
    commands.spawn(Screenshot::primary_window()).observe(
        |captured: On<ScreenshotCaptured>, mut desk: ResMut<super::ReportDesk>| {
            desk.shooting = false;
            desk.gathered.screenshot = encode(captured.image.clone());
        },
    );
    true
}

/// A browser build takes no picture.
#[cfg(target_arch = "wasm32")]
pub(super) fn take(_commands: &mut Commands) -> bool {
    false
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
