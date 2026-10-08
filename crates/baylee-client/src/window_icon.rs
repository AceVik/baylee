//! The window's own icon on Windows and X11, and the names that tie the
//! running window to the entry a player started it from.
//!
//! Bevy 0.19 has no window icon, so [`install`] sets it through winit on the
//! main thread once the primary window exists. Windows: the icon resource
//! `build.rs` embeds, loaded at the size each place draws (title bar and
//! taskbar), so each is the `.ico`'s crisp entry rather than a scaled one.
//! X11: `baylee-window.png` as `_NET_WM_ICON`. Wayland has no window icon in
//! winit 0.30; its shell finds the icon through the desktop entry whose name
//! is the window's `app_id`, [`APP_ID`]. macOS takes the application's icon
//! (`app_icon.rs`), and a phone or a browser has none.
//!
//! On Windows the program a player starts is the launcher, which starts this
//! client as another executable (`baylee-runtime.exe`, a different one after
//! an update). [`claim_taskbar_identity`] gives the process the same
//! `AppUserModelID` the setup writes on its shortcuts (`baylee.iss`), so the
//! window joins a pinned Baylee button instead of opening a second one with
//! a pin that would bypass the launcher.

/// The desktop entry's name (`baylee.desktop`): the window's Wayland
/// `app_id` and X11 `WM_CLASS`, which is how a Linux dock pairs the window
/// with the entry's icon. `scripts/installers/linux/baylee.desktop` says the
/// same (`StartupWMClass`), and a test holds the two together.
pub const APP_ID: &str = "baylee";

/// The Windows `AppUserModelID`, the same as the `AppUserModelID:` of the
/// setup's shortcuts (`scripts/installers/windows/baylee.iss`).
#[cfg(any(windows, test))]
pub const WINDOWS_APP_USER_MODEL_ID: &str = "AceVik.Baylee";

/// The icon resource's ordinal, as `build.rs` names it (`set_icon`).
#[cfg(windows)]
const RESOURCE: u16 = 1;

/// The window icon an X11 client sets on itself (128 px, `make-art.py`).
#[cfg(any(target_os = "linux", all(test, not(target_arch = "wasm32"))))]
const PNG: &[u8] = include_bytes!("../assets/brand/baylee-window.png");

/// The X11 icon as straight RGBA, with its width and height.
#[cfg(any(target_os = "linux", all(test, not(target_arch = "wasm32"))))]
fn rgba() -> Option<(Vec<u8>, u32, u32)> {
    let image = image::load_from_memory_with_format(PNG, image::ImageFormat::Png)
        .ok()?
        .into_rgba8();
    let (width, height) = image.dimensions();
    Some((image.into_raw(), width, height))
}

/// Sets the primary window's icon once winit has made the window; runs every
/// frame until then, and is a single flag read afterwards.
#[cfg(any(windows, target_os = "linux"))]
pub(crate) fn install(
    mut done: bevy::prelude::Local<bool>,
    primary: bevy::prelude::Query<
        bevy::prelude::Entity,
        bevy::prelude::With<bevy::window::PrimaryWindow>,
    >,
    // winit's windows live on the main thread.
    _main_thread: bevy::ecs::system::NonSendMarker,
) {
    if *done {
        return;
    }
    let Ok(entity) = primary.single() else {
        return;
    };
    bevy::winit::WINIT_WINDOWS.with_borrow(|windows| {
        if let Some(window) = windows.get_window(entity) {
            *done = true;
            apply(window);
        }
    });
}

#[cfg(windows)]
fn apply(window: &winit::window::Window) {
    use winit::dpi::PhysicalSize;
    use winit::platform::windows::{IconExtWindows, WindowExtWindows};
    use winit::window::Icon;
    // The title bar draws the small icon (16 pt), the taskbar and Alt-Tab
    // the big one (32 pt); Windows scales neither well, so each is loaded at
    // the window's own scale.
    let scale = window.scale_factor();
    let at = |points: f64| {
        // Clamped to what the .ico holds, so the cast cannot wrap.
        let side = (points * scale).round().clamp(16.0, 256.0) as u32;
        Icon::from_resource(RESOURCE, Some(PhysicalSize::new(side, side)))
    };
    match (at(16.0), at(32.0)) {
        (Ok(small), Ok(big)) => {
            window.set_window_icon(Some(small));
            window.set_taskbar_icon(Some(big));
            bevy::log::info!("window icon set from the executable's resource");
        }
        (Err(err), _) | (_, Err(err)) => {
            bevy::log::warn!("the executable carries no usable icon resource: {err}");
        }
    }
}

#[cfg(target_os = "linux")]
fn apply(window: &winit::window::Window) {
    let icon = rgba()
        .and_then(|(rgba, width, height)| winit::window::Icon::from_rgba(rgba, width, height).ok());
    if let Some(icon) = icon {
        // A no-op under Wayland, where the desktop entry gives the icon.
        window.set_window_icon(Some(icon));
    } else {
        bevy::log::warn!("the bundled window icon did not decode");
    }
}

/// Gives this process the shortcuts' `AppUserModelID` (see the module doc).
/// Called before the first window exists, as Windows asks.
#[cfg(windows)]
pub(crate) fn claim_taskbar_identity() {
    let id = wide(WINDOWS_APP_USER_MODEL_ID);
    // SAFETY: `id` is a NUL-terminated UTF-16 string (`wide`, tested below)
    // that outlives the call; the function copies it and keeps no pointer.
    #[allow(unsafe_code)] // One Win32 call, no safe binding exists.
    let result = unsafe {
        windows_sys::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID(id.as_ptr())
    };
    if result != 0 {
        bevy::log::warn!("Windows refused the AppUserModelID ({result:#x})");
    }
}

/// `text` as NUL-terminated UTF-16; it may hold no NUL of its own.
#[cfg(any(windows, test))]
fn wide(text: &str) -> Vec<u16> {
    assert!(!text.contains('\0'), "an AppUserModelID has no NUL");
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn the_window_icon_decodes_to_a_square_with_clear_corners() {
        let (rgba, width, height) = rgba().expect("the bundled window icon decodes");
        assert_eq!((width, height), (128, 128));
        assert_eq!(rgba.len(), 128 * 128 * 4);
        // Rounded: the corner pixel is transparent, the centre opaque.
        assert_eq!(rgba[3], 0, "the top-left corner is transparent");
        let centre = (64 * 128 + 64) * 4;
        assert_eq!(rgba[centre + 3], 255, "the centre is opaque");
    }

    #[test]
    fn the_app_user_model_id_is_a_terminated_wide_string() {
        let id = wide(WINDOWS_APP_USER_MODEL_ID);
        assert_eq!(id.last(), Some(&0));
        assert_eq!(id.iter().filter(|&&unit| unit == 0).count(), 1);
        // Windows' limit is 128 characters, and no spaces.
        assert!(WINDOWS_APP_USER_MODEL_ID.len() <= 128);
        assert!(!WINDOWS_APP_USER_MODEL_ID.contains(' '));
    }

    #[cfg(windows)]
    #[test]
    fn windows_takes_the_app_user_model_id() {
        let id = wide(WINDOWS_APP_USER_MODEL_ID);
        // SAFETY: as in `claim_taskbar_identity`.
        #[allow(unsafe_code)]
        let result = unsafe {
            windows_sys::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID(id.as_ptr())
        };
        assert_eq!(result, 0);
    }

    /// The icon files the installers ship, read the way the systems read
    /// them: by their headers, so no image library is needed to hold every
    /// size present (`scripts/installers/make-art.py` writes them all).
    #[cfg(not(target_arch = "wasm32"))]
    mod shipped {
        use super::{APP_ID, WINDOWS_APP_USER_MODEL_ID};
        use std::path::PathBuf;

        fn repo(path: &str) -> PathBuf {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(path)
        }

        fn read(path: &str) -> Vec<u8> {
            std::fs::read(repo(path)).unwrap_or_else(|err| panic!("{path}: {err}"))
        }

        fn text(path: &str) -> String {
            String::from_utf8(read(path)).expect("UTF-8")
        }

        /// A PNG's width and height and whether it has an alpha channel.
        fn png(bytes: &[u8]) -> (u32, u32, bool) {
            assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "a PNG");
            assert_eq!(&bytes[12..16], b"IHDR");
            let width = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
            let height = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
            // Colour types 4 and 6 carry alpha (a tRNS chunk could too; the
            // script writes RGBA).
            (width, height, matches!(bytes[25], 4 | 6))
        }

        #[test]
        fn linux_has_every_hicolor_size_with_clear_corners() {
            for n in [16, 24, 32, 48, 64, 128, 256, 512] {
                let path = format!("scripts/installers/linux/hicolor/{n}x{n}/apps/{APP_ID}.png");
                assert_eq!(png(&read(&path)), (n, n, true), "{path}");
            }
            let appimage = png(&read(&format!("scripts/installers/linux/{APP_ID}.png")));
            assert_eq!(appimage, (256, 256, true));
        }

        #[test]
        fn the_desktop_entry_is_named_for_the_window() {
            // Wayland pairs a window with the entry whose file is its app_id;
            // X11 with StartupWMClass; both look the icon up by `Icon=`.
            let entry = text(&format!("scripts/installers/linux/{APP_ID}.desktop"));
            let lines: Vec<&str> = entry.lines().collect();
            for line in [format!("Icon={APP_ID}"), format!("StartupWMClass={APP_ID}")] {
                assert!(
                    lines.contains(&line.as_str()),
                    "baylee.desktop lacks {line}"
                );
            }
        }

        #[test]
        fn the_ico_holds_every_windows_size() {
            let ico = read("scripts/installers/windows/baylee.ico");
            assert_eq!(&ico[..4], &[0, 0, 1, 0], "an icon directory");
            let count = usize::from(u16::from_le_bytes([ico[4], ico[5]]));
            let sizes: Vec<u32> = (0..count)
                .map(|i| match ico[6 + 16 * i] {
                    0 => 256,
                    n => u32::from(n),
                })
                .collect();
            assert_eq!(sizes, [16, 24, 32, 48, 64, 128, 256]);
        }

        #[test]
        fn the_icns_holds_every_macos_size() {
            let icns = read("crates/baylee-client/assets/brand/baylee.icns");
            assert_eq!(&icns[..4], b"icns");
            let mut kinds = Vec::new();
            let mut at = 8;
            while at + 8 <= icns.len() {
                let len = u32::from_be_bytes(icns[at + 4..at + 8].try_into().unwrap()) as usize;
                kinds.push(String::from_utf8_lossy(&icns[at..at + 4]).into_owned());
                let body = &icns[at + 8..at + len];
                if body.starts_with(b"\x89PNG") {
                    assert!(
                        png(body).2,
                        "{} has no alpha: the corners would be square",
                        kinds.last().unwrap()
                    );
                }
                at += len;
            }
            // 16, 32, 64 (32@2x), 128, 256, 512, 1024 and the @2x twins.
            for kind in [
                "ic04", "ic05", "ic11", "ic12", "ic07", "ic13", "ic08", "ic14", "ic09", "ic10",
            ] {
                assert!(kinds.iter().any(|k| k == kind), "baylee.icns lacks {kind}");
            }
        }

        #[test]
        fn the_setup_puts_the_icon_and_the_taskbar_identity_on_its_shortcuts() {
            let iss = text("scripts/installers/windows/baylee.iss");
            assert!(iss.contains("SetupIconFile={#Icon}"));
            assert!(iss.contains(r"UninstallDisplayIcon={app}\baylee.ico"));
            let shortcuts: Vec<&str> = iss
                .lines()
                .filter(|line| line.starts_with("Name: \"{user"))
                .collect();
            assert_eq!(
                shortcuts.len(),
                2,
                "the Start-menu and the desktop shortcut"
            );
            for line in shortcuts {
                assert!(
                    line.contains(r#"IconFilename: "{app}\baylee.ico""#),
                    "{line}"
                );
                assert!(
                    line.contains(&format!("AppUserModelID: \"{WINDOWS_APP_USER_MODEL_ID}\"")),
                    "{line}"
                );
            }
        }
    }
}
