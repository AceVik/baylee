//! macOS Dock identity, including a client started directly through Cargo.
use bevy::{ecs::system::NonSendMarker, prelude::*};
use objc2::{AllocAnyThread, MainThreadMarker};
use objc2_app_kit::{NSApplication, NSImage};
use objc2_foundation::NSData;

/// The same original icon that the release bundle installs in Resources.
const ICON: &[u8] = include_bytes!("../assets/brand/baylee.icns");

/// Bevy runs this on the main thread after Winit creates the application.
pub(super) fn install(_main_thread: NonSendMarker) {
    let Some(main_thread) = MainThreadMarker::new() else {
        warn!("Baylee app icon requires the main thread");
        return;
    };
    let data = NSData::with_bytes(ICON);
    let Some(icon) = NSImage::initWithData(NSImage::alloc(), &data) else {
        warn!("Could not decode the Baylee app icon");
        return;
    };
    let app = NSApplication::sharedApplication(main_thread);
    let previous = app
        .applicationIconImage()
        .and_then(|i| i.TIFFRepresentation());
    set_icon(&app, &icon);
    // AppKit may copy the NSImage; compare its image data, not object identity.
    let installed = app
        .applicationIconImage()
        .and_then(|i| i.TIFFRepresentation());
    let expected = icon.TIFFRepresentation();
    if installed
        .zip(expected)
        .is_some_and(|(a, b)| a == b || previous.is_some_and(|previous| a != previous))
    {
        info!("Baylee app icon installed and verified");
    } else {
        warn!("macOS did not retain the Baylee app icon");
    }
}

/// A non-null image makes `AppKit`'s nullable setter safe for this caller.
#[allow(unsafe_code)] // AppKit binding marks only the nullable argument unsafe.
fn set_icon(app: &NSApplication, icon: &NSImage) {
    // SAFETY: the binding requires a non-null image; the reference guarantees
    // one and Some never passes nil. NSApplication is main-thread-only.
    unsafe { app.setApplicationIconImage(Some(icon)) };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_icon_decodes_to_a_real_image() {
        // Decode the exact embedded bytes before any call can reach the setter.
        let data = NSData::with_bytes(ICON);
        let icon = NSImage::initWithData(NSImage::alloc(), &data).expect("valid app icon");
        let size = icon.size();
        assert!(size.width > 0.0 && size.height > 0.0);
    }
}
